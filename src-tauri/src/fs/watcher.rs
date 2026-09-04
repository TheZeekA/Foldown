use std::path::PathBuf;
use std::sync::Mutex;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

use crate::error::{AppError, AppResult};

pub const FILE_CHANGED_EVENT: &str = "file-changed";
pub const WORKSPACE_CHANGED_EVENT: &str = "workspace-changed";

fn is_workspace_change(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
    )
}

/// Watches the parent directory of the currently open file (not the file
/// itself — a watch on the exact path can silently stop firing once our own
/// atomic save replaces it via rename) and emits `FILE_CHANGED_EVENT` when
/// that specific file is modified or replaced.
pub struct FileWatcher {
    inner: Mutex<Option<(RecommendedWatcher, PathBuf)>>,
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// `canonical_path` is used to resolve the directory to watch;
    /// `display_path` (the same path as the frontend's `openPath`, *not*
    /// canonicalized) is what gets emitted, so the frontend's plain string
    /// comparison against `openPath` actually matches — canonicalize() adds a
    /// `\\?\` extended-path prefix on Windows that the tree listing never has.
    pub fn watch(
        &self,
        app: AppHandle,
        canonical_path: PathBuf,
        display_path: String,
    ) -> AppResult<()> {
        let parent = canonical_path
            .parent()
            .ok_or_else(|| AppError::Message("File has no parent folder".to_string()))?
            .to_path_buf();

        let watched_name = canonical_path.file_name().map(|n| n.to_os_string());
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let Ok(event) = res else { return };
            let Some(name) = &watched_name else { return };
            let is_relevant = matches!(
                event.kind,
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
            ) && event
                .paths
                .iter()
                .any(|p| p.file_name() == Some(name.as_os_str()));
            if is_relevant {
                let _ = app.emit(FILE_CHANGED_EVENT, display_path.clone());
            }
        })
        .map_err(|e| AppError::Message(format!("Could not watch file: {e}")))?;

        watcher
            .watch(&parent, RecursiveMode::NonRecursive)
            .map_err(|e| AppError::Message(format!("Could not watch file: {e}")))?;

        *self.inner.lock().unwrap() = Some((watcher, canonical_path));
        Ok(())
    }

    pub fn unwatch(&self) {
        *self.inner.lock().unwrap() = None;
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Watches the active workspace recursively so the sidebar can notice files
/// created or removed outside Foldown, including when no file is open.
pub struct WorkspaceWatcher {
    inner: Mutex<Option<(RecommendedWatcher, PathBuf)>>,
}

impl WorkspaceWatcher {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    pub fn watch(&self, app: AppHandle, root: PathBuf) -> AppResult<()> {
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let Ok(event) = res else { return };
            if is_workspace_change(&event) {
                let _ = app.emit(WORKSPACE_CHANGED_EVENT, ());
            }
        })
        .map_err(|e| AppError::Message(format!("Could not watch workspace: {e}")))?;

        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| AppError::Message(format!("Could not watch workspace: {e}")))?;

        *self.inner.lock().unwrap() = Some((watcher, root));
        Ok(())
    }
}

impl Default for WorkspaceWatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, CreateKind};

    #[test]
    fn workspace_changes_include_external_file_creation() {
        let event = Event::new(EventKind::Create(CreateKind::File));
        assert!(is_workspace_change(&event));
    }

    #[test]
    fn workspace_changes_ignore_access_notifications() {
        let event = Event::new(EventKind::Access(AccessKind::Read));
        assert!(!is_workspace_change(&event));
    }
}
