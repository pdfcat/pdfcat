mod cli;
pub mod config;
mod error;
pub use error::*;
pub mod io;
mod merge;
mod output;
pub(crate) mod utils;

use crate::{
    cli::Cli,
    config::Config,
    io::PdfWriter,
    merge::{MergeResult, PdfMerger},
};

use clap::Parser;

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::try_from(&cli)?;

    if config.dry_run {
        println!("🔍 DRY RUN MODE - No files will be created\n");
    }

    println!("Merging {} PDF files...", config.inputs.len());

    if cli.dry_run {
        println!("\n✓ Dry run completed successfully");
        println!("  Output would be: {}", &config.output.display());
        println!("  Run without --dry-run to create the merged PDF");
    }

    let merged = PdfMerger::merge(config.inputs, config.dry_run, config.verbose)?;

    match merged {
        MergeResult::DryRun { .. } => {}
        MergeResult::Document(mut doc) => {
            println!("\nWriting to: {}", &cli.output);
            PdfWriter::write(&mut doc, std::env::current_dir()?.join(&config.output))?;
            println!("✓ Successfully created {}", &cli.output);
        }
    }

    Ok(())
}
