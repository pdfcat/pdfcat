//! Output formatting and display for pdfcat.
//!
//! This module handles all user-facing output including:
//! - Formatted status messages
//! - Progress indicators
//! - Error and warning display
//! - Summary reports
//! - Quiet and verbose modes
//!
//! # Examples
//!
//! ```no_run
//! use pdfcat::output::OutputFormatter;
//! use pdfcat::config::Config;
//!
//! # fn example(config: Config) {
//! let formatter = OutputFormatter::from_config(&config);
//! formatter.info("Starting merge operation");
//! formatter.success("Merge completed successfully");
//! # }
//! ```

pub mod formatter;
pub mod progress;

pub use formatter::{MessageLevel, OutputFormatter};
pub use progress::{ProgressBar, ProgressStyle};

use crate::config::Config;
use crate::io::LoadStatistics;
use crate::validation::ValidationSummary;

/// Create an output formatter from configuration.
///
/// # Arguments
///
/// * `config` - Configuration containing output preferences
///
/// # Returns
///
/// An OutputFormatter configured according to quiet/verbose settings.
pub fn create_formatter(config: &Config) -> OutputFormatter {
    OutputFormatter::from_config(config)
}

/// Display validation summary to the user.
///
/// # Arguments
///
/// * `formatter` - Output formatter to use
/// * `summary` - Validation summary to display
pub fn display_validation_summary(formatter: &OutputFormatter, summary: &ValidationSummary) {
    if summary.files_failed > 0 {
        formatter.warning(&format!(
            "Warning: {} file(s) failed validation",
            summary.files_failed
        ));
    }

    formatter.info(&format!(
        "Validated {} file(s): {} pages, {}",
        summary.files_validated,
        summary.total_pages,
        summary.format_total_size()
    ));
}

/// Display load statistics to the user.
///
/// # Arguments
///
/// * `formatter` - Output formatter to use
/// * `stats` - Load statistics to display
pub fn display_load_statistics(formatter: &OutputFormatter, stats: &LoadStatistics) {
    if stats.failure_count > 0 {
        formatter.warning(&format!(
            "Warning: {} file(s) failed to load",
            stats.failure_count
        ));
    }

    formatter.info(&format!(
        "Loaded {} file(s) in {:.2}s: {} pages, {}",
        stats.success_count,
        stats.total_time.as_secs_f64(),
        stats.total_pages,
        stats.format_total_size()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CompressionLevel, Config, Metadata, OverwriteMode};
    use std::path::PathBuf;

    fn create_test_config(quiet: bool, verbose: bool) -> Config {
        Config {
            inputs: vec![PathBuf::from("test.pdf")],
            output: PathBuf::from("output.pdf"),
            dry_run: false,
            verbose,
            overwrite_mode: OverwriteMode::Prompt,
            quiet,
            bookmarks: false,
            compression: CompressionLevel::Standard,
            metadata: Metadata::default(),
            continue_on_error: false,
            jobs: None,
            page_range: None,
            rotation: None,
        }
    }

    #[test]
    fn test_create_formatter() {
        let config = create_test_config(false, false);
        let _formatter = create_formatter(&config);
        // Should create without panicking
    }

    #[test]
    fn test_create_formatter_quiet() {
        let config = create_test_config(true, false);
        let _formatter = create_formatter(&config);
        // Should create without panicking
    }

    #[test]
    fn test_create_formatter_verbose() {
        let config = create_test_config(false, true);
        let _formatter = create_formatter(&config);
        // Should create without panicking
    }
}
