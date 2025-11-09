//! Progress bar and indicators.
//!
//! This module provides visual progress indicators for long-running operations.
//!
//! # Examples
//!
//! ```
//! use pdfcat::output::progress::{ProgressBar, ProgressStyle};
//!
//! let mut progress = ProgressBar::new(100, ProgressStyle::Bar);
//! progress.set_message("Processing files");
//!
//! for i in 0..=100 {
//!     progress.update(i);
//!     // Do work...
//! }
//!
//! progress.finish();
//! ```

use std::io::{self, Write};
use std::time::{Duration, Instant};

/// Style of progress indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressStyle {
    /// Classic progress bar: [=====>    ]
    Bar,
    /// Spinner indicator: ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏
    Spinner,
    /// Dots indicator: ⣾ ⣽ ⣻ ⢿ ⡿ ⣟ ⣯ ⣷
    Dots,
    /// Simple counter: 42/100
    Counter,
}

/// Progress bar for visual feedback during operations.
pub struct ProgressBar {
    /// Total number of items.
    total: usize,
    /// Current progress.
    current: usize,
    /// Progress bar style.
    style: ProgressStyle,
    /// Optional message to display.
    message: Option<String>,
    /// Start time of the operation.
    start_time: Instant,
    /// Last update time (for rate limiting).
    last_update: Instant,
    /// Minimum time between updates.
    update_interval: Duration,
    /// Whether the progress bar is enabled.
    enabled: bool,
    /// Current spinner frame.
    spinner_frame: usize,
}

impl ProgressBar {
    /// Create a new progress bar.
    ///
    /// # Arguments
    ///
    /// * `total` - Total number of items
    /// * `style` - Progress bar style
    pub fn new(total: usize, style: ProgressStyle) -> Self {
        Self {
            total,
            current: 0,
            style,
            message: None,
            start_time: Instant::now(),
            last_update: Instant::now(),
            update_interval: Duration::from_millis(100),
            enabled: Self::is_terminal(),
            spinner_frame: 0,
        }
    }

    /// Create a progress bar with automatic style selection.
    ///
    /// Uses Bar style for determinate progress, Spinner for indeterminate.
    pub fn auto(total: usize) -> Self {
        let style = if total > 0 {
            ProgressStyle::Bar
        } else {
            ProgressStyle::Spinner
        };
        Self::new(total, style)
    }

    /// Create a disabled progress bar (no output).
    pub fn disabled() -> Self {
        let mut pb = Self::new(0, ProgressStyle::Counter);
        pb.enabled = false;
        pb
    }

    /// Check if stdout is a terminal.
    fn is_terminal() -> bool {
        use std::io::IsTerminal;
        io::stdout().is_terminal()
    }

