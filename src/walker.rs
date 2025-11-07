use crate::error::PdfError;
use std::path::PathBuf;

pub fn resolve_pdf_paths<'a, T>(patterns: T) -> Result<Vec<PathBuf>, PdfError>
where
    T: IntoIterator,
    T::Item: AsRef<str>,
{
    let mut resolved_paths = Vec::new();

    for pattern in patterns.into_iter() {
        let pattern = pattern.as_ref();

        match glob::glob(pattern) {
            Ok(paths) => {
                for entry in paths {
                    match entry {
                        Ok(path) => resolved_paths.push(path),
                        Err(e) => return Err(PdfError::FailedToProcessGlobEntry(e)),
                    }
                }
            }
            Err(e) => return Err(PdfError::FailedToParseGlobPattern(e)),
        }
    }

    Ok(resolved_paths)
}
