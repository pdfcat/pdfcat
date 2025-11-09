//! CLI argument parsing for pdfcat.
//!
//! This module defines the command-line interface structure using `clap`.
//! It handles argument parsing, validation, and help text generation.
//!
//! # Examples
//!
//! ```no_run
//! use pdfcat::cli::Cli;
//! use clap::Parser;
//!
//! let cli = Cli::parse();
//! println!("Merging {} files", cli.inputs.len());
//! ```

use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;

use crate::config::{CompressionLevel, Config, Metadata, OverwriteMode, PageRange, Rotation};
use crate::error::{PdfCatError, Result};

/// Concatenate PDF files into a single document.
///
/// pdfcat merges multiple PDF files while preserving quality, structure,
/// and metadata. It supports bookmarks, page ranges, rotation, and
/// comprehensive error handling.
#[derive(Parser, Debug)]
#[command(name = "pdfcat")]
#[command(version)]
#[command(about = "Concatenate PDF files into a single document", long_about = None)]
#[command(author)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Input PDF files to merge (in order)
    ///
    /// Specify multiple files or use glob patterns.
    /// Files are merged in the order provided.
    ///
    /// Examples:
    ///   pdfcat file1.pdf file2.pdf -o output.pdf
    ///   pdfcat chapter*.pdf -o book.pdf
    #[arg(required = true, value_name = "FILE")]
    pub inputs: Vec<PathBuf>,

    /// Output PDF file path
    ///
    /// The merged PDF will be written to this location.
    /// Use --force to overwrite existing files without confirmation.
    #[arg(short, long, value_name = "FILE")]
    pub output: PathBuf,

    /// Dry run - validate inputs and preview merge without creating output
    ///
    /// Validates that all input files exist and are readable PDFs,
    /// then displays what the merge operation would do without
    /// actually creating the output file.
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Verbose output - show detailed information about each PDF
    ///
    /// Displays additional information including PDF version,
    /// page dimensions, object count, and other metadata
    /// for each input file.
    #[arg(short, long)]
    pub verbose: bool,

    /// Force overwrite of existing output file without confirmation
    ///
    /// By default, pdfcat will prompt before overwriting an existing file.
    /// Use this flag to skip the confirmation prompt.
    #[arg(short, long)]
    pub force: bool,

    /// Never overwrite existing output file
    ///
    /// If the output file already exists, exit with an error
    /// instead of prompting or overwriting.
    #[arg(long, conflicts_with = "force")]
    pub no_clobber: bool,

    /// Suppress all non-error output
    ///
    /// Only errors and warnings will be printed.
    /// Useful for scripts and automation.
    #[arg(short, long, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Add bookmarks for each merged document
    ///
    /// Creates a bookmark (outline entry) at the start of each
    /// input PDF using the filename as the bookmark title.
    /// Preserves existing bookmarks from source PDFs.
    #[arg(short, long)]
    pub bookmarks: bool,

    /// Compression level for output PDF
    ///
    /// Controls the compression applied to the merged PDF.
    /// - none: No compression (preserves exact quality)
    /// - standard: Balanced compression (default)
    /// - maximum: Aggressive compression (smaller file size)
    #[arg(short, long, value_name = "LEVEL", default_value = "standard")]
    #[arg(value_parser = ["none", "standard", "maximum"])]
    pub compression: String,

    /// Set title metadata for output PDF
    ///
    /// If not specified, title from the first input PDF is preserved.
    #[arg(long, value_name = "TEXT")]
    pub title: Option<String>,

    /// Set author metadata for output PDF
    ///
    /// If not specified, author from the first input PDF is preserved.
    #[arg(long, value_name = "TEXT")]
    pub author: Option<String>,

    /// Set subject metadata for output PDF
    #[arg(long, value_name = "TEXT")]
    pub subject: Option<String>,

    /// Set keywords metadata for output PDF (comma-separated)
    #[arg(long, value_name = "TEXT")]
    pub keywords: Option<String>,

    /// Continue processing even if some PDFs fail to load
    ///
    /// By default, pdfcat stops on the first error.
    /// With this flag, problematic PDFs are skipped with a warning
    /// and processing continues with remaining files.
    #[arg(long)]
    pub continue_on_error: bool,

    /// Read input file list from a file (one path per line)
    ///
    /// Instead of specifying files on command line, read from a file.
    /// Use '-' to read from stdin. Can be combined with direct inputs.
    ///
    /// Example:
    ///   pdfcat --input-list files.txt -o output.pdf
    #[arg(long, value_name = "FILE")]
    pub input_list: Option<PathBuf>,

    /// Number of parallel jobs for loading PDFs
    ///
    /// Controls how many PDFs are loaded concurrently.
    /// Default is number of CPU cores. Use 1 for sequential processing.
    #[arg(short, long, value_name = "N")]
    pub jobs: Option<usize>,

    /// Page ranges to extract from each input (e.g., "1-5,10,15-20")
    ///
    /// Apply the same page range to all input PDFs.
    /// Page numbers are 1-indexed. Use commas to separate ranges.
    ///
    /// Examples:
    ///   --pages "1-10"      # First 10 pages from each PDF
    ///   --pages "1,3,5"     # Pages 1, 3, and 5 from each PDF
    ///   --pages "1-5,10-15" # Pages 1-5 and 10-15 from each PDF
    #[arg(long, value_name = "RANGE")]
    pub pages: Option<String>,

    /// Rotate pages by specified degrees (90, 180, 270)
    ///
    /// Applies rotation to all pages in all input PDFs.
    #[arg(long, value_name = "DEGREES")]
    #[arg(value_parser = ["90", "180", "270"])]
    pub rotate: Option<String>,
}

