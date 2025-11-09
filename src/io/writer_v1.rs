use std::io::{BufWriter, Write};
use std::path::Path;

use lopdf::Document;

use crate::Result;

/// A utility struct responsible for serializing a PDF document to a file.
pub struct PdfWriterV1;

impl PdfWriterV1 {
    /// Writes the given PDF [`Document`] to the specified file path.
    ///
    /// This function handles creating any necessary parent directories for the file
    /// before performing the write operation. It uses a buffered writer for efficiency.
    ///
    /// # Errors
    ///
    /// Returns a [`PdfWriteError`] if:
    ///
    /// * The parent directories cannot be created (e.g., due to permissions).
    /// * The file cannot be created or opened for writing.
    /// * An I/O error occurs during the document serialization or flushing to disk.
    pub fn write<P: AsRef<Path>>(doc: &mut Document, path: P) -> Result<()> {
        let path = path.as_ref();

        // Ensure the containing directory exists.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open the file and wrap it in a BufWriter for efficient disk I/O.
        let file = std::fs::File::create(path)?;
        let mut writer = BufWriter::new(file);

        doc.save_to(&mut writer)?;

        // Flush the buffered writer to ensure all data is written to the file system.
        writer.flush()?;

        Ok(())
    }
}
