//! I/O operations for pdfcat.
//!
//! This module handles all file I/O operations including:
//! - Loading PDF documents from disk
//! - Writing merged PDFs to disk
//! - Parallel PDF loading
//! - Memory-efficient file handling
//!
//! # Examples
//!
//! ```no_run
//! use pdfcat::io::{PdfReader, PdfWriter};
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let reader = PdfReader::new();
//! let doc = reader.load(&PathBuf::from("input.pdf")).await?;
//!
//! let writer = PdfWriter::new();
//! writer.save(&doc.document, &PathBuf::from("output.pdf")).await?;
//! # Ok(())
//! # }
//! ```

pub mod reader;
pub mod writer;

pub use reader::{LoadResult, LoadStatistics, LoadedPdf, PdfReader};
pub use writer::PdfWriter;

use crate::error::Result;
use lopdf::Document;
use std::path::Path;

/// Load a PDF document from a file.
///
/// Convenience function for loading a single PDF.
///
/// # Arguments
///
/// * `path` - Path to the PDF file
///
/// # Errors
///
/// Returns an error if the file cannot be read or is not a valid PDF.
///
/// # Examples
///
/// ```no_run
/// use pdfcat::io::load_pdf;
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let doc = load_pdf(Path::new("document.pdf")).await?;
/// println!("Loaded PDF with {} pages", doc.get_pages().len());
/// # Ok(())
/// # }
/// ```
pub async fn load_pdf(path: &Path) -> Result<Document> {
    let reader = PdfReader::new();
    let loaded = reader.load(path).await?;
    Ok(loaded.document)
}

/// Save a PDF document to a file.
///
/// Convenience function for saving a single PDF.
///
/// # Arguments
///
/// * `doc` - PDF document to save
/// * `path` - Output file path
///
/// # Errors
///
/// Returns an error if the file cannot be written.
///
/// # Examples
///
/// ```no_run
/// use pdfcat::io::save_pdf;
/// use lopdf::Document;
/// use std::path::Path;
///
/// # async fn example(doc: Document) -> Result<(), Box<dyn std::error::Error>> {
/// save_pdf(&doc, Path::new("output.pdf")).await?;
/// # Ok(())
/// # }
/// ```
pub async fn save_pdf(doc: &Document, path: &Path) -> Result<()> {
    let writer = PdfWriter::new();
    writer.save(doc, path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_minimal_pdf(dir: &TempDir, name: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let mut file = std::fs::File::create(&path).unwrap();

        // Minimal valid PDF
        let pdf_content = std::fs::read("tests/fixtures/basic.pdf").unwrap();

        file.write_all(&pdf_content).unwrap();
        path
    }

    #[tokio::test]
    async fn test_load_pdf_convenience() {
        let temp_dir = TempDir::new().unwrap();
        let pdf_path = create_minimal_pdf(&temp_dir, "test.pdf");

        let doc = load_pdf(&pdf_path).await.unwrap();
        assert_eq!(doc.get_pages().len(), 1);
    }

    #[tokio::test]
    async fn test_save_pdf_convenience() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = create_minimal_pdf(&temp_dir, "input.pdf");
        let output_path = temp_dir.path().join("output.pdf");

        let doc = load_pdf(&input_path).await.unwrap();
        save_pdf(&doc, &output_path).await.unwrap();

        assert!(output_path.exists());
    }
}
