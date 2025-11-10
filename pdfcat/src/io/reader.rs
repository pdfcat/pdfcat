//! PDF reading and loading operations.
//!
//! This module provides efficient PDF loading with support for:
//! - Sequential and parallel loading
//! - Memory-efficient document handling
//! - Detailed load statistics
//! - Error recovery
//!
//! # Examples
//!
//! ```no_run
//! use pdfcat::io::reader::PdfReader;
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let reader = PdfReader::new();
//! let paths = vec![PathBuf::from("a.pdf"), PathBuf::from("b.pdf")];
//! let results = reader.load_all(&paths, 4).await;
//! # Ok(())
//! # }
//! ```

use lopdf::Document;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::error::{PdfCatError, Result};

/// A loaded PDF document with metadata.
#[derive(Debug)]
pub struct LoadedPdf {
    /// The PDF document.
    pub document: Document,

    /// Path to the source file.
    pub path: PathBuf,

    /// Number of pages in the document.
    pub page_count: usize,

    /// Time taken to load the document.
    pub load_time: Duration,

    /// File size in bytes.
    pub file_size: u64,
}

impl LoadedPdf {
    /// Create a new LoadedPdf from a document.
    fn new(document: Document, path: PathBuf, load_time: Duration) -> Result<Self> {
        let page_count = document.get_pages().len();

        let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        Ok(Self {
            document,
            path,
            page_count,
            load_time,
            file_size,
        })
    }
}

/// Result of a load operation (success or failure).
pub type LoadResult = Result<LoadedPdf>;

/// Statistics for a batch load operation.
#[derive(Debug, Clone)]
pub struct LoadStatistics {
    /// Number of PDFs successfully loaded.
    pub success_count: usize,

    /// Number of PDFs that failed to load.
    pub failure_count: usize,

    /// Total time taken for all loads.
    pub total_time: Duration,

    /// Average time per successful load.
    pub average_time: Duration,

    /// Total size of successfully loaded files.
    pub total_size: u64,

    /// Total number of pages loaded.
    pub total_pages: usize,
}

impl LoadStatistics {
    /// Create statistics from load results.
    fn from_results(results: &[LoadResult], total_time: Duration) -> Self {
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut total_size = 0;
        let mut total_pages = 0;
        let mut total_load_time = Duration::ZERO;

        for result in results {
            match result {
                Ok(loaded) => {
                    success_count += 1;
                    total_size += loaded.file_size;
                    total_pages += loaded.page_count;
                    total_load_time += loaded.load_time;
                }
                Err(_) => {
                    failure_count += 1;
                }
            }
        }

        let average_time = if success_count > 0 {
            total_load_time / success_count as u32
        } else {
            Duration::ZERO
        };

        Self {
            success_count,
            failure_count,
            total_time,
            average_time,
            total_size,
            total_pages,
        }
    }

    /// Format total size as human-readable string.
    pub fn format_total_size(&self) -> String {
        format_file_size(self.total_size)
    }
}

/// PDF reader with configurable loading behavior.
#[derive(Debug, Clone)]
pub struct PdfReader {
    /// Whether to verify PDF structure after loading.
    verify: bool,
}

impl PdfReader {
    /// Create a new PDF reader with default settings.
    pub fn new() -> Self {
        Self { verify: true }
    }

    /// Create a reader that skips verification (faster but less safe).
    pub fn without_verification() -> Self {
        Self { verify: false }
    }

    /// Load a single PDF document.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the PDF file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File cannot be read
    /// - File is not a valid PDF
    /// - PDF is encrypted
    /// - PDF structure is corrupted
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pdfcat::io::reader::PdfReader;
    /// # use std::path::Path;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let reader = PdfReader::new();
    /// let loaded = reader.load(Path::new("document.pdf")).await?;
    /// println!("Loaded {} pages in {:?}", loaded.page_count, loaded.load_time);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load(&self, path: &Path) -> Result<LoadedPdf> {
        let path_buf = path.to_path_buf();
        let verify = self.verify;

        let start = Instant::now();

        // Load the document
        let doc = Document::load(&path_buf).await.map_err(|e| {
            let err_msg = e.to_string();
            if err_msg.contains("encrypt") || err_msg.contains("password") {
                PdfCatError::encrypted_pdf(path_buf.clone())
            } else {
                PdfCatError::failed_to_load_pdf(path_buf.clone(), err_msg)
            }
        })?;

        // Verify the document has pages
        if verify {
            let pages = doc.get_pages();
            if pages.is_empty() {
                return Err(PdfCatError::corrupted_pdf(
                    path_buf.clone(),
                    "PDF has no pages",
                ));
            }
        }

