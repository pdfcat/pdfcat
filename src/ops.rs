use anyhow::{Context, Result};
use lopdf::{Document, Object, ObjectId};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::error::PdfError;

pub fn merge_pdfs(paths: &[PathBuf], dry_run: bool, verbose: bool) -> Result<Document> {
    if paths.is_empty() {
        anyhow::bail!(PdfError::NoFilesToMerge);
    }

    // Validate all input files exist and are readable PDFs
    println!("\nValidating input files...");
    for (idx, path) in paths.iter().enumerate() {
        print!(
            "  [{}/{}] Checking: {}",
            idx + 1,
            paths.len(),
            path.display()
        );

        if !path.exists() {
            anyhow::bail!("\n✗ File does not exist: {}", path.display());
        }

        if !path.is_file() {
            anyhow::bail!("\n✗ Not a file: {}", path.display());
        }

        // Try to load the PDF to validate it
        match Document::load(path) {
            Ok(doc) => {
                let pages = doc.get_pages();
                println!(" ✓ ({} pages)", pages.len());

                if verbose {
                    print_pdf_info(&doc, path);
                }
            }
            Err(e) => {
                anyhow::bail!("\n✗ Failed to load PDF: {}\n  Error: {}", path.display(), e);
            }
        }
    }

    if dry_run {
        println!("\n📋 Merge plan:");
        let mut total_pages = 0;
        for (idx, path) in paths.iter().enumerate() {
            let doc = Document::load(path)?;
            let page_count = doc.get_pages().len();
            total_pages += page_count;
            println!("  {}. {} ({} pages)", idx + 1, path.display(), page_count);
        }
        println!("\n  Total pages in merged document: {}", total_pages);

        // Return a dummy document for dry run
        let doc = Document::load(&paths[0])?;
        return Ok(doc);
    }

    if paths.len() == 1 {
        return Document::load(&paths[0]).with_context(|| PdfError::FailedToLoadFromPath {
            path: paths[0].to_path_buf().display().to_string(),
        });
    }

    println!("\nMerging documents...");
    println!("  [1/{}] Processing: {}", paths.len(), paths[0].display());

    println!(
        "  [1/{}] Processing: {}",
        paths.len(),
        paths[0].to_path_buf().display()
    );

    // Load first document as base
    let mut merged = Document::load(&paths[0]).context("Failed to load first PDF")?;

    let pages = merged.get_pages();
    println!("    → {} pages", pages.len());

    let mut max_id = merged.max_id;

    // Merge remaining documents
    for (idx, path) in paths[1..].iter().enumerate() {
        println!(
            "  [{}/{}] Processing: {}",
            idx + 2,
            paths.len(),
            path.display()
        );

        let mut doc = Document::load(path)
            .with_context(|| format!("Failed to load PDF: {}", path.display()))?;

        let pages = doc.get_pages();
        println!("    → {} pages", pages.len());

        // Adjust object IDs to avoid conflicts
        doc.renumber_objects_with(max_id + 1);
        max_id = doc.max_id;

        // Get page references from the document
        let doc_pages: Vec<ObjectId> = doc.get_pages().into_iter().map(|(_, id)| id).collect();

        // Add all objects from doc to merged
        merged.objects.extend(doc.objects);

        // Add pages to merged document's page tree
        if let Ok(catalog) = merged.catalog_mut() {
            if let Ok(pages_id) = catalog.get(b"Pages").and_then(|p| p.as_reference()) {
                if let Ok(Object::Dictionary(pages_dict)) = merged.get_object_mut(pages_id) {
                    // Get existing kids array
                    if let Ok(Object::Array(kids)) = pages_dict.get_mut(b"Kids") {
                        // Add new page references
                        for page_id in doc_pages {
                            kids.push(Object::Reference(page_id));
                        }
                    }

                    // Update count
                    if let Ok(Object::Integer(count)) = pages_dict.get(b"Count") {
                        let new_count = count + pages.len() as i64;
                        pages_dict.set("Count", Object::Integer(new_count));
                    }
                }
            }
        }
    }

    // Renumber objects for consistency and compress
    merged.renumber_objects();
    merged.compress();

    let total_pages = merged.get_pages().len();
    println!("  Total pages: {}", total_pages);

    Ok(merged)
}

pub fn save_pdf(doc: &mut Document, path: &str) -> Result<()> {
    let file = File::create(path).with_context(|| PdfError::FailedToCreateOutput {
        path: path.to_owned(),
    })?;

    let mut writer = BufWriter::new(file);
    doc.save_to(&mut writer).context(PdfError::FailedToWrite)?;

    writer.flush()?;

    Ok(())
}

fn print_pdf_info(doc: &Document, path: &PathBuf) {
    println!("    File: {}", path.display());

    // Get PDF version
    println!("    Version: {}", &doc.version);

    // Get page dimensions if available
    if let Some(pages) = doc.get_pages().into_iter().next() {
        if let Ok(page_obj) = doc.get_object(pages.1) {
            if let Object::Dictionary(page_dict) = page_obj {
                if let Ok(Object::Array(mediabox)) = page_dict.get(b"MediaBox") {
                    if mediabox.len() >= 4 {
                        if let (Ok(w), Ok(h)) = (mediabox[2].as_float(), mediabox[3].as_float()) {
                            println!("    Page size: {:.1} x {:.1} pts", w, h);
                        }
                    }
                }
            }
        }
    }

    // Count total objects
    println!("    Objects: {}", doc.objects.len());
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