impl Cli {
    /// Convert CLI arguments into a validated Config.
    ///
    /// This method performs the following:
    /// - Parses compression level and rotation
    /// - Resolves overwrite mode
    /// - Parses page ranges
    /// - Constructs metadata
    /// - Validates the resulting configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Compression level is invalid
    /// - Rotation degrees are invalid
    /// - Page range format is invalid
    /// - Configuration validation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pdfcat::cli::Cli;
    /// use clap::Parser;
    ///
    /// let cli = Cli::parse();
    /// let config = cli.to_config().expect("Invalid configuration");
    /// ```
    pub fn to_config(&self) -> Result<Config> {
        // Parse compression level
        let compression = CompressionLevel::from_str(&self.compression)
            .map_err(|e| PdfCatError::invalid_config(e.to_string()))?;

        // Parse rotation if provided
        let rotation = if let Some(ref rotate_str) = self.rotate {
            let degrees: u16 = rotate_str
                .parse()
                .map_err(|_| PdfCatError::invalid_config("Invalid rotation degrees"))?;
            Some(
                Rotation::from_degrees(degrees)
                    .map_err(|e| PdfCatError::invalid_config(e.to_string()))?,
            )
        } else {
            None
        };

        // Determine overwrite mode
        let overwrite_mode = if self.force {
            OverwriteMode::Force
        } else if self.no_clobber {
            OverwriteMode::NoClobber
        } else {
            OverwriteMode::Prompt
        };

        // Parse page range if provided
        let page_range = if let Some(ref pages_str) = self.pages {
            Some(
                PageRange::parse(pages_str)
                    .map_err(|e| PdfCatError::invalid_config(e.to_string()))?,
            )
        } else {
            None
        };

        // Construct metadata
        let metadata = Metadata::new(
            self.title.clone(),
            self.author.clone(),
            self.subject.clone(),
            self.keywords.clone(),
        );

        // Build config
        let config = Config {
            inputs: self.inputs.clone(),
            output: self.output.clone(),
            dry_run: self.dry_run,
            verbose: self.verbose,
            overwrite_mode,
            quiet: self.quiet,
            bookmarks: self.bookmarks,
            compression,
            metadata,
            continue_on_error: self.continue_on_error,
            jobs: self.jobs,
            page_range,
            rotation,
        };

        // Validate the configuration
        config.validate().map_err(|e| {
            PdfCatError::invalid_config(format!("Configuration validation failed: {e}"))
        })?;

        Ok(config)
    }

