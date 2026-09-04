use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_CHUNK_CHARS: usize = 2_000;
const CHUNK_OVERLAP_CHARS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Chunk {
    pub path: String,
    pub heading: String,
    pub text: String,
    pub ordinal: usize,
}

pub fn content_hash(content: &str) -> String {
    hex::encode(Sha256::digest(content.as_bytes()))
}

fn char_slice(text: &str, start: usize, end: usize) -> String {
    text.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

pub fn chunk_markdown(path: &str, content: &str) -> Vec<Chunk> {
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut heading = "Document".to_string();
    let mut body = String::new();
    for line in content.lines() {
        if let Some(label) = line.trim_start().strip_prefix('#') {
            let label = label.trim_start_matches('#').trim();
            if !label.is_empty() {
                if !body.trim().is_empty() {
                    sections.push((heading, body.trim().to_string()));
                    body.clear();
                }
                heading = label.to_string();
                continue;
            }
        }
        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(line);
    }
    if !body.trim().is_empty() || sections.is_empty() {
        sections.push((heading, body.trim().to_string()));
    }

    let mut chunks = Vec::new();
    for (heading, text) in sections {
        let count = text.chars().count();
        if count == 0 {
            continue;
        }
        let mut start = 0;
        while start < count {
            let end = (start + MAX_CHUNK_CHARS).min(count);
            chunks.push(Chunk {
                path: path.to_string(),
                heading: heading.clone(),
                text: char_slice(&text, start, end),
                ordinal: chunks.len(),
            });
            if end == count {
                break;
            }
            start = end.saturating_sub(CHUNK_OVERLAP_CHARS);
        }
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_markdown_by_heading() {
        let chunks = chunk_markdown("guide.md", "# Intro\nHello\n## Setup\nInstall things");
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].heading, "Intro");
        assert_eq!(chunks[1].heading, "Setup");
        assert!(chunks[1].text.contains("Install things"));
    }

    #[test]
    fn long_sections_are_bounded_and_overlap() {
        let body = format!("# Long\n{}", "word ".repeat(900));
        let chunks = chunk_markdown("long.md", &body);
        assert!(chunks.len() > 1);
        assert!(chunks
            .iter()
            .all(|chunk| chunk.text.chars().count() <= MAX_CHUNK_CHARS));
        let tail: String = chunks[0].text.chars().rev().take(30).collect();
        let head: String = chunks[1].text.chars().take(200).collect();
        assert!(tail
            .chars()
            .rev()
            .collect::<String>()
            .split_whitespace()
            .any(|word| head.contains(word)));
    }

    #[test]
    fn hash_is_stable_and_content_sensitive() {
        assert_eq!(content_hash("same"), content_hash("same"));
        assert_ne!(content_hash("same"), content_hash("different"));
    }
}
