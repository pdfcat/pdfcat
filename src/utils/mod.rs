use crate::{error::PdfCatError, io::PdfReader};
use anyhow::Result;
use lopdf::{Document, Object};
use std::path::PathBuf;

pub fn collect_paths_for_patterns<T>(patterns: T) -> Result<Vec<PathBuf>>
where
    T: IntoIterator,
    T::Item: AsRef<str>,
{
    let mut resolved_paths = Vec::new();

    for pattern in patterns.into_iter() {
        let paths = collect_paths_for_pattern(pattern)?;
        resolved_paths.push(paths);
    }

    let resolved_paths = resolved_paths.iter().flatten().cloned().collect();

    Ok(resolved_paths)
}

fn collect_paths_for_pattern<P: AsRef<str>>(pattern: P) -> Result<Vec<PathBuf>> {
    let mut resolved_paths = Vec::new();

    let paths = glob::glob(pattern.as_ref()).map_err(|err| PdfCatError::Other {
        message: err.to_string(),
    })?;

    for entry in paths {
        let path = entry.map_err(|err| PdfCatError::Other {
            message: err.to_string(),
        })?;
        PdfReader::check_path_exists(&path)?;
        resolved_paths.push(path);
    }

    Ok(resolved_paths)
}

#[allow(unused)]
pub fn copy_references(target: &mut Document, source: &Document, obj: &Object) {
    match obj {
        Object::Reference(ref_id) => {
            if !target.objects.contains_key(ref_id) {
                if let Ok(referenced_obj) = source.get_object(*ref_id) {
                    target.objects.insert(*ref_id, referenced_obj.clone());
                    copy_references(target, source, referenced_obj);
                }
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
