use crate::{PdfCatError, Result};
use std::path::Path;

pub struct PdfReaderV1;

impl PdfReaderV1 {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<lopdf::Document> {
        let path = path.as_ref();
        let doc = lopdf::Document::load(path).map_err(|err| PdfCatError::FailedToLoadPdf {
            path: path.to_path_buf(),
            reason: err.to_string(),
        })?;
        Ok(doc)
    }

    pub fn check_path_exists<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = path.as_ref();
        let exists = path.try_exists()?;
        if !exists {
            return Err(PdfCatError::file_not_found(path.to_path_buf()));
        }

        if path.is_dir() {
            return Err(PdfCatError::not_a_file(path.to_path_buf()));
        }

        Ok(())
    }
}
