#[derive(Debug, thiserror::Error)]
pub enum Error {}

#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("No PDF files to merge")]
    NoFilesToMerge,

    #[error("Failed to load PDF at: {path}")]
    FailedToLoadFromPath { path: String },

    #[error("Failed to write PDF")]
    FailedToWrite,

    #[error("Failed to create PDF output file at: {path}")]
    FailedToCreateOutput { path: String },

    #[error("Failed to process glob entry: {0}")]
    FailedToProcessGlobEntry(#[from] glob::GlobError),

    #[error("Failed to process glob entry: {0}")]
    FailedToParseGlobPattern(#[from] glob::PatternError),
}
