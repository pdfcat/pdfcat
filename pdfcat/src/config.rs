//! Configuration module for pdfcat.
//!
//! This module transforms CLI arguments into a validated, normalized configuration
//! that drives the PDF merging process. It handles:
//! - Validation of argument combinations
//! - Resolution of conflicting options
//! - Application of defaults
//! - Path canonicalization

use anyhow::{Context, Result, bail};

use crate::PdfCatError;
use std::{path::PathBuf, str::FromStr};

/// Compression level for the output PDF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionLevel {
    /// No compression - preserves exact quality and structure.
    None,
    /// Balanced compression - good trade-off between size and processing time.
    #[default]
    Standard,
    /// Maximum compression - smallest file size, longer processing time.
    Maximum,
}

impl FromStr for CompressionLevel {
    type Err = crate::PdfCatError;
    /// Parse compression level from string.
    ///
    /// # Arguments
    ///
    /// * `s` - String representation: "none", "standard", or "maximum"
    ///
    /// # Errors
    ///
    /// Returns an error if the string doesn't match a valid compression level.
    fn from_str(s: &str) -> crate::Result<Self> {
        match s.to_lowercase().as_str() {
            "none" => Ok(Self::None),
            "standard" => Ok(Self::Standard),
            "maximum" => Ok(Self::Maximum),
            _ => Err(PdfCatError::InvalidConfig {
                message: format!(
                    "Invalid compression level: {s}. Must be one of: none, standard, maximum"
                ),
            }),
        }
    }
}

/// Page rotation in degrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// Rotate 90 degrees clockwise.
    Clockwise90,
    /// Rotate 180 degrees.
    Rotate180,
    /// Rotate 270 degrees clockwise (90 counter-clockwise).
    Clockwise270,
}

impl Rotation {
    /// Parse rotation from degrees.
    ///
    /// # Arguments
    ///
    /// * `degrees` - Rotation in degrees: 90, 180, or 270
    ///
    /// # Errors
    ///
    /// Returns an error if the degrees value is not 90, 180, or 270.
    pub fn from_degrees(degrees: u16) -> Result<Self> {
        match degrees {
            90 => Ok(Self::Clockwise90),
            180 => Ok(Self::Rotate180),
            270 => Ok(Self::Clockwise270),
            _ => bail!(PdfCatError::InvalidConfig {
                message: format!("Invalid rotation: {degrees}. Must be 90, 180, or 270"),
            }),
        }
    }

    /// Get rotation as degrees.
    pub fn as_degrees(&self) -> u16 {
        match self {
            Self::Clockwise90 => 90,
            Self::Rotate180 => 180,
            Self::Clockwise270 => 270,
        }
    }
}

/// Page range specification for extraction.
///
/// Supports individual pages and ranges:
/// - "1" - single page
/// - "1-5" - range of pages (inclusive)
/// - "1,3,5" - multiple individual pages
/// - "1-5,10-15" - combination of ranges
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRange {
    ranges: Vec<PageRangeItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PageRangeItem {
    Single(u32),
    Range(u32, u32),
}

impl PageRange {
    /// Parse a page range string.
    ///
    /// # Arguments
    ///
    /// * `s` - Page range string (e.g., "1-5,10,15-20")
    ///
    /// # Errors
    ///
    /// Returns an error if the string format is invalid or contains invalid page numbers.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfcat::config::PageRange;
    ///
    /// let range = PageRange::parse("1-5,10").unwrap();
    /// assert!(range.contains(3));
    /// assert!(range.contains(10));
    /// assert!(!range.contains(7));
    /// ```
    pub fn parse(s: &str) -> Result<Self> {
        let mut ranges = Vec::new();

        for part in s.split(',') {
            let part = part.trim();

            if part.contains('-') {
                let parts: Vec<&str> = part.split('-').collect();
                if parts.len() != 2 {
                    bail!("Invalid page range format: {part}. Expected format like '1-5'");
                }

                let start: u32 = parts[0]
                    .trim()
                    .parse()
                    .with_context(|| format!("Invalid page number: {}", parts[0]))?;

                let end: u32 = parts[1]
                    .trim()
                    .parse()
                    .with_context(|| format!("Invalid page number: {}", parts[1]))?;

                if start == 0 || end == 0 {
                    bail!("Page numbers must be positive (1-indexed)");
                }

                if start > end {
                    bail!(
                        "Invalid range {start}-{end}: start page must be less than or equal to end page"
                    );
                }

                ranges.push(PageRangeItem::Range(start, end));
            } else {
                let page: u32 = part
                    .parse()
                    .with_context(|| format!("Invalid page number: {part}"))?;

                if page == 0 {
                    bail!("Page numbers must be positive (1-indexed)");
                }

                ranges.push(PageRangeItem::Single(page));
            }
        }

        if ranges.is_empty() {
            bail!("Page range cannot be empty");
        }

        Ok(Self { ranges })
    }

    /// Check if a page number is included in this range.
    ///
    /// # Arguments
    ///
    /// * `page` - 1-indexed page number
    pub fn contains(&self, page: u32) -> bool {
        self.ranges.iter().any(|item| match item {
            PageRangeItem::Single(p) => *p == page,
            PageRangeItem::Range(start, end) => page >= *start && page <= *end,
        })
    }

    /// Get all page numbers included in this range up to a maximum.
    ///
    /// # Arguments
    ///
    /// * `max_pages` - Maximum page number to consider
    ///
    /// # Returns
    ///
    /// A sorted vector of 1-indexed page numbers.
    pub fn to_pages(&self, max_pages: u32) -> Vec<u32> {
        let mut pages: Vec<u32> = (1..=max_pages).filter(|p| self.contains(*p)).collect();
        pages.sort_unstable();
        pages.dedup();
        pages
    }
}

/// PDF metadata to set on the output document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    /// Document title.
    pub title: Option<String>,
    /// Document author.
    pub author: Option<String>,
    /// Document subject.
    pub subject: Option<String>,
    /// Document keywords (comma-separated).
    pub keywords: Option<String>,
}

