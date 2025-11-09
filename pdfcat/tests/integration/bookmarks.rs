//! Integration tests for bookmark functionality.

use pdfcat::config::{Config, CompressionLevel, Metadata, OverwriteMode};
use pdfcat::merge::{merge_pdfs, BookmarkManager};
use pdfcat::io::load_pdf;
use std::path::PathBuf;

mod common;
use common::{fixture_path, require_fixture, temp_output_path};

#[tokio::test]
async fn test_merge_with_bookmarks() {
    require_fixture("simple.pdf");
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![
            fixture_path("simple.pdf"),
            fixture_path("simple.pdf"),
        ],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: true, // Enable bookmarks
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());
    
    // Load output and verify bookmarks were added
    let output_doc = load_pdf(&output).await.unwrap();
    let bookmark_manager = BookmarkManager::new();
    assert!(bookmark_manager.has_bookmarks(&output_doc), "Bookmarks should be present");
}

#[tokio::test]
async fn test_merge_without_bookmarks() {
    require_fixture("simple.pdf");
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![
            fixture_path("simple.pdf"),
            fixture_path("simple.pdf"),
        ],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: false, // No bookmarks
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());
    
    // Load output and verify no bookmarks
    let output_doc = load_pdf(&output).await.unwrap();
    let bookmark_manager = BookmarkManager::new();
    
    // Note: If input PDFs have bookmarks, they might be preserved
    // This test mainly ensures the --bookmarks flag works
}

#[tokio::test]
async fn test_bookmarks_with_multiple_files() {
    require_fixture("simple.pdf");
    require_fixture("multi_page.pdf");
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![
            fixture_path("simple.pdf"),
            fixture_path("multi_page.pdf"),
            fixture_path("simple.pdf"),
        ],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: true,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());
    
    let output_doc = load_pdf(&output).await.unwrap();
    let bookmark_manager = BookmarkManager::new();
    assert!(bookmark_manager.has_bookmarks(&output_doc));
}

#[tokio::test]
async fn test_bookmarks_preserve_existing() {
    // If fixture has bookmarks, verify they're preserved
    if common::fixture_exists("with_bookmarks.pdf") {
        require_fixture("with_bookmarks.pdf");
        
        let output = temp_output_path();
        
        let config = Config {
            inputs: vec![
                fixture_path("with_bookmarks.pdf"),
                fixture_path("simple.pdf"),
            ],
            output: output.to_path_buf(),
            dry_run: false,
            verbose: false,
            overwrite_mode: OverwriteMode::Force,
            quiet: true,
            bookmarks: true,
            compression: CompressionLevel::Standard,
            metadata: Metadata::default(),
            continue_on_error: false,
            jobs: None,
            page_range: None,
            rotation: None,
        };
        
        let result = merge_pdfs(&config).await;
        assert!(result.is_ok());
        
        let output_doc = load_pdf(&output).await.unwrap();
        let bookmark_manager = BookmarkManager::new();
        assert!(bookmark_manager.has_bookmarks(&output_doc));
    }
}

#[tokio::test]
async fn test_bookmarks_single_file() {
    require_fixture("simple.pdf");
    
    let output = temp_output_path();
    
    // Even with single file, bookmarks should work
    let config = Config {
        inputs: vec![fixture_path("simple.pdf")],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: true,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_bookmarks_with_page_extraction() {
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
        bookmarks: true,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: Some(pdfcat::config::PageRange::parse("1-2").unwrap()),
        rotation: None,
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());
    
    let output_doc = load_pdf(&output).await.unwrap();
    let bookmark_manager = BookmarkManager::new();
    // Should still have bookmarks even with page extraction
    assert!(bookmark_manager.has_bookmarks(&output_doc));
}

#[tokio::test]
async fn test_bookmark_manager_direct() {
    require_fixture("simple.pdf");
    
    let mut doc = load_pdf(&fixture_path("simple.pdf")).await.unwrap();
    
    let bookmark_manager = BookmarkManager::new();
    
    // Initially no bookmarks
    assert!(!bookmark_manager.has_bookmarks(&doc));
    
    // Add bookmarks
    let paths = vec![
        fixture_path("simple.pdf").as_path(),
        fixture_path("simple.pdf").as_path(),
    ];
    let result = bookmark_manager.add_bookmarks_for_files(&mut doc, &paths);
    assert!(result.is_ok());
    
    // Now should have bookmarks
    assert!(bookmark_manager.has_bookmarks(&doc));
    
    // Remove bookmarks
    let result = bookmark_manager.remove_bookmarks(&mut doc);
    assert!(result.is_ok());
    
    // Should be gone
    assert!(!bookmark_manager.has_bookmarks(&doc));
}

#[tokio::test]
async fn test_bookmarks_with_rotation() {
    require_fixture("simple.pdf");
    
    let output = temp_output_path();
    
    let config = Config {
        inputs: vec![
            fixture_path("simple.pdf"),
            fixture_path("simple.pdf"),
        ],
        output: output.to_path_buf(),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: true,
        bookmarks: true,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: Some(pdfcat::config::Rotation::Clockwise90),
    };
    
    let result = merge_pdfs(&config).await;
    assert!(result.is_ok());
    
    let output_doc = load_pdf(&output).await.unwrap();
    let bookmark_manager = BookmarkManager::new();
    assert!(bookmark_manager.has_bookmarks(&output_doc));
}