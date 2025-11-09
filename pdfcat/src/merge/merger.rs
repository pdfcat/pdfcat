//! Core PDF merging implementation.
//!
//! This module implements the main merge algorithm that combines
//! multiple PDF documents while preserving quality and structure.

use lopdf::{Document, Object, ObjectId};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::error::{PdfCatError, Result};
use crate::io::{LoadedPdf, PdfReader};
use crate::merge::bookmarks::BookmarkManager;
use crate::merge::metadata::MetadataManager;
use crate::merge::pages::PageExtractor;

/// Statistics about a merge operation.
#[derive(Debug, Clone)]
pub struct MergeStatistics {
    /// Number of PDFs successfully merged.
    pub files_merged: usize,

    /// Total number of pages in merged document.
    pub total_pages: usize,

    /// Total time taken for merge.
    pub merge_time: Duration,

    /// Time taken to load all PDFs.
    pub load_time: Duration,

    /// Total size of input files.
    pub input_size: u64,

    /// Number of bookmarks added.
    pub bookmarks_added: usize,

    /// Whether compression was applied.
    pub compressed: bool,
}

impl MergeStatistics {
    /// Format input size as human-readable string.
    pub fn format_input_size(&self) -> String {
        format_file_size(self.input_size)
    }
}

/// Result of a merge operation.
pub struct MergeResult {
    /// The merged PDF document.
    pub document: Document,

    /// Statistics about the merge.
    pub statistics: MergeStatistics,

    /// Paths of files that were merged.
    pub merged_files: Vec<PathBuf>,
}

/// PDF merger that combines multiple documents.
pub struct Merger {
    /// Reader for loading PDFs.
    reader: PdfReader,

    /// Page extractor for page operations.
    page_extractor: PageExtractor,

    /// Bookmark manager for outline handling.
    bookmark_manager: BookmarkManager,

    /// Metadata manager for document properties.
    metadata_manager: MetadataManager,
}

impl Merger {
    /// Create a new merger with default settings.
    pub fn new() -> Self {
        Self {
            reader: PdfReader::new(),
            page_extractor: PageExtractor::new(),
            bookmark_manager: BookmarkManager::new(),
            metadata_manager: MetadataManager::new(),
        }
    }

    /// Merge multiple PDF documents according to configuration.
    ///
    /// This is the main entry point for merging operations.
    ///
    /// # Arguments
    ///
    /// * `config` - Merge configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Input files cannot be loaded
    /// - Merge operation fails
    /// - Page extraction fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pdfcat::merge::Merger;
    /// # use pdfcat::config::Config;
    /// # async fn example(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    /// let merger = Merger::new();
    /// let result = merger.merge(&config).await?;
    /// println!("Merged {} files into {} pages",
    ///          result.statistics.files_merged,
    ///          result.statistics.total_pages);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn merge(&self, config: &Config) -> Result<MergeResult> {
        let merge_start = Instant::now();

        // Load all input PDFs
        let load_start = Instant::now();
        let (load_results, _load_stats) = self
            .reader
            .load_all(&config.inputs, config.effective_jobs())
            .await;
        let load_time = load_start.elapsed();

