use docx_rs::{
    read_docx, DocumentChild, Paragraph, ParagraphChild, ParagraphProperty, Run, RunChild, Table,
    TableCellContent, TableChild, TableRowChild,
};

use crate::error::{AppError, AppResult};

use super::markdown_table::{escape_markdown_inline, rows_to_markdown_table};

pub fn docx_to_markdown(bytes: &[u8]) -> AppResult<String> {
    let docx = read_docx(bytes)
        .map_err(|e| AppError::Message(format!("Could not read .docx file: {e}")))?;

    let mut blocks: Vec<String> = Vec::new();
    for child in &docx.document.children {
        match child {
            DocumentChild::Paragraph(p) => {
                let md = paragraph_markdown(p);
                if !md.is_empty() {
                    blocks.push(md);
                }
            }
            DocumentChild::Table(t) => {
                let md = table_markdown(t);
                if !md.is_empty() {
                    blocks.push(md);
                }
            }
            _ => {}
        }
    }
    Ok(blocks.join("\n\n"))
}

/// Word's built-in "HeadingN" (or "Title") paragraph styles map to Markdown
/// headings; anything else stays a plain paragraph.
fn heading_level(property: &ParagraphProperty) -> Option<u8> {
    let style_id = &property.style.as_ref()?.val;
    let normalized = style_id.to_lowercase().replace(' ', "");
    if let Some(rest) = normalized.strip_prefix("heading") {
        return rest.parse::<u8>().ok().filter(|n| (1..=6).contains(n));
    }
    (normalized == "title").then_some(1)
}

fn run_markdown(run: &Run) -> String {
    let mut text = String::new();
    for child in &run.children {
        match child {
            RunChild::Text(t) => text.push_str(&escape_markdown_inline(&t.text)),
            RunChild::Tab(_) => text.push('\t'),
            RunChild::Break(_) | RunChild::CarriageReturn(_) => text.push('\n'),
            _ => {}
        }
    }
    if text.trim().is_empty() {
        return text;
    }
    // Presence of a Bold/Italic run property is treated as "on" — docx-rs
    // doesn't expose their inner value, so an explicit "turn off inherited
    // bold" override (rare in practice) would be mis-rendered as bold.
    match (
        run.run_property.bold.is_some(),
        run.run_property.italic.is_some(),
    ) {
        (true, true) => format!("***{text}***"),
        (true, false) => format!("**{text}**"),
        (false, true) => format!("*{text}*"),
        (false, false) => text,
    }
}

fn paragraph_children_markdown(children: &[ParagraphChild]) -> String {
    let mut out = String::new();
    for child in children {
        match child {
            ParagraphChild::Run(run) => out.push_str(&run_markdown(run)),
            ParagraphChild::Hyperlink(link) => {
                out.push_str(&paragraph_children_markdown(&link.children))
            }
            _ => {}
        }
    }
    out
}

fn paragraph_markdown(paragraph: &Paragraph) -> String {
    let text = paragraph_children_markdown(&paragraph.children);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    match heading_level(&paragraph.property) {
        Some(level) => format!("{} {}", "#".repeat(level as usize), trimmed),
        None => trimmed.to_string(),
    }
}

fn table_markdown(table: &Table) -> String {
    let mut rows: Vec<Vec<String>> = Vec::new();
    for TableChild::TableRow(row) in &table.rows {
        let mut cells = Vec::new();
        for TableRowChild::TableCell(cell) in &row.cells {
            let mut cell_lines = Vec::new();
            for content in &cell.children {
                if let TableCellContent::Paragraph(p) = content {
                    let md = paragraph_markdown(p);
                    if !md.is_empty() {
                        cell_lines.push(md);
                    }
                }
            }
            cells.push(cell_lines.join(" ").replace('|', "\\|"));
        }
        rows.push(cells);
    }
    rows_to_markdown_table(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use docx_rs::{Docx, Run as DocxRun, TableCell, TableRow};
    use std::io::Cursor;

    fn build_docx(docx: Docx) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        docx.pack(&mut buf).unwrap();
        buf.into_inner()
    }

    #[test]
    fn converts_heading_and_plain_paragraph() {
        let docx = Docx::new()
            .add_paragraph(
                Paragraph::new()
                    .add_run(DocxRun::new().add_text("Title Here"))
                    .style("Heading1"),
            )
            .add_paragraph(Paragraph::new().add_run(DocxRun::new().add_text("Just a sentence.")));
        let bytes = build_docx(docx);

        let md = docx_to_markdown(&bytes).unwrap();
        assert_eq!(md, "# Title Here\n\nJust a sentence.");
    }

    #[test]
    fn converts_bold_and_italic_runs() {
        let docx = Docx::new().add_paragraph(
            Paragraph::new()
                .add_run(DocxRun::new().add_text("bold").bold())
                .add_run(DocxRun::new().add_text(" normal "))
                .add_run(DocxRun::new().add_text("italic").italic()),
        );
        let bytes = build_docx(docx);

        let md = docx_to_markdown(&bytes).unwrap();
        assert_eq!(md, "**bold** normal *italic*");
    }

    #[test]
    fn converts_table_to_markdown_table() {
        let table = docx_rs::Table::new(vec![
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(DocxRun::new().add_text("Name"))),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(DocxRun::new().add_text("Age"))),
            ]),
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(DocxRun::new().add_text("Ada"))),
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(DocxRun::new().add_text("30"))),
            ]),
        ]);
        let docx = Docx::new().add_table(table);
        let bytes = build_docx(docx);

        let md = docx_to_markdown(&bytes).unwrap();
        assert_eq!(md, "| Name | Age |\n| --- | --- |\n| Ada | 30 |\n");
    }

    #[test]
    fn rejects_invalid_docx_bytes() {
        assert!(docx_to_markdown(b"not a docx file").is_err());
    }

    #[test]
    fn escapes_markdown_metacharacters_in_run_text() {
        let docx = Docx::new().add_paragraph(
            Paragraph::new().add_run(DocxRun::new().add_text("file_name_v2.txt has an *asterisk*")),
        );
        let bytes = build_docx(docx);

        let md = docx_to_markdown(&bytes).unwrap();
        assert_eq!(md, r"file\_name\_v2.txt has an \*asterisk\*");
    }
}
