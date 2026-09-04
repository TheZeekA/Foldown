use std::path::Path;

use tauri::{AppHandle, State};

use crate::ai::index::KnowledgeIndex;
use crate::error::AppResult;
use crate::fs::ops;
use crate::fs::watcher::{FileWatcher, WorkspaceWatcher};
use crate::native;
use crate::workspace_authority::ActiveWorkspace;

#[tauri::command]
pub fn read_file(
    active: State<ActiveWorkspace>,
    path: String,
    workspace_root: String,
) -> AppResult<String> {
    let root = active.require(Path::new(&workspace_root))?;
    let target = ops::ensure_within_workspace(Path::new(&path), &root)?;
    let contents = ops::read_file(&target)?;
    // Keep the frontend's plain path here; Windows shell APIs cannot resolve
    // the canonical path's extended `\\?\` prefix.
    native::add_to_recent_docs(Path::new(&path));
    Ok(contents)
}

#[tauri::command]
/// Copies an external file into the active workspace root.
pub fn import_file(
    active: State<ActiveWorkspace>,
    index: State<KnowledgeIndex>,
    source_path: String,
    workspace_root: String,
) -> AppResult<String> {
    let root = active.require(Path::new(&workspace_root))?;
    let imported = ops::import_file(Path::new(&source_path), &root)?;
    index.refresh_path(&root, &imported)?;
    Ok(imported.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn save_file(
    active: State<ActiveWorkspace>,
    index: State<KnowledgeIndex>,
    path: String,
    workspace_root: String,
    contents: String,
) -> AppResult<()> {
    let root = active.require(Path::new(&workspace_root))?;
    let target = ops::ensure_within_workspace(Path::new(&path), &root)?;
    ops::write_file_atomic(&target, &contents)?;
    index.refresh_path(&root, &target)
}

#[tauri::command]
pub fn watch_file(
    app: AppHandle,
    watcher: State<FileWatcher>,
    active: State<ActiveWorkspace>,
    path: String,
    workspace_root: String,
) -> AppResult<()> {
    let root = active.require(Path::new(&workspace_root))?;
    let target = ops::ensure_within_workspace(Path::new(&path), &root)?;
    watcher.watch(app, target, path)
}

#[tauri::command]
pub fn unwatch_file(watcher: State<FileWatcher>) {
    watcher.unwatch();
}

#[tauri::command]
pub fn watch_workspace(
    app: AppHandle,
    watcher: State<WorkspaceWatcher>,
    active: State<ActiveWorkspace>,
    workspace_root: String,
) -> AppResult<()> {
    let root = active.require(Path::new(&workspace_root))?;
    watcher.watch(app, root)
}

#[tauri::command]
pub fn create_file(
    active: State<ActiveWorkspace>,
    index: State<KnowledgeIndex>,
    path: String,
    workspace_root: String,
) -> AppResult<()> {
    let root = active.require(Path::new(&workspace_root))?;
    let target = ops::ensure_within_workspace(Path::new(&path), &root)?;
    ops::create_file(&target)?;
    index.refresh_path(&root, &target)
}

#[tauri::command]
pub fn create_folder(
    active: State<ActiveWorkspace>,
    path: String,
    workspace_root: String,
) -> AppResult<()> {
    let root = active.require(Path::new(&workspace_root))?;
    let target = ops::ensure_within_workspace(Path::new(&path), &root)?;
    ops::create_folder(&target)
}

#[tauri::command]
pub fn move_path(
    active: State<ActiveWorkspace>,
    index: State<KnowledgeIndex>,
    old_path: String,
    new_path: String,
    workspace_root: String,
) -> AppResult<()> {
    let root = active.require(Path::new(&workspace_root))?;
    let old = ops::ensure_within_workspace(Path::new(&old_path), &root)?;
    let new = ops::ensure_within_workspace(Path::new(&new_path), &root)?;
    ops::move_path(&old, &new)?;
    index.sync_workspace(&root)
}

#[tauri::command]
pub fn delete_path(
    active: State<ActiveWorkspace>,
    index: State<KnowledgeIndex>,
    path: String,
    workspace_root: String,
) -> AppResult<()> {
    let root = active.require(Path::new(&workspace_root))?;
    let target = ops::ensure_within_workspace(Path::new(&path), &root)?;
    ops::delete_path(&target)?;
    index.sync_workspace(&root)
}

#[tauri::command]
pub fn duplicate_path(
    active: State<ActiveWorkspace>,
    index: State<KnowledgeIndex>,
    path: String,
    workspace_root: String,
) -> AppResult<String> {
    let root = active.require(Path::new(&workspace_root))?;
    let target = ops::ensure_within_workspace(Path::new(&path), &root)?;
    let new_path = ops::duplicate_path(&target)?;
    index.sync_workspace(&root)?;
    Ok(new_path.to_string_lossy().into_owned())
}
