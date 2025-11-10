//! pdfcat - Concatenate PDF files into a single document.
//!
//! A high-performance CLI tool for merging PDF files with quality preservation.

mod cli;

use clap::Parser;
use std::process;

use crate::cli::Cli;
use pdfcat::config::Config;
use pdfcat::error::PdfCatError;
use pdfcat::io::PdfWriter;
use pdfcat::merge::Merger;
use pdfcat::output::{OutputFormatter, display_validation_summary};
use pdfcat::validation::Validator; // display_load_statistics 

#[tokio::main]
async fn main() {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Run the application and handle errors
    if let Err(err) = run(cli).await {
        eprintln!("Error: {err}");
        process::exit(err.exit_code());
    }
}

/// Main application logic.
async fn run(cli: Cli) -> Result<(), PdfCatError> {
    // Validate CLI arguments
    cli.validate()?;

    // Get all inputs (including from input-list if specified)
    let all_inputs = cli.get_all_inputs().await?;

    // Convert CLI to config
    let mut config = cli.to_config()?;
    config.inputs = all_inputs;

    // Create output formatter
    let formatter = OutputFormatter::from_config(&config);

    // Print header
    if formatter.should_print() {
        formatter.section(&format!("{} v{}", pdfcat::NAME, pdfcat::VERSION));
        formatter.blank_line();
    }

    // Validate configuration and inputs
    formatter.info("Validating input files...");
    let validator = Validator::new();
    let validation_summary = validator.validate_config(&config).await?;

    if formatter.should_print() {
        display_validation_summary(&formatter, &validation_summary);
        formatter.blank_line();
    }

    // Validate output
    validator.validate_output(&config).await?;

    // Handle output file existence
    if !config.dry_run {
        handle_output_overwrite(&config, &formatter).await?;
    }

    // Dry run mode - stop here
    if config.dry_run {
        formatter.blank_line();
        formatter.success("Dry run completed successfully");
        formatter.info(&format!("  Output would be: {}", config.output.display()));
        formatter.info("  Run without --dry-run to create the merged PDF");
        return Ok(());
    }

    // Perform the merge
    formatter.info("Merging documents...");
    formatter.blank_line();

    let merger = Merger::new();
    let result = merger.merge(&config).await?;

    if formatter.should_print() {
        formatter.blank_line();
        formatter.info(&format!(
            "Merged {} file(s) into {} pages in {:.2}s",
            result.statistics.files_merged,
            result.statistics.total_pages,
            result.statistics.merge_time.as_secs_f64()
        ));
    }

    // Write the output
    formatter.info(&format!("Writing to: {}", config.output.display()));

    let writer = PdfWriter::new();
    let write_stats = writer
        .save_with_stats(&result.document, &config.output)
        .await?;

    if formatter.should_print() {
        formatter.blank_line();
        formatter.success(&format!(
            "Successfully created {} ({})",
            config.output.display(),
            write_stats.format_file_size()
        ));

        if formatter.is_verbose() {
            formatter.blank_line();
            formatter.section("Statistics");
            formatter.detail("Input files", &result.statistics.files_merged.to_string());
            formatter.detail("Total pages", &result.statistics.total_pages.to_string());
            formatter.detail("Input size", &result.statistics.format_input_size());
            formatter.detail("Output size", &write_stats.format_file_size());
            formatter.detail(
                "Load time",
                &format!("{:.2}s", result.statistics.load_time.as_secs_f64()),
            );
            formatter.detail(
                "Merge time",
                &format!("{:.2}s", result.statistics.merge_time.as_secs_f64()),
            );
            formatter.detail(
                "Write time",
                &format!("{:.2}s", write_stats.write_time.as_secs_f64()),
            );
            formatter.detail(
                "Compression",
                if write_stats.compressed { "Yes" } else { "No" },
            );

            if config.bookmarks {
                formatter.detail("Bookmarks", "Added");
            }

            if !config.metadata.is_empty() {
                formatter.detail("Metadata", "Set");
            }
        }
    }

    Ok(())
}

/// Handle output file overwrite scenarios.
async fn handle_output_overwrite(
    config: &Config,
    formatter: &OutputFormatter,
) -> Result<(), PdfCatError> {
    use pdfcat::config::OverwriteMode;

    // Check if output exists
    if !config.output.exists() {
        return Ok(());
    }

    match config.overwrite_mode {
        OverwriteMode::Force => {
            // Just overwrite, no questions asked
            Ok(())
        }
        OverwriteMode::NoClobber => {
            // Error if file exists
            Err(PdfCatError::output_exists(config.output.clone()))
        }
        OverwriteMode::Prompt => {
            // Ask user for confirmation
            if formatter.is_quiet() {
                // In quiet mode, treat as no-clobber
                return Err(PdfCatError::output_exists(config.output.clone()));
            }

            formatter.warning(&format!(
                "Output file already exists: {}",
                config.output.display()
            ));

            // Simple yes/no prompt
            use std::io::{self, Write};
            print!("Overwrite? [y/N]: ");
            io::stdout().flush().ok();

            let mut response = String::new();
            io::stdin()
                .read_line(&mut response)
                .map_err(|err| PdfCatError::other(format!("Failed to read input: {err}")))?;

            let response = response.trim().to_lowercase();
            if response == "y" || response == "yes" {
                Ok(())
            } else {
                Err(PdfCatError::Cancelled)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdfcat::config::{CompressionLevel, Metadata, OverwriteMode};
    use std::path::PathBuf;

    fn create_test_config() -> Config {
        Config {
            inputs: vec![PathBuf::from("test.pdf")],
            output: PathBuf::from("output.pdf"),
            dry_run: false,
            verbose: false,
            overwrite_mode: OverwriteMode::Force,
            quiet: false,
            bookmarks: false,
            compression: CompressionLevel::Standard,
            metadata: Metadata::default(),
            continue_on_error: false,
            jobs: None,
            page_range: None,
            rotation: None,
        }
    }

    #[tokio::test]
    async fn test_handle_output_overwrite_force() {
        let config = create_test_config();
        let formatter = OutputFormatter::quiet();

        // Should not error with force mode
        let result = handle_output_overwrite(&config, &formatter).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_output_overwrite_no_clobber() {
        let mut config = create_test_config();
        config.overwrite_mode = OverwriteMode::NoClobber;

        // Create a temp file to test against
        use tempfile::NamedTempFile;
        let temp_file = NamedTempFile::new().unwrap();
        config.output = temp_file.path().to_path_buf();

        let formatter = OutputFormatter::quiet();

        // Should error with no-clobber when file exists
        let result = handle_output_overwrite(&config, &formatter).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_output_overwrite_nonexistent() {
        let config = create_test_config();
        let formatter = OutputFormatter::quiet();

        // Should not error when file doesn't exist
        let result = handle_output_overwrite(&config, &formatter).await;
        assert!(result.is_ok());
    }
}
