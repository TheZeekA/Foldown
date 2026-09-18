use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::Manager;

use crate::error::{AppError, AppResult};

/// Grants the asset protocol read access to `root` so local images inside it
/// can be loaded via `convertFileSrc`. The protocol's static config scope is
/// intentionally empty (see `tauri.conf.json`) — access is granted only for
/// the folder the user actually opened, instead of the whole filesystem.
pub fn allow_asset_scope(app: &tauri::AppHandle, root: &Path) {
    let _ = app.asset_protocol_scope().allow_directory(root, true);
}

#[derive(Default)]
pub struct ActiveWorkspace {
    root: Mutex<Option<PathBuf>>,
}

impl ActiveWorkspace {
    pub fn activate(&self, path: &Path) -> AppResult<PathBuf> {
        let canonical = path
            .canonicalize()
            .map_err(|_| AppError::Message("Workspace folder not found".to_string()))?;
        if !canonical.is_dir() {
            return Err(AppError::Message(
                "Workspace path is not a folder".to_string(),
            ));
        }
        *self.root.lock().unwrap() = Some(canonical.clone());
        Ok(canonical)
    }

    pub fn require(&self, requested: &Path) -> AppResult<PathBuf> {
        let requested = requested
            .canonicalize()
            .map_err(|_| AppError::Message("Workspace folder not found".to_string()))?;
        let active = self.root.lock().unwrap();
        match active.as_ref() {
            Some(root) if *root == requested => Ok(root.clone()),
            Some(_) => Err(AppError::Message(
                "The requested folder is not the active workspace".to_string(),
            )),
            None => Err(AppError::Message(
                "No workspace is currently open".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "foldown-authority-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn rejects_a_root_other_than_the_active_workspace() {
        let active = temp_dir("active");
        let other = temp_dir("other");
        let authority = ActiveWorkspace::default();

        authority.activate(&active).unwrap();

        assert_eq!(
            authority.require(&active).unwrap(),
            active.canonicalize().unwrap()
        );
        assert!(authority.require(&other).is_err());
    }

    #[test]
    fn rejects_commands_before_a_workspace_is_active() {
        let root = temp_dir("inactive");
        assert!(ActiveWorkspace::default().require(&root).is_err());
    }
}
