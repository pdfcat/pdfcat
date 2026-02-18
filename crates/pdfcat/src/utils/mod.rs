//! Utilities for path collection, PDF merge helpers, etc.

use crate::Result;
use lopdf::{Document, Object};
use std::path::PathBuf;

/// Expand multiple glob patterns into filesystem paths.
///
/// Accepts anything iterable with items that convert to `&str`, e.g.:
/// `&[&str]`, `Vec<String>`, or `Vec<&str>`.
///
/// Returns a flattened list of resolved paths.
///
/// Errors:
/// - Propagates `glob` parse errors.
/// - Propagates filesystem errors from glob iterator.
pub fn collect_paths_for_patterns<T>(patterns: T) -> Result<Vec<PathResult>>
where
    T: IntoIterator,
    T::Item: AsRef<str>,
{
    let mut resolved_paths = Vec::new();

    for pattern in patterns.into_iter() {
        let paths = collect_paths_for_pattern(pattern)?;
        resolved_paths.extend(paths);
    }

    Ok(resolved_paths)
}

/// Result of attempting to resolve a single path from a glob pattern.
#[derive(Debug)]
pub enum PathResult {
    /// Successfully resolved path from glob pattern or literal path lookup.
    Found(PathBuf),
    /// Error encountered during glob expansion (e.g., permission denied, broken symlink, invalid pattern).
    Error(String),
}

/// Expand a single glob pattern into filesystem paths.
///
/// Pattern examples:
/// - `"**/*.pdf"`
/// - `"./docs/*.pdf"`
pub fn collect_paths_for_pattern<P: AsRef<str>>(pattern: P) -> Result<Vec<PathResult>> {
    let pattern_str = pattern.as_ref();
    let mut results = Vec::new();

    match glob::glob(pattern_str) {
        Ok(entries) => {
            for entry in entries {
                results.push(match entry {
                    Ok(path) => PathResult::Found(path),
                    Err(e) => PathResult::Error(e.to_string()),
                });
            }
        }
        Err(e) => results.push(PathResult::Error(e.to_string())),
    }

    // If we found at least one path, return all results (successes + errors)
    if results.iter().any(|r| matches!(r, PathResult::Found(_))) {
        return Ok(results);
    }

    // Try as literal path
    let literal_path = PathBuf::from(pattern_str);
    if literal_path.exists() {
        return Ok(vec![PathResult::Found(literal_path)]);
    }

    // Nothing worked, return errors
    if !results.is_empty() {
        return Ok(results);
    }

    // Empty result
    Ok(vec![])
}

/// Copy object references from one PDF document to another.
///
/// If `obj` is a reference, this walks the structure recursively and inserts
/// missing referenced objects into the `target` document.
///
/// Required when merging PDFs using `lopdf` to ensure that all referenced
/// objects exist in the final document.
pub fn copy_references(target: &mut Document, source: &Document, obj: &Object) {
    match obj {
        Object::Reference(ref_id) => {
            if !target.objects.contains_key(ref_id)
                && let Ok(referenced_obj) = source.get_object(*ref_id)
            {
                target.objects.insert(*ref_id, referenced_obj.clone());
                copy_references(target, source, referenced_obj);
            }
        }
        Object::Dictionary(dict) => {
            for (_, value) in dict.iter() {
                copy_references(target, source, value);
            }
        }
        Object::Array(arr) => {
            for item in arr {
                copy_references(target, source, item);
            }
        }
        Object::Stream(stream) => {
            copy_references(target, source, &Object::Dictionary(stream.dict.clone()));
        }
        _ => {}
    }
}
