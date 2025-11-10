//! Utilities for path collection, PDF merge helpers, etc.

use crate::{Result, error::PdfCatError};
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
pub fn collect_paths_for_patterns<T>(patterns: T) -> Result<Vec<PathBuf>>
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

/// Expand a single glob pattern into filesystem paths.
///
/// Pattern examples:
/// - `"**/*.pdf"`
/// - `"./docs/*.pdf"`
fn collect_paths_for_pattern<P: AsRef<str>>(pattern: P) -> Result<Vec<PathBuf>> {
    let mut resolved_paths = Vec::new();

    let paths = glob::glob(pattern.as_ref()).map_err(|err| PdfCatError::Other {
        message: err.to_string(),
    })?;

    for entry in paths {
        let path = entry.map_err(|err| PdfCatError::Other {
            message: err.to_string(),
        })?;
        resolved_paths.push(path);
    }

    Ok(resolved_paths)
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
