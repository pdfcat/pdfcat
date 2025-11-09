//! Integration tests for dry-run functionality.

use pdfcat::config::{Config, CompressionLevel, Metadata, OverwriteMode};
use pdfcat::validation::Validator;
use std::path::PathBuf;

mod common;
use common::{fixture_path, require_fixture, temp_output_path};

#[tokio::test]
async fn test_dry_run_does_not_create_output() {
    require_fixture("simple.pdf");
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![fixture_path("simple.pdf")],
        output: output.to_path_buf(),
        dry_run: true, // DRY RUN
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
    let result = validator.validate_config(&config).await;
    
    assert!(result.is_ok(), "Validation failed: {:?}", result.err());
    assert!(!output.exists(), "Output file should not be created in dry run");
}

#[tokio::test]
async fn test_dry_run_validates_all_inputs() {
    require_fixture("simple.pdf");
    require_fixture("multi_page.pdf");
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![
            fixture_path("simple.pdf"),
            fixture_path("multi_page.pdf"),
        ],
        output: output.to_path_buf(),
        dry_run: true,
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
    let result = validator.validate_config(&config).await;
    
    assert!(result.is_ok());
    let summary = result.unwrap();
    assert_eq!(summary.files_validated, 2);
    assert!(summary.total_pages > 0);
}

#[tokio::test]
async fn test_dry_run_detects_missing_files() {
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![
            PathBuf::from("/nonexistent/file1.pdf"),
            PathBuf::from("/nonexistent/file2.pdf"),
        ],
        output: output.to_path_buf(),
        dry_run: true,
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
    let result = validator.validate_config(&config).await;
    
    assert!(result.is_err(), "Should fail with missing files");
}

#[tokio::test]
async fn test_dry_run_detects_corrupted_files() {
    // Create a corrupted PDF (empty file)
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let corrupted_path = temp_file.path().to_path_buf();
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![corrupted_path],
        output: output.to_path_buf(),
        dry_run: true,
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
    let result = validator.validate_config(&config).await;
    
    assert!(result.is_err(), "Should fail with corrupted file");
}

#[tokio::test]
async fn test_dry_run_validates_output_directory() {
    require_fixture("simple.pdf");
    
    let config = Config {
        inputs: vec![fixture_path("simple.pdf")],
        output: PathBuf::from("/nonexistent/directory/output.pdf"),
        dry_run: true,
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
async fn test_dry_run_with_continue_on_error() {
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
        dry_run: true,
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
    
    let validator = Validator::new();
    let result = validator.validate_config(&config).await;
    
    // Should succeed with 2 valid files
    assert!(result.is_ok());
    let summary = result.unwrap();
    assert_eq!(summary.files_validated, 2);
    assert_eq!(summary.files_failed, 1);
}

#[tokio::test]
async fn test_dry_run_reports_total_pages() {
    require_fixture("simple.pdf");
    require_fixture("multi_page.pdf");
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![
            fixture_path("simple.pdf"),
            fixture_path("multi_page.pdf"),
        ],
        output: output.to_path_buf(),
        dry_run: true,
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
    let result = validator.validate_config(&config).await;
    
    assert!(result.is_ok());
    let summary = result.unwrap();
    assert!(summary.total_pages > 0);
    assert!(summary.total_size > 0);
}

#[tokio::test]
async fn test_dry_run_with_page_range_validation() {
    require_fixture("simple.pdf");
    
    let output = temp_output_path();
    
    // Request pages that don't exist
    let config = Config {
        inputs: vec![fixture_path("simple.pdf")],
        output: output.to_path_buf(),
        dry_run: true,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: Some(pdfcat::config::PageRange::parse("1-1000").unwrap()),
        rotation: None,
    };
    
    let validator = Validator::new();
    let result = validator.validate_config(&config).await;
    
    // Should fail because page range exceeds document
    assert!(result.is_err(), "Should fail with out-of-range pages");
}

#[tokio::test]
async fn test_dry_run_no_clobber_existing_file() {
    require_fixture("simple.pdf");
    
    // Create an existing output file
    let output = tempfile::NamedTempFile::new().unwrap();
    let output_path = output.path().to_path_buf();
    
    let config = Config {
        inputs: vec![fixture_path("simple.pdf")],
        output: output_path,
        dry_run: true,
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
    
    // Should fail with no-clobber when file exists
    assert!(result.is_err(), "Should fail with existing output and no-clobber");
}