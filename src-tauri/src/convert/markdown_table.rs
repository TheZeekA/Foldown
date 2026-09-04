/// Escapes characters that would otherwise be interpreted as Markdown
/// emphasis, code spans, or link/image syntax when they occur in plain
/// source text (e.g. a filename like `file_name_v2.txt` appearing in
/// converted prose, which would otherwise render with unintended emphasis
/// from the underscore pair).
pub fn escape_markdown_inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '*' | '_' | '`' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Renders rows (the first row treated as the header) as a GitHub-flavored
/// Markdown table, padding ragged rows out to the widest row's column count.
pub fn rows_to_markdown_table(rows: Vec<Vec<String>>) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if col_count == 0 {
        return String::new();
    }

    let mut out = String::new();
    for (i, row) in rows.iter().enumerate() {
        let mut cells = row.clone();
        cells.resize(col_count, String::new());
        // A raw newline inside a cell (from a multi-line CSV field, or a Word
        // soft line break) would otherwise split this row across output
        // lines and break the pipe-delimited table structure.
        for cell in &mut cells {
            if cell.contains(['\n', '\r']) {
                *cell = cell.replace("\r\n", "<br>").replace(['\n', '\r'], "<br>");
            }
        }
        out.push_str("| ");
        out.push_str(&cells.join(" | "));
        out.push_str(" |\n");
        if i == 0 {
            out.push_str("| ");
            out.push_str(&vec!["---"; col_count].join(" | "));
            out.push_str(" |\n");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_header_and_rows() {
        let rows = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Ada".to_string(), "30".to_string()],
        ];
        let md = rows_to_markdown_table(rows);
        assert_eq!(md, "| Name | Age |\n| --- | --- |\n| Ada | 30 |\n");
    }

    #[test]
    fn pads_ragged_rows() {
        let rows = vec![
            vec!["A".to_string(), "B".to_string()],
            vec!["C".to_string()],
        ];
        let md = rows_to_markdown_table(rows);
        assert!(md.contains("| C |  |\n"));
    }

    #[test]
    fn empty_rows_produce_empty_string() {
        assert_eq!(rows_to_markdown_table(vec![]), "");
    }

    #[test]
    fn embedded_newlines_in_a_cell_do_not_break_the_table_structure() {
        let rows = vec![
            vec!["Name".to_string(), "Notes".to_string()],
            vec!["Ada".to_string(), "Line one\nLine two".to_string()],
        ];
        let md = rows_to_markdown_table(rows);
        assert_eq!(
            md.lines().count(),
            3,
            "a cell newline must not add an output line:\n{md}"
        );
        assert!(md.contains("Line one<br>Line two"));
    }

    #[test]
    fn escape_markdown_inline_escapes_emphasis_and_code_metacharacters() {
        assert_eq!(
            escape_markdown_inline("file_name_v2.txt"),
            r"file\_name\_v2.txt"
        );
        assert_eq!(
            escape_markdown_inline("*bold* `code` [link]"),
            r"\*bold\* \`code\` \[link\]"
        );
        assert_eq!(escape_markdown_inline("plain text"), "plain text");
    }
}
