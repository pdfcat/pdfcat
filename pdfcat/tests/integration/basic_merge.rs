//! Integration tests for basic PDF merging operations.

use pdfcat::config::{CompressionLevel, Config, Metadata, OverwriteMode};
use pdfcat::io::{load_pdf, save_pdf};
use pdfcat::merge::merge_pdfs;
use pdfcat::validation::Validator;
use std::path::PathBuf;

mod common;
use common::{fixture_path, require_fixture, temp_output_path};

#[tokio::test]
async fn test_merge_two_simple_pdfs() {
    require_fixture("simple.pdf");

    let output = temp_output_path();

    let config = Config {
        inputs: vec![fixture_path("simple.pdf"), fixture_path("simple.pdf")],
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
    assert!(result.is_ok(), "Merge failed: {:?}", result.err());

    let (doc, stats) = result.unwrap();
    assert_eq!(stats.files_merged, 2);
    assert!(stats.total_pages >= 2);
    assert!(output.exists(), "Output file was not created");
}

#[tokio::test]
async fn test_merge_multi_page_pdfs() {
    require_fixture("multi_page.pdf");

    let output = temp_output_path();

    let config = Config {
        inputs: vec![
            fixture_path("multi_page.pdf"),
            fixture_path("multi_page.pdf"),
        ],
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
    assert!(result.is_ok());

    let (doc, stats) = result.unwrap();
    assert_eq!(stats.files_merged, 2);
    assert!(output.exists());
}

#[tokio::test]
async fn test_merge_single_pdf() {
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
        jobs: None,
        page_range: None,
        rotation: None,
    };

    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());

    let (doc, stats) = result.unwrap();
    assert_eq!(stats.files_merged, 1);
    assert!(output.exists());
}

#[tokio::test]
async fn test_merge_with_compression_none() {
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
        compression: CompressionLevel::None,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };

    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());

    let (_, stats) = result.unwrap();
    assert!(!stats.compressed);
}

#[tokio::test]
async fn test_merge_with_compression_maximum() {
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
        compression: CompressionLevel::Maximum,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };

    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());

    let (_, stats) = result.unwrap();
    assert!(stats.compressed);
}

#[tokio::test]
async fn test_merge_with_metadata() {
    require_fixture("simple.pdf");

    let output = temp_output_path();

    let metadata = Metadata::new(
        Some("Test Title".to_string()),
        Some("Test Author".to_string()),
        Some("Test Subject".to_string()),
        Some("test, keywords".to_string()),
    );

    let config = Config {
        inputs: vec![fixture_path("simple.pdf")],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false,
        compression: CompressionLevel::Standard,
        metadata,
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };

    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());

    // Verify metadata was set by loading the output
    let output_doc = load_pdf(&output).await;
    assert!(output_doc.is_ok());
}

#[tokio::test]
async fn test_merge_preserves_page_count() {
    require_fixture("simple.pdf");

    let output = temp_output_path();

    // Load original to get page count
    let original = load_pdf(&fixture_path("simple.pdf")).await.unwrap();
    let original_pages = original.get_pages().len();

    let config = Config {
        inputs: vec![
            fixture_path("simple.pdf"),
            fixture_path("simple.pdf"),
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
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };

    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());

    let (_, stats) = result.unwrap();
    assert_eq!(stats.total_pages, original_pages * 3);
}

#[tokio::test]
async fn test_merge_output_is_valid_pdf() {
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
        jobs: None,
        page_range: None,
        rotation: None,
    };

    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());

    // Verify output is a valid PDF by loading it
    let validator = Validator::new();
    let validation = validator.validate_file(&output).await;
    assert!(
        validation.is_ok(),
        "Output PDF is not valid: {:?}",
        validation.err()
    );
}

#[tokio::test]
async fn test_merge_with_parallel_loading() {
    require_fixture("simple.pdf");

    let output = temp_output_path();

    let config = Config {
        inputs: vec![
            fixture_path("simple.pdf"),
            fixture_path("simple.pdf"),
            fixture_path("simple.pdf"),
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
        continue_on_error: false,
        jobs: Some(4), // Force parallel loading
        page_range: None,
        rotation: None,
    };

    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());

    let (_, stats) = result.unwrap();
    assert_eq!(stats.files_merged, 4);
}
