use lopdf::{Document, Object, ObjectId};
use std::path::{Path, PathBuf};

use crate::Result;
// Assuming these types are defined in your crate
use crate::error::PdfCatError;
use crate::io::PdfReaderV1;
use crate::validation::Validator;

/// The result of a merge operation, which may be a DryRun report or the final Document.
#[derive(Debug, Clone)]
#[expect(clippy::large_enum_variant)]
pub enum MergeResult {
    /// Indicates a simulation was run, containing the total number of pages.
    #[allow(unused)]
    DryRun { total_pages: usize },
    /// The resulting merged PDF document.
    Document(Document),
}

/// Utility struct for merging multiple PDF documents into a single document.
pub struct PdfMerger;

impl PdfMerger {
    /// Merges a list of PDF files into a single document.
    ///
    /// The function first validates all input files and optionally performs a dry run
    /// before executing the merge.
    ///
    /// # Arguments
    ///
    /// * `paths` - A vector of paths to the PDF files to be merged, in order.
    /// * `dry_run` - If true, only calculates the total page count and returns.
    /// * `verbose` - If true, prints detailed information about each file during validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the input list is empty, if any file cannot be read, or
    /// if a structural error occurs during the PDF merging process.
    pub fn merge(paths: Vec<PathBuf>, dry_run: bool, verbose: bool) -> Result<MergeResult> {

        
        if paths.is_empty() {
            return Err(PdfCatError::NoFilesToMerge);
        }

        let mut total_pages = 0;
        let mut documents = Vec::with_capacity(paths.len());

        println!("\nValidating input files...");
        for (idx, path) in paths.iter().enumerate() {
            print!(
                "  [{}/{}] Checking: {}",
                idx + 1,
                paths.len(),
                path.display()
            );

            // We load the document here for validation and to reuse it later,
            // avoiding a second load in the non-dry-run path.
            let doc = PdfReaderV1::read(path)?;

            let page_count = doc.get_pages().len();
            println!(" ✓ ({page_count} pages)");
            total_pages += page_count;

            if verbose {
                Self::print_pdf_info(&doc, path);
            }
            documents.push(doc);
        }

        if dry_run {
            Self::execute_dry_run(&paths, total_pages)?;
            return Ok(MergeResult::DryRun { total_pages });
        }

        // Handle the single file case
        if documents.len() == 1 {
            return Ok(MergeResult::Document(documents.remove(0)));
        }

        println!("\nMerging documents...");

        // This takes ownership of the first document.
        let mut merged = documents.remove(0);

        let initial_pages = merged.get_pages().len();
        println!(
            "  [1/{}] Base: {} ({} pages)",
            paths.len(),
            paths[0].display(),
            initial_pages
        );

        let mut max_id = merged.max_id;

        for (idx, mut doc) in documents.into_iter().enumerate() {
            let original_idx = idx + 1; // since we skipped the 0th document
            let path = &paths[original_idx];

            println!(
                "  [{}/{}] Processing: {}",
                original_idx + 1,
                paths.len(),
                path.display()
            );

            // Avoid object id collisions by renumbering the incoming document
            doc.renumber_objects_with(max_id + 1);
            max_id = doc.max_id;

            let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();
            let page_count = page_ids.len();

            // Merge objects from the new document into the merged document
            merged.objects.extend(doc.objects);

            // Attach pages to the merged document's page tree structure
            Self::append_pages_to_page_tree(&mut merged, page_ids, page_count)?;

            println!("    → {page_count} pages added");
        }

        // Final cleanup for the merged document
        merged.renumber_objects();
        merged.compress();

        let final_count = merged.get_pages().len();
        println!("  Total pages: {final_count}");

        Ok(MergeResult::Document(merged))
    }

    /// Appends the given page references to the merged document's main Pages dictionary.
    ///
    /// This is the low-level, performance-critical part of the merge.
    fn append_pages_to_page_tree(
        merged: &mut Document,
        page_ids: Vec<ObjectId>,
        page_count: usize,
    ) -> Result<()> {
        let pages_id = merged.catalog_mut()?.get(b"Pages")?.as_reference()?;

        let pages_dict = merged.get_object_mut(pages_id)?.as_dict_mut()?;

        // Extend Kids[] array with new page references
        let kids_array = pages_dict.get_mut(b"Kids")?.as_array_mut()?;

        for id in page_ids {
            kids_array.push(Object::Reference(id));
        }

        // Patch Count
        let current_count = pages_dict.get(b"Count")?.as_i64()?;
        pages_dict.set(b"Count", Object::Integer(current_count + page_count as i64));

        Ok(())
    }