impl Metadata {
    /// Check if any metadata fields are set.
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.author.is_none()
            && self.subject.is_none()
            && self.keywords.is_none()
    }

    /// Create metadata from optional strings, trimming whitespace.
    pub fn new(
        title: Option<String>,
        author: Option<String>,
        subject: Option<String>,
        keywords: Option<String>,
    ) -> Self {
        let to_string_opt = |opt: Option<String>| {
            opt.filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().to_string())
        };

        Self {
            title: to_string_opt(title),
            author: to_string_opt(author),
            subject: to_string_opt(subject),
            keywords: to_string_opt(keywords),
        }
    }
}

/// Output file overwrite behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverwriteMode {
    /// Prompt the user before overwriting (default).
    #[default]
    Prompt,
    /// Always overwrite without prompting.
    Force,
    /// Never overwrite, error if file exists.
    NoClobber,
}

/// Complete configuration for a PDF merge operation.
///
/// This structure contains all settings needed to perform a merge,
/// derived and validated from CLI arguments.
#[derive(Debug, Clone)]
pub struct Config {
    /// Input PDF file paths (in merge order).
    pub inputs: Vec<PathBuf>,

    /// Output PDF file path.
    pub output: PathBuf,

    /// Dry run mode - validate without creating output.
    pub dry_run: bool,

    /// Verbose output mode.
    pub verbose: bool,

    /// File overwrite behavior.
    pub overwrite_mode: OverwriteMode,

    /// Quiet mode - suppress non-error output.
    pub quiet: bool,

    /// Add bookmarks for each merged document.
    pub bookmarks: bool,

    /// Compression level for output.
    pub compression: CompressionLevel,

    /// Metadata to set on output document.
    pub metadata: Metadata,

    /// Continue on errors instead of stopping.
    pub continue_on_error: bool,

    /// Number of parallel jobs (None = auto-detect).
    pub jobs: Option<usize>,

    /// Page range to extract from each input.
    pub page_range: Option<PageRange>,

    /// Rotation to apply to all pages.
    pub rotation: Option<Rotation>,
}

impl Config {
    /// Returns a reference to inputs.
    pub fn inputs(&self) -> &[PathBuf] {
        self.inputs.as_ref()
    }

    /// Validate the configuration.
    ///
    /// Checks for logical inconsistencies and invalid combinations.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No input files are specified
    /// - Verbose and quiet modes are both enabled
    /// - Jobs count is zero
    /// - Other validation rules fail
    pub fn validate(&self) -> Result<()> {
        if self.inputs.is_empty() {
            bail!("No input files specified");
        }

        if self.verbose && self.quiet {
            bail!("Cannot use both --verbose and --quiet");
        }

        if let Some(jobs) = self.jobs
            && jobs == 0
        {
            bail!("Number of jobs must be at least 1");
        }

        // Validate that output path is not in inputs
        for input in &self.inputs {
            if input == &self.output {
                bail!(
                    "Output file cannot be the same as an input file: {}",
                    self.output.display()
                );
            }
        }

        Ok(())
    }

