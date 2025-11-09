//! Message formatting and display.
//!
//! This module provides formatted output for different message types
//! with support for quiet and verbose modes.
//!
//! # Examples
//!
//! ```
//! use pdfcat::output::formatter::{OutputFormatter, MessageLevel};
//!
//! let formatter = OutputFormatter::new(false, false);
//! formatter.info("Processing files...");
//! formatter.success("Operation completed");
//! formatter.error("Something went wrong");
//! ```

use crate::config::Config;
use std::io::{self, Write};

/// Level of output message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    /// Informational message.
    Info,
    /// Success message.
    Success,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
    /// Debug/verbose message.
    Debug,
}

/// Output formatter with configurable verbosity.
pub struct OutputFormatter {
    /// Whether to suppress non-error output.
    quiet: bool,
    /// Whether to show verbose output.
    verbose: bool,
    /// Whether to use colored output.
    colored: bool,
}

impl OutputFormatter {
    /// Create a new output formatter.
    ///
    /// # Arguments
    ///
    /// * `quiet` - Suppress non-error output
    /// * `verbose` - Show verbose output
    pub fn new(quiet: bool, verbose: bool) -> Self {
        Self {
            quiet,
            verbose,
            colored: Self::should_use_color(),
        }
    }

    /// Create a formatter from configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration containing output settings
    pub fn from_config(config: &Config) -> Self {
        Self::new(config.quiet, config.verbose)
    }

    /// Create a quiet formatter (only errors).
    pub fn quiet() -> Self {
        Self::new(true, false)
    }

    /// Create a verbose formatter.
    pub fn verbose() -> Self {
        Self::new(false, true)
    }

    /// Detect if colored output should be used.
    ///
    /// Returns true if stdout is a TTY and TERM is set.
    fn should_use_color() -> bool {
        use std::io::IsTerminal;
        io::stdout().is_terminal() && std::env::var("TERM").is_ok()
    }

    /// Print an informational message.
    ///
    /// Suppressed in quiet mode.
    ///
    /// # Arguments
    ///
    /// * `message` - Message to display
    pub fn info(&self, message: &str) {
        if !self.quiet {
            self.print_message(MessageLevel::Info, message);
        }
    }

    /// Print a success message.
    ///
    /// Suppressed in quiet mode.
    ///
    /// # Arguments
    ///
    /// * `message` - Message to display
    pub fn success(&self, message: &str) {
        if !self.quiet {
            self.print_message(MessageLevel::Success, message);
        }
    }

    /// Print a warning message.
    ///
    /// Always displayed (even in quiet mode).
    ///
    /// # Arguments
    ///
    /// * `message` - Message to display
    pub fn warning(&self, message: &str) {
        self.print_message(MessageLevel::Warning, message);
    }

    /// Print an error message.
    ///
    /// Always displayed.
    ///
    /// # Arguments
    ///
    /// * `message` - Message to display
    pub fn error(&self, message: &str) {
        self.print_message(MessageLevel::Error, message);
    }

    /// Print a debug/verbose message.
    ///
    /// Only displayed in verbose mode.
    ///
    /// # Arguments
    ///
    /// * `message` - Message to display
    pub fn debug(&self, message: &str) {
        if self.verbose {
            self.print_message(MessageLevel::Debug, message);
        }
    }

    /// Print a message with level-appropriate formatting.
    fn print_message(&self, level: MessageLevel, message: &str) {
        let (prefix, color_code) = match level {
            MessageLevel::Info => ("", ""),
            MessageLevel::Success => ("✓ ", "\x1b[32m"), // Green
            MessageLevel::Warning => ("⚠ ", "\x1b[33m"), // Yellow
            MessageLevel::Error => ("✗ ", "\x1b[31m"),   // Red
            MessageLevel::Debug => ("→ ", "\x1b[36m"),   // Cyan
        };

        let reset = "\x1b[0m";

        if self.colored && !color_code.is_empty() {
            println!("{color_code}{prefix}{message}{reset}");
        } else {
            println!("{prefix}{message}");
        }
    }

    /// Print a section header.
    ///
    /// Suppressed in quiet mode.
    ///
    /// # Arguments
    ///
    /// * `title` - Section title
    pub fn section(&self, title: &str) {
        if !self.quiet {
            println!("\n{title}");
        }
    }

    /// Print a separator line.
    ///
    /// Only shown in verbose mode.
    pub fn separator(&self) {
        if self.verbose {
            println!("────────────────────────────────────────");
        }
    }

    /// Print detailed information about a file.
    ///
    /// Only shown in verbose mode.
    ///
    /// # Arguments
    ///
    /// * `label` - Label for the information
    /// * `value` - Value to display
    pub fn detail(&self, label: &str, value: &str) {
        if self.verbose {
            println!("  {label}: {value}");
        }
    }