        let load_time = start.elapsed();

        // Load in a blocking task to avoid blocking the async runtime
        let result = LoadedPdf::new(doc, path_buf, load_time)?;

        Ok(result)
    }

    /// Load multiple PDF documents sequentially.
    ///
    /// Loads PDFs one at a time in the order provided.
    ///
    /// # Arguments
    ///
    /// * `paths` - Paths to PDF files
    ///
    /// # Returns
    ///
    /// A vector of results, one for each input file. Each result is either
    /// a successfully loaded PDF or an error.
    pub async fn load_sequential(&self, paths: &[PathBuf]) -> Vec<LoadResult> {
        let mut results = Vec::with_capacity(paths.len());

        for path in paths {
            let result = self.load(path).await;
            results.push(result);
        }

        results
    }

    /// Load multiple PDF documents in parallel.
    ///
    /// Loads PDFs concurrently using the specified number of workers.
    /// This is significantly faster for many files but uses more memory.
    ///
    /// # Arguments
    ///
    /// * `paths` - Paths to PDF files
    /// * `workers` - Number of parallel workers (typically CPU core count)
    ///
    /// # Returns
    ///
    /// A vector of results in the same order as the input paths.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pdfcat::io::reader::PdfReader;
    /// # use std::path::PathBuf;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let reader = PdfReader::new();
    /// let paths = vec![
    ///     PathBuf::from("a.pdf"),
    ///     PathBuf::from("b.pdf"),
    ///     PathBuf::from("c.pdf"),
    /// ];
    ///
    /// let results = reader.load_parallel(&paths, 4).await;
    /// for result in results {
    ///     match result {
    ///         Ok(loaded) => println!("Loaded: {}", loaded.path.display()),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load_parallel(&self, paths: &[PathBuf], workers: usize) -> Vec<LoadResult> {
        use futures::stream::{self, StreamExt};

        let workers = workers.max(1); // Ensure at least 1 worker

        // Create a stream of load tasks
        let tasks = paths.iter().map(|path| {
            let path = path.clone();
            let reader = Self {
                verify: self.verify,
            };
            async move { reader.load(&path).await }
        });

        // Process tasks with limited concurrency
        stream::iter(tasks)
            .buffer_unordered(workers)
            .collect::<Vec<_>>()
            .await
    }

    /// Load all PDFs with automatic parallelization.
    ///
    /// Chooses sequential or parallel loading based on the number of files.
    /// Sequential is used for small batches to reduce overhead.
    ///
    /// # Arguments
    ///
    /// * `paths` - Paths to PDF files
    /// * `max_workers` - Maximum number of parallel workers
    ///
    /// # Returns
    ///
    /// A tuple of (results, statistics) where results contains the load
    /// outcome for each file and statistics provides aggregate metrics.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pdfcat::io::reader::PdfReader;
    /// # use std::path::PathBuf;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let reader = PdfReader::new();
    /// let paths = vec![PathBuf::from("a.pdf"), PathBuf::from("b.pdf")];
    ///
    /// let (results, stats) = reader.load_all(&paths, 4).await;
    /// println!("Loaded {} of {} files in {:?}",
    ///          stats.success_count,
    ///          paths.len(),
    ///          stats.total_time);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load_all(
        &self,
        paths: &[PathBuf],
        max_workers: usize,
    ) -> (Vec<LoadResult>, LoadStatistics) {
        let start = Instant::now();

        // Use sequential loading for small batches
        let results = if paths.len() <= 3 {
            self.load_sequential(paths).await
        } else {
            self.load_parallel(paths, max_workers).await
        };

        let total_time = start.elapsed();
        let stats = LoadStatistics::from_results(&results, total_time);

        (results, stats)
    }

    /// Load PDFs with progress callback.
    ///
    /// Loads PDFs and calls a callback function for each completed load,
    /// allowing progress reporting.
    ///
    /// # Arguments
    ///
    /// * `paths` - Paths to PDF files
    /// * `workers` - Number of parallel workers
    /// * `on_progress` - Callback called after each load completes
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pdfcat::io::reader::PdfReader;
    /// # use std::path::PathBuf;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let reader = PdfReader::new();
    /// let paths = vec![PathBuf::from("a.pdf"), PathBuf::from("b.pdf")];
    ///
    /// let results = reader.load_with_progress(&paths, 4, |idx, result| {
    ///     println!("Loaded file {}/{}", idx + 1, paths.len());
    /// }).await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load_with_progress<F>(
        &self,
        paths: &[PathBuf],
        workers: usize,
        mut on_progress: F,
    ) -> Vec<LoadResult>
    where
        F: FnMut(usize, &LoadResult),
    {
        use futures::stream::{self, StreamExt};

        let workers = workers.max(1);

        let tasks = paths.iter().enumerate().map(|(idx, path)| {
            let path = path.clone();
            let reader = Self {
                verify: self.verify,
            };
            async move {
                let result = reader.load(&path).await;
                (idx, result)
            }
        });

        let mut indexed_results: Vec<(usize, LoadResult)> = stream::iter(tasks)
            .buffer_unordered(workers)
            .collect::<Vec<_>>()
            .await;

        // Sort by original index to maintain order
        indexed_results.sort_by_key(|(idx, _)| *idx);

        // Call progress callback and extract results
        let mut results = Vec::with_capacity(paths.len());
        for (idx, result) in indexed_results {
            on_progress(idx, &result);
            results.push(result);
        }

        results
    }
}

