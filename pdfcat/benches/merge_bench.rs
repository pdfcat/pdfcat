//! Performance benchmarks for pdfcat.
//!
//! Run with: cargo bench
//!
//! These benchmarks measure the performance of core operations
//! using criterion for statistical analysis.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use pdfcat::config::{CompressionLevel, Config, Metadata, OverwriteMode};
use pdfcat::io::{PdfReader, PdfWriter};
use pdfcat::merge::merge_pdfs;
use pdfcat::validation::Validator;
use std::path::PathBuf;
use tempfile::TempDir;

/// Get fixture path for benchmarks
fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    path
}

/// Benchmark: Load a single PDF
fn bench_load_single_pdf(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let reader = PdfReader::new();
    let path = fixture_path("basic.pdf");

    if !path.exists() {
        eprintln!("Skipping benchmark - fixture not found: {}", path.display());
        return;
    }

    c.bench_function("load_single_pdf", |b| {
        b.to_async(&rt).iter(|| async {
            let result = reader.load(black_box(&path)).await;
            assert!(result.is_ok());
            result.unwrap()
        });
    });
}

/// Benchmark: Load multiple PDFs sequentially
fn bench_load_sequential(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let reader = PdfReader::new();
    let paths = vec![
        fixture_path("basic.pdf"),
        fixture_path("basic.pdf"),
        fixture_path("basic.pdf"),
    ];

    if !paths[0].exists() {
        eprintln!("Skipping benchmark - fixture not found");
        return;
    }

    c.bench_function("load_sequential_3_files", |b| {
        b.to_async(&rt).iter(|| async {
            let results = reader.load_sequential(black_box(&paths)).await;
            assert_eq!(results.len(), 3);
        });
    });
}

