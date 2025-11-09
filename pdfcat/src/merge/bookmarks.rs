//! Bookmark (outline) management for PDFs.
//!
//! This module handles PDF bookmarks/outlines, allowing creation of
//! navigational structure in merged documents.

use crate::error::{PdfCatError, Result};
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::path::Path;

/// Manager for PDF bookmarks (outlines).
pub struct BookmarkManager;

impl BookmarkManager {
    /// Create a new bookmark manager.
    pub fn new() -> Self {
        Self
    }

    /// Add bookmarks for each merged file.
    ///
    /// Creates a bookmark at the start of each input PDF using
    /// the filename as the bookmark title.
    ///
    /// # Arguments
    ///
    /// * `doc` - Document to add bookmarks to
    /// * `file_paths` - Paths of the files that were merged
    ///
    /// # Errors
    ///
    /// Returns an error if bookmark creation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pdfcat::merge::bookmarks::BookmarkManager;
    /// # use lopdf::Document;
    /// # use std::path::Path;
    /// # fn example(mut doc: Document) -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = BookmarkManager::new();
    /// let paths = vec![Path::new("file1.pdf"), Path::new("file2.pdf")];
    /// manager.add_bookmarks_for_files(&mut doc, &paths)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_bookmarks_for_files(&self, doc: &mut Document, file_paths: &[&Path]) -> Result<()> {
        if file_paths.is_empty() {
            return Ok(());
        }

        // Get page IDs for bookmark destinations
        let pages: Vec<(u32, ObjectId)> = doc.get_pages().into_iter().collect();

        if pages.is_empty() {
            return Ok(());
        }

        // Create outline entries
        let mut outline_items = Vec::new();

        // Calculate pages per file (simple distribution)
        let pages_per_file = if file_paths.len() > 1 {
            pages.len() / file_paths.len()
        } else {
            pages.len()
        };

        for (file_idx, path) in file_paths.iter().enumerate() {
            let page_idx = file_idx * pages_per_file;

            if page_idx >= pages.len() {
                break;
            }

            let title = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");

            let page_id = pages[page_idx].1;

            outline_items.push((title.to_string(), page_id));
        }

        if outline_items.is_empty() {
            return Ok(());
        }

        // Create the outline structure
        self.create_outline_structure(doc, &outline_items)?;

        Ok(())
    }

    /// Create the PDF outline structure.
    fn create_outline_structure(
        &self,
        doc: &mut Document,
        items: &[(String, ObjectId)],
    ) -> Result<()> {
        // Create outline dictionary (root)
        let outline_id = doc.new_object_id();

        // Create outline items
        let mut item_ids = Vec::new();
        for (title, page_id) in items {
            let item_id = doc.new_object_id();
            item_ids.push(item_id);

            // Create destination array [page /XYZ null null null]
            let dest = vec![
                Object::Reference(*page_id),
                Object::Name(b"XYZ".to_vec()),
                Object::Null,
                Object::Null,
                Object::Null,
            ];

            let mut item_dict = Dictionary::new();
            item_dict.set(
                "Title",
                Object::String(title.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            );
            item_dict.set("Parent", Object::Reference(outline_id));
            item_dict.set("Dest", Object::Array(dest));

            doc.objects.insert(item_id, Object::Dictionary(item_dict));
        }

        // Link items together (Prev/Next)
        for i in 0..item_ids.len() {
            if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(item_ids[i]) {
                if i > 0 {
                    dict.set("Prev", Object::Reference(item_ids[i - 1]));
                }
                if i < item_ids.len() - 1 {
                    dict.set("Next", Object::Reference(item_ids[i + 1]));
                }
            }
        }

        // Create root outline dictionary
        let mut outline_dict = Dictionary::new();
        outline_dict.set("Type", Object::Name(b"Outlines".to_vec()));
        outline_dict.set("Count", Object::Integer(item_ids.len() as i64));

        if !item_ids.is_empty() {
            outline_dict.set("First", Object::Reference(item_ids[0]));
            outline_dict.set("Last", Object::Reference(*item_ids.last().unwrap()));
        }

        doc.objects
            .insert(outline_id, Object::Dictionary(outline_dict));

        // Add outline to catalog
        if let Ok(catalog) = doc.catalog_mut() {
            catalog.set("Outlines", Object::Reference(outline_id));
        } else {
            return Err(PdfCatError::BookmarkFailed {
                path: std::path::PathBuf::from("document"),
                reason: "Failed to get catalog".to_string(),
            });
        }

        Ok(())
    }

    /// Check if a document has bookmarks.
    pub fn has_bookmarks(&self, doc: &Document) -> bool {
        if let Ok(catalog) = doc.catalog() {
            catalog.has(b"Outlines")
        } else {
            false
        }
    }

    /// Remove all bookmarks from a document.
    pub fn remove_bookmarks(&self, doc: &mut Document) -> Result<()> {
        if let Ok(catalog) = doc.catalog_mut() {
            catalog.remove(b"Outlines");
        }
        Ok(())
    }
}

impl Default for BookmarkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;
    use std::path::PathBuf;

    fn create_test_document_with_pages(page_count: usize) -> Document {
        let mut doc = Document::with_version("1.4");

        let catalog_id = doc.new_object_id();
        let pages_id = doc.new_object_id();

        let mut page_ids = Vec::new();
        for _ in 0..page_count {
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
            "Count" => page_count as i64,
        };

        doc.objects.insert(catalog_id, catalog.into());
        doc.objects.insert(pages_id, pages_dict.into());
        doc.trailer.set("Root", catalog_id);

        doc
    }

    #[test]
    fn test_bookmark_manager_creation() {
        let _manager = BookmarkManager::new();
        // Should create without panicking
    }

    #[test]
    fn test_add_bookmarks_empty() {
        let mut doc = create_test_document_with_pages(5);
        let manager = BookmarkManager::new();

        let result = manager.add_bookmarks_for_files(&mut doc, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_bookmarks_single_file() {
        let mut doc = create_test_document_with_pages(5);
        let manager = BookmarkManager::new();

        let path = PathBuf::from("test.pdf");
        let paths = vec![path.as_path()];

        let result = manager.add_bookmarks_for_files(&mut doc, &paths);
        assert!(result.is_ok());
        assert!(manager.has_bookmarks(&doc));
    }

    #[test]
    fn test_add_bookmarks_multiple_files() {
        let mut doc = create_test_document_with_pages(10);
        let manager = BookmarkManager::new();

        let paths = vec![
            PathBuf::from("file1.pdf"),
            PathBuf::from("file2.pdf"),
            PathBuf::from("file3.pdf"),
        ];
        let path_refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();

        let result = manager.add_bookmarks_for_files(&mut doc, &path_refs);
        assert!(result.is_ok());
        assert!(manager.has_bookmarks(&doc));
    }

    #[test]
    fn test_has_bookmarks() {
        let doc = create_test_document_with_pages(5);
        let manager = BookmarkManager::new();

        assert!(!manager.has_bookmarks(&doc));
    }

    #[test]
    fn test_remove_bookmarks() {
        let mut doc = create_test_document_with_pages(5);
        let manager = BookmarkManager::new();

        let path = PathBuf::from("test.pdf");
        let paths = vec![path.as_path()];

        manager.add_bookmarks_for_files(&mut doc, &paths).unwrap();
        assert!(manager.has_bookmarks(&doc));

        let result = manager.remove_bookmarks(&mut doc);
        assert!(result.is_ok());
        assert!(!manager.has_bookmarks(&doc));
    }
}
