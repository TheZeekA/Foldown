use std::path::Path;

use crate::error::{AppError, AppResult};

use super::docx::docx_to_markdown;
use super::markdown_table::{escape_markdown_inline, rows_to_markdown_table};

fn extension_lower(path: &Path) -> Option<String> {
    path.extension().map(|e| e.to_string_lossy().to_lowercase())
}

/// Converts a source file to Markdown based on its extension. Plain text is
/// passed through unchanged (it's already Markdown-compatible); everything
/// else goes through a dedicated converter.
pub fn convert_to_markdown(path: &Path) -> AppResult<String> {
    match extension_lower(path).as_deref() {
        Some("txt") => Ok(std::fs::read_to_string(path)?),
        Some("html") | Some("htm") => {
            let html = std::fs::read_to_string(path)?;
            Ok(html2md::parse_html(&html))
        }
        Some("csv") => convert_csv(path),
        Some("docx") => {
            let bytes = std::fs::read(path)?;
            docx_to_markdown(&bytes)
        }
        _ => Err(AppError::Message(format!(
            "\"{}\" is not a supported file type for conversion",
            path.display()
        ))),
    }
}

/// The first row is treated as the table header — the common case for
/// business/data CSVs.
fn convert_csv(path: &Path) -> AppResult<String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .map_err(|e| AppError::Message(format!("Could not read CSV: {e}")))?;

    let headers = reader
        .headers()
        .map_err(|e| AppError::Message(format!("Could not read CSV headers: {e}")))?
        .clone();

    let mut rows: Vec<Vec<String>> = vec![headers.iter().map(escape_cell).collect()];
    for record in reader.records() {
        let record =
            record.map_err(|e| AppError::Message(format!("Could not read CSV row: {e}")))?;
        rows.push(record.iter().map(escape_cell).collect());
    }

    Ok(rows_to_markdown_table(rows))
}

fn escape_cell(cell: &str) -> String {
    escape_markdown_inline(cell).replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_file(name: &str, contents: &str) -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("foldown-convert-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn txt_passes_through_unchanged() {
        let path = temp_file("notes.txt", "Hello, plain text.");
        assert_eq!(convert_to_markdown(&path).unwrap(), "Hello, plain text.");
    }

    #[test]
    fn html_converts_to_markdown() {
        let path = temp_file(
            "page.html",
            "<h1>Title</h1><p>Some <strong>bold</strong> text.</p>",
        );
        let md = convert_to_markdown(&path).unwrap();
        // html2md renders <h1> as a Setext-style heading ("Title\n===="),
        // not ATX ("# Title") — both are valid Markdown.
        assert!(md.contains("Title\n=="));
        assert!(md.contains("**bold**"));
    }

    #[test]
    fn csv_converts_to_markdown_table() {
        let path = temp_file("data.csv", "Name,Age\nAda,30\nGrace,85\n");
        let md = convert_to_markdown(&path).unwrap();
        assert_eq!(
            md,
            "| Name | Age |\n| --- | --- |\n| Ada | 30 |\n| Grace | 85 |\n"
        );
    }

    #[test]
    fn unsupported_extension_is_rejected() {
        let path = temp_file("image.png", "not really an image");
        assert!(convert_to_markdown(&path).is_err());
    }

    #[test]
    fn csv_cells_are_escaped_and_embedded_newlines_do_not_break_the_table() {
        let path = temp_file(
            "data.csv",
            "Name,Notes\n\"file_v2.txt\",\"Line one\nLine two\"\n",
        );
        let md = convert_to_markdown(&path).unwrap();
        assert_eq!(
            md.lines().count(),
            3,
            "a cell newline must not add an output line:\n{md}"
        );
        assert!(md.contains(r"file\_v2.txt"));
        assert!(md.contains("Line one<br>Line two"));
    }
}
