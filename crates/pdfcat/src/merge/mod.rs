//! PDF merging operations.
//!
//! This module provides the core PDF merging functionality with:
//! - Document concatenation
//! - Page extraction and manipulation
//! - Bookmark handling
//! - Metadata management
//! - Order preservation
//! - Quality preservation
//!
//! # Examples
//!
//! ```no_run
//! use pdfcat::merge::Merger;
//! use pdfcat::config::Config;
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = Config {
//!     inputs: vec![PathBuf::from("a.pdf"), PathBuf::from("b.pdf")],
//!     output: PathBuf::from("merged.pdf"),
//!     // ... other config fields
//! #   dry_run: false,
//! #   verbose: false,
//! #   overwrite_mode: pdfcat::config::OverwriteMode::Prompt,
//! #   quiet: false,
//! #   bookmarks: false,
//! #   compression: pdfcat::config::CompressionLevel::Standard,
//! #   metadata: pdfcat::config::Metadata::default(),
//! #   continue_on_error: false,
//! #   jobs: None,
//! #   page_range: None,
//! #   rotation: None,
//! };
//!
//! let merger = Merger::new();
//! let result = merger.merge(&config).await?;
//! println!("Merged {} pages", result.statistics.total_pages);
//! # Ok(())
//! # }
//! ```

pub mod bookmarks;
pub mod merger;
pub mod metadata;
pub mod pages;

pub use bookmarks::BookmarkManager;
pub use merger::{MergeResult, MergeStatistics, Merger};
pub use metadata::MetadataManager;
pub use pages::{PageExtractor, PageRotation};

use crate::config::Config;
use crate::error::Result;
use lopdf::Document;

/// Merge multiple PDF files according to configuration.
///
/// Convenience function that creates a merger and performs the merge.
///
/// # Arguments
///
/// * `config` - Merge configuration
///
/// # Returns
///
/// The merged document and statistics about the operation.
///
/// # Errors
///
/// Returns an error if any merge step fails.
///
/// # Examples
///
/// ```no_run
/// use pdfcat::merge::merge_pdfs;
/// use pdfcat::config::Config;
///
/// # async fn example(config: Config) -> Result<(), Box<dyn std::error::Error>> {
/// let (document, stats) = merge_pdfs(&config).await?;
/// println!("Created {} page document", stats.total_pages);
/// # Ok(())
/// # }
/// ```
pub async fn merge_pdfs(config: &Config) -> Result<(Document, MergeStatistics)> {
    let merger = Merger::new();
    let result = merger.merge(config).await?;
    Ok((result.document, result.statistics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CompressionLevel, Metadata, OverwriteMode};
    use std::path::PathBuf;

    #[expect(unused)]
    fn create_test_config() -> Config {
        Config {
            inputs: vec![PathBuf::from("test1.pdf"), PathBuf::from("test2.pdf")],
            output: PathBuf::from("output.pdf"),
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

    #[test]
    fn test_merger_creation() {
        let _merger = Merger::new();
        // Should create without panicking
    }

    #[test]
    fn test_page_extractor_creation() {
        let _extractor = PageExtractor::new();
        // Should create without panicking
    }

    #[test]
    fn test_bookmark_manager_creation() {
        let _manager = BookmarkManager::new();
        // Should create without panicking
    }

    #[test]
    fn test_metadata_manager_creation() {
        let _manager = MetadataManager::new();
        // Should create without panicking
    }
}
