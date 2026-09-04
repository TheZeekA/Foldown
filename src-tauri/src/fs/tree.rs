use std::fs;
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TreeNode {
    File {
        name: String,
        path: String,
    },
    Folder {
        name: String,
        path: String,
        children: Vec<TreeNode>,
    },
}

/// Builds the folder/file tree for a workspace root. Dotfiles/dot-folders are
/// always skipped. When `show_all_files` is false, only `.md` files are kept
/// and folders with no surviving descendants are pruned.
pub fn build_tree(root: &Path, show_all_files: bool) -> Vec<TreeNode> {
    read_dir_nodes(root, show_all_files)
}

/// `root` (and therefore every child path built from it) is canonicalized
/// upstream, which on Windows carries the `\\?\` extended-length prefix.
/// That form round-trips fine through Rust's own path APIs, but Windows shell
/// APIs (Open Recent, Jump List) and the display paths this app hands back to
/// the frontend need the plain form — see `commands/files.rs::read_file`'s
/// comment for the same hazard on the read path.
fn display_path(path: &Path) -> String {
    let s = path.to_string_lossy().into_owned();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        s
    }
}

fn read_dir_nodes(dir: &Path, show_all_files: bool) -> Vec<TreeNode> {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());

    let mut folders = Vec::new();
    let mut files = Vec::new();

    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }

        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();

        if file_type.is_dir() {
            // A folder with nothing in it at all (e.g. one the user just
            // created) has an empty filtered children list same as a folder
            // full of non-markdown clutter — but unlike clutter, it should
            // never be pruned, or creating a folder in the Markdown-only view
            // makes it vanish with no feedback that anything happened.
            let is_truly_empty = fs::read_dir(&path)
                .map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .all(|e| e.file_name().to_string_lossy().starts_with('.'))
                })
                .unwrap_or(true);
            let children = read_dir_nodes(&path, show_all_files);
            if show_all_files || !children.is_empty() || is_truly_empty {
                folders.push(TreeNode::Folder {
                    name,
                    path: display_path(&path),
                    children,
                });
            }
        } else if file_type.is_file() {
            let is_markdown = path
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("md"))
                .unwrap_or(false);
            if show_all_files || is_markdown {
                files.push(TreeNode::File {
                    name,
                    path: display_path(&path),
                });
            }
        }
    }

    folders.into_iter().chain(files).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("foldown-tree-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn display_path_strips_windows_extended_length_prefix() {
        // Regression test: get_tree's paths are built from a canonicalized
        // root, which on Windows carries the `\\?\` prefix — that form
        // breaks Windows shell APIs (Open Recent, Jump List) fed a path from
        // the sidebar tree.
        assert_eq!(
            display_path(Path::new(r"\\?\C:\Users\test\notes.md")),
            r"C:\Users\test\notes.md"
        );
        assert_eq!(
            display_path(Path::new(r"\\?\UNC\server\share\notes.md")),
            r"\\server\share\notes.md"
        );
        assert_eq!(
            display_path(Path::new(r"C:\Users\test\notes.md")),
            r"C:\Users\test\notes.md"
        );
    }

    #[test]
    fn filters_to_markdown_only_and_prunes_empty_folders() {
        let root = temp_dir();
        fs::write(root.join("notes.md"), "# hi").unwrap();
        fs::write(root.join("image.png"), []).unwrap();
        fs::create_dir(root.join("assets")).unwrap();
        fs::write(root.join("assets").join("logo.png"), []).unwrap();
        fs::create_dir(root.join("journal")).unwrap();
        fs::write(root.join("journal").join("2024-01-01.md"), "# day").unwrap();

        let tree = build_tree(&root, false);

        // "assets" has no markdown files, so it should be pruned entirely.
        let names: Vec<&str> = tree
            .iter()
            .map(|n| match n {
                TreeNode::File { name, .. } => name.as_str(),
                TreeNode::Folder { name, .. } => name.as_str(),
            })
            .collect();
        assert_eq!(names, vec!["journal", "notes.md"]);
    }

    #[test]
    fn a_freshly_created_empty_folder_is_never_pruned() {
        // Regression test: the Markdown-only view used to drop a folder from
        // the tree the moment it had no markdown descendants — indistinguishable
        // from a folder full of non-markdown clutter, so creating a new empty
        // folder made it vanish with no feedback that anything happened.
        let root = temp_dir();
        fs::write(root.join("notes.md"), "# hi").unwrap();
        fs::create_dir(root.join("New Folder")).unwrap();

        let tree = build_tree(&root, false);
        let names: Vec<&str> = tree
            .iter()
            .map(|n| match n {
                TreeNode::File { name, .. } => name.as_str(),
                TreeNode::Folder { name, .. } => name.as_str(),
            })
            .collect();
        assert_eq!(names, vec!["New Folder", "notes.md"]);
    }

    #[test]
    fn show_all_files_includes_everything() {
        let root = temp_dir();
        fs::write(root.join("notes.md"), "# hi").unwrap();
        fs::write(root.join("image.png"), []).unwrap();
        fs::create_dir(root.join("empty")).unwrap();

        let tree = build_tree(&root, true);
        let names: Vec<&str> = tree
            .iter()
            .map(|n| match n {
                TreeNode::File { name, .. } => name.as_str(),
                TreeNode::Folder { name, .. } => name.as_str(),
            })
            .collect();
        assert_eq!(names, vec!["empty", "image.png", "notes.md"]);
    }

    #[test]
    fn skips_dotfiles_and_dot_folders() {
        let root = temp_dir();
        fs::write(root.join("notes.md"), "# hi").unwrap();
        fs::create_dir(root.join(".obsidian")).unwrap();
        fs::write(root.join(".obsidian").join("config.json"), "{}").unwrap();
        fs::write(root.join(".hidden.md"), "# secret").unwrap();

        let tree = build_tree(&root, true);
        let names: Vec<&str> = tree
            .iter()
            .map(|n| match n {
                TreeNode::File { name, .. } => name.as_str(),
                TreeNode::Folder { name, .. } => name.as_str(),
            })
            .collect();
        assert_eq!(names, vec!["notes.md"]);
    }
}
