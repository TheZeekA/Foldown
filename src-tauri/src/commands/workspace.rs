use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::fs::tree::{build_tree, TreeNode};
use crate::settings::store::SettingsStore;
use crate::workspace_authority::ActiveWorkspace;
use tauri::State;

#[tauri::command]
pub fn get_tree(
    active: State<ActiveWorkspace>,
    workspace_path: String,
    show_all_files: bool,
) -> AppResult<Vec<TreeNode>> {
    let root = active.require(Path::new(&workspace_path))?;
    Ok(build_tree(&root, show_all_files))
}

pub fn validate_workspace_name(name: &str) -> AppResult<()> {
    if name.is_empty()
        || name.trim() != name
        || matches!(name, "." | "..")
        || name.ends_with(['.', ' '])
        || name.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
        })
    {
        return Err(AppError::Message(
            "Enter a valid workspace name without path separators or Windows reserved characters"
                .to_string(),
        ));
    }
    Ok(())
}

pub fn create_workspace_folder(parent: &Path, name: &str) -> AppResult<PathBuf> {
    validate_workspace_name(name)?;
    if !parent.is_dir() {
        return Err(AppError::Message(format!(
            "\"{}\" is not a folder that exists",
            parent.display()
        )));
    }
    let target = parent.join(name);
    std::fs::create_dir(&target).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            AppError::Message(format!("A folder named \"{name}\" already exists"))
        } else {
            error.into()
        }
    })?;
    Ok(target.canonicalize()?)
}

#[tauri::command]
pub fn create_workspace(
    store: State<SettingsStore>,
    active: State<ActiveWorkspace>,
    parent_path: String,
    name: String,
) -> AppResult<String> {
    let created = create_workspace_folder(Path::new(&parent_path), &name)?;
    let root = active.activate(&created)?;
    store.touch_recent_workspace(&root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn validates_a_single_safe_windows_folder_name() {
        assert!(validate_workspace_name("Project Notes").is_ok());
        for invalid in [
            "",
            "   ",
            ".",
            "..",
            "a/b",
            "a\\b",
            "a:b",
            "a*",
            "a?",
            "a\"b",
            "a<",
            "a>",
            "a|",
            "trailing.",
            "trailing ",
        ] {
            assert!(
                validate_workspace_name(invalid).is_err(),
                "accepted {invalid:?}"
            );
        }
    }

    #[test]
    fn creates_a_new_folder_without_overwriting() {
        let root = std::env::temp_dir().join(format!(
            "foldown-create-workspace-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let created = create_workspace_folder(&root, "Project Notes").unwrap();
        assert!(created.is_dir());
        assert_eq!(created.file_name().unwrap(), "Project Notes");
        assert!(create_workspace_folder(&root, "Project Notes").is_err());
    }
}
