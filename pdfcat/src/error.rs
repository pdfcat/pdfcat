//! Error types for pdfcat.
//!
//! This module defines all error types that can occur during PDF operations.
//! Errors are designed to be informative and actionable, providing clear
//! context about what went wrong and how to fix it.
//!
//! # Error Categories
//!
//! - **I/O Errors**: File not found, permission denied, etc.
//! - **PDF Errors**: Invalid PDF structure, corrupted files
//! - **Validation Errors**: Invalid arguments or configuration
//! - **Merge Errors**: Problems during the merge process

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Result type alias for pdfcat operations.
pub type Result<T> = std::result::Result<T, PdfCatError>;

/// Main error type for pdfcat operations.
///
/// All errors in pdfcat use this type, which provides detailed context
/// about what went wrong and where.
#[derive(Debug)]
pub enum PdfCatError {
    /// Input file was not found.
    FileNotFound {
        /// Path to the file that was not found.
        path: PathBuf,
    },

    /// Input file is not accessible (permission denied, etc.).
    FileNotAccessible {
        /// Path to the inaccessible file.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },

    /// Input file is not a directory when expected.
    NotAFile {
        /// Path that is not a file.
        path: PathBuf,
    },

    /// Failed to load PDF file.
    FailedToLoadPdf {
        /// Path to the PDF file.
        path: PathBuf,
        /// Reason for the failure.
        reason: String,
    },

    /// PDF file is corrupted or has invalid structure.
    CorruptedPdf {
        /// Path to the corrupted PDF.
        path: PathBuf,
        /// Details about the corruption.
        details: String,
    },

    /// PDF file is encrypted and cannot be processed.
    EncryptedPdf {
        /// Path to the encrypted PDF.
        path: PathBuf,
    },

    /// No files were provided for merging.
    NoFilesToMerge,

    /// Output file already exists and overwrite is not allowed.
    OutputExists {
        /// Path to the existing output file.
        path: PathBuf,
    },

    /// Failed to create output file.
    FailedToCreateOutput {
        /// Path where output should be created.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },

    /// Failed to write to output file.
    FailedToWrite {
        /// Path being written to.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },

    /// Failed to read input list file.
    FailedToReadInputList {
        /// Path to the input list file.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },

    /// Input list file contains invalid paths.
    InvalidInputList {
        /// Path to the input list file.
        path: PathBuf,
        /// Line number with the error.
        line_number: usize,
        /// Details about what's invalid.
        details: String,
    },

    /// Page range is invalid for the PDF.
    InvalidPageRange {
        /// Path to the PDF file.
        path: PathBuf,
        /// Requested page range.
        range: String,
        /// Total pages in the PDF.
        total_pages: usize,
    },

    /// Merge operation failed.
    MergeFailed {
        /// Description of what went wrong.
        reason: String,
    },

    /// Bookmark operation failed.
    BookmarkFailed {
        /// Path to the PDF where bookmark operation failed.
        path: PathBuf,
        /// Details about the failure.
        reason: String,
    },

    /// Metadata operation failed.
    MetadataFailed {
        /// Details about the failure.
        reason: String,
    },

    /// Invalid configuration.
    InvalidConfig {
        /// Description of what's wrong with the configuration.
        message: String,
    },

    /// User cancelled the operation.
    Cancelled,

    /// Generic I/O error.
    Io {
        /// Underlying I/O error.
        source: io::Error,
    },

    /// Generic error with a custom message.
    Other {
        /// Error message.
        message: String,
    },
}

impl fmt::Display for PdfCatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound { path } => {
                write!(f, "File not found: {}", path.display())
            }
            Self::FileNotAccessible { path, source } => {
                write!(
                    f,
                    "Cannot access file: {}\n  Reason: {}",
                    path.display(),
                    source
                )
            }
            Self::NotAFile { path } => {
                write!(f, "Not a file: {}", path.display())
            }
            Self::FailedToLoadPdf { path, reason } => {
                write!(
                    f,
                    "Failed to load PDF: {}\n  Reason: {}",
                    path.display(),
                    reason
                )
            }
            Self::CorruptedPdf { path, details } => {
                write!(
                    f,
                    "Corrupted or invalid PDF: {}\n  Details: {}",
                    path.display(),
                    details
                )
            }
            Self::EncryptedPdf { path } => {
                write!(
                    f,
                    "PDF is encrypted and cannot be processed: {}\n  \
                     Hint: Decrypt the PDF first using 'qpdf --decrypt' or similar tools",
                    path.display()
                )
            }
            Self::NoFilesToMerge => {
                write!(f, "No input files specified for merging")
            }
            Self::OutputExists { path } => {
                write!(
                    f,
                    "Output file already exists: {}\n  \
                     Use --force to overwrite or choose a different output path",
                    path.display()
                )
            }
            Self::FailedToCreateOutput { path, source } => {
                write!(
                    f,
                    "Failed to create output file: {}\n  Reason: {}",
                    path.display(),
                    source
                )
            }
            Self::FailedToWrite { path, source } => {
                write!(
                    f,
                    "Failed to write to output file: {}\n  Reason: {}",
                    path.display(),
                    source
                )
            }
            Self::FailedToReadInputList { path, source } => {
                write!(
                    f,
                    "Failed to read input list file: {}\n  Reason: {}",
                    path.display(),
                    source
                )
            }
            Self::InvalidInputList {
                path,
                line_number,
                details,
            } => {
                write!(
                    f,
                    "Invalid entry in input list file: {} at line {}\n  Details: {}",
                    path.display(),
                    line_number,
                    details
                )
            }
            Self::InvalidPageRange {
                path,
                range,
                total_pages,
            } => {
                write!(
                    f,
                    "Invalid page range '{}' for PDF: {}\n  \
                     PDF has {} page(s). Page numbers must be between 1 and {}",
                    range,
                    path.display(),
                    total_pages,
                    total_pages
                )
            }
            Self::MergeFailed { reason } => {
                write!(f, "Merge operation failed: {reason}")
            }
            Self::BookmarkFailed { path, reason } => {
                write!(
                    f,
                    "Failed to process bookmarks for: {}\n  Reason: {}",
                    path.display(),
                    reason
                )
            }
            Self::MetadataFailed { reason } => {
                write!(f, "Failed to set metadata: {reason}")
            }
            Self::InvalidConfig { message } => {
                write!(f, "Invalid configuration: {message}")
            }
            Self::Cancelled => {
                write!(f, "Operation cancelled by user")
            }
            Self::Io { source } => {
                write!(f, "I/O error: {source}")
            }
            Self::Other { message } => {
                write!(f, "{message}")
            }
        }
    }
}

impl std::error::Error for PdfCatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FileNotAccessible { source, .. } => Some(source),
            Self::FailedToCreateOutput { source, .. } => Some(source),
            Self::FailedToWrite { source, .. } => Some(source),
            Self::FailedToReadInputList { source, .. } => Some(source),
            Self::Io { source } => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for PdfCatError {
    fn from(err: io::Error) -> Self {
        Self::Io { source: err }
    }
}

impl From<lopdf::Error> for PdfCatError {
    fn from(err: lopdf::Error) -> Self {
        Self::other(err.to_string())
    }
}

impl From<anyhow::Error> for PdfCatError {
    fn from(err: anyhow::Error) -> Self {
        Self::other(err.to_string())
    }
}

impl PdfCatError {
    /// Create a FileNotFound error.
    pub fn file_not_found(path: PathBuf) -> Self {
        Self::FileNotFound { path }
    }

