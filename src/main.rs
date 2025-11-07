mod error;
mod ops;
mod walker;

use crate::ops::{merge_pdfs, save_pdf};
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "pdfjoin")]
#[command(about = "Merge multiple PDF files into one", long_about = None)]
struct Cli {
    /// Input PDF files to merge (in order)
    #[arg(required = true)]
    inputs: Vec<String>,

    /// Output PDF file path
    #[arg(short, long)]
    output: String,

    /// Dry run - validate inputs and show what would be done without creating output
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Verbose output - show detailed information about each PDF
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.inputs.is_empty() {
        anyhow::bail!("At least one input PDF file is required");
    }

    if cli.dry_run {
        println!("🔍 DRY RUN MODE - No files will be created\n");
    }

    let pdf_paths = walker::resolve_pdf_paths(&cli.inputs)?;
    println!("Merging {} PDF files...", pdf_paths.len());

    if cli.dry_run {
        println!("\n✓ Dry run completed successfully");
        println!("  Output would be: {}", &cli.output);
        println!("  Run without --dry-run to create the merged PDF");
    } else {
        let mut merged = merge_pdfs(pdf_paths.as_slice(), cli.dry_run, cli.verbose)?;
        println!("\nWriting to: {}", &cli.output);
        save_pdf(&mut merged, &cli.output)?;
        println!("✓ Successfully created {}", &cli.output);
    }

    Ok(())
}
