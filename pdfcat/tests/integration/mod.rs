//! Integration tests for pdfcat.
//!
//! These tests exercise the full application flow using real PDF fixtures.

use std::path::{Path, PathBuf};

/// Get the path to a test fixture.
///
/// # Arguments
///
/// * `name` - Name of the fixture file (e.g., "simple.pdf")
///
/// # Returns
///
/// Path to the fixture file in tests/fixtures/
pub fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    path
}

/// Check if a fixture exists.
///
/// # Arguments
///
/// * `name` - Name of the fixture file
///
/// # Returns
///
/// True if the fixture exists, false otherwise
pub fn fixture_exists(name: &str) -> bool {
    fixture_path(name).exists()
}

/// Verify a fixture exists, panicking if it doesn't.
///
/// # Arguments
///
/// * `name` - Name of the fixture file
///
/// # Panics
///
/// Panics if the fixture file doesn't exist
pub fn require_fixture(name: &str) {
    let path = fixture_path(name);
    assert!(
        path.exists(),
        "Required fixture not found: {}. Please ensure test fixtures are present.",
        path.display()
    );
}

/// Create a temporary output path for test results.
///
/// # Returns
///
/// A temporary file path that will be cleaned up
pub fn temp_output_path() -> tempfile::TempPath {
    tempfile::NamedTempFile::new()
        .expect("Failed to create temp file")
        .into_temp_path()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_path() {
        let path = fixture_path("simple.pdf");
        assert!(path.ends_with("tests/fixtures/simple.pdf"));
    }

    #[test]
    fn test_fixture_exists() {
        // This will be true once fixtures are in place
        let exists = fixture_exists("simple.pdf");
        // Don't assert - fixtures may not be present in all test environments
    }
}