    /// Validate CLI arguments before processing.
    ///
    /// Performs early validation that doesn't require file I/O:
    /// - Check for conflicting flags
    /// - Validate numeric ranges
    /// - Check for empty required fields
    ///
    /// # Errors
    ///
    /// Returns an error if any validation checks fail.
    pub fn validate(&self) -> Result<()> {
        // Check for empty inputs (shouldn't happen with clap, but be safe)
        if self.inputs.is_empty() {
            return Err(PdfCatError::invalid_config("No input files specified"));
        }

        // Validate jobs count
        if let Some(jobs) = self.jobs
            && jobs == 0
        {
            return Err(PdfCatError::invalid_config(
                "Number of jobs must be at least 1",
            ));
        }

        // Validate compression level
        if !["none", "standard", "maximum"].contains(&self.compression.as_str()) {
            return Err(PdfCatError::invalid_config(format!(
                "Invalid compression level: {}",
                self.compression
            )));
        }

        // Validate rotation if provided
        if let Some(ref rotate) = self.rotate
            && !["90", "180", "270"].contains(&rotate.as_str())
        {
            return Err(PdfCatError::invalid_config(format!(
                "Invalid rotation: {rotate}. Must be 90, 180, or 270"
            )));
        }

        // Validate page range format if provided
        if let Some(ref pages) = self.pages {
            PageRange::parse(pages).map_err(|e| PdfCatError::invalid_config(e.to_string()))?;
        }

        Ok(())
    }

    /// Get all input paths including those from input-list file.
    ///
    /// This method combines:
    /// - Direct input arguments
    /// - Paths from --input-list file (if provided)
    ///
    /// Paths from the file are appended after direct inputs.
    ///
    /// # Errors
    ///
    /// Returns an error if the input list file cannot be read or parsed.
    pub async fn get_all_inputs(&self) -> Result<Vec<PathBuf>> {
        let mut all_inputs = self.inputs.clone();

        if let Some(ref input_list_path) = self.input_list {
            let additional_inputs = self.read_input_list(input_list_path).await?;
            all_inputs.extend(additional_inputs);
        }

        if all_inputs.is_empty() {
            return Err(PdfCatError::NoFilesToMerge);
        }

        Ok(all_inputs)
    }