/// Benchmark: Load multiple PDFs in parallel
fn bench_load_parallel(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let reader = PdfReader::new();
    let paths = vec![
        fixture_path("basic.pdf"),
        fixture_path("basic.pdf"),
        fixture_path("basic.pdf"),
        fixture_path("basic.pdf"),
    ];

    if !paths[0].exists() {
        eprintln!("Skipping benchmark - fixture not found");
        return;
    }

    let mut group = c.benchmark_group("load_parallel");

    for workers in [1, 2, 4].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_workers", workers)),
            workers,
            |b, &workers| {
                b.to_async(&rt).iter(|| async {
                    let results = reader.load_parallel(black_box(&paths), workers).await;
                    assert_eq!(results.len(), 4);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Validate a PDF
fn bench_validate_pdf(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let validator = Validator::new();
    let path = fixture_path("basic.pdf");

    if !path.exists() {
        eprintln!("Skipping benchmark - fixture not found");
        return;
    }

    c.bench_function("validate_pdf", |b| {
        b.to_async(&rt).iter(|| async {
            let result = validator.validate_file(black_box(&path)).await;
            assert!(result.is_ok());
        });
    });
}

/// Benchmark: Merge two PDFs
fn bench_merge_two_pdfs(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    let path = fixture_path("basic.pdf");
    if !path.exists() {
        eprintln!("Skipping benchmark - fixture not found");
        return;
    }

    c.bench_function("merge_two_pdfs", |b| {
        b.to_async(&rt).iter(|| async {
            let output = temp_dir
                .path()
                .join(format!("out_{}.pdf", rand::random::<u32>()));

            let config = Config {
                inputs: vec![path.clone(), path.clone()],
                output: output.clone(),
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

            let result = merge_pdfs(black_box(&config)).await;
            assert!(result.is_ok());
        });
    });
}

/// Benchmark: Merge with different compression levels
fn bench_merge_compression(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    let path = fixture_path("basic.pdf");
    if !path.exists() {
        eprintln!("Skipping benchmark - fixture not found");
        return;
    }

    let mut group = c.benchmark_group("merge_compression");

    for level in [
        CompressionLevel::None,
        CompressionLevel::Standard,
        CompressionLevel::Maximum,
    ]
    .iter()
    {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", level)),
            level,
            |b, &level| {
                b.to_async(&rt).iter(|| async {
                    let output = temp_dir
                        .path()
                        .join(format!("out_{}.pdf", rand::random::<u32>()));

                    let config = Config {
                        inputs: vec![path.clone()],
                        output: output.clone(),
                        dry_run: false,
                        verbose: false,
                        overwrite_mode: OverwriteMode::Force,
                        quiet: true,
                        bookmarks: false,
                        compression: level,
                        metadata: Metadata::default(),
                        continue_on_error: false,
                        jobs: None,
                        page_range: None,
                        rotation: None,
                    };

                    let result = merge_pdfs(black_box(&config)).await;
                    assert!(result.is_ok());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Merge with bookmarks
fn bench_merge_with_bookmarks(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    let path = fixture_path("basic.pdf");
    if !path.exists() {
        eprintln!("Skipping benchmark - fixture not found");
        return;
    }

    c.bench_function("merge_with_bookmarks", |b| {
        b.to_async(&rt).iter(|| async {
            let output = temp_dir
                .path()
                .join(format!("out_{}.pdf", rand::random::<u32>()));

            let config = Config {
                inputs: vec![path.clone(), path.clone(), path.clone()],
                output: output.clone(),
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

            let result = merge_pdfs(black_box(&config)).await;
            assert!(result.is_ok());
        });
    });
}

/// Benchmark: Write PDF with different options
fn bench_write_pdf(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    let path = fixture_path("basic.pdf");
    if !path.exists() {
        eprintln!("Skipping benchmark - fixture not found");
        return;
    }

    // Load once for reuse
    let doc = rt.block_on(async { pdfcat::io::load_pdf(&path).await.unwrap() });

    let mut group = c.benchmark_group("write_pdf");

    // Atomic write
    group.bench_function("atomic", |b| {
        b.to_async(&rt).iter(|| async {
            let output = temp_dir
                .path()
                .join(format!("out_{}.pdf", rand::random::<u32>()));
            let writer = PdfWriter::new();
            let result = writer.save(black_box(&doc), &output).await;
            assert!(result.is_ok());
        });
    });

    // Non-atomic write
    group.bench_function("non_atomic", |b| {
        b.to_async(&rt).iter(|| async {
            let output = temp_dir
                .path()
                .join(format!("out_{}.pdf", rand::random::<u32>()));
            let writer = PdfWriter::non_atomic();
            let result = writer.save(black_box(&doc), &output).await;
            assert!(result.is_ok());
        });
    });

    group.finish();
}

/// Benchmark: Merge scaling with number of files
fn bench_merge_scaling(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    let path = fixture_path("basic.pdf");
    if !path.exists() {
        eprintln!("Skipping benchmark - fixture not found");
        return;
    }

    let mut group = c.benchmark_group("merge_scaling");

    for count in [2, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_files", count)),
            count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let output = temp_dir
                        .path()
                        .join(format!("out_{}.pdf", rand::random::<u32>()));
                    let inputs = vec![path.clone(); count];

                    let config = Config {
                        inputs,
                        output: output.clone(),
                        dry_run: false,
                        verbose: false,
                        overwrite_mode: OverwriteMode::Force,
                        quiet: true,
                        bookmarks: false,
                        compression: CompressionLevel::Standard,
                        metadata: Metadata::default(),
                        continue_on_error: false,
                        jobs: Some(4),
                        page_range: None,
                        rotation: None,
                    };

                    let result = merge_pdfs(black_box(&config)).await;
                    assert!(result.is_ok());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_load_single_pdf,
    bench_load_sequential,
    bench_load_parallel,
    bench_validate_pdf,
    bench_merge_two_pdfs,
    bench_merge_compression,
    bench_merge_with_bookmarks,
    bench_write_pdf,
    bench_merge_scaling,
);

criterion_main!(benches);
