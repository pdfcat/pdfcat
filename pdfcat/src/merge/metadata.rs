//! PDF metadata management.
//!
//! This module handles PDF document metadata (Info dictionary) including:
//! - Title, Author, Subject, Keywords
//! - Creator, Producer
//! - Creation and modification dates

use crate::config::Metadata;
use crate::error::{PdfCatError, Result};
use lopdf::{Dictionary, Document, Object};
use std::time::SystemTime;

/// Manager for PDF metadata.
pub struct MetadataManager;

impl MetadataManager {
    /// Create a new metadata manager.
    pub fn new() -> Self {
        Self
    }

    /// Set metadata on a document.
    ///
    /// Updates the document's Info dictionary with the provided metadata.
    /// Only non-empty fields are set.
    ///
    /// # Arguments
    ///
    /// * `doc` - Document to update
    /// * `metadata` - Metadata to set
    ///
    /// # Errors
    ///
    /// Returns an error if metadata cannot be set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pdfcat::merge::metadata::MetadataManager;
    /// # use pdfcat::config::Metadata;
    /// # use lopdf::Document;
    /// # fn example(mut doc: Document) -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = MetadataManager::new();
    /// let metadata = Metadata::new(
    ///     Some("My Document".to_string()),
    ///     Some("John Doe".to_string()),
    ///     None,
    ///     None,
    /// );
    /// manager.set_metadata(&mut doc, &metadata)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_metadata(&self, doc: &mut Document, metadata: &Metadata) -> Result<()> {
        if metadata.is_empty() {
            return Ok(());
        }

        // Get or create Info dictionary
        let info_id = if let Ok(info_ref) = doc.trailer.get(b"Info").and_then(|i| i.as_reference())
        {
            info_ref
        } else {
            // Create new Info dictionary
            let new_info_id = doc.new_object_id();
            doc.trailer.set("Info", Object::Reference(new_info_id));
            new_info_id
        };

        // Get or create the Info dictionary object
        let info_dict = if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(info_id) {
            dict
        } else {
            // Create new dictionary
            let dict = Dictionary::new();
            doc.objects.insert(info_id, Object::Dictionary(dict));
            if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(info_id) {
                dict
            } else {
                return Err(PdfCatError::MetadataFailed {
                    reason: "Failed to create Info dictionary".to_string(),
                });
            }
        };