    /// Create a NotAFile error.
    pub fn not_a_file(path: PathBuf) -> Self {
        Self::NotAFile { path }
    }

    /// Create a FailedToLoadPdf error.
    pub fn failed_to_load_pdf(path: PathBuf, reason: impl Into<String>) -> Self {
        Self::FailedToLoadPdf {
            path,
            reason: reason.into(),
        }
    }

    /// Create a CorruptedPdf error.
    pub fn corrupted_pdf(path: PathBuf, details: impl Into<String>) -> Self {
        Self::CorruptedPdf {
            path,
            details: details.into(),
        }
    }

    /// Create an EncryptedPdf error.
    pub fn encrypted_pdf(path: PathBuf) -> Self {
        Self::EncryptedPdf { path }
    }

    /// Create an OutputExists error.
    pub fn output_exists(path: PathBuf) -> Self {
        Self::OutputExists { path }
    }

    /// Create a MergeFailed error.
    pub fn merge_failed(reason: impl Into<String>) -> Self {
        Self::MergeFailed {
            reason: reason.into(),
        }
    }

    /// Create an InvalidConfig error.
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfig {
            message: message.into(),
        }
    }

    /// Create an Other error with a custom message.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
        }
    }

    /// Check if this error is recoverable (operation can continue).
    ///
    /// Returns true for errors that might be acceptable in continue-on-error mode.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::FailedToLoadPdf { .. }
                | Self::CorruptedPdf { .. }
                | Self::EncryptedPdf { .. }
                | Self::InvalidPageRange { .. }
                | Self::BookmarkFailed { .. }
        )
    }

    /// Check if this error should stop all processing immediately.
    ///
    /// Returns true for fatal errors that should always terminate.
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::NoFilesToMerge
                | Self::FailedToCreateOutput { .. }
                | Self::FailedToWrite { .. }
                | Self::Cancelled
        )
    }

    /// Get the exit code for this error.
    ///
    /// Returns the appropriate process exit code based on error type.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::FileNotFound { .. } => 2,
            Self::FileNotAccessible { .. } => 2,
            Self::NotAFile { .. } => 2,
            Self::FailedToLoadPdf { .. } => 3,
            Self::CorruptedPdf { .. } => 3,
            Self::EncryptedPdf { .. } => 3,
            Self::NoFilesToMerge => 1,
            Self::OutputExists { .. } => 4,
            Self::FailedToCreateOutput { .. } => 5,
            Self::FailedToWrite { .. } => 5,
            Self::FailedToReadInputList { .. } => 2,
            Self::InvalidInputList { .. } => 1,
            Self::InvalidPageRange { .. } => 1,
            Self::MergeFailed { .. } => 6,
            Self::BookmarkFailed { .. } => 6,
            Self::MetadataFailed { .. } => 6,
            Self::InvalidConfig { .. } => 1,
            Self::Cancelled => 130, // Standard exit code for SIGINT
            Self::Io { .. } => 5,
            Self::Other { .. } => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{error::Error, io};

    #[test]
    fn test_file_not_found_display() {
        let err = PdfCatError::file_not_found(PathBuf::from("/tmp/missing.pdf"));
        let msg = format!("{err}");
        assert!(msg.contains("File not found"));
        assert!(msg.contains("missing.pdf"));
    }

    #[test]
    fn test_failed_to_load_pdf_display() {
        let err = PdfCatError::failed_to_load_pdf(PathBuf::from("bad.pdf"), "Invalid PDF header");
        let msg = format!("{err}");
        assert!(msg.contains("Failed to load PDF"));
        assert!(msg.contains("bad.pdf"));
        assert!(msg.contains("Invalid PDF header"));
    }

    #[test]
    fn test_encrypted_pdf_display() {
        let err = PdfCatError::encrypted_pdf(PathBuf::from("secret.pdf"));
        let msg = format!("{err}");
        assert!(msg.contains("encrypted"));
        assert!(msg.contains("secret.pdf"));
        assert!(msg.contains("Decrypt")); // Helpful hint
    }

    #[test]
    fn test_output_exists_display() {
        let err = PdfCatError::output_exists(PathBuf::from("existing.pdf"));
        let msg = format!("{err}");
        assert!(msg.contains("already exists"));
        assert!(msg.contains("existing.pdf"));
        assert!(msg.contains("--force")); // Helpful hint
    }

    #[test]
    fn test_invalid_page_range_display() {
        let err = PdfCatError::InvalidPageRange {
            path: PathBuf::from("doc.pdf"),
            range: "1-100".to_string(),
            total_pages: 10,
        };
        let msg = format!("{err}");
        assert!(msg.contains("Invalid page range"));
        assert!(msg.contains("1-100"));
        assert!(msg.contains("doc.pdf"));
        assert!(msg.contains("10"));
    }

    #[test]
    fn test_is_recoverable() {
        assert!(
            PdfCatError::failed_to_load_pdf(PathBuf::from("bad.pdf"), "error").is_recoverable()
        );
        assert!(PdfCatError::corrupted_pdf(PathBuf::from("bad.pdf"), "error").is_recoverable());
        assert!(PdfCatError::encrypted_pdf(PathBuf::from("secret.pdf")).is_recoverable());

        assert!(!PdfCatError::NoFilesToMerge.is_recoverable());
        assert!(!PdfCatError::Cancelled.is_recoverable());
    }

    #[test]
    fn test_is_fatal() {
        assert!(PdfCatError::NoFilesToMerge.is_fatal());
        assert!(PdfCatError::Cancelled.is_fatal());
        assert!(
            PdfCatError::FailedToCreateOutput {
                path: PathBuf::from("out.pdf"),
                source: io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
            }
            .is_fatal()
        );

        assert!(!PdfCatError::failed_to_load_pdf(PathBuf::from("bad.pdf"), "error").is_fatal());
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(
            PdfCatError::file_not_found(PathBuf::from("x")).exit_code(),
            2
        );
        assert_eq!(
            PdfCatError::failed_to_load_pdf(PathBuf::from("x"), "error").exit_code(),
            3
        );
        assert_eq!(PdfCatError::NoFilesToMerge.exit_code(), 1);
        assert_eq!(
            PdfCatError::output_exists(PathBuf::from("x")).exit_code(),
            4
        );
        assert_eq!(PdfCatError::Cancelled.exit_code(), 130);
    }

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "not found");
        let err: PdfCatError = io_err.into();
        assert!(matches!(err, PdfCatError::Io { .. }));
    }

    #[test]
    fn test_error_source() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let err = PdfCatError::FileNotAccessible {
            path: PathBuf::from("test.pdf"),
            source: io_err,
        };
        assert!(err.source().is_some());

        let err = PdfCatError::NoFilesToMerge;
        assert!(err.source().is_none());
    }

    #[test]
    fn test_builder_methods() {
        let err = PdfCatError::file_not_found(PathBuf::from("test.pdf"));
        assert!(matches!(err, PdfCatError::FileNotFound { .. }));

        let err = PdfCatError::merge_failed("test reason");
        assert!(matches!(err, PdfCatError::MergeFailed { .. }));

        let err = PdfCatError::invalid_config("test message");
        assert!(matches!(err, PdfCatError::InvalidConfig { .. }));

        let err = PdfCatError::other("generic error");
        assert!(matches!(err, PdfCatError::Other { .. }));
    }
}
