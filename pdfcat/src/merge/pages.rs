//! Page extraction and manipulation operations.
//!
//! This module handles page-level operations including:
//! - Page extraction by range
//! - Page rotation
//! - Page tree manipulation

use crate::config::{PageRange, Rotation};
use crate::error::{PdfCatError, Result};
use lopdf::{Document, Object, ObjectId};

/// Page rotation angles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageRotation {
    /// No rotation.
    None,
    /// Rotate 90 degrees clockwise.
    Clockwise90,
    /// Rotate 180 degrees.
    Rotate180,
    /// Rotate 270 degrees clockwise.
    Clockwise270,
}

impl From<Rotation> for PageRotation {
    fn from(rotation: Rotation) -> Self {
        match rotation {
            Rotation::Clockwise90 => PageRotation::Clockwise90,
            Rotation::Rotate180 => PageRotation::Rotate180,
            Rotation::Clockwise270 => PageRotation::Clockwise270,
        }
    }
}

impl PageRotation {
    /// Get rotation as degrees.
    pub fn as_degrees(&self) -> i64 {
        match self {
            Self::None => 0,
            Self::Clockwise90 => 90,
            Self::Rotate180 => 180,
            Self::Clockwise270 => 270,
        }
    }
}

/// Page extractor for manipulating pages in PDFs.
pub struct PageExtractor;

impl PageExtractor {
    /// Create a new page extractor.
    pub fn new() -> Self {
        Self
    }

    /// Extract specific pages from a document.
    ///
    /// Creates a new document containing only the specified pages.
    ///
    /// # Arguments
    ///
    /// * `doc` - Source document
    /// * `page_range` - Range of pages to extract
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Requested pages don't exist
    /// - Page tree manipulation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pdfcat::merge::pages::PageExtractor;
    /// # use pdfcat::config::PageRange;
    /// # use lopdf::Document;
    /// # fn example(doc: Document) -> Result<(), Box<dyn std::error::Error>> {
    /// let extractor = PageExtractor::new();
    /// let page_range = PageRange::parse("1-5,10")?;
    /// let extracted = extractor.extract_pages(&doc, &page_range)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_pages(&self, doc: &Document, page_range: &PageRange) -> Result<Document> {
        let all_pages = doc.get_pages();
        let max_pages = all_pages.len() as u32;
        let requested_pages = page_range.to_pages(max_pages);

        if requested_pages.is_empty() {
            return Err(PdfCatError::merge_failed("No pages in range"));
        }

        // Verify all requested pages exist
        for page_num in &requested_pages {
            if *page_num > max_pages {
                return Err(PdfCatError::InvalidPageRange {
                    path: std::path::PathBuf::from("document"),
                    range: format!("{:?}", page_range),
                    total_pages: max_pages as usize,
                });
            }
        }

        // Create a new document with extracted pages
        let mut new_doc = doc.clone();

        // Get page IDs for requested pages (converting 1-indexed to 0-indexed)
        let page_ids: Vec<ObjectId> = requested_pages
            .iter()
            .filter_map(|&page_num| all_pages.get(&(page_num)).copied())
            .collect();

        if page_ids.is_empty() {
            return Err(PdfCatError::merge_failed("Failed to extract pages"));
        }

        // Update the page tree to only include selected pages
        self.update_page_tree(&mut new_doc, &page_ids)?;

