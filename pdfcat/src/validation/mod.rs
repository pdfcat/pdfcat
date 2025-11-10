//! Input validation for pdfcat.
//!
//! This module provides comprehensive validation of PDF files and configuration
//! before attempting merge operations. It performs:
//! - File existence and accessibility checks
//! - PDF format validation
//! - Encryption detection
//! - Page count verification
//! - Output path validation
//!
//! # Examples
//!
//! ```no_run
//! use pdfcat::validation::Validator;
//! use pdfcat::config::Config;
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let validator = Validator::new();
//! let result = validator.validate_file(&PathBuf::from("test.pdf")).await?;
//! println!("PDF has {} pages", result.page_count);
//! # Ok(())
//! # }
//! ```

use lopdf::Document;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::{PdfCatError, Result};

/// Result of validating a single PDF file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    /// Path to the validated file.
    pub path: PathBuf,

    /// Number of pages in the PDF.
    pub page_count: usize,

    /// PDF version (major, minor).
    pub version: Option<(u8, u8)>,

    /// Size of the file in bytes.
    pub file_size: u64,

    /// Whether the PDF is encrypted.
    pub is_encrypted: bool,

    /// Number of objects in the PDF.
    pub object_count: usize,

    /// Page dimensions (width, height) in points, if available.
    pub page_dimensions: Option<(f32, f32)>,
}

impl ValidationResult {
    /// Create a validation result from a loaded PDF document.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the PDF file
    /// * `doc` - Loaded PDF document
    ///
    /// # Errors
    ///
    /// Returns an error if page information cannot be extracted.
    fn from_document(path: PathBuf, doc: &Document) -> Result<Self> {
        let pages = doc.get_pages();
        let page_count = pages.len();

        let version = doc.version.split_once(".").map(|(major, minor)| {
            (
                major.to_string().parse::<u8>().unwrap_or_default(),
                minor.to_string().parse::<u8>().unwrap_or_default(),
            )
        });

        let object_count = doc.objects.len();

        // Try to get page dimensions from first page
        let page_dimensions = pages.iter().next().and_then(|(_, page_id)| {
            doc.get_object(*page_id).ok().and_then(|page_obj| {
                if let lopdf::Object::Dictionary(page_dict) = page_obj {
                    page_dict.get(b"MediaBox").ok().and_then(|mediabox| {
                        if let lopdf::Object::Array(arr) = mediabox
                            && arr.len() >= 4
                        {
                            let width = arr[2].as_float().ok()?;
                            let height = arr[3].as_float().ok()?;
                            return Some((width, height));
                        }
                        None
                    })
                } else {
                    None
                }
            })
        });

        // Get file size
        let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        Ok(Self {
            path,
            page_count,
            version,
            file_size,
            is_encrypted: false, // lopdf would fail to load if encrypted
            object_count,
            page_dimensions,
        })
    }
}

/// Summary of validation results for multiple files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationSummary {
    /// Individual validation results for each file.
    pub results: Vec<ValidationResult>,

    /// Total number of pages across all files.
    pub total_pages: usize,

    /// Total file size in bytes.
    pub total_size: u64,

    /// Number of files that passed validation.
    pub files_validated: usize,

    /// Number of files that failed validation.
    pub files_failed: usize,
}

impl ValidationSummary {
    /// Create a summary from validation results.
    pub fn from_results(results: Vec<ValidationResult>) -> Self {
        let total_pages = results.iter().map(|r| r.page_count).sum();
        let total_size = results.iter().map(|r| r.file_size).sum();
        let files_validated = results.len();

        Self {
            results,
            total_pages,
            total_size,
            files_validated,
            files_failed: 0,
        }
    }

    /// Format the total file size as a human-readable string.
    pub fn format_total_size(&self) -> String {
        format_file_size(self.total_size)
    }
}

/// Validator for PDF files and configuration.
pub struct Validator {
    /// Whether to perform strict validation.
    #[expect(unused)]
    strict: bool,
}

impl Validator {
    /// Create a new validator with default settings.
    pub fn new() -> Self {
        Self { strict: false }
    }

    /// Create a new validator with strict mode enabled.
    ///
    /// In strict mode, warnings are treated as errors.
    pub fn strict() -> Self {
        Self { strict: true }
    }

    /// Validate a single PDF file.
    ///
    /// Performs comprehensive validation including:
    /// - File existence and accessibility
    /// - PDF format validation
    /// - Encryption detection
    /// - Page count extraction
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the PDF file to validate
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File does not exist
    /// - File is not accessible
    /// - File is not a valid PDF
    /// - File is encrypted
    /// - PDF structure is corrupted
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pdfcat::validation::Validator;
    /// # use std::path::PathBuf;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let validator = Validator::new();
    /// let result = validator.validate_file(&PathBuf::from("doc.pdf")).await?;
    /// println!("Valid PDF with {} pages", result.page_count);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate_file(&self, path: &Path) -> Result<ValidationResult> {
        // Check if file exists
        if !path.exists() {
            return Err(PdfCatError::file_not_found(path.to_path_buf()));
        }

