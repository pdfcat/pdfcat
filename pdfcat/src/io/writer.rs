//! PDF writing and saving operations.
//!
//! This module provides safe and efficient PDF writing with:
//! - Atomic writes (write to temp file, then rename)
//! - Compression support
//! - File permission handling
//! - Overwrite protection
//! - Write statistics
//!
//! # Examples
//!
//! ```no_run
//! use pdfcat::io::writer::PdfWriter;
//! use lopdf::Document;
//! use std::path::Path;
//!
//! # async fn example(doc: Document) -> Result<(), Box<dyn std::error::Error>> {
//! let writer = PdfWriter::new();
//! writer.save(&doc, Path::new("output.pdf")).await?;
//! # Ok(())
//! # }
//! ```

use lopdf::Document;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::task;

use crate::error::{PdfCatError, Result};

/// Options for writing PDF files.
#[derive(Debug, Clone)]
pub struct WriteOptions {
    /// Use atomic writes (write to temp file, then rename).
    pub atomic: bool,

    /// Compress the PDF before writing.
    pub compress: bool,

    /// Optimize the PDF structure.
    pub optimize: bool,

    /// Buffer size for writing (in bytes).
    pub buffer_size: usize,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            atomic: true,
            compress: true,
            optimize: true,
            buffer_size: 8192,
        }
    }
}

/// Statistics about a write operation.
#[derive(Debug, Clone)]
pub struct WriteStatistics {
    /// Time taken to write the file.
    pub write_time: Duration,

    /// Size of the written file in bytes.
    pub file_size: u64,

    /// Path where the file was written.
    pub output_path: PathBuf,

    /// Whether compression was applied.
    pub compressed: bool,

    /// Whether optimization was applied.
    pub optimized: bool,
}

impl WriteStatistics {
    /// Format file size as human-readable string.
    pub fn format_file_size(&self) -> String {
        format_file_size(self.file_size)
    }
}

/// PDF writer with configurable behavior.
pub struct PdfWriter {
    options: WriteOptions,
}

impl PdfWriter {
    /// Create a new PDF writer with default options.
    pub fn new() -> Self {
        Self {
            options: WriteOptions::default(),
        }
    }

    /// Create a writer with custom options.
    pub fn with_options(options: WriteOptions) -> Self {
        Self { options }
    }

    /// Create a writer without atomic writes (faster but less safe).
    pub fn non_atomic() -> Self {
        Self {
            options: WriteOptions {
                atomic: false,
                ..Default::default()
            },
        }
    }

    /// Create a writer without compression (faster but larger files).
    pub fn without_compression() -> Self {
        Self {
            options: WriteOptions {
                compress: false,
                ..Default::default()
            },
        }
    }

    /// Save a PDF document to a file.
    ///
    /// # Arguments
    ///
    /// * `doc` - PDF document to save
    /// * `path` - Output file path
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Output directory doesn't exist
    /// - Insufficient permissions
    /// - Disk full
    /// - Write operation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pdfcat::io::writer::PdfWriter;
    /// # use lopdf::Document;
    /// # use std::path::Path;
    /// # async fn example(doc: Document) -> Result<(), Box<dyn std::error::Error>> {
    /// let writer = PdfWriter::new();
    /// writer.save(&doc, Path::new("output.pdf")).await?;
    /// println!("PDF saved successfully");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn save(&self, doc: &Document, path: &Path) -> Result<()> {
        let _stats = self.save_with_stats(doc, path).await?;
        Ok(())
    }

    /// Save a PDF and return statistics about the operation.
    ///
    /// # Arguments
    ///
    /// * `doc` - PDF document to save
    /// * `path` - Output file path
    ///
    /// # Returns
    ///
    /// Statistics about the write operation including time and file size.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pdfcat::io::writer::PdfWriter;
    /// # use lopdf::Document;
    /// # use std::path::Path;
    /// # async fn example(doc: Document) -> Result<(), Box<dyn std::error::Error>> {
    /// let writer = PdfWriter::new();
    /// let stats = writer.save_with_stats(&doc, Path::new("output.pdf")).await?;
    /// println!("Wrote {} in {:?}", stats.format_file_size(), stats.write_time);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn save_with_stats(&self, doc: &Document, path: &Path) -> Result<WriteStatistics> {
        let path_buf = path.to_path_buf();
        let options = self.options.clone();

        // Clone the document for processing in blocking task
        let mut doc_clone = doc.clone();

        let stats = task::spawn_blocking(move || {
            let start = Instant::now();

            // Apply optimizations
            if options.compress {
                doc_clone.compress();
            }

            if options.optimize {
                doc_clone.renumber_objects();
            }

            // Determine write path (temp or final)
            let write_path = if options.atomic {
                // Write to temp file first

                path_buf.with_extension("tmp")
            } else {
                path_buf.clone()
            };

            // Create the file
            let file = std::fs::File::create(&write_path).map_err(|e| {
                PdfCatError::FailedToCreateOutput {
                    path: write_path.clone(),
                    source: e,
                }
            })?;

            // Write with buffering
            let mut writer = std::io::BufWriter::with_capacity(options.buffer_size, file);

            doc_clone
                .save_to(&mut writer)
                .map_err(|e| PdfCatError::FailedToWrite {
                    path: write_path.clone(),
                    source: std::io::Error::other(e),
                })?;

            writer.flush().map_err(|e| PdfCatError::FailedToWrite {
                path: write_path.clone(),
                source: e,
            })?;

            // Atomic rename if needed
            if options.atomic {
                std::fs::rename(&write_path, &path_buf).map_err(|e| {
                    PdfCatError::FailedToWrite {
                        path: path_buf.clone(),
                        source: e,
                    }
                })?;
            }

            let write_time = start.elapsed();

            // Get file size
            let file_size = std::fs::metadata(&path_buf).map(|m| m.len()).unwrap_or(0);

            Ok::<_, PdfCatError>(WriteStatistics {
                write_time,
                file_size,
                output_path: path_buf,
                compressed: options.compress,
                optimized: options.optimize,
            })
        })
        .await
        .map_err(|e| PdfCatError::other(format!("Write task failed: {e}")))??;

        Ok(stats)
    }

    /// Check if a file can be written to the given path.
    ///
    /// Performs pre-flight checks without actually writing.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to check
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Parent directory doesn't exist
    /// - Parent directory is not writable
    pub async fn can_write(&self, path: &Path) -> Result<()> {
        // Check parent directory exists and is writable
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(PdfCatError::invalid_config(format!(
                    "Output directory does not exist: {}",
                    parent.display()
                )));
            }

            let metadata =
                tokio::fs::metadata(parent)
                    .await
                    .map_err(|e| PdfCatError::FileNotAccessible {
                        path: parent.to_path_buf(),
                        source: e,
                    })?;

            if metadata.permissions().readonly() {
                return Err(PdfCatError::invalid_config(format!(
                    "Output directory is not writable: {}",
                    parent.display()
                )));
            }
        }

        Ok(())
    }

    /// Check if output file exists.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to check
    ///
    /// # Returns
    ///
    /// True if the file exists, false otherwise.
    pub async fn exists(&self, path: &Path) -> bool {
        tokio::fs::metadata(path).await.is_ok()
    }

    /// Safely remove an output file if it exists.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to remove
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be removed.
    pub async fn remove_if_exists(&self, path: &Path) -> Result<()> {
        if self.exists(path).await {
            tokio::fs::remove_file(path)
                .await
                .map_err(|e| PdfCatError::FailedToWrite {
                    path: path.to_path_buf(),
                    source: e,
                })?;
        }
        Ok(())
    }
}