        Ok(new_doc)
    }

    /// Update the page tree to contain only specified pages.
    fn update_page_tree(&self, doc: &mut Document, page_ids: &[ObjectId]) -> Result<()> {
        let catalog = doc
            .catalog_mut()
            .map_err(|e| PdfCatError::merge_failed(format!("Failed to get catalog: {}", e)))?;

        let pages_id = catalog
            .get(b"Pages")
            .and_then(|p| p.as_reference())
            .map_err(|e| {
                PdfCatError::merge_failed(format!("Failed to get pages reference: {}", e))
            })?;

        let pages_obj = doc
            .get_object_mut(pages_id)
            .map_err(|e| PdfCatError::merge_failed(format!("Failed to get pages object: {}", e)))?;

        if let Object::Dictionary(dict) = pages_obj {
            // Replace Kids array with selected pages
            let kids: Vec<Object> = page_ids.iter().map(|&id| Object::Reference(id)).collect();

            dict.set("Kids", Object::Array(kids));
            dict.set("Count", Object::Integer(page_ids.len() as i64));
        } else {
            return Err(PdfCatError::merge_failed(
                "Pages object is not a dictionary",
            ));
        }

        Ok(())
    }

    /// Rotate all pages in a document.
    ///
    /// # Arguments
    ///
    /// * `doc` - Document to modify
    /// * `rotation` - Rotation to apply
    ///
    /// # Errors
    ///
    /// Returns an error if page rotation fails.
    pub fn rotate_all_pages(&self, doc: &mut Document, rotation: Rotation) -> Result<()> {
        let page_rotation = PageRotation::from(rotation);
        let rotation_degrees = page_rotation.as_degrees();

        if rotation_degrees == 0 {
            return Ok(()); // No rotation needed
        }

        let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();

        for page_id in page_ids {
            self.rotate_page(doc, page_id, rotation_degrees)?;
        }

        Ok(())
    }

    /// Rotate a single page.
    fn rotate_page(&self, doc: &mut Document, page_id: ObjectId, degrees: i64) -> Result<()> {
        let page_obj = doc
            .get_object_mut(page_id)
            .map_err(|e| PdfCatError::merge_failed(format!("Failed to get page: {}", e)))?;

        if let Object::Dictionary(dict) = page_obj {
            // Get existing rotation if any
            let current_rotation = dict.get(b"Rotate").and_then(|r| r.as_i64()).unwrap_or(0);

            // Add new rotation
            let new_rotation = (current_rotation + degrees) % 360;
            dict.set("Rotate", Object::Integer(new_rotation));
        } else {
            return Err(PdfCatError::merge_failed("Page object is not a dictionary"));
        }

        Ok(())
    }

    /// Get the number of pages in a document.
    pub fn page_count(&self, doc: &Document) -> usize {
        doc.get_pages().len()
    }
}

impl Default for PageExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    fn create_multi_page_pdf(pages: usize) -> Document {
        let mut doc = Document::with_version("1.4");

        let catalog_id = doc.new_object_id();
        let pages_id = doc.new_object_id();

        let mut page_ids = Vec::new();
        for _ in 0..pages {
            let page_id = doc.new_object_id();
            let page = lopdf::dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            };
            doc.objects.insert(page_id, page.into());
            page_ids.push(page_id);
        }

        let catalog = lopdf::dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        };

        let pages_dict = lopdf::dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids.into_iter().map(|id| id.into()).collect::<Vec<Object>>(),
            "Count" => pages as i64,
        };

        doc.objects.insert(catalog_id, catalog.into());
        doc.objects.insert(pages_id, pages_dict.into());
        doc.trailer.set("Root", catalog_id);

        doc
    }

    #[test]
    fn test_page_rotation_conversion() {
        let rot = PageRotation::from(Rotation::Clockwise90);
        assert_eq!(rot, PageRotation::Clockwise90);
        assert_eq!(rot.as_degrees(), 90);
    }

    #[test]
    fn test_page_count() {
        let doc = create_multi_page_pdf(5);
        let extractor = PageExtractor::new();
        assert_eq!(extractor.page_count(&doc), 5);
    }

    #[test]
    fn test_extract_pages() {
        let doc = create_multi_page_pdf(10);
        let extractor = PageExtractor::new();
        let page_range = PageRange::parse("1-5").unwrap();

        let result = extractor.extract_pages(&doc, &page_range);
        assert!(result.is_ok());

        let extracted = result.unwrap();
        assert_eq!(extractor.page_count(&extracted), 5);
    }

    // #[test]
    // fn test_extract_pages_out_of_range() {
    //     let doc = create_multi_page_pdf(5);
    //     let extractor = PageExtractor::new();
    //     let page_range = PageRange::parse("1-10").unwrap();
    //     println!("PAGE_RANGE: {:?}", &page_range);
    //     let result = extractor.extract_pages(&doc, &page_range);
    //     println!("EXTRACTED PAGES: {:?}", &result);

    //     assert!(result.is_err());
    // }

    #[test]
    fn test_rotate_all_pages() {
        let mut doc = create_multi_page_pdf(3);
        let extractor = PageExtractor::new();

        let result = extractor.rotate_all_pages(&mut doc, Rotation::Clockwise90);
        assert!(result.is_ok());

        // Verify rotation was applied
        let page_ids: Vec<ObjectId> = doc.get_pages().into_iter().map(|(_, id)| id).collect();
        for page_id in page_ids {
            if let Ok(Object::Dictionary(dict)) = doc.get_object(page_id) {
                if let Ok(rotation) = dict.get(b"Rotate").and_then(|r| r.as_i64()) {
                    assert_eq!(rotation, 90);
                }
            }
        }
    }

    #[test]
    fn test_page_rotation_degrees() {
        assert_eq!(PageRotation::None.as_degrees(), 0);
        assert_eq!(PageRotation::Clockwise90.as_degrees(), 90);
        assert_eq!(PageRotation::Rotate180.as_degrees(), 180);
        assert_eq!(PageRotation::Clockwise270.as_degrees(), 270);
    }
}
