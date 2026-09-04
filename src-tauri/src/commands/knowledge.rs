use std::path::Path;

use serde::Serialize;
use tauri::State;

use crate::ai::index::KnowledgeIndex;
use crate::error::AppResult;
use crate::fs::ops;
use crate::knowledge::{HealthFinding, LinkRecord, LinkStatus, TagSummary};
use crate::workspace_authority::ActiveWorkspace;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLinks {
    pub backlinks: Vec<LinkRecord>,
    pub outgoing: Vec<LinkRecord>,
    pub unresolved: Vec<LinkRecord>,
}

fn active_relative_path(active: &ActiveWorkspace, workspace_root: &str, path: &str) -> AppResult<String> {
    let root = active.require(Path::new(workspace_root))?;
    let requested = Path::new(path);
    let requested = if requested.is_absolute() { requested.to_path_buf() } else { root.join(requested) };
    let target = ops::ensure_within_workspace(&requested, &root)?;
    Ok(target.strip_prefix(&root).unwrap_or(&target).to_string_lossy().replace('\\', "/").to_lowercase())
}

#[tauri::command]
pub fn get_workspace_links(index: State<KnowledgeIndex>, active: State<ActiveWorkspace>, workspace_root: String, active_path: Option<String>) -> AppResult<WorkspaceLinks> {
    let root = active.require(Path::new(&workspace_root))?;
    let links = index.workspace_links(&root)?;
    let active_key = active_path.as_deref().map(|path| active_relative_path(&active, &workspace_root, path)).transpose()?;
    let backlinks = active_key.as_deref().map(|key| links.iter().filter(|link| link.resolved_path.as_deref().is_some_and(|path| path.to_lowercase() == *key)).cloned().collect()).unwrap_or_default();
    let outgoing: Vec<LinkRecord> = active_key.as_deref().map(|key| links.iter().filter(|link| link.source_path.to_lowercase() == *key).cloned().collect()).unwrap_or_default();
    let unresolved: Vec<LinkRecord> = outgoing.iter().filter(|link| link.status != LinkStatus::Resolved).cloned().collect();
    Ok(WorkspaceLinks { backlinks, outgoing, unresolved })
}

#[tauri::command]
pub fn get_workspace_tags(index: State<KnowledgeIndex>, active: State<ActiveWorkspace>, workspace_root: String) -> AppResult<Vec<TagSummary>> {
    let root = active.require(Path::new(&workspace_root))?;
    index.workspace_tags(&root)
}

#[tauri::command]
pub fn get_files_for_tag(index: State<KnowledgeIndex>, active: State<ActiveWorkspace>, workspace_root: String, tag: String) -> AppResult<Vec<String>> {
    let root = active.require(Path::new(&workspace_root))?;
    index.files_for_tag(&root, &tag)
}

#[tauri::command]
pub fn get_workspace_health(index: State<KnowledgeIndex>, active: State<ActiveWorkspace>, workspace_root: String) -> AppResult<Vec<HealthFinding>> {
    let root = active.require(Path::new(&workspace_root))?;
    index.workspace_health(&root)
}
