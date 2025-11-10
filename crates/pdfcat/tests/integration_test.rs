#[path = "integration/common/mod.rs"]
mod common;

#[path = "integration/basic_merge.rs"]
mod basic_merge;

// TODO: Fix tests failure due to "couldn't parse input: invalid file header"
// #[path = "integration/bookmarks.rs"]
// mod bookmarks;

#[path = "integration/dry_run.rs"]
mod dry_run;

#[path = "integration/error_cases.rs"]
mod error_cases;
