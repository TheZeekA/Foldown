use std::path::{Path, PathBuf};

use tauri::State;

use crate::ai::index::KnowledgeIndex;
use crate::error::{AppError, AppResult};
use crate::fs::ops;
use crate::history::{HistoryEntry, HistoryStore};
use crate::workspace_authority::ActiveWorkspace;

const DEFAULT_RETENTION: usize = 50;

fn resolve_target(active: &ActiveWorkspace, workspace_root: &str, path: &str) -> AppResult<(PathBuf, PathBuf, PathBuf)> {
    let root = active.require(Path::new(workspace_root))?;
    let requested = Path::new(path);
    let requested = if requested.is_absolute() { requested.to_path_buf() } else { root.join(requested) };
    let target = ops::ensure_within_workspace(&requested, &root)?;
    let relative = target
        .strip_prefix(&root)
        .map_err(|_| AppError::Message("The file is outside the workspace".to_string()))?
        .to_path_buf();
    Ok((root, target, relative))
}

fn snapshot_belongs_to(root: &Path, relative: &Path, snapshot: &(String, String, String)) -> AppResult<()> {
    let expected_root = root.canonicalize()?.to_string_lossy().to_lowercase();
    let expected_relative = relative.to_string_lossy().replace('\\', "/").to_lowercase();
    if snapshot.0 != expected_root || snapshot.1 != expected_relative {
        return Err(AppError::Message("That history entry does not belong to this file".to_string()));
    }
    Ok(())
}

#[tauri::command]
pub fn record_history_snapshot(
    history: State<HistoryStore>,
    active: State<ActiveWorkspace>,
    workspace_root: String,
    path: String,
    content: String,
) -> AppResult<()> {
    let (root, _target, relative) = resolve_target(&active, &workspace_root, &path)?;
    history.record_snapshot(&root, &relative, &content, DEFAULT_RETENTION)
}

#[tauri::command]
pub fn list_history(
    history: State<HistoryStore>,
    active: State<ActiveWorkspace>,
    workspace_root: String,
    path: String,
) -> AppResult<Vec<HistoryEntry>> {
    let (root, _target, relative) = resolve_target(&active, &workspace_root, &path)?;
    history.list_snapshots(&root, &relative)
}

#[tauri::command]
pub fn get_history_content(
    history: State<HistoryStore>,
    active: State<ActiveWorkspace>,
    id: i64,
    workspace_root: String,
    path: String,
) -> AppResult<String> {
    let (root, _target, relative) = resolve_target(&active, &workspace_root, &path)?;
    let snapshot = history.snapshot(id)?.ok_or_else(|| AppError::Message("History entry not found".to_string()))?;
    snapshot_belongs_to(&root, &relative, &snapshot)?;
    Ok(snapshot.2)
}

#[tauri::command]
pub fn delete_history_snapshot(history: State<HistoryStore>, id: i64) -> AppResult<()> {
    history.delete_snapshot(id)
}

#[tauri::command]
pub fn clear_history(
    history: State<HistoryStore>,
    active: State<ActiveWorkspace>,
    workspace_root: String,
    path: Option<String>,
) -> AppResult<()> {
    let root = active.require(Path::new(&workspace_root))?;
    let relative = path.as_deref().map(|value| resolve_target(&active, &workspace_root, value).map(|(_, _, relative)| relative)).transpose()?;
    history.clear_snapshots(&root, relative.as_deref())
}

#[tauri::command]
pub fn restore_history_snapshot(
    history: State<HistoryStore>,
    index: State<KnowledgeIndex>,
    active: State<ActiveWorkspace>,
    id: i64,
    workspace_root: String,
    path: String,
    expected_current_content: String,
) -> AppResult<()> {
    let (root, target, relative) = resolve_target(&active, &workspace_root, &path)?;
    let snapshot = history.snapshot(id)?.ok_or_else(|| AppError::Message("History entry not found".to_string()))?;
    snapshot_belongs_to(&root, &relative, &snapshot)?;
    let current = ops::read_file(&target)?;
    if current != expected_current_content {
        return Err(AppError::Message("The file changed outside Foldown; history restore was cancelled".to_string()));
    }
    history.record_snapshot(&root, &relative, &current, DEFAULT_RETENTION)?;
    ops::write_file_atomic(&target, &snapshot.2)?;
    index.refresh_path(&root, &target)
}

