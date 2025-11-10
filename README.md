# pdfcat

**Concatenate PDF files into a single document**.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

## Features

✨ **High Quality**

- Direct PDF object copying (no re-rendering)
- Preserves images, fonts, annotations, and form fields
- No quality loss

📋 **Comprehensive Functionality**

- Merge multiple PDFs while preserving order
- Extract specific page ranges
- Rotate pages (90°, 180°, 270°)
- Add bookmarks for navigation
- Set custom metadata (title, author, subject, keywords)
- Configurable compression (none, standard, maximum)

🚀 **Performance**

- Parallel PDF loading
- Efficient memory usage
- Optimized for large files

🛡️ **Robust**

- Comprehensive error handling
- Continue-on-error mode
- Input validation
- Dry-run capability

## Installation

### From Source

```bash
git clone https://github.com/pdfcat/pdfcat
cd pdfcat
cargo build --release
sudo cp target/release/pdfcat /usr/local/bin/
```

### From Cargo

```bash
cargo install pdfcat
```

## Quick Start

### Basic Usage

Merge two PDFs:

```bash
pdfcat file1.pdf file2.pdf -o merged.pdf
```

Merge all PDFs in a directory:

```bash
pdfcat *.pdf -o combined.pdf
```

### Common Options

**Add bookmarks:**

```bash
pdfcat chapter*.pdf -o book.pdf --bookmarks
```

**Extract specific pages:**

```bash
pdfcat document.pdf -o excerpt.pdf --pages "1-5,10,15-20"
```

**Rotate pages:**

```bash
pdfcat scan.pdf -o rotated.pdf --rotate 90
```

**Set metadata:**

```bash
pdfcat files*.pdf -o output.pdf \
  --title "Combined Report" \
  --author "John Doe" \
  --subject "Q4 Results"
```

**Maximum compression:**

```bash
pdfcat large*.pdf -o compressed.pdf --compression maximum
```

**Dry run (validate without creating output):**

```bash
pdfcat file1.pdf file2.pdf -o test.pdf --dry-run
```

## Usage

```
pdfcat [OPTIONS] <FILE>... -o <FILE>

Arguments:
  <FILE>...  Input PDF files to merge (in order)

Options:
  -o, --output <FILE>              Output PDF file path
  -n, --dry-run                    Validate inputs without creating output
  -v, --verbose                    Show detailed information
  -q, --quiet                      Suppress non-error output
  -f, --force                      Overwrite existing output without confirmation
      --no-clobber                 Never overwrite existing output
  -b, --bookmarks                  Add bookmarks for each merged document
  -c, --compression <LEVEL>        Compression level [default: standard]
                                   [possible values: none, standard, maximum]
      --title <TEXT>               Set title metadata
      --author <TEXT>              Set author metadata
      --subject <TEXT>             Set subject metadata
      --keywords <TEXT>            Set keywords metadata
      --continue-on-error          Continue if some PDFs fail to load
      --input-list <FILE>          Read input file list from file
  -j, --jobs <N>                   Number of parallel jobs
      --pages <RANGE>              Page ranges to extract (e.g., "1-5,10")
      --rotate <DEGREES>           Rotate pages [possible values: 90, 180, 270]
  -h, --help                       Print help
  -V, --version                    Print version
```

## Examples

### Academic Paper Assembly

Merge chapters with bookmarks and metadata:

```bash
pdfcat \
  chapter1.pdf \
  chapter2.pdf \
  chapter3.pdf \
  references.pdf \
  -o dissertation.pdf \
  --bookmarks \
  --title "My Dissertation" \
  --author "Jane Smith" \
  --subject "Computer Science" \
  --keywords "AI, Machine Learning, Neural Networks"
```

### Report Generation

Merge report sections with compression:

```bash
pdfcat \
  cover.pdf \
  executive-summary.pdf \
  sections/*.pdf \
  appendix.pdf \
  -o quarterly-report.pdf \
  --compression maximum \
  --bookmarks
```

### Document Processing Pipeline

Process multiple PDFs with error handling:

```bash
pdfcat invoices/*.pdf \
  -o all-invoices.pdf \
  --continue-on-error \
  --quiet
```

### Extract and Merge Specific Pages

```bash
# Extract first 5 pages from each document
pdfcat doc1.pdf doc2.pdf doc3.pdf \
  -o summary.pdf \
  --pages "1-5"
```

### Batch Processing with Input List

Create a file list:

```bash
echo "report1.pdf" > files.txt
echo "report2.pdf" >> files.txt
echo "report3.pdf" >> files.txt
```

Merge from list:

```bash
pdfcat --input-list files.txt -o combined.pdf
```

## Performance

Benchmarks on a typical laptop (Intel i7, 16GB RAM):

| Operation               | Files | Total Pages | Time  |
| ----------------------- | ----- | ----------- | ----- |
| Basic merge             | 10    | 500         | ~2s   |
| With bookmarks          | 10    | 500         | ~2.5s |
| Parallel load (4 cores) | 50    | 2,500       | ~8s   |
| Maximum compression     | 10    | 500         | ~4s   |

Memory usage scales linearly with document size, typically:

- Small PDFs (< 1MB): ~10MB RAM
- Medium PDFs (5-10MB): ~50MB RAM
- Large PDFs (50MB+): ~200MB RAM

## Testing

Run the test suite:

```bash
cargo test
```

Run benchmarks:

```bash
cargo bench
```

## Library Usage

pdfcat can also be used as a library:

```rust
use pdfcat::merge;
use pdfcat::config::{Config, CompressionLevel, Metadata, OverwriteMode};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        inputs: vec![
            PathBuf::from("file1.pdf"),
            PathBuf::from("file2.pdf"),
        ],
        output: PathBuf::from("merged.pdf"),
        dry_run: false,
        verbose: false,
        overwrite_mode: OverwriteMode::Force,
        quiet: false,
        bookmarks: true,
        compression: CompressionLevel::Standard,
        metadata: Metadata::default(),
        continue_on_error: false,
        jobs: None,
        page_range: None,
        rotation: None,
    };

    let (document, stats) = merge::merge_pdfs(&config).await?;
    println!("Merged {} pages", stats.total_pages);

    Ok(())
}
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

Licensed under [AGPL-3.0](LICENSE).

## Acknowledgments

Built with:

- [clap](https://github.com/clap-rs/clap) - CLI argument parsing
- [lopdf](https://github.com/J-F-Liu/lopdf) - PDF manipulation
- [tokio](https://github.com/tokio-rs/tokio) - Async runtime

## Roadmap

...TBD

## Support

- 📖 [Documentation](https://docs.rs/pdfcat)
- 🐛 [Issue Tracker](https://github.com/pdfcat/pdfcat/issues)
- 💬 [Discussions](https://github.com/pdfcat/pdfcat/discussions)