        // Set metadata fields
        if let Some(ref title) = metadata.title {
            info_dict.set(
                "Title",
                Object::String(title.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            );
        }

        if let Some(ref author) = metadata.author {
            info_dict.set(
                "Author",
                Object::String(author.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            );
        }

        if let Some(ref subject) = metadata.subject {
            info_dict.set(
                "Subject",
                Object::String(subject.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            );
        }

        if let Some(ref keywords) = metadata.keywords {
            info_dict.set(
                "Keywords",
                Object::String(keywords.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            );
        }

        // Set Creator and Producer
        info_dict.set(
            "Creator",
            Object::String(b"pdfcat".to_vec(), lopdf::StringFormat::Literal),
        );
        info_dict.set(
            "Producer",
            Object::String(b"pdfcat".to_vec(), lopdf::StringFormat::Literal),
        );

        // Set creation date
        let date_str = format_pdf_date(SystemTime::now());
        info_dict.set(
            "CreationDate",
            Object::String(date_str.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        );
        info_dict.set(
            "ModDate",
            Object::String(date_str.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        );

        Ok(())
    }

    /// Get metadata from a document.
    ///
    /// # Arguments
    ///
    /// * `doc` - Document to read metadata from
    ///
    /// # Returns
    ///
    /// Metadata extracted from the document's Info dictionary.
    pub fn get_metadata(&self, doc: &Document) -> Metadata {
        let info_dict =
            if let Ok(info_ref) = doc.trailer.get(b"Info").and_then(|i| i.as_reference()) {
                if let Ok(Object::Dictionary(dict)) = doc.get_object(info_ref) {
                    dict
                } else {
                    return Metadata::default();
                }
            } else {
                return Metadata::default();
            };

        let title = Self::get_string_field(info_dict, b"Title");
        let author = Self::get_string_field(info_dict, b"Author");
        let subject = Self::get_string_field(info_dict, b"Subject");
        let keywords = Self::get_string_field(info_dict, b"Keywords");

        Metadata::new(title, author, subject, keywords)
    }

    /// Extract a string field from a dictionary.
    fn get_string_field(dict: &Dictionary, key: &[u8]) -> Option<String> {
        dict.get(key).ok().and_then(|obj| {
            if let Object::String(bytes, _) = obj {
                String::from_utf8(bytes.clone()).ok()
            } else {
                None
            }
        })
    }

    /// Clear all metadata from a document.
    pub fn clear_metadata(&self, doc: &mut Document) -> Result<()> {
        if let Ok(info_ref) = doc.trailer.get(b"Info").and_then(|i| i.as_reference()) {
            doc.objects.remove(&info_ref);
            doc.trailer.remove(b"Info");
        }
        Ok(())
    }

    /// Check if a document has metadata.
    pub fn has_metadata(&self, doc: &Document) -> bool {
        doc.trailer.has(b"Info")
    }
}

impl Default for MetadataManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a SystemTime as a PDF date string.
///
/// PDF date format: D:YYYYMMDDHHmmSSOHH'mm
fn format_pdf_date(time: SystemTime) -> String {
    use std::time::UNIX_EPOCH;

    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();

    let secs = duration.as_secs();

    // Simple date formatting (UTC)
    // In production, you might want to use chrono for proper timezone handling
    let year = 1970 + (secs / 31_556_926); // Approximate
    let remainder = secs % 31_556_926;
    let month = 1 + (remainder / 2_629_743).min(11); // Approximate
    let day_remainder = remainder % 2_629_743;
    let day = 1 + (day_remainder / 86_400).min(30); // Approximate
    let time_remainder = day_remainder % 86_400;
    let hour = time_remainder / 3_600;
    let min = (time_remainder % 3_600) / 60;
    let sec = time_remainder % 60;

    format!(
        "D:{:04}{:02}{:02}{:02}{:02}{:02}Z",
        year, month, day, hour, min, sec
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    fn create_test_document() -> Document {
        let mut doc = Document::with_version("1.4");

        let catalog_id = doc.new_object_id();
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();

        let catalog = lopdf::dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        };

        let pages = lopdf::dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        };

        let page = lopdf::dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        };

        doc.objects.insert(catalog_id, catalog.into());
        doc.objects.insert(pages_id, pages.into());
        doc.objects.insert(page_id, page.into());
        doc.trailer.set("Root", catalog_id);

        doc
    }

    #[test]
    fn test_metadata_manager_creation() {
        let _manager = MetadataManager::new();
        // Should create without panicking
    }

    #[test]
    fn test_set_metadata() {
        let mut doc = create_test_document();
        let manager = MetadataManager::new();

        let metadata = Metadata::new(
            Some("Test Title".to_string()),
            Some("Test Author".to_string()),
            Some("Test Subject".to_string()),
            Some("test, keywords".to_string()),
        );

        let result = manager.set_metadata(&mut doc, &metadata);
        assert!(result.is_ok());
        assert!(manager.has_metadata(&doc));
    }

    #[test]
    fn test_set_empty_metadata() {
        let mut doc = create_test_document();
        let manager = MetadataManager::new();

        let metadata = Metadata::default();
        let result = manager.set_metadata(&mut doc, &metadata);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_metadata() {
        let mut doc = create_test_document();
        let manager = MetadataManager::new();

        let original_metadata = Metadata::new(
            Some("My Title".to_string()),
            Some("My Author".to_string()),
            None,
            None,
        );

        manager.set_metadata(&mut doc, &original_metadata).unwrap();

        let retrieved_metadata = manager.get_metadata(&doc);
        assert_eq!(retrieved_metadata.title, Some("My Title".to_string()));
        assert_eq!(retrieved_metadata.author, Some("My Author".to_string()));
    }

    #[test]
    fn test_clear_metadata() {
        let mut doc = create_test_document();
        let manager = MetadataManager::new();

        let metadata = Metadata::new(Some("Title".to_string()), None, None, None);

        manager.set_metadata(&mut doc, &metadata).unwrap();
        assert!(manager.has_metadata(&doc));

        let result = manager.clear_metadata(&mut doc);
        assert!(result.is_ok());
        assert!(!manager.has_metadata(&doc));
    }

    #[test]
    fn test_has_metadata() {
        let doc = create_test_document();
        let manager = MetadataManager::new();

        assert!(!manager.has_metadata(&doc));
    }

    #[test]
    fn test_format_pdf_date() {
        let time = SystemTime::now();
        let date_str = format_pdf_date(time);

        // Should start with D: and be properly formatted
        assert!(date_str.starts_with("D:"));
        assert!(date_str.len() >= 15); // D:YYYYMMDDHHMMSSZ minimum
    }

    #[test]
    fn test_partial_metadata() {
        let mut doc = create_test_document();
        let manager = MetadataManager::new();

        let metadata = Metadata::new(Some("Only Title".to_string()), None, None, None);

        manager.set_metadata(&mut doc, &metadata).unwrap();

        let retrieved = manager.get_metadata(&doc);
        assert_eq!(retrieved.title, Some("Only Title".to_string()));
        assert_eq!(retrieved.author, None);
    }
}