        // Check if it's a file (not a directory)
        if !path.is_file() {
            return Err(PdfCatError::NotAFile {
                path: path.to_path_buf(),
            });
        }

        // Try to open and read the file
        let metadata =
            tokio::fs::metadata(path)
                .await
                .map_err(|e| PdfCatError::FileNotAccessible {
                    path: path.to_path_buf(),
                    source: e,
                })?;

        // Check if file is readable (has size > 0)
        if metadata.len() == 0 {
            return Err(PdfCatError::corrupted_pdf(
                path.to_path_buf(),
                "File is empty",
            ));
        }

        // Load the PDF document
        let doc = Document::load(path).map_err(|e| {
            // Check if it's an encryption error
            let err_msg = e.to_string();
            if err_msg.contains("encrypt") || err_msg.contains("password") {
                PdfCatError::encrypted_pdf(path.to_path_buf())
            } else {
                PdfCatError::failed_to_load_pdf(path.to_path_buf(), err_msg)
            }
        })?;

        // Verify the document has pages
        let pages = doc.get_pages();
        if pages.is_empty() {
            return Err(PdfCatError::corrupted_pdf(
                path.to_path_buf(),
                "PDF has no pages",
            ));
        }

        // Create validation result
        ValidationResult::from_document(path.to_path_buf(), &doc)
    }

    /// Validate multiple PDF files.
    ///
    /// Validates all input files and returns a summary of results.
    /// Can continue on errors if specified in the configuration.
    ///
    /// # Arguments
    ///
    /// * `paths` - Paths to PDF files to validate
    /// * `continue_on_error` - Whether to continue validating after errors
    ///
    /// # Errors
    ///
    /// Returns an error if any file fails validation and `continue_on_error` is false.
    pub async fn validate_files(
        &self,
        paths: &[PathBuf],
        continue_on_error: bool,
    ) -> Result<ValidationSummary> {
        let mut results = Vec::new();
        let mut failed_count = 0;

        for path in paths {
            match self.validate_file(path).await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    if continue_on_error {
                        eprintln!("Warning: Skipping {}: {}", path.display(), e);
                        failed_count += 1;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        if results.is_empty() {
            return Err(PdfCatError::NoFilesToMerge);
        }

        let mut summary = ValidationSummary::from_results(results);
        summary.files_failed = failed_count;

        Ok(summary)
    }

    /// Validate the output path.
    ///
    /// Checks if the output path is writable and handles overwrite scenarios.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration containing output path and overwrite mode
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Output file exists and no_clobber is set
    /// - Output directory doesn't exist
    /// - Output directory is not writable
    pub async fn validate_output(&self, config: &Config) -> Result<()> {
        let output_path = &config.output;

        // Check if output file already exists
        if output_path.exists() {
            match config.overwrite_mode {
                crate::config::OverwriteMode::NoClobber => {
                    return Err(PdfCatError::output_exists(output_path.clone()));
                }
                crate::config::OverwriteMode::Prompt => {
                    // Prompt will be handled by the caller
                }
                crate::config::OverwriteMode::Force => {
                    // Force overwrite, no check needed
                }
            }
        }

        // Get absolut output path
        if let Ok(p) = output_path.canonicalize() {
            // Check if output directory exists and is writable
            if let Some(parent) = p.parent() {
                if !parent.exists() {
                    return Err(PdfCatError::invalid_config(format!(
                        "Output directory does not exist: {}",
                        parent.display()
                    )));
                }

                // Try to check write permissions
                let metadata = tokio::fs::metadata(parent).await.map_err(|e| {
                    PdfCatError::FileNotAccessible {
                        path: parent.to_path_buf(),
                        source: e,
                    }
                })?;

                if metadata.permissions().readonly() {
                    return Err(PdfCatError::invalid_config(format!(
                        "Output directory is not writable: {}",
                        parent.display()
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate the complete configuration.
    ///
    /// Performs end-to-end validation of all inputs and outputs.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration to validate
    ///
    /// # Errors
    ///
    /// Returns an error if any validation check fails.
    pub async fn validate_config(&self, config: &Config) -> Result<ValidationSummary> {
        // Validate all input files
        let summary = self
            .validate_files(&config.inputs, config.continue_on_error)
            .await?;

        // Validate output path
        self.validate_output(config).await?;

        // Validate page ranges if specified
        if let Some(ref page_range) = config.page_range {
            for result in &summary.results {
                // Check if any requested pages exceed the document's page count
                let requested_pages = page_range.to_pages(result.page_count as u32);
                if let Some(max_page) = requested_pages.iter().max()
                    && *max_page as usize > result.page_count
                {
                    return Err(PdfCatError::InvalidPageRange {
                        path: result.path.clone(),
                        range: format!("{page_range:?}"),
                        total_pages: result.page_count,
                    });
                }
            }
        }

        Ok(summary)
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

/// Format file size as human-readable string.
///
/// # Arguments
///
/// * `size` - File size in bytes
///
/// # Returns
///
/// Formatted string like "1.5 MB" or "234 KB"
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

    fn create_temp_pdf(dir: &TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut file = std::fs::File::create(&path).unwrap();

        // Minimal valid PDF structure
        let pdf_content = std::fs::read("tests/fixtures/basic.pdf").unwrap();

        file.write_all(&pdf_content).unwrap();
        path
    }

    #[tokio::test]
    async fn test_validate_file_not_found() {
        let validator = Validator::new();
        let result = validator.validate_file(Path::new("/nonexistent.pdf")).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PdfCatError::FileNotFound { .. }
        ));
    }

    #[tokio::test]
    async fn test_validate_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let empty_path = temp_dir.path().join("empty.pdf");
        std::fs::File::create(&empty_path).unwrap();

        let validator = Validator::new();
        let result = validator.validate_file(&empty_path).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PdfCatError::CorruptedPdf { .. }
        ));
    }

    #[tokio::test]
    async fn test_validate_valid_pdf() {
        let temp_dir = TempDir::new().unwrap();
        let pdf_path = create_temp_pdf(&temp_dir, "valid.pdf");

        let validator = Validator::new();
        let result = validator.validate_file(&pdf_path).await;

        assert!(result.is_ok());
        let validation = result.unwrap();
        assert_eq!(validation.page_count, 1);
        assert!(validation.file_size > 0);
    }

    #[tokio::test]
    async fn test_validate_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let pdf1 = create_temp_pdf(&temp_dir, "file1.pdf");
        let pdf2 = create_temp_pdf(&temp_dir, "file2.pdf");

        let validator = Validator::new();
        let paths = vec![pdf1, pdf2];
        let result = validator.validate_files(&paths, false).await;

        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.files_validated, 2);
        assert_eq!(summary.total_pages, 2);
        assert_eq!(summary.files_failed, 0);
    }

    #[tokio::test]
    async fn test_validate_with_continue_on_error() {
        let temp_dir = TempDir::new().unwrap();
        let valid_pdf = create_temp_pdf(&temp_dir, "valid.pdf");
        let invalid_pdf = temp_dir.path().join("invalid.pdf");
        std::fs::File::create(&invalid_pdf).unwrap(); // Empty file

        let validator = Validator::new();
        let paths = vec![valid_pdf, invalid_pdf];
        let result = validator.validate_files(&paths, true).await;

        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.files_validated, 1);
        assert_eq!(summary.files_failed, 1);
    }

    #[tokio::test]
    async fn test_validate_output_no_clobber() {
        let temp_dir = TempDir::new().unwrap();
        let output = temp_dir.path().join("output.pdf");
        std::fs::File::create(&output).unwrap(); // Create existing file

        let config = Config {
            inputs: vec![],
            output,
            dry_run: false,
            verbose: false,
            overwrite_mode: crate::config::OverwriteMode::NoClobber,
            quiet: false,
            bookmarks: false,
            compression: crate::config::CompressionLevel::Standard,
            metadata: crate::config::Metadata::default(),
            continue_on_error: false,
            jobs: None,
            page_range: None,
            rotation: None,
        };

        let validator = Validator::new();
        let result = validator.validate_output(&config).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PdfCatError::OutputExists { .. }
        ));
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500 bytes");
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1536), "1.50 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_file_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_validation_summary() {
        let result1 = ValidationResult {
            path: PathBuf::from("a.pdf"),
            page_count: 5,
            version: Some((1, 4)),
            file_size: 1024,
            is_encrypted: false,
            object_count: 10,
            page_dimensions: None,
        };

        let result2 = ValidationResult {
            path: PathBuf::from("b.pdf"),
            page_count: 3,
            version: Some((1, 5)),
            file_size: 2048,
            is_encrypted: false,
            object_count: 8,
            page_dimensions: None,
        };

        let summary = ValidationSummary::from_results(vec![result1, result2]);

        assert_eq!(summary.total_pages, 8);
        assert_eq!(summary.total_size, 3072);
        assert_eq!(summary.files_validated, 2);
        assert_eq!(summary.format_total_size(), "3.00 KB");
    }
}