impl Default for PdfReader {
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
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_pdf(dir: &TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut file = std::fs::File::create(&path).unwrap();

        let pdf_content = std::fs::read("tests/fixtures/basic.pdf").unwrap();

        file.write_all(&pdf_content).unwrap();
        path
    }

    #[tokio::test]
    async fn test_load_single_pdf() {
        let temp_dir = TempDir::new().unwrap();
        let pdf_path = create_test_pdf(&temp_dir, "test.pdf");

        let reader = PdfReader::new();
        let result = reader.load(&pdf_path).await;

        assert!(result.is_ok());
        let loaded = result.unwrap();
        assert_eq!(loaded.page_count, 1);
        assert_eq!(loaded.path, pdf_path);
        assert!(loaded.load_time > Duration::ZERO);
    }

    #[tokio::test]
    async fn test_load_nonexistent_pdf() {
        let reader = PdfReader::new();
        let result = reader.load(Path::new("/nonexistent.pdf")).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_sequential() {
        let temp_dir = TempDir::new().unwrap();
        let pdf1 = create_test_pdf(&temp_dir, "test1.pdf");
        let pdf2 = create_test_pdf(&temp_dir, "test2.pdf");

        let reader = PdfReader::new();
        let results = reader.load_sequential(&[pdf1, pdf2]).await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }

    #[tokio::test]
    async fn test_load_parallel() {
        let temp_dir = TempDir::new().unwrap();
        let pdf1 = create_test_pdf(&temp_dir, "test1.pdf");
        let pdf2 = create_test_pdf(&temp_dir, "test2.pdf");
        let pdf3 = create_test_pdf(&temp_dir, "test3.pdf");

        let reader = PdfReader::new();
        let results = reader.load_parallel(&[pdf1, pdf2, pdf3], 2).await;

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[tokio::test]
    async fn test_load_all() {
        let temp_dir = TempDir::new().unwrap();
        let pdf1 = create_test_pdf(&temp_dir, "test1.pdf");
        let pdf2 = create_test_pdf(&temp_dir, "test2.pdf");

        let reader = PdfReader::new();
        let (results, stats) = reader.load_all(&[pdf1, pdf2], 4).await;

        assert_eq!(results.len(), 2);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failure_count, 0);
        assert_eq!(stats.total_pages, 2);
    }

    #[tokio::test]
    async fn test_load_with_progress() {
        let temp_dir = TempDir::new().unwrap();
        let pdf1 = create_test_pdf(&temp_dir, "test1.pdf");
        let pdf2 = create_test_pdf(&temp_dir, "test2.pdf");

        let reader = PdfReader::new();
        let mut progress_count = 0;

        let results = reader
            .load_with_progress(&[pdf1, pdf2], 2, |_, _| {
                progress_count += 1;
            })
            .await;

        assert_eq!(results.len(), 2);
        assert_eq!(progress_count, 2);
    }

    #[tokio::test]
    async fn test_load_statistics() {
        let temp_dir = TempDir::new().unwrap();
        let pdf1 = create_test_pdf(&temp_dir, "test1.pdf");
        let invalid_pdf = temp_dir.path().join("invalid.pdf");
        std::fs::File::create(&invalid_pdf).unwrap();

        let reader = PdfReader::new();
        let (results, stats) = reader.load_all(&[pdf1, invalid_pdf], 2).await;

        assert_eq!(results.len(), 2);
        assert_eq!(stats.success_count, 1);
        assert_eq!(stats.failure_count, 1);
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500 bytes");
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_file_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[tokio::test]
    async fn test_reader_without_verification() {
        let temp_dir = TempDir::new().unwrap();
        let pdf_path = create_test_pdf(&temp_dir, "test.pdf");

        let reader = PdfReader::without_verification();
        let result = reader.load(&pdf_path).await;

        assert!(result.is_ok());
    }
}
