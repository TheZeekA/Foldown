use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::convert::formats::convert_to_markdown;
use crate::error::AppResult;
use crate::fs::ops::write_file_atomic;

#[tauri::command]
pub fn convert_document(source_path: String, dest_path: String) -> AppResult<()> {
    let markdown = convert_to_markdown(Path::new(&source_path))?;
    write_file_atomic(Path::new(&dest_path), &markdown)
}

#[derive(Debug, Serialize)]
pub struct BulkConvertResult {
    pub source_path: String,
    pub dest_path: Option<String>,
    pub error: Option<String>,
}

/// Converts every source file into `dest_dir`, keeping the original base
/// name (with a numeric suffix on collision). A failure on one file doesn't
/// abort the rest of the batch — each result reports its own outcome.
#[tauri::command]
pub fn bulk_convert_documents(
    source_paths: Vec<String>,
    dest_dir: String,
) -> Vec<BulkConvertResult> {
    let dest_dir = PathBuf::from(dest_dir);

    source_paths
        .into_iter()
        .map(|source| {
            let source_path = PathBuf::from(&source);
            let outcome = (|| -> AppResult<PathBuf> {
                let markdown = convert_to_markdown(&source_path)?;
                let stem = source_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "untitled".to_string());
                let dest = unique_markdown_path(&dest_dir, &stem);
                write_file_atomic(&dest, &markdown)?;
                Ok(dest)
            })();

            match outcome {
                Ok(dest) => BulkConvertResult {
                    source_path: source,
                    dest_path: Some(dest.to_string_lossy().into_owned()),
                    error: None,
                },
                Err(e) => BulkConvertResult {
                    source_path: source,
                    dest_path: None,
                    error: Some(e.to_string()),
                },
            }
        })
        .collect()
}

fn unique_markdown_path(dir: &Path, stem: &str) -> PathBuf {
    let mut candidate = dir.join(format!("{stem}.md"));
    let mut n = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{stem} {n}.md"));
        n += 1;
    }
    candidate
}