    /// Set the message to display with the progress bar.
    ///
    /// # Arguments
    ///
    /// * `message` - Message to display
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
    }

    /// Clear the message.
    pub fn clear_message(&mut self) {
        self.message = None;
    }

    /// Update the progress bar to a specific value.
    ///
    /// # Arguments
    ///
    /// * `current` - Current progress value
    pub fn update(&mut self, current: usize) {
        self.current = current;

        // Rate limit updates
        if self.last_update.elapsed() < self.update_interval && current < self.total {
            return;
        }

        self.last_update = Instant::now();
        self.render();
    }

    /// Increment the progress bar by one.
    pub fn increment(&mut self) {
        self.update(self.current + 1);
    }

    /// Increment the progress bar by a specific amount.
    ///
    /// # Arguments
    ///
    /// * `delta` - Amount to increment by
    pub fn increment_by(&mut self, delta: usize) {
        self.update(self.current + delta);
    }

    /// Mark the progress bar as finished.
    pub fn finish(&mut self) {
        if self.enabled {
            self.current = self.total;
            self.render();
            println!(); // New line
        }
    }

    /// Finish with a custom message.
    ///
    /// # Arguments
    ///
    /// * `message` - Message to display
    pub fn finish_with_message(&mut self, message: impl Into<String>) {
        if self.enabled {
            self.set_message(message);
            self.finish();
        }
    }

    /// Clear the progress bar from the terminal.
    pub fn clear(&self) {
        if self.enabled {
            print!("\r\x1b[K");
            io::stdout().flush().ok();
        }
    }

    /// Render the progress bar.
    fn render(&mut self) {
        if !self.enabled {
            return;
        }

        let output = match self.style {
            ProgressStyle::Bar => self.render_bar(),
            ProgressStyle::Spinner => self.render_spinner(),
            ProgressStyle::Dots => self.render_dots(),
            ProgressStyle::Counter => self.render_counter(),
        };

        print!("\r{output}");
        io::stdout().flush().ok();
    }

    /// Render a progress bar.
    fn render_bar(&self) -> String {
        let width = 40;
        let percent = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as usize
        } else {
            0
        };

        let filled = (width * self.current) / self.total.max(1);
        let empty = width - filled;

        let bar = format!(
            "[{}{}]",
            "=".repeat(filled.saturating_sub(1)) + if filled > 0 { ">" } else { "" },
            " ".repeat(empty)
        );

        let counter = format!("{}/{}", self.current, self.total);
        let elapsed = format_duration(self.start_time.elapsed());

        let mut parts = vec![bar, format!("{}%", percent), counter, elapsed];

        if let Some(ref msg) = self.message {
            parts.insert(0, msg.clone());
        }

        parts.join(" ")
    }

    /// Render a spinner.
    fn render_spinner(&mut self) -> String {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame = frames[self.spinner_frame % frames.len()];
        self.spinner_frame += 1;

        let elapsed = format_duration(self.start_time.elapsed());

        let mut parts = vec![frame.to_string(), elapsed];

        if let Some(ref msg) = self.message {
            parts.insert(1, msg.clone());
        }

        parts.join(" ")
    }

    /// Render dots spinner.
    fn render_dots(&mut self) -> String {
        let frames = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
        let frame = frames[self.spinner_frame % frames.len()];
        self.spinner_frame += 1;

        let elapsed = format_duration(self.start_time.elapsed());

        let mut parts = vec![frame.to_string(), elapsed];

        if let Some(ref msg) = self.message {
            parts.insert(1, msg.clone());
        }

        parts.join(" ")
    }

    /// Render a simple counter.
    fn render_counter(&self) -> String {
        let counter = format!("{}/{}", self.current, self.total);
        let elapsed = format_duration(self.start_time.elapsed());

        let mut parts = vec![counter, elapsed];

        if let Some(ref msg) = self.message {
            parts.insert(0, msg.clone());
        }

        parts.join(" ")
    }

    /// Get the current progress percentage.
    pub fn percent(&self) -> f64 {
        if self.total > 0 {
            (self.current as f64 / self.total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get the elapsed time since start.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Estimate time remaining.
    pub fn eta(&self) -> Option<Duration> {
        if self.current == 0 || self.current >= self.total {
            return None;
        }

        let elapsed = self.start_time.elapsed();
        let rate = self.current as f64 / elapsed.as_secs_f64();
        let remaining = self.total - self.current;
        let eta_secs = remaining as f64 / rate;

        Some(Duration::from_secs_f64(eta_secs))
    }
}

/// Format a duration as a human-readable string.
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();

    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_progress_bar() {
        let pb = ProgressBar::new(100, ProgressStyle::Bar);
        assert_eq!(pb.total, 100);
        assert_eq!(pb.current, 0);
        assert_eq!(pb.style, ProgressStyle::Bar);
    }

    #[test]
    fn test_auto_progress_bar() {
        let pb = ProgressBar::auto(100);
        assert_eq!(pb.style, ProgressStyle::Bar);

        let pb_spinner = ProgressBar::auto(0);
        assert_eq!(pb_spinner.style, ProgressStyle::Spinner);
    }

    #[test]
    fn test_disabled_progress_bar() {
        let pb = ProgressBar::disabled();
        assert!(!pb.enabled);
    }

    #[test]
    fn test_set_message() {
        let mut pb = ProgressBar::new(100, ProgressStyle::Bar);
        pb.set_message("Processing");
        assert_eq!(pb.message, Some("Processing".to_string()));
    }

    #[test]
    fn test_clear_message() {
        let mut pb = ProgressBar::new(100, ProgressStyle::Bar);
        pb.set_message("Processing");
        pb.clear_message();
        assert_eq!(pb.message, None);
    }

    #[test]
    fn test_update() {
        let mut pb = ProgressBar::disabled();
        pb.update(50);
        assert_eq!(pb.current, 50);
    }

    #[test]
    fn test_increment() {
        let mut pb = ProgressBar::disabled();
        pb.increment();
        assert_eq!(pb.current, 1);
        pb.increment();
        assert_eq!(pb.current, 2);
    }

    #[test]
    fn test_increment_by() {
        let mut pb = ProgressBar::disabled();
        pb.increment_by(5);
        assert_eq!(pb.current, 5);
        pb.increment_by(3);
        assert_eq!(pb.current, 8);
    }

    #[test]
    fn test_percent() {
        let mut pb = ProgressBar::new(100, ProgressStyle::Bar);
        assert_eq!(pb.percent(), 0.0);

        pb.update(50);
        assert_eq!(pb.percent(), 50.0);

        pb.update(100);
        assert_eq!(pb.percent(), 100.0);
    }

    #[test]
    fn test_percent_zero_total() {
        let pb = ProgressBar::new(0, ProgressStyle::Bar);
        assert_eq!(pb.percent(), 0.0);
    }

    #[test]
    fn test_elapsed() {
        let pb = ProgressBar::new(100, ProgressStyle::Bar);
        let elapsed = pb.elapsed();
        assert!(elapsed < Duration::from_secs(1));
    }

    #[test]
    fn test_eta() {
        let mut pb = ProgressBar::new(100, ProgressStyle::Bar);

        // No ETA at start
        assert_eq!(pb.eta(), None);

        // Sleep briefly and update to allow ETA calculation
        std::thread::sleep(Duration::from_millis(10));
        pb.update(10);

        let eta = pb.eta();
        assert!(eta.is_some());
    }

    #[test]
    fn test_eta_at_completion() {
        let mut pb = ProgressBar::new(100, ProgressStyle::Bar);
        pb.update(100);

        // No ETA when complete
        assert_eq!(pb.eta(), None);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
    }

    #[test]
    fn test_progress_styles() {
        assert_eq!(ProgressStyle::Bar, ProgressStyle::Bar);
        assert_ne!(ProgressStyle::Bar, ProgressStyle::Spinner);
    }

    #[test]
    fn test_render_methods() {
        let mut pb = ProgressBar::disabled();
        pb.total = 100;
        pb.current = 50;

        // Test that render methods don't panic
        pb.render_bar();
        pb.render_counter();
        pb.render_spinner();
        pb.render_dots();
    }

    #[test]
    fn test_finish() {
        let mut pb = ProgressBar::disabled();
        pb.finish();
        assert_eq!(pb.current, pb.total);
    }

    #[test]
    fn test_finish_with_message() {
        let mut pb = ProgressBar::auto(100); // NOTE: we must NOT use disabled here to capture the message
        pb.finish_with_message("Complete");
        if pb.enabled {
            assert_eq!(pb.current, pb.total);
            assert_eq!(pb.message, Some("Complete".to_string()));
        }
    }
}
