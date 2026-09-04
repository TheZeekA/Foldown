use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path};

use serde::Serialize;
use serde_yaml::Value;

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WikiLink {
    pub raw_target: String,
    pub display_text: String,
    pub target_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LinkStatus {
    Resolved,
    Unresolved,
    Ambiguous,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinkRecord {
    pub source_path: String,
    pub raw_target: String,
    pub display_text: String,
    pub resolved_path: Option<String>,
    pub status: LinkStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TagSummary {
    pub tag: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthFinding {
    pub category: String,
    pub severity: String,
    pub path: String,
    pub message: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TagExtraction {
    pub tags: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceMetadata {
    pub links: Vec<LinkRecord>,
    pub tags: Vec<TagSummary>,
    pub files_for_tag: HashMap<String, Vec<String>>,
    pub health: Vec<HealthFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkResolution {
    Resolved(String),
    Unresolved,
    Ambiguous(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceKind {
    MarkdownLink,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub target: String,
    pub kind: ReferenceKind,
}

pub fn normalize_tag(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('#').trim().to_lowercase();
    (!value.is_empty()).then_some(value)
}

pub fn extract_tags(frontmatter: &str) -> TagExtraction {
    let parsed = match serde_yaml::from_str::<Value>(frontmatter) {
        Ok(value) => value,
        Err(error) => return TagExtraction { tags: Vec::new(), error: Some(error.to_string()) },
    };
    let Some(mapping) = parsed.as_mapping() else {
        return TagExtraction { tags: Vec::new(), error: None };
    };
    let mut tags = Vec::new();
    for key in ["tags", "tag"] {
        let Some(value) = mapping.get(Value::String(key.to_string())) else { continue };
        match value {
            Value::Sequence(values) => tags.extend(values.iter().filter_map(Value::as_str).filter_map(normalize_tag)),
            Value::String(value) => tags.extend(value.split(',').filter_map(normalize_tag)),
            _ => {}
        }
    }
    tags.sort();
    tags.dedup();
    TagExtraction { tags, error: None }
}

fn frontmatter(content: &str) -> Option<(&str, usize, Option<String>)> {
    let rest = content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n"))?;
    let closing = rest.find("\n---").or_else(|| rest.find("\n..."));
    let Some(index) = closing else { return Some((rest, 0, Some("Frontmatter has no closing delimiter".to_string()))); };
    let body = &rest[..index];
    let consumed = content.len() - rest.len() + index + 1;
    Some((body, consumed, None))
}

fn normalized_path(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => { let _ = parts.pop(); }
            Component::Normal(value) => parts.push(value.to_string_lossy().into_owned()),
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    parts.join("/").to_lowercase()
}

fn normalized(path: &str) -> String { normalized_path(Path::new(path)) }

fn is_external(target: &str) -> bool {
    let lower = target.trim().to_lowercase();
    lower.starts_with("#") || lower.starts_with("mailto:") || lower.contains("://")
}

pub fn parse_wiki_links(content: &str) -> Vec<WikiLink> {
    let mut output = Vec::new();
    let mut fenced = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") { fenced = !fenced; continue; }
        if fenced { continue; }
        let mut cursor = 0;
        while let Some(start) = line[cursor..].find("[[") {
            let start = cursor + start;
            let Some(end) = line[start + 2..].find("]] ").map(|v| v).or_else(|| line[start + 2..].find("]]")) else { break };
            let end = start + 2 + end;
            let raw = line[start + 2..end].trim();
            cursor = end + 2;
            if raw.is_empty() || raw.contains('#') || is_external(raw) { continue; }
            let mut parts = raw.splitn(2, '|');
            let target = parts.next().unwrap_or_default().trim();
            if target.is_empty() || is_external(target) { continue; }
            output.push(WikiLink { raw_target: target.to_string(), display_text: parts.next().unwrap_or(target).trim().to_string(), target_path: target.to_string() });
        }
    }
    output
}

pub fn resolve_wiki_link(source_relative: &Path, target: &str, documents: &[String]) -> LinkResolution {
    let target = target.trim().replace('\\', "/");
    let source_dir = source_relative.parent().unwrap_or_else(|| Path::new(""));
    let candidate = normalized_path(&source_dir.join(&target));
    let candidate = if candidate.ends_with(".md") { candidate } else { format!("{candidate}.md") };
    let document_keys: HashSet<String> = documents.iter().map(|path| normalized(path)).collect();
    if document_keys.contains(&candidate) { return LinkResolution::Resolved(candidate); }
    if !target.contains('/') {
        let name = normalized(&target);
        let matches = documents.iter().filter(|path| {
            let key = normalized(path);
            key == format!("{name}.md") || key.rsplit('/').next().is_some_and(|part| part == name || part == format!("{name}.md"))
        }).map(|path| normalized(path)).collect::<Vec<_>>();
        if matches.len() == 1 { return LinkResolution::Resolved(matches[0].clone()); }
        if matches.len() > 1 { return LinkResolution::Ambiguous(matches); }
    }
    LinkResolution::Unresolved
}

pub fn scan_markdown_references(source_relative: &Path, content: &str) -> Vec<Reference> {
    let mut output = Vec::new();
    let mut fenced = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") { fenced = !fenced; continue; }
        if fenced { continue; }
        let mut cursor = 0;
        while let Some(open) = line[cursor..].find('(') {
            let open = cursor + open;
            let Some(close) = line[open + 1..].find(')') else { break };
            let close = open + 1 + close;
            let target = line[open + 1..close].split_whitespace().next().unwrap_or_default().trim_matches(['<', '>']);
            cursor = close + 1;
            if target.is_empty() || is_external(target) { continue; }
            let kind = if line[..open].rfind("![").is_some_and(|marker| line[marker..open].contains(']')) { ReferenceKind::Image } else { ReferenceKind::MarkdownLink };
            let _ = source_relative;
            output.push(Reference { target: target.to_string(), kind });
        }
    }
    output
}

fn walk(root: &Path, dir: &Path, docs: &mut Vec<(String, String)>, errors: &mut Vec<HealthFinding>) -> AppResult<()> {
    for entry in fs::read_dir(dir)?.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') { continue; }
        let path = entry.path();
        if path.is_dir() { walk(root, &path, docs, errors)?; continue; }
        if !path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")) { continue; }
        let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        match fs::read_to_string(&path) {
            Ok(content) => docs.push((relative, content)),
            Err(error) => errors.push(HealthFinding { category: "unreadable-file".to_string(), severity: "error".to_string(), path: relative, message: error.to_string(), target: None }),
        }
    }
    Ok(())
}

pub fn scan_workspace(root: &Path) -> AppResult<WorkspaceMetadata> {
    let root = root.canonicalize()?;
    let mut docs = Vec::new();
    let mut health = Vec::new();
    walk(&root, &root, &mut docs, &mut health)?;
    let paths = docs.iter().map(|(path, _)| path.clone()).collect::<Vec<_>>();
    let mut links = Vec::new();
    let mut tag_files: HashMap<String, Vec<String>> = HashMap::new();
    for (path, content) in &docs {
        if content.trim().is_empty() { health.push(HealthFinding { category: "empty-file".to_string(), severity: "warning".to_string(), path: path.clone(), message: "Markdown file is empty".to_string(), target: None }); }
        let (body, front_error) = match frontmatter(content) { Some((yaml, consumed, error)) => (&content[consumed..], extract_tags(yaml).error.or(error)), None => (content.as_str(), None) };
        if let Some(error) = front_error { health.push(HealthFinding { category: "invalid-frontmatter".to_string(), severity: "error".to_string(), path: path.clone(), message: error, target: None }); }
        if let Some((yaml, _, _)) = frontmatter(content) { for tag in extract_tags(yaml).tags { tag_files.entry(tag).or_default().push(path.clone()); } }
        for wiki in parse_wiki_links(body) {
            let resolution = resolve_wiki_link(Path::new(path), &wiki.target_path, &paths);
            let (resolved_path, status) = match resolution { LinkResolution::Resolved(path) => (Some(path), LinkStatus::Resolved), LinkResolution::Unresolved => (None, LinkStatus::Unresolved), LinkResolution::Ambiguous(_) => (None, LinkStatus::Ambiguous) };
            if status != LinkStatus::Resolved { health.push(HealthFinding { category: "wiki-link".to_string(), severity: "warning".to_string(), path: path.clone(), message: if status == LinkStatus::Ambiguous { "Wiki link has multiple possible targets".to_string() } else { "Wiki link target was not found".to_string() }, target: Some(wiki.raw_target.clone()) }); }
            links.push(LinkRecord { source_path: path.clone(), raw_target: wiki.raw_target, display_text: wiki.display_text, resolved_path, status });
        }
        for reference in scan_markdown_references(Path::new(path), body) {
            let target = reference.target.split(['#', '?']).next().unwrap_or_default();
            let target_path = Path::new(path).parent().unwrap_or_else(|| Path::new("")).join(target);
            if !root.join(&target_path).is_file() {
                health.push(HealthFinding { category: match reference.kind { ReferenceKind::Image => "missing-asset", ReferenceKind::MarkdownLink => "broken-link" }.to_string(), severity: "warning".to_string(), path: path.clone(), message: "Local target was not found".to_string(), target: Some(reference.target) });
            }
        }
    }
    for files in tag_files.values_mut() { files.sort(); }
    let mut tags = tag_files.iter().map(|(tag, files)| TagSummary { tag: tag.clone(), count: files.len() }).collect::<Vec<_>>();
    tags.sort_by(|left, right| left.tag.cmp(&right.tag));
    links.sort_by(|left, right| (&left.source_path, &left.raw_target).cmp(&(&right.source_path, &right.raw_target)));
    health.sort_by(|left, right| (&left.path, &left.category, &left.message).cmp(&(&right.path, &right.category, &right.message)));
    Ok(WorkspaceMetadata { links, tags, files_for_tag: tag_files, health })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_links_and_labels_but_ignores_code_and_heading_targets() {
        let links = parse_wiki_links("[[Project Plan|the plan]]\n```\n[[hidden]]\n```\n[[Project Plan#Next]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].display_text, "the plan");
    }

    #[test]
    fn resolves_exact_nested_and_reports_ambiguity() {
        let docs = vec!["Project Plan.md".to_string(), "archive/Project Plan.md".to_string()];
        assert_eq!(resolve_wiki_link(Path::new("notes/today.md"), "../Project Plan", &docs), LinkResolution::Resolved("project plan.md".to_string()));
        assert!(matches!(resolve_wiki_link(Path::new("notes/today.md"), "Project Plan", &docs), LinkResolution::Ambiguous(_)));
    }

    #[test]
    fn extracts_list_and_string_tags() {
        let tags = extract_tags("tags: [Rust, '#Notes']\ntag: work, urgent");
        assert_eq!(tags.tags, vec!["notes", "rust", "urgent", "work"]);
        assert!(tags.error.is_none());
    }

    #[test]
    fn scans_local_references_and_ignores_external_urls() {
        let refs = scan_markdown_references(Path::new("note.md"), "[local](other.md) ![image](img.png) [web](https://example.com)");
        assert_eq!(refs, vec![Reference { target: "other.md".to_string(), kind: ReferenceKind::MarkdownLink }, Reference { target: "img.png".to_string(), kind: ReferenceKind::Image }]);
    }
}
