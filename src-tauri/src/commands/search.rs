use std::path::PathBuf;

use tauri::State;

use crate::error::AppResult;
use crate::search::index::{SearchIndex, SearchResult};
use crate::workspace_authority::ActiveWorkspace;

#[tauri::command]
pub fn index_workspace(
    active: State<ActiveWorkspace>,
    index: State<SearchIndex>,
    workspace_root: String,
) -> AppResult<()> {
    let root = active.require(&PathBuf::from(workspace_root))?;
    index.build(root)
}

#[tauri::command]
pub fn search_workspace(index: State<SearchIndex>, query: String) -> AppResult<Vec<SearchResult>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    index.search(&query)
}
