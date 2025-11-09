//! Integration tests for error handling and edge cases.

use pdfcat::config::{Config, CompressionLevel, Metadata, OverwriteMode, PageRange, Rotation};
use pdfcat::merge::merge_pdfs;
use pdfcat::validation::Validator;
use pdfcat::error::PdfCatError;
use std::path::PathBuf;

mod common;
use common::{fixture_path, require_fixture, temp_output_path};

#[tokio::test]
async fn test_error_nonexistent_input() {
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![PathBuf::from("/nonexistent/file.pdf")],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_err(), "Should fail with nonexistent file");
    
    let err = result.unwrap_err();
    assert!(matches!(err, PdfCatError::FileNotFound { .. }));
}

#[tokio::test]
async fn test_error_empty_input_list() {
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let result = config.validate();
    assert!(result.is_err(), "Should fail with empty input list");
}

#[tokio::test]
async fn test_error_corrupted_pdf() {
    // Create a corrupted PDF (empty file)
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let corrupted_path = temp_file.path().to_path_buf();
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![corrupted_path],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_err(), "Should fail with corrupted PDF");
}

#[tokio::test]
async fn test_error_continue_on_error_recovers() {
    require_fixture("simple.pdf");
    
    // Create a corrupted PDF
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let corrupted_path = temp_file.path().to_path_buf();
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![
            fixture_path("simple.pdf"),
            corrupted_path,
            fixture_path("simple.pdf"),
        ],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: true, // Continue on error
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_ok(), "Should succeed with continue-on-error");
    
    let (_, stats) = result.unwrap();
    assert_eq!(stats.files_merged, 2, "Should merge only valid files");
}

#[tokio::test]
async fn test_error_invalid_page_range() {
    require_fixture("simple.pdf");
    
    let output = temp_output_path();
    
    // Parse a page range that's out of bounds
    let config = Config {
        inputs: vec![fixture_path("simple.pdf")],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: Some(PageRange::parse("100-200").unwrap()),
        rotation: None,
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_err(), "Should fail with invalid page range");
}

#[tokio::test]
async fn test_error_output_directory_not_exist() {
    require_fixture("simple.pdf");
    
    let config = Config {
        inputs: vec![fixture_path("simple.pdf")],
        output: PathBuf::from("/nonexistent/directory/output.pdf"),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let validator = Validator::new();
    let result = validator.validate_output(&config).await;
    
    assert!(result.is_err(), "Should fail with nonexistent output directory");
}

#[tokio::test]
async fn test_error_output_same_as_input() {
    require_fixture("simple.pdf");
    
    let input_path = fixture_path("simple.pdf");
    
    let config = Config {
        inputs: vec![input_path.clone()],
        output: input_path, // Same as input!
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let result = config.validate();
    assert!(result.is_err(), "Should fail when output same as input");
}

#[tokio::test]
async fn test_error_no_clobber_with_existing_file() {
    require_fixture("simple.pdf");
    
    // Create an existing output file
    let output = tempfile::NamedTempFile::new().unwrap();
    let output_path = output.path().to_path_buf();
    
    let config = Config {
        inputs: vec![fixture_path("simple.pdf")],
        output: output_path,
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::NoClobber,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let validator = Validator::new();
    let result = validator.validate_output(&config).await;
    
    assert!(result.is_err(), "Should fail with no-clobber and existing file");
    let err = result.unwrap_err();
    assert!(matches!(err, PdfCatError::OutputExists { .. }));
}

#[tokio::test]
async fn test_error_verbose_and_quiet_conflict() {
    require_fixture("simple.pdf");
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![fixture_path("simple.pdf")],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: true,
        overwrite_mode: OverwriteMode::Force,
        quiet: true, // Both verbose and quiet!
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let result = config.validate();
    assert!(result.is_err(), "Should fail with verbose + quiet");
}

#[tokio::test]
async fn test_error_zero_jobs() {
    require_fixture("simple.pdf");
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![fixture_path("simple.pdf")],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: Some(0), // Zero jobs!
        page_range: None,
        rotation: None,
    };
    
    let result = config.validate();
    assert!(result.is_err(), "Should fail with zero jobs");
}

#[tokio::test]
async fn test_error_invalid_page_range_format() {
    let result = PageRange::parse("invalid");
    assert!(result.is_err(), "Should fail with invalid page range format");
    
    let result = PageRange::parse("0");
    assert!(result.is_err(), "Should fail with page 0");
    
    let result = PageRange::parse("5-3");
    assert!(result.is_err(), "Should fail with reversed range");
    
    let result = PageRange::parse("");
    assert!(result.is_err(), "Should fail with empty range");
}

#[tokio::test]
async fn test_error_invalid_rotation() {
    let result = Rotation::from_degrees(45);
    assert!(result.is_err(), "Should fail with invalid rotation");
    
    let result = Rotation::from_degrees(360);
    assert!(result.is_err(), "Should fail with 360 degrees");
}

#[tokio::test]
async fn test_error_all_files_fail_with_continue() {
    // Create multiple corrupted PDFs
    let temp1 = tempfile::NamedTempFile::new().unwrap();
    let temp2 = tempfile::NamedTempFile::new().unwrap();
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![
            temp1.path().to_path_buf(),
            temp2.path().to_path_buf(),
        ],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: true,
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_err(), "Should fail when all files are invalid");
    
    let err = result.unwrap_err();
    assert!(matches!(err, PdfCatError::NoFilesToMerge));
}

#[tokio::test]
async fn test_edge_case_single_page_extraction() {
    require_fixture("multi_page.pdf");
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![fixture_path("multi_page.pdf")],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: Some(PageRange::parse("1").unwrap()),
        rotation: None,
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());
    
    let (_, stats) = result.unwrap();
    assert_eq!(stats.total_pages, 1);
}

#[tokio::test]
async fn test_edge_case_multiple_rotations() {
    require_fixture("simple.pdf");
    
    let output = temp_output_path();
    
    // Rotate 90 degrees (should work)
    let config = Config {
        inputs: vec![fixture_path("simple.pdf")],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: Some(Rotation::Clockwise90),
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());
}