    /// Prints the plan for a dry run operation.
    fn execute_dry_run(paths: &[PathBuf], total_pages: usize) -> Result<()> {
        println!("\n📋 Merge plan:");

        // Reloading documents here is redundant if they were loaded in the initial loop,
        // but since the previous code did this, we keep the printing style while
        // ensuring total_pages is correct (which it is from the validation loop).

        for (idx, path) in paths.iter().enumerate() {
            // We can safely assume the load works since validation passed
            let doc = Document::load(path)?;
            let page_count = doc.get_pages().len();
            println!("  {}. {} ({} pages)", idx + 1, path.display(), page_count);
        }

        println!("\n  Total pages in merged document: {total_pages}");
        Ok(())
    }

    /// Prints verbose information about a single PDF document.
    fn print_pdf_info(doc: &Document, path: &Path) {
        println!("    File: {}", path.display());
        println!("    Version: {}", &doc.version);

        // Try to get page dimensions of the first page
        if let Some((_, page_id)) = doc.get_pages().into_iter().next()
            && let Ok(page_dict) = doc.get_object(page_id).and_then(|o| o.as_dict())
            && let Ok(mediabox) = page_dict.get(b"MediaBox").and_then(|o| o.as_array())
            && mediabox.len() >= 4
            && let (Ok(w), Ok(h)) = (mediabox[2].as_float(), mediabox[3].as_float())
        {
            println!("    Page size: {w:.1} x {h:.1} pts");
        }

        // Count total objects
        println!("    Objects: {}", doc.objects.len());
    }
}

#[cfg(test)]
mod tests {
    // Import necessary items for testing
    use super::*; // Assuming PdfMerger is in the parent scope or use 'crate::PdfMerger'
    use rstest::rstest; // For data-driven tests
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    // --- MOCKS (You would replace these with actual Document/Reader logic) ---
    // For a robust unit test, you'd mock PdfReader::read to return dummy Documents.
    // However, for integration-level testing (which PDF merging often requires),
    // we generate real, minimal PDF files using lopdf.

    // Helper function to create a minimal single-page PDF for testing
    use lopdf::{Document, Object, Stream, dictionary};
    use std::path::Path;

    /// Helper function to create a minimal PDF document with a specific number of pages
    /// using low-level lopdf structure manipulation.
    ///
    /// This ensures the generated PDFs have valid internal structures (Catalog and Pages tree)
    /// for robust merging tests.
    fn create_test_pdf(path: &Path, pages: u32) -> Result<()> {
        // 1. Initialize Document
        let mut doc = Document::with_version("1.5");
        let mut pages_kids = Vec::new(); // Holds references to the individual pages

        // 2. Define standard resource dictionary (minimal requirement)
        let resources = dictionary! {
            "ProcSet" => Object::Array(vec![
                Object::Name(b"PDF".to_vec()),
                Object::Name(b"Text".to_vec()),
            ]),
        };
        let resources_id = doc.add_object(Object::Dictionary(resources));

        // 3. Create pages
        for _ in 0..pages {
            // Create a minimal content stream (empty)
            let content = Stream::new(dictionary! {}, vec![]);
            let content_id = doc.add_object(Object::Stream(content));

            // Create a Page object dictionary
            let page_dict = dictionary! {
                "Type" => Object::Name(b"Page".to_vec()),
                // Parent ID will be updated later
                "MediaBox" => Object::Array(vec![0.into(), 0.into(), 595.0.into(), 842.0.into()]), // A4 size
                "Resources" => Object::Reference(resources_id),
                "Contents" => Object::Reference(content_id),
            };

            let page_id = doc.add_object(Object::Dictionary(page_dict));
            pages_kids.push(Object::Reference(page_id));
        }

        // 4. Create the root Pages dictionary
        let pages_dict = dictionary! {
            "Type" => Object::Name(b"Pages".to_vec()),
            "Kids" => Object::Array(pages_kids),
            "Count" => Object::Integer(pages as i64),
        };
        let pages_id = doc.add_object(Object::Dictionary(pages_dict));

        // 5. Create the Catalog
        let catalog_dict = dictionary! {
            "Type" => Object::Name(b"Catalog".to_vec()),
            "Pages" => Object::Reference(pages_id),
        };
        let catalog_id = doc.add_object(Object::Dictionary(catalog_dict));
        doc.trailer.set("Root", Object::Reference(catalog_id));

        // 6. Fix parent references in individual Page dictionaries
        // The previous loop only created the page objects, now we link them to the Pages root.
        for kid_ref in doc.get_pages().into_iter().map(|(_, id)| id) {
            if let Some(Object::Dictionary(page_dict)) = doc.objects.get_mut(&kid_ref) {
                page_dict.set("Parent", Object::Reference(pages_id));
            }
        }

        // 7. Save to file
        doc.compress(); // Good practice for saving, even in tests
        let mut file = File::create(path)?;
        doc.save_to(&mut file)?;
        file.flush()?;

        Ok(())
    }
    // ------------------------------------------------------------------------
    #[test]
    fn test_merge_two_files_success() -> Result<()> {
        let temp_dir = tempdir()?;
        let path1 = temp_dir.path().join("doc1.pdf");
        let path2 = temp_dir.path().join("doc2.pdf");

        // Arrange: Create two small files
        create_test_pdf(&path1, 2)?;
        create_test_pdf(&path2, 3)?;

        // Act
        let result = PdfMerger::merge(vec![path1, path2], false, false)?;

        // Assert
        if let MergeResult::Document(merged_doc) = result {
            // The merged document should have 2 + 3 = 5 pages
            assert_eq!(
                merged_doc.get_pages().len(),
                5,
                "Merged document must have 5 pages."
            );

            // Optional: Check if the internal object IDs were correctly renumbered
            assert!(
                merged_doc.max_id > 10,
                "Object IDs should have been renumbered."
            );
        } else {
            panic!("Merge failed, expected a Document result.");
        }

        Ok(())
    }

