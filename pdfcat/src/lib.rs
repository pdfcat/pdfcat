//! pdfcat - Concatenate PDF files into a single document.
//!
//! This library provides functionality for merging multiple PDF files while
//! preserving quality, structure, and metadata. It supports:
//!
//! - High-quality PDF merging
//! - Page extraction and manipulation
//! - Bookmark creation
//! - Metadata management
//! - Parallel processing
//! - Comprehensive error handling
//!
//! # Examples
//!
//! ## Basic Merge
//!
//! ```no_run
//! use pdfcat::merge;
//! use pdfcat::config::{Config, CompressionLevel, Metadata, OverwriteMode};
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = Config {
//!     inputs: vec![PathBuf::from("a.pdf"), PathBuf::from("b.pdf")],
//!     output: PathBuf::from("merged.pdf"),
//!     dry_run: false,
//!     verbose: false,
//!     overwrite_mode: OverwriteMode::Prompt,
//!     quiet: false,
//!     bookmarks: true,
//!     compression: CompressionLevel::Standard,
//!     metadata: Metadata::default(),
//!     continue_on_error: false,
//!     jobs: None,
//!     page_range: None,
//!     rotation: None,
//! };
//!
//! let (document, stats) = merge::merge_pdfs(&config).await?;
//! println!("Created {} page document", stats.total_pages);
//! # Ok(())
//! # }
//! ```
//!
//! ## Using Individual Components
//!
//! ```no_run
//! use pdfcat::io::{PdfReader, PdfWriter};
//! use pdfcat::validation::Validator;
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Validate input
//! let validator = Validator::new();
//! let result = validator.validate_file(&PathBuf::from("input.pdf")).await?;
//! println!("PDF has {} pages", result.page_count);
//!
//! // Load PDF
//! let reader = PdfReader::new();
//! let loaded = reader.load(&PathBuf::from("input.pdf")).await?;
//!
//! // Save PDF
//! let writer = PdfWriter::new();
//! writer.save(&loaded.document, &PathBuf::from("output.pdf")).await?;
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod config;
pub mod error;
pub mod io;
pub mod merge;
pub mod output;
pub mod utils;
pub mod validation;

// Re-export commonly used types
pub use config::Config;
pub use error::{PdfCatError, Result};

/// Library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name.
pub const NAME: &str = env!("CARGO_PKG_NAME");