    /// Print a progress indicator.
    ///
    /// Suppressed in quiet mode.
    ///
    /// # Arguments
    ///
    /// * `current` - Current progress value
    /// * `total` - Total value
    /// * `message` - Optional message to display
    pub fn progress(&self, current: usize, total: usize, message: Option<&str>) {
        if !self.quiet {
            let msg = message.unwrap_or("");
            print!("\r  [{current}/{total}] {msg}");
            io::stdout().flush().ok();

            if current == total {
                println!(); // New line when complete
            }
        }
    }

    /// Clear the current line (useful for progress updates).
    pub fn clear_line(&self) {
        if !self.quiet {
            print!("\r\x1b[K");
            io::stdout().flush().ok();
        }
    }

    /// Print a blank line.
    ///
    /// Suppressed in quiet mode.
    pub fn blank_line(&self) {
        if !self.quiet {
            println!();
        }
    }

    /// Print a formatted table row.
    ///
    /// Only shown in verbose mode.
    ///
    /// # Arguments
    ///
    /// * `columns` - Column values to display
    pub fn table_row(&self, columns: &[&str]) {
        if self.verbose {
            let row = columns.join(" │ ");
            println!("  {row}");
        }
    }

    /// Print a list item.
    ///
    /// Suppressed in quiet mode.
    ///
    /// # Arguments
    ///
    /// * `index` - Item index (1-based)
    /// * `message` - Item message
    pub fn list_item(&self, index: usize, message: &str) {
        if !self.quiet {
            println!("  {index}. {message}");
        }
    }

    /// Check if output should be shown.
    ///
    /// # Returns
    ///
    /// True if non-quiet mode, false if quiet mode.
    pub fn should_print(&self) -> bool {
        !self.quiet
    }

    /// Check if verbose output should be shown.
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Check if quiet mode is enabled.
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }
}

impl Default for OutputFormatter {
    fn default() -> Self {
        Self::new(false, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_formatter() {
        let formatter = OutputFormatter::new(false, false);
        assert!(!formatter.is_quiet());
        assert!(!formatter.is_verbose());
        assert!(formatter.should_print());
    }

    #[test]
    fn test_quiet_formatter() {
        let formatter = OutputFormatter::quiet();
        assert!(formatter.is_quiet());
        assert!(!formatter.is_verbose());
        assert!(!formatter.should_print());
    }

    #[test]
    fn test_verbose_formatter() {
        let formatter = OutputFormatter::verbose();
        assert!(!formatter.is_quiet());
        assert!(formatter.is_verbose());
        assert!(formatter.should_print());
    }

    #[test]
    fn test_info_message() {
        let formatter = OutputFormatter::new(false, false);
        // Should not panic
        formatter.info("Test info message");
    }

    #[test]
    fn test_info_message_quiet() {
        let formatter = OutputFormatter::quiet();
        // Should be suppressed but not panic
        formatter.info("This should not appear");
    }

    #[test]
    fn test_success_message() {
        let formatter = OutputFormatter::new(false, false);
        formatter.success("Test success");
    }

    #[test]
    fn test_warning_message() {
        let formatter = OutputFormatter::new(false, false);
        formatter.warning("Test warning");
    }

    #[test]
    fn test_warning_message_quiet() {
        let formatter = OutputFormatter::quiet();
        // Warnings always shown, even in quiet mode
        formatter.warning("Important warning");
    }

    #[test]
    fn test_error_message() {
        let formatter = OutputFormatter::new(false, false);
        formatter.error("Test error");
    }

    #[test]
    fn test_error_message_quiet() {
        let formatter = OutputFormatter::quiet();
        // Errors always shown
        formatter.error("Critical error");
    }

    #[test]
    fn test_debug_message() {
        let formatter = OutputFormatter::verbose();
        formatter.debug("Debug information");
    }

    #[test]
    fn test_debug_message_not_verbose() {
        let formatter = OutputFormatter::new(false, false);
        // Should be suppressed
        formatter.debug("This should not appear");
    }

    #[test]
    fn test_section() {
        let formatter = OutputFormatter::new(false, false);
        formatter.section("Test Section");
    }

    #[test]
    fn test_detail() {
        let formatter = OutputFormatter::verbose();
        formatter.detail("File", "test.pdf");
    }

    #[test]
    fn test_progress() {
        let formatter = OutputFormatter::new(false, false);
        formatter.progress(1, 10, Some("Processing"));
        formatter.progress(10, 10, Some("Complete"));
    }

    #[test]
    fn test_list_item() {
        let formatter = OutputFormatter::new(false, false);
        formatter.list_item(1, "First item");
        formatter.list_item(2, "Second item");
    }

    #[test]
    fn test_table_row() {
        let formatter = OutputFormatter::verbose();
        formatter.table_row(&["Column1", "Column2", "Column3"]);
    }

    #[test]
    fn test_message_levels() {
        assert_eq!(MessageLevel::Info, MessageLevel::Info);
        assert_ne!(MessageLevel::Info, MessageLevel::Error);
    }
}