    /// Read input paths from a file.
    ///
    /// Reads a file containing one path per line. Lines starting with '#'
    /// are treated as comments and ignored. Empty lines are skipped.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the input list file. Use "-" to read from stdin.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid paths.
    async fn read_input_list(&self, path: &PathBuf) -> Result<Vec<PathBuf>> {
        use tokio::fs::File;
        use tokio::io::{AsyncBufReadExt, BufReader};

        let file = if path.as_os_str() == "-" {
            // Read from stdin
            return Err(PdfCatError::invalid_config(
                "Reading from stdin not yet implemented",
            ));
        } else {
            File::open(path)
                .await
                .map_err(|e| PdfCatError::FailedToReadInputList {
                    path: path.clone(),
                    source: e,
                })?
        };

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut paths = Vec::new();
        let mut line_number = 0;

        while let Some(line) =
            lines
                .next_line()
                .await
                .map_err(|e| PdfCatError::FailedToReadInputList {
                    path: path.clone(),
                    source: e,
                })?
        {
            line_number += 1;
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let input_path = PathBuf::from(line);

            // Basic validation - check if path looks reasonable
            if input_path.as_os_str().is_empty() {
                return Err(PdfCatError::InvalidInputList {
                    path: path.clone(),
                    line_number,
                    details: "Empty path".to_string(),
                });
            }

            paths.push(input_path);
        }

        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_cli(inputs: Vec<&str>, output: &str) -> Cli {
        Cli {
            inputs: inputs.iter().map(|s| PathBuf::from(s)).collect(),
            output: PathBuf::from(output),
            dry_run: false,
            verbose: false,
            force: false,
            no_clobber: false,
            quiet: false,
            bookmarks: false,
            compression: "standard".to_string(),
            title: None,
            author: None,
            subject: None,
            keywords: None,
            continue_on_error: false,
            input_list: None,
            jobs: None,
            pages: None,
            rotate: None,
        }
    }

    #[test]
    fn test_basic_cli_to_config() {
        let cli = create_test_cli(vec!["a.pdf", "b.pdf"], "out.pdf");
        let config = cli.to_config().unwrap();

        assert_eq!(config.inputs.len(), 2);
        assert_eq!(config.output, PathBuf::from("out.pdf"));
        assert!(!config.dry_run);
        assert!(!config.verbose);
    }

    #[test]
    fn test_cli_with_compression() {
        let mut cli = create_test_cli(vec!["a.pdf"], "out.pdf");
        cli.compression = "maximum".to_string();

        let config = cli.to_config().unwrap();
        assert_eq!(config.compression, CompressionLevel::Maximum);
    }

    #[test]
    fn test_cli_with_invalid_compression() {
        let mut cli = create_test_cli(vec!["a.pdf"], "out.pdf");
        cli.compression = "invalid".to_string();

        assert!(cli.to_config().is_err());
    }

    #[test]
    fn test_cli_with_rotation() {
        let mut cli = create_test_cli(vec!["a.pdf"], "out.pdf");
        cli.rotate = Some("90".to_string());

        let config = cli.to_config().unwrap();
        assert_eq!(config.rotation, Some(Rotation::Clockwise90));
    }

    #[test]
    fn test_cli_with_invalid_rotation() {
        let mut cli = create_test_cli(vec!["a.pdf"], "out.pdf");
        cli.rotate = Some("45".to_string());

        assert!(cli.to_config().is_err());
    }

    #[test]
    fn test_cli_overwrite_modes() {
        let mut cli = create_test_cli(vec!["a.pdf"], "out.pdf");

        // Default mode
        let config = cli.to_config().unwrap();
        assert_eq!(config.overwrite_mode, OverwriteMode::Prompt);

        // Force mode
        cli.force = true;
        let config = cli.to_config().unwrap();
        assert_eq!(config.overwrite_mode, OverwriteMode::Force);

        // No clobber mode
        cli.force = false;
        cli.no_clobber = true;
        let config = cli.to_config().unwrap();
        assert_eq!(config.overwrite_mode, OverwriteMode::NoClobber);
    }

    #[test]
    fn test_cli_with_page_range() {
        let mut cli = create_test_cli(vec!["a.pdf"], "out.pdf");
        cli.pages = Some("1-5,10".to_string());

        let config = cli.to_config().unwrap();
        assert!(config.page_range.is_some());

        let page_range = config.page_range.unwrap();
        assert!(page_range.contains(3));
        assert!(page_range.contains(10));
        assert!(!page_range.contains(7));
    }

    #[test]
    fn test_cli_with_invalid_page_range() {
        let mut cli = create_test_cli(vec!["a.pdf"], "out.pdf");
        cli.pages = Some("invalid".to_string());

        assert!(cli.to_config().is_err());
    }

    #[test]
    fn test_cli_with_metadata() {
        let mut cli = create_test_cli(vec!["a.pdf"], "out.pdf");
        cli.title = Some("Test Title".to_string());
        cli.author = Some("Test Author".to_string());

        let config = cli.to_config().unwrap();
        assert_eq!(config.metadata.title, Some("Test Title".to_string()));
        assert_eq!(config.metadata.author, Some("Test Author".to_string()));
    }

    #[test]
    fn test_cli_validate_no_inputs() {
        let mut cli = create_test_cli(vec!["a.pdf"], "out.pdf");
        cli.inputs.clear();

        assert!(cli.validate().is_err());
    }

    #[test]
    fn test_cli_validate_zero_jobs() {
        let mut cli = create_test_cli(vec!["a.pdf"], "out.pdf");
        cli.jobs = Some(0);

        assert!(cli.validate().is_err());
    }

    #[test]
    fn test_cli_validate_invalid_compression() {
        let mut cli = create_test_cli(vec!["a.pdf"], "out.pdf");
        cli.compression = "super".to_string();

        assert!(cli.validate().is_err());
    }

    #[test]
    fn test_cli_validate_invalid_rotation() {
        let mut cli = create_test_cli(vec!["a.pdf"], "out.pdf");
        cli.rotate = Some("360".to_string());

        assert!(cli.validate().is_err());
    }

    #[tokio::test]
    async fn test_get_all_inputs_no_list() {
        let cli = create_test_cli(vec!["a.pdf", "b.pdf"], "out.pdf");
        let inputs = cli.get_all_inputs().await.unwrap();

        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0], PathBuf::from("a.pdf"));
        assert_eq!(inputs[1], PathBuf::from("b.pdf"));
    }

    #[tokio::test]
    async fn test_get_all_inputs_empty() {
        let mut cli = create_test_cli(vec![], "out.pdf");
        cli.inputs.clear();

        assert!(cli.get_all_inputs().await.is_err());
    }
}
