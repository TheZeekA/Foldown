use serde::Serialize;
use std::path::Path;
use tauri::State;

use crate::native::PendingOpen;
use crate::workspace_authority::ActiveWorkspace;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SingleFileSession {
    pub path: String,
    pub root: String,
}

fn prepare_single_file_session(
    path: &Path,
    active: &ActiveWorkspace,
) -> Result<SingleFileSession, String> {
    let canonical = path
        .canonicalize()
        .map_err(|_| "Markdown file not found".to_string())?;
    if !canonical.is_file()
        || canonical
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| !value.eq_ignore_ascii_case("md"))
            .unwrap_or(true)
    {
        return Err("Single-file mode requires a Markdown (.md) file".into());
    }
    let parent = canonical
        .parent()
        .ok_or_else(|| "Markdown file has no parent folder".to_string())?;
    let root = active.activate(parent).map_err(|error| error.to_string())?;
    Ok(SingleFileSession {
        path: canonical.to_string_lossy().into_owned(),
        root: root.to_string_lossy().into_owned(),
    })
}

/// Pulled once by the frontend on launch to pick up a file passed via
/// "Open with Foldown" or a second-instance relaunch — see `PendingOpen`.
#[tauri::command]
pub fn take_pending_open(state: State<PendingOpen>) -> Option<String> {
    state.0.lock().unwrap().take()
}

#[tauri::command]
pub fn open_single_file(
    active: State<ActiveWorkspace>,
    path: String,
) -> Result<SingleFileSession, String> {
    prepare_single_file_session(Path::new(&path), &active)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn prepares_only_an_existing_markdown_file() {
        let dir = std::env::temp_dir().join(format!("foldown-single-file-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let markdown = dir.join("note.md");
        let text = dir.join("note.txt");
        fs::write(&markdown, "# Note").unwrap();
        fs::write(&text, "text").unwrap();
        let active = ActiveWorkspace::default();

        let session = prepare_single_file_session(&markdown, &active).unwrap();
        assert_eq!(PathBuf::from(session.path), markdown.canonicalize().unwrap());
        assert_eq!(PathBuf::from(session.root), dir.canonicalize().unwrap());
        assert!(prepare_single_file_session(&text, &active).is_err());
        assert!(prepare_single_file_session(&dir.join("missing.md"), &active).is_err());
    }
}