    /// Get the effective number of parallel jobs.
    ///
    /// Returns the configured job count, or the number of CPU cores if auto-detect.
    pub fn effective_jobs(&self) -> usize {
        self.jobs.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        })
    }

    /// Check if output should be displayed.
    ///
    /// Returns false if in quiet mode and not doing a dry run.
    pub fn should_print(&self) -> bool {
        !self.quiet || self.dry_run
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_level_from_str() {
        assert_eq!(
            CompressionLevel::from_str("none").unwrap(),
            CompressionLevel::None
        );
        assert_eq!(
            CompressionLevel::from_str("standard").unwrap(),
            CompressionLevel::Standard
        );
        assert_eq!(
            CompressionLevel::from_str("maximum").unwrap(),
            CompressionLevel::Maximum
        );
        assert_eq!(
            CompressionLevel::from_str("STANDARD").unwrap(),
            CompressionLevel::Standard
        );
        assert!(CompressionLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_rotation_from_degrees() {
        assert_eq!(Rotation::from_degrees(90).unwrap(), Rotation::Clockwise90);
        assert_eq!(Rotation::from_degrees(180).unwrap(), Rotation::Rotate180);
        assert_eq!(Rotation::from_degrees(270).unwrap(), Rotation::Clockwise270);
        assert!(Rotation::from_degrees(45).is_err());
    }

    #[test]
    fn test_rotation_as_degrees() {
        assert_eq!(Rotation::Clockwise90.as_degrees(), 90);
        assert_eq!(Rotation::Rotate180.as_degrees(), 180);
        assert_eq!(Rotation::Clockwise270.as_degrees(), 270);
    }

    #[test]
    fn test_page_range_single() {
        let range = PageRange::parse("5").unwrap();
        assert!(range.contains(5));
        assert!(!range.contains(4));
        assert!(!range.contains(6));
    }

    #[test]
    fn test_page_range_range() {
        let range = PageRange::parse("5-10").unwrap();
        assert!(!range.contains(4));
        assert!(range.contains(5));
        assert!(range.contains(7));
        assert!(range.contains(10));
        assert!(!range.contains(11));
    }

    #[test]
    fn test_page_range_multiple() {
        let range = PageRange::parse("1-3,5,7-9").unwrap();
        assert!(range.contains(1));
        assert!(range.contains(2));
        assert!(range.contains(3));
        assert!(!range.contains(4));
        assert!(range.contains(5));
        assert!(!range.contains(6));
        assert!(range.contains(7));
        assert!(range.contains(8));
        assert!(range.contains(9));
        assert!(!range.contains(10));
    }

    #[test]
    fn test_page_range_to_pages() {
        let range = PageRange::parse("2-4,6").unwrap();
        assert_eq!(range.to_pages(10), vec![2, 3, 4, 6]);
    }

    #[test]
    fn test_page_range_invalid() {
        assert!(PageRange::parse("0").is_err());
        assert!(PageRange::parse("5-3").is_err());
        assert!(PageRange::parse("abc").is_err());
        assert!(PageRange::parse("").is_err());
        assert!(PageRange::parse("1-2-3").is_err());
    }

    #[test]
    fn test_metadata_is_empty() {
        let empty = Metadata::default();
        assert!(empty.is_empty());

        let not_empty = Metadata {
            title: Some("Title".to_string()),
            ..Default::default()
        };
        assert!(!not_empty.is_empty());
    }

    #[test]
    fn test_metadata_new_trims_whitespace() {
        let meta = Metadata::new(
            Some("  Title  ".to_string()),
            Some("   ".to_string()),
            None,
            Some("keyword".to_string()),
        );

        assert_eq!(meta.title, Some("Title".to_string()));
        assert_eq!(meta.author, None); // Whitespace-only becomes None
        assert_eq!(meta.subject, None);
        assert_eq!(meta.keywords, Some("keyword".to_string()));
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config {
            inputs: vec![PathBuf::from("a.pdf")],
            output: PathBuf::from("out.pdf"),
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
        };

        assert!(config.validate().is_ok());

        // Test no inputs
        config.inputs.clear();
        assert!(config.validate().is_err());
        config.inputs = vec![PathBuf::from("a.pdf")];

        // Test verbose + quiet conflict
        config.verbose = true;
        config.quiet = true;
        assert!(config.validate().is_err());
        config.verbose = false;
        config.quiet = false;

        // Test zero jobs
        config.jobs = Some(0);
        assert!(config.validate().is_err());
        config.jobs = None;

        // Test output same as input
        config.output = PathBuf::from("a.pdf");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_effective_jobs() {
        let config = Config {
            inputs: vec![PathBuf::from("a.pdf")],
            output: PathBuf::from("out.pdf"),
            dry_run: false,
            verbose: false,
            overwrite_mode: OverwriteMode::Prompt,
            quiet: false,
            bookmarks: false,
            compression: CompressionLevel::Standard,
            metadata: Metadata::default(),
            continue_on_error: false,
            jobs: Some(4),
            page_range: None,
            rotation: None,
        };

        assert_eq!(config.effective_jobs(), 4);

        let auto_config = Config {
            jobs: None,
            ..config
        };

        assert!(auto_config.effective_jobs() >= 1);
    }

    #[test]
    fn test_should_print() {
        let mut config = Config {
            inputs: vec![PathBuf::from("a.pdf")],
            output: PathBuf::from("out.pdf"),
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
        };

        assert!(config.should_print());

        config.quiet = true;
        assert!(!config.should_print());

        config.dry_run = true;
        assert!(config.should_print()); // Dry run always prints
    }
}
