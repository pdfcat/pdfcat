#![no_main]

use libfuzzer_sys::fuzz_target;
use pdfcat::{Config, merge::merge_pdfs};
use std::{path::PathBuf, sync::OnceLock};
use tokio::runtime::{Builder, Runtime};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Builder::new_current_thread().enable_all().build().unwrap())
}

fuzz_target!(|data: &[u8]| {
    let s = std::str::from_utf8(data).unwrap_or("");

    let paths: Vec<PathBuf> = s.split_whitespace().map(PathBuf::from).collect();

    let config = Config {
        inputs: paths.clone(),
        ..Config::default()
    };

    let result = runtime().block_on(merge_pdfs(&config));

    // Guarantee that result is error <--- No valid PDF file paths
    assert!(result.is_err());
});