    #[test]
    fn test_merge_single_file_returns_document() -> Result<()> {
        let temp_dir = tempdir()?;
        let path = temp_dir.path().join("single.pdf");

        // Arrange
        create_test_pdf(&path, 4)?;

        // Act
        let result = PdfMerger::merge(vec![path], false, false)?;

        // Assert
        if let MergeResult::Document(merged_doc) = result {
            assert_eq!(
                merged_doc.get_pages().len(),
                4,
                "Single file merge should preserve page count."
            );
        } else {
            panic!("Expected Document result for single file.");
        }

        Ok(())
    }

    // Use rstest to parameterize the dry run test cases
    #[rstest(
        paths_data, expected_total_pages,
        case(vec![2, 1], 3),     // Two files
        case(vec![5], 5),        // Single file
        case(vec![1, 1, 1, 10], 13) // Multiple files
    )]
    fn test_dry_run_reports_correct_total_pages(
        paths_data: Vec<u32>,
        expected_total_pages: usize,
    ) -> Result<()> {
        let temp_dir = tempdir()?;
        let paths: Vec<PathBuf> = paths_data
            .into_iter()
            .enumerate()
            .map(|(i, pages)| {
                let path = temp_dir.path().join(format!("doc_{}.pdf", i));
                create_test_pdf(&path, pages).unwrap();
                path
            })
            .collect();

        // Act
        let result = PdfMerger::merge(paths, true, false)?;

        // Assert
        if let MergeResult::DryRun { total_pages } = result {
            assert_eq!(
                total_pages, expected_total_pages,
                "Dry run total must match expected sum."
            );
        } else {
            panic!("Expected DryRun result.");
        }

        Ok(())
    }

    #[test]
    fn test_merge_empty_paths_fails_correctly() {
        // Act
        let result = PdfMerger::merge(vec![], false, false);

        // Assert
        assert!(result.is_err(), "Merging an empty list must fail.");
        // TODO: Check the specific error kind if PdfCatError implements proper comparison
        // assert!(matches!(result.unwrap_err().downcast_ref::<PdfCatError>(), Some(PdfCatError::NoFilesToMerge)));
    }

    #[test]
    fn test_merge_non_existent_file_fails_on_validation() {
        let path = PathBuf::from("non_existent_file.pdf");

        // Act
        let result = PdfMerger::merge(vec![path], false, false);

        // Assert
        println!("{:?}", &result);
        assert!(result.is_err(), "Merging a non-existent file must fail.");
        // Ensure the error context clearly indicates the failure (from the 'with_context' in merge)
        // let error_message = result.unwrap_err().to_string();
        // assert!(
        //     error_message.contains("Failed to read PDF file"),
        //     "Error message should show read failure."
        // );
    }
}
