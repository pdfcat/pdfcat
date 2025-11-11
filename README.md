# pdfcat

[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

`pdfcat` is a high-performance _command line utilitiy_ to **merge PDF files into a single document** without losing quality.

**You may be looking for**:

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Examples](#examples)
- [Library](#library-usage)

## Features

✨ **Zero Quality Loss**

- Direct PDF object copying (no re-rendering).
- Preserves images, fonts, annotations, and form fields

📋 **Full Control**

- Extract ranges, rotate pages, add bookmarks.
- Set custom metadata (title, author, subject, keywords)
- Configurable compression (none, standard, maximum)

🚀 **High Performance**

- Parallel PDF loading with efficient memory usage
- Optimized for large files

## Installation

**pdfcat** currently supports the following architectures:

- **x86_64** (64-bit Intel/AMD processors)
- **aarch64** (64-bit ARM processors)

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

### Linux (Debian)

> For more details see [installation script](/scripts/install.sh).

```shell
bash -c "$(curl -fsSL https://raw.githubusercontent.com/pdfcat/pdfcat/main/scripts/install.sh)"
```

This script will:

- Automatically detect your machine's architecture
- Download and unpack the necessary .tar
- Copy the `pdfcat` binary to `usr/local/bin`

### Windows 11 (PowerShell)

> For more details see [installation script](/scripts/install.ps1).

Ensure your PowerShell execution policy allows (remote) script execution:

```powershell
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser
```

Executing the following script will walk you through the installation process (you may need to run this command from an elevated shell):

```powershell
Invoke-Expression -Command (Invoke-WebRequest -Uri "https://raw.githubusercontent.com/pdfcat/pdfcat/main/scripts/install.ps1" -UseBasicParsing).Content
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

## Testing

> 📄 **Licensing Note for Test Fixtures**
>
> The PDF files located within the [`pdfcat/tests/fixtures`](./crates/pdfcat/tests/fixtures/) directory are included solely for testing and validation purposes.
>
> These files were sourced from various public domains or created specifically for testing structure and complexity. They belong to their respective original owners/creators.
>
> **Do not copy, distribute, or reuse these fixture files outside of the pdfcat project's testing context.**

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

## Support

- 📖 [Documentation](https://docs.rs/pdfcat)
- 🐛 [Issue Tracker](https://github.com/pdfcat/pdfcat/issues)