impl Default for PdfWriter {
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
        format!("{size} bytes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;
    use tempfile::TempDir;

    fn create_test_document() -> Document {
        let mut doc = Document::with_version("1.4");

        // Add minimal structure for a valid PDF
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

    #[tokio::test]
    async fn test_save_pdf() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.pdf");

        let doc = create_test_document();
        let writer = PdfWriter::new();

        let result = writer.save(&doc, &output_path).await;
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[tokio::test]
    async fn test_save_with_stats() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.pdf");

        let doc = create_test_document();
        let writer = PdfWriter::new();

        let stats = writer.save_with_stats(&doc, &output_path).await.unwrap();

        assert!(stats.write_time > Duration::ZERO);
        assert!(stats.file_size > 0);
        assert_eq!(stats.output_path, output_path);
        assert!(stats.compressed);
        assert!(stats.optimized);
    }

    #[tokio::test]
    async fn test_non_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.pdf");

        let doc = create_test_document();
        let writer = PdfWriter::non_atomic();

        let result = writer.save(&doc, &output_path).await;
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[tokio::test]
    async fn test_without_compression() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.pdf");

        let doc = create_test_document();
        let writer = PdfWriter::without_compression();

        let stats = writer.save_with_stats(&doc, &output_path).await.unwrap();
        assert!(!stats.compressed);
    }

    #[tokio::test]
    async fn test_can_write() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.pdf");

        let writer = PdfWriter::new();
        let result = writer.can_write(&output_path).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_can_write_nonexistent_directory() {
        let writer = PdfWriter::new();
        let result = writer.can_write(Path::new("/nonexistent/output.pdf")).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_exists() {
        let temp_dir = TempDir::new().unwrap();
        let existing_path = temp_dir.path().join("existing.pdf");
        std::fs::File::create(&existing_path).unwrap();

        let writer = PdfWriter::new();

        assert!(writer.exists(&existing_path).await);
        assert!(
            !writer
                .exists(&temp_dir.path().join("nonexistent.pdf"))
                .await
        );
    }

    #[tokio::test]
    async fn test_remove_if_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.pdf");
        std::fs::File::create(&file_path).unwrap();

        let writer = PdfWriter::new();

        assert!(file_path.exists());
        writer.remove_if_exists(&file_path).await.unwrap();
        assert!(!file_path.exists());

        // Should not error on non-existent file
        let result = writer.remove_if_exists(&file_path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_custom_options() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.pdf");

        let options = WriteOptions {
            atomic: false,
            compress: false,
            optimize: false,
            buffer_size: 4096,
        };

        let doc = create_test_document();
        let writer = PdfWriter::with_options(options);

        let stats = writer.save_with_stats(&doc, &output_path).await.unwrap();
        assert!(!stats.compressed);
        assert!(!stats.optimized);
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(100), "100 bytes");
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_file_size(1536 * 1024), "1.50 MB");
    }
}
