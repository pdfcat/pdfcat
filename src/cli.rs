use clap::Parser;
use std::path::PathBuf;

use crate::config::Config;

const USAGE: &str = "\x1b[1mpdfcat\x1b[0m <FILES>... \x1b[1m--output\x1b[0m <FILE> [OPTIONS]
       \x1b[1mpdfcat\x1b[0m file1.pdf file2.pdf \x1b[1m--output\x1b[0m out.pdf
       \x1b[1mpdfcat\x1b[0m mypdfs/*.pdf backup/**/unique/*.pdf \x1b[1m--output\x1b[0m out.pdf
       \x1b[1mpdfcat\x1b[0m **/deeply/nested/*.pdf \x1b[1m--output\x1b[0m out.pdf
       \x1b[1mpdfcat\x1b[0m **/*.pdf \x1b[1m--output\x1b[0m out.pdf
";

#[derive(Parser)]
#[command(name = "pdfcat")]
#[command(version)]
#[command(about = "Concatenate PDF files into a single PDF document", long_about = None)]
#[command(author)]
#[command(
    override_usage = USAGE
)]
pub struct Cli {
    /// Input PDF files to merge (in order)
    ///
    /// Specify multiple files or use glob patterns.
    /// Files are merged in the order provided.
    ///
    /// Examples:
    ///   pdfcat file1.pdf file2.pdf -o output.pdf
    ///   pdfcat chapter*.pdf -o book.pdf
    #[arg(required = true, value_name = "FILE")]
    pub(crate) inputs: Vec<String>,

    /// Output PDF file path
    ///
    /// The merged PDF will be written to this location.
    /// Use --force to overwrite existing files without confirmation.
    #[arg(short, long, value_name = "FILE")]
    pub(crate) output: String,

    /// Dry run - validate inputs and preview merge without creating output
    ///
    /// Validates that all input files exist and are readable PDFs,
    /// then displays what the merge operation would do without
    /// actually creating the output file.
    #[arg(short = 'n', long)]
    pub(crate) dry_run: bool,

    /// Verbose output - show detailed information about each PDF
    ///
    /// Displays additional information including PDF version,
    /// page dimensions, object count, and other metadata
    /// for each input file.
    #[arg(short, long)]
    pub(crate) verbose: bool,

    /// Force overwrite of existing output file without confirmation
    ///
    /// By default, pdfcat will prompt before overwriting an existing file.
    /// Use this flag to skip the confirmation prompt.
    #[arg(short, long)]
    pub(crate) force: bool,

    /// Never overwrite existing output file
    ///
    /// If the output file already exists, exit with an error
    /// instead of prompting or overwriting.
    #[arg(long, conflicts_with = "force")]
    pub(crate) no_clobber: bool,

    /// Suppress all non-error output
    ///
    /// Only errors and warnings will be printed.
    /// Useful for scripts and automation.
    #[arg(short, long, conflicts_with = "verbose")]
    pub(crate) quiet: bool,

    /// Add bookmarks for each merged document
    ///
    /// Creates a bookmark (outline entry) at the start of each
    /// input PDF using the filename as the bookmark title.
    /// Preserves existing bookmarks from source PDFs.
    #[arg(short, long)]
    pub(crate) bookmarks: bool,

    /// Compression level for output PDF
    ///
    /// Controls the compression applied to the merged PDF.
    /// - none: No compression (preserves exact quality)
    /// - standard: Balanced compression (default)
    /// - maximum: Aggressive compression (smaller file size)
    #[arg(short, long, value_name = "LEVEL", default_value = "standard")]
    #[arg(value_parser = ["none", "standard", "maximum"])]
    pub(crate) compression: String,

    /// Set title metadata for output PDF
    ///
    /// If not specified, title from the first input PDF is preserved.
    #[arg(long, value_name = "TEXT")]
    pub(crate) title: Option<String>,

    /// Set author metadata for output PDF
    ///
    /// If not specified, author from the first input PDF is preserved.
    #[arg(long, value_name = "TEXT")]
    pub(crate) author: Option<String>,

    /// Set subject metadata for output PDF
    #[arg(long, value_name = "TEXT")]
    pub(crate) subject: Option<String>,

    /// Set keywords metadata for output PDF (comma-separated)
    #[arg(long, value_name = "TEXT")]
    pub(crate) keywords: Option<String>,

    /// Continue processing even if some PDFs fail to load
    ///
    /// By default, pdfcat stops on the first error.
    /// With this flag, problematic PDFs are skipped with a warning
    /// and processing continues with remaining files.
    #[arg(long)]
    pub(crate) continue_on_error: bool,

    /// Read input file list from a file (one path per line)
    ///
    /// Instead of specifying files on command line, read from a file.
    /// Use '-' to read from stdin. Can be combined with direct inputs.
    ///
    /// Example:
    ///   pdfcat --input-list files.txt -o output.pdf
    #[arg(long, value_name = "FILE")]
    pub(crate) input_list: Option<PathBuf>,

    /// Number of parallel jobs for loading PDFs
    ///
    /// Controls how many PDFs are loaded concurrently.
    /// Default is number of CPU cores. Use 1 for sequential processing.
    #[arg(short, long, value_name = "N")]
    pub(crate) jobs: Option<usize>,

    /// Page ranges to extract from each input (e.g., "1-5,10,15-20")
    ///
    /// Apply the same page range to all input PDFs.
    /// Page numbers are 1-indexed. Use commas to separate ranges.
    ///
    /// Examples:
    ///   --pages "1-10"      # First 10 pages from each PDF
    ///   --pages "1,3,5"     # Pages 1, 3, and 5 from each PDF
    ///   --pages "1-5,10-15" # Pages 1-5 and 10-15 from each PDF
    #[arg(long, value_name = "RANGE")]
    pub(crate) pages: Option<String>,

    /// Rotate pages by specified degrees (90, 180, 270)
    ///
    /// Applies rotation to all pages in all input PDFs.
    #[arg(long, value_name = "DEGREES")]
    #[arg(value_parser = ["90", "180", "270"])]
    pub(crate) rotate: Option<u16>,
}

impl TryFrom<&Cli> for Config {
    type Error = crate::PdfCatError;
    fn try_from(args: &Cli) -> Result<Self, Self::Error> {
        use crate::config::*;

        let inputs = crate::utils::collect_paths_for_patterns(&args.inputs)?;

        let page_range: Option<PageRange> = match args.pages.as_ref() {
            Some(p) => Some(PageRange::parse(&p)?),
            None => None,
        };

        let rotation: Option<Rotation> = match args.rotate.as_ref() {
            Some(p) => Some(Rotation::from_degrees(*p)?),
            None => None,
        };

        let config = Config {
            bookmarks: args.bookmarks,
            compression: CompressionLevel::from_str(&args.compression)?,
            continue_on_error: args.continue_on_error,
            dry_run: args.dry_run,
            inputs,
            jobs: args.jobs,
            metadata: Metadata::new(
                args.title.clone(),
                args.author.clone(),
                args.subject.clone(),
                args.keywords.clone(),
            ),
            output: PathBuf::from(&args.output),
            overwrite_mode: OverwriteMode::NoClobber,
            page_range,
            quiet: args.quiet,
            rotation,
            verbose: args.verbose,
        };

        config.validate()?;

        Ok(config)
    }
}