        // Separate successful loads from failures
        let mut loaded_pdfs = Vec::new();
        for result in load_results {
            match result {
                Ok(loaded) => loaded_pdfs.push(loaded),
                Err(e) => {
                    if config.continue_on_error {
                        eprintln!("Warning: Skipping file due to error: {}", e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        if loaded_pdfs.is_empty() {
            return Err(PdfCatError::NoFilesToMerge);
        }

        // Perform the merge
        let document = self.merge_documents(&loaded_pdfs, config).await?;

        let merge_time = merge_start.elapsed();

        // Calculate statistics
        let statistics = MergeStatistics {
            files_merged: loaded_pdfs.len(),
            total_pages: document.get_pages().len(),
            merge_time,
            load_time,
            input_size: loaded_pdfs.iter().map(|p| p.file_size).sum(),
            bookmarks_added: 0, // Updated if bookmarks are added
            compressed: config.compression != crate::config::CompressionLevel::None,
        };

        let merged_files: Vec<PathBuf> = loaded_pdfs.into_iter().map(|p| p.path).collect();

        Ok(MergeResult {
            document,
            statistics,
            merged_files,
        })
    }

    /// Merge loaded PDF documents.
    async fn merge_documents(
        &self,
        loaded_pdfs: &[LoadedPdf],
        config: &Config,
    ) -> Result<Document> {
        if loaded_pdfs.is_empty() {
            return Err(PdfCatError::NoFilesToMerge);
        }

        // Start with the first document as base
        let mut merged = loaded_pdfs[0].document.clone();
        let mut max_id = merged.max_id;

        // Process first document for page ranges
        if let Some(ref page_range) = config.page_range {
            merged = self.page_extractor.extract_pages(&merged, page_range)?;
        }

        // Apply rotation to first document if specified
        if let Some(rotation) = config.rotation {
            self.page_extractor
                .rotate_all_pages(&mut merged, rotation)?;
        }

        // Merge remaining documents
        for loaded in &loaded_pdfs[1..] {
            let mut doc = loaded.document.clone();

            // Extract pages if page range specified
            if let Some(ref page_range) = config.page_range {
                doc = self.page_extractor.extract_pages(&doc, page_range)?;
            }

            // Apply rotation if specified
            if let Some(rotation) = config.rotation {
                self.page_extractor.rotate_all_pages(&mut doc, rotation)?;
            }

            // Renumber objects to avoid ID conflicts
            doc.renumber_objects_with(max_id + 1);
            max_id = doc.max_id;

            // Get page references from the document
            let doc_pages: Vec<ObjectId> = doc.get_pages().into_values().collect();

            // Add all objects from doc to merged
            merged.objects.extend(doc.objects);

            // Update the page tree
            self.add_pages_to_tree(&mut merged, &doc_pages)?;
        }

        // Add bookmarks if requested
        if config.bookmarks {
            let file_paths = loaded_pdfs
                .iter()
                .map(|p| p.path.as_path())
                .collect::<Vec<_>>();
            self.bookmark_manager
                .add_bookmarks_for_files(&mut merged, file_paths.as_slice())?;
        }

        // Set metadata if specified
        if !config.metadata.is_empty() {
            self.metadata_manager
                .set_metadata(&mut merged, &config.metadata)?;
        }

        // Apply compression based on config
        match config.compression {
            crate::config::CompressionLevel::None => {
                // No compression
            }
            crate::config::CompressionLevel::Standard => {
                merged.compress();
            }
            crate::config::CompressionLevel::Maximum => {
                merged.compress();
                // Additional optimizations for maximum compression
                merged.prune_objects();
            }
        }

        // Always renumber for consistency
        merged.renumber_objects();

        Ok(merged)
    }

    /// Add pages to the merged document's page tree.
    fn add_pages_to_tree(&self, merged: &mut Document, page_ids: &[ObjectId]) -> Result<()> {
        // Get the catalog and pages reference
        let catalog = merged
            .catalog_mut()
            .map_err(|e| PdfCatError::merge_failed(format!("Failed to get catalog: {}", e)))?;

        let pages_id = catalog
            .get(b"Pages")
            .and_then(|p| p.as_reference())
            .map_err(|e| {
                PdfCatError::merge_failed(format!("Failed to get pages reference: {}", e))
            })?;

        // Get the pages dictionary
        let pages_dict = merged
            .get_object_mut(pages_id)
            .map_err(|e| PdfCatError::merge_failed(format!("Failed to get pages object: {}", e)))?;

        if let Object::Dictionary(dict) = pages_dict {
            // Get existing kids array
            let kids = dict
                .get_mut(b"Kids")
                .map_err(|_| PdfCatError::merge_failed("Pages dictionary missing Kids array"))?;

            if let Object::Array(kids_array) = kids {
                // Add new page references
                for &page_id in page_ids {
                    kids_array.push(Object::Reference(page_id));
                }
            } else {
                return Err(PdfCatError::merge_failed("Kids is not an array"));
            }

            // Update page count
            let current_count = dict.get(b"Count").and_then(|c| c.as_i64()).unwrap_or(0);

            let new_count = current_count + page_ids.len() as i64;
            dict.set("Count", Object::Integer(new_count));
        } else {
            return Err(PdfCatError::merge_failed(
                "Pages object is not a dictionary",
            ));
        }

        Ok(())
    }
}

impl Default for Merger {
    fn default() -> Self {
        Self::new()
    }
}

/// Format file size as human-readable string.
fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} bytes", size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CompressionLevel, Metadata, OverwriteMode};
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_pdf(dir: &TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut file = std::fs::File::create(&path).unwrap();

        let pdf_content = std::fs::read("tests/fixtures/basic.pdf").unwrap();

        file.write_all(&pdf_content).unwrap();
        path
    }

    fn create_test_config(inputs: Vec<PathBuf>, output: PathBuf) -> Config {
        Config {
            inputs,
            output,
            dry_run: false,
            verbose: false,
            overwrite_mode: OverwriteMode::Prompt,
            quiet: false,
            bookmarks: false,
            compression: CompressionLevel::Standard,
            metadata: Metadata::default(),
            continue_on_error: false,
            jobs: None,
            page_range: None,
            rotation: None,
        }
    }

    #[tokio::test]
    async fn test_merge_two_pdfs() {
        let temp_dir = TempDir::new().unwrap();
        let pdf1 = create_test_pdf(&temp_dir, "file1.pdf");
        let pdf2 = create_test_pdf(&temp_dir, "file2.pdf");
        let output = temp_dir.path().join("output.pdf");

        let config = create_test_config(vec![pdf1, pdf2], output);

        let merger = Merger::new();
        let result = merger.merge(&config).await;

        assert!(result.is_ok());
        let merge_result = result.unwrap();
        assert_eq!(merge_result.statistics.files_merged, 2);
        assert_eq!(merge_result.statistics.total_pages, 2);
    }

    #[tokio::test]
    async fn test_merge_single_pdf() {
        let temp_dir = TempDir::new().unwrap();
        let pdf = create_test_pdf(&temp_dir, "single.pdf");
        let output = temp_dir.path().join("output.pdf");

        let config = create_test_config(vec![pdf], output);

        let merger = Merger::new();
        let result = merger.merge(&config).await;

        assert!(result.is_ok());
        let merge_result = result.unwrap();
        assert_eq!(merge_result.statistics.files_merged, 1);
        assert_eq!(merge_result.statistics.total_pages, 1);
    }

    #[tokio::test]
    async fn test_merge_with_compression() {
        let temp_dir = TempDir::new().unwrap();
        let pdf1 = create_test_pdf(&temp_dir, "file1.pdf");
        let pdf2 = create_test_pdf(&temp_dir, "file2.pdf");
        let output = temp_dir.path().join("output.pdf");

        let mut config = create_test_config(vec![pdf1, pdf2], output);
        config.compression = CompressionLevel::Maximum;

        let merger = Merger::new();
        let result = merger.merge(&config).await;

        assert!(result.is_ok());
        assert!(result.unwrap().statistics.compressed);
    }

    #[tokio::test]
    async fn test_merge_no_compression() {
        let temp_dir = TempDir::new().unwrap();
        let pdf = create_test_pdf(&temp_dir, "file.pdf");
        let output = temp_dir.path().join("output.pdf");

        let mut config = create_test_config(vec![pdf], output);
        config.compression = CompressionLevel::None;

        let merger = Merger::new();
        let result = merger.merge(&config).await;

        assert!(result.is_ok());
        assert!(!result.unwrap().statistics.compressed);
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500 bytes");
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_file_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_merge_statistics() {
        let stats = MergeStatistics {
            files_merged: 3,
            total_pages: 15,
            merge_time: Duration::from_secs(2),
            load_time: Duration::from_secs(1),
            input_size: 1024 * 1024,
            bookmarks_added: 3,
            compressed: true,
        };

        assert_eq!(stats.format_input_size(), "1.00 MB");
    }
}
