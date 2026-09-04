use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{AppError, AppResult};

/// Resolves `path` and checks it falls inside `workspace_root`, canonicalizing
/// through the nearest existing ancestor when `path` itself doesn't exist yet
/// (e.g. a create target). Every mutating operation goes through this first.
///
/// A symlink (or, on Windows, any other reparse point) at `path` itself is
/// always rejected, whether it resolves inside the workspace, resolves
/// outside it, or is dangling. Silently following it would mean the
/// containment check and the actual read/write could land on two different
/// files — see the "doesn't exist yet" branch below, which only resolves the
/// *parent* and reuses `path`'s own file name unchanged.
pub fn ensure_within_workspace(path: &Path, workspace_root: &Path) -> AppResult<PathBuf> {
    let root = workspace_root
        .canonicalize()
        .map_err(|_| AppError::Message("Workspace folder not found".to_string()))?;

    let is_symlink = fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false);
    if is_symlink {
        return Err(AppError::Message(format!(
            "\"{}\" is a symlink, which Foldown doesn't follow",
            path.display()
        )));
    }

    let resolved = if path.exists() {
        path.canonicalize().ok()
    } else {
        path.parent().and_then(|parent| {
            parent
                .canonicalize()
                .ok()
                .map(|p| p.join(path.file_name().unwrap_or_default()))
        })
    };

    match resolved {
        Some(p) if p.starts_with(&root) => Ok(p),
        _ => Err(AppError::Message(format!(
            "\"{}\" is outside the workspace",
            path.display()
        ))),
    }
}

pub fn read_file(path: &Path) -> AppResult<String> {
    if !path.is_file() {
        return Err(AppError::Message(format!(
            "\"{}\" is not a file",
            path.display()
        )));
    }
    let bytes = fs::read(path)?;
    let content = String::from_utf8(bytes).map_err(|_| {
        AppError::Message(format!(
            "\"{}\" is not valid UTF-8 text. Only UTF-8 encoded Markdown files are supported.",
            path.display()
        ))
    })?;
    // A leading BOM is invisible in most editors but would otherwise land as
    // the file's first character, e.g. stopping a leading "#" from being
    // recognized as a heading.
    Ok(content
        .strip_prefix('\u{feff}')
        .map(str::to_string)
        .unwrap_or(content))
}

/// Writes `contents` to `path` by writing a temp file first and atomically
/// renaming it into place, so a crash mid-write can never leave `path`
/// truncated or corrupted — worst case the temp file is left behind and the
/// original content is untouched.
/// Distinguishes temp files from concurrent `write_file_atomic` calls in the
/// same process — two saves to the same path (e.g. an autosave firing while a
/// manual save is in flight) must not share a temp file, or one write can
/// clobber the other before either rename completes.
static TMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn write_file_atomic(path: &Path, contents: &str) -> AppResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Message(format!("\"{}\" has no parent folder", path.display())))?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AppError::Message(format!("\"{}\" has no file name", path.display())))?;

    let unique = TMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_path = parent.join(format!(
        ".{file_name}.foldown-tmp-{}-{unique}",
        std::process::id()
    ));
    fs::write(&tmp_path, contents)?;

    match fs::rename(&tmp_path, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            Err(AppError::from(e))
        }
    }
}

pub fn create_file(path: &Path) -> AppResult<()> {
    if path.exists() {
        return Err(AppError::Message(format!(
            "\"{}\" already exists",
            path.display()
        )));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, "")?;
    Ok(())
}

pub fn create_folder(path: &Path) -> AppResult<()> {
    if path.exists() {
        return Err(AppError::Message(format!(
            "\"{}\" already exists",
            path.display()
        )));
    }
    fs::create_dir_all(path)?;
    Ok(())
}

pub fn move_path(old: &Path, new: &Path) -> AppResult<()> {
    if !old.exists() {
        return Err(AppError::Message(format!(
            "\"{}\" does not exist",
            old.display()
        )));
    }
    if new.exists() {
        // On a case-insensitive filesystem (NTFS), renaming "readme.md" to
        // "README.md" makes `new.exists()` true even though nothing actually
        // collides — `new` and `old` canonicalize to the exact same file.
        // Only reject when `new` is a genuinely different, already-existing entry.
        let is_case_only_rename = old
            .canonicalize()
            .ok()
            .zip(new.canonicalize().ok())
            .is_some_and(|(o, n)| o == n);
        if !is_case_only_rename {
            return Err(AppError::Message(format!(
                "\"{}\" already exists",
                new.display()
            )));
        }
    }
    if old.is_dir() && new.starts_with(old) {
        return Err(AppError::Message(
            "Can't move a folder into itself or one of its own subfolders".to_string(),
        ));
    }
    if let Some(parent) = new.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(old, new)?;
    Ok(())
}

pub fn delete_path(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Err(AppError::Message(format!(
            "\"{}\" does not exist",
            path.display()
        )));
    }
    trash::delete(path).map_err(|e| AppError::Message(format!("Could not delete: {e}")))?;
    Ok(())
}

pub fn duplicate_path(path: &Path) -> AppResult<PathBuf> {
    if !path.exists() {
        return Err(AppError::Message(format!(
            "\"{}\" does not exist",
            path.display()
        )));
    }
    let target = unique_sibling_path(path);
    if path.is_dir() {
        copy_dir_recursive(path, &target)?;
    } else {
        fs::copy(path, &target)?;
    }
    Ok(target)
}

/// Copies an external file (e.g. dropped in from Explorer) into `dest_dir`,
/// renaming with a "copy"/"copy N" suffix on a name collision.
pub fn import_file(source: &Path, dest_dir: &Path) -> AppResult<PathBuf> {
    if !source.is_file() {
        return Err(AppError::Message(format!(
            "\"{}\" is not a file",
            source.display()
        )));
    }
    let name = source
        .file_name()
        .ok_or_else(|| AppError::Message("Source file has no name".to_string()))?;
    let mut candidate = dest_dir.join(name);
    if candidate.exists() {
        candidate = unique_sibling_path(&candidate);
    }
    fs::copy(source, &candidate)?;
    Ok(candidate)
}

fn unique_sibling_path(path: &Path) -> PathBuf {
    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned());

    let name_with_suffix = |suffix: String| match &ext {
        Some(ext) => format!("{stem} {suffix}.{ext}"),
        None => format!("{stem} {suffix}"),
    };

    let mut candidate = parent.join(name_with_suffix("copy".to_string()));
    let mut n = 2;
    while candidate.exists() {
        candidate = parent.join(name_with_suffix(format!("copy {n}")));
        n += 1;
    }
    candidate
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> AppResult<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dest_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("foldown-ops-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn read_file_returns_contents() {
        let dir = temp_dir();
        let path = dir.join("a.md");
        fs::write(&path, "hello").unwrap();
        assert_eq!(read_file(&path).unwrap(), "hello");
    }

    #[test]
    fn read_file_rejects_missing() {
        let dir = temp_dir();
        let path = dir.join("missing.md");
        assert!(read_file(&path).is_err());
    }

    #[test]
    fn write_file_atomic_creates_new_file() {
        let dir = temp_dir();
        let path = dir.join("notes.md");
        write_file_atomic(&path, "hello").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn write_file_atomic_overwrites_existing_and_leaves_no_temp_file() {
        let dir = temp_dir();
        let path = dir.join("notes.md");
        fs::write(&path, "old content").unwrap();

        write_file_atomic(&path, "new content").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new content");

        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("foldown-tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temp file should not remain after a successful save"
        );
    }

    #[test]
    fn create_file_rejects_existing() {
        let dir = temp_dir();
        let path = dir.join("notes.md");
        create_file(&path).unwrap();
        assert!(path.exists());
        assert!(create_file(&path).is_err());
    }

    #[test]
    fn create_folder_rejects_existing() {
        let dir = temp_dir();
        let path = dir.join("sub");
        create_folder(&path).unwrap();
        assert!(path.is_dir());
        assert!(create_folder(&path).is_err());
    }

    #[test]
    fn move_path_renames_and_rejects_collisions() {
        let dir = temp_dir();
        let a = dir.join("a.md");
        let b = dir.join("b.md");
        fs::write(&a, "hi").unwrap();
        move_path(&a, &b).unwrap();
        assert!(!a.exists());
        assert_eq!(fs::read_to_string(&b).unwrap(), "hi");

        fs::write(&a, "other").unwrap();
        assert!(move_path(&a, &b).is_err());
    }

    #[test]
    fn move_path_rejects_folder_into_itself() {
        let dir = temp_dir();
        let folder = dir.join("parent");
        fs::create_dir_all(&folder).unwrap();
        let nested_target = folder.join("child").join("parent");
        assert!(move_path(&folder, &nested_target).is_err());
    }

    #[test]
    fn move_path_allows_a_case_only_rename() {
        // Regression test: on NTFS (case-insensitive), `new.exists()` is true
        // for a pure case change even though nothing actually collides.
        let dir = temp_dir();
        let original = dir.join("readme.md");
        fs::write(&original, "hello").unwrap();
        let renamed = dir.join("README.md");

        move_path(&original, &renamed).unwrap();
        assert_eq!(fs::read_to_string(&renamed).unwrap(), "hello");
    }

    #[test]
    fn write_file_atomic_survives_concurrent_writes_to_the_same_path() {
        // Regression test: the temp filename used to be derived only from the
        // process id, so two concurrent saves to the same path shared one temp
        // file and could clobber each other before either rename completed.
        let dir = temp_dir();
        let path = dir.join("notes.md");
        fs::write(&path, "").unwrap();

        let content_a = "a".repeat(200_000);
        let content_b = "b".repeat(200_000);
        let (path_a, path_b) = (path.clone(), path.clone());
        let (data_a, data_b) = (content_a.clone(), content_b.clone());
        let a = std::thread::spawn(move || write_file_atomic(&path_a, &data_a));
        let b = std::thread::spawn(move || write_file_atomic(&path_b, &data_b));
        a.join().unwrap().unwrap();
        b.join().unwrap().unwrap();

        let result = fs::read_to_string(&path).unwrap();
        assert!(
            result == content_a || result == content_b,
            "expected one full write to win, got a result of length {}",
            result.len()
        );
    }

    #[test]
    fn duplicate_file_generates_unique_name() {
        let dir = temp_dir();
        let original = dir.join("notes.md");
        fs::write(&original, "content").unwrap();

        let first = duplicate_path(&original).unwrap();
        assert_eq!(
            first.file_name().unwrap().to_str().unwrap(),
            "notes copy.md"
        );

        let second = duplicate_path(&original).unwrap();
        assert_eq!(
            second.file_name().unwrap().to_str().unwrap(),
            "notes copy 2.md"
        );
    }

    #[test]
    fn duplicate_folder_copies_recursively() {
        let dir = temp_dir();
        let original = dir.join("folder");
        fs::create_dir_all(original.join("nested")).unwrap();
        fs::write(original.join("a.md"), "a").unwrap();
        fs::write(original.join("nested").join("b.md"), "b").unwrap();

        let copy = duplicate_path(&original).unwrap();
        assert_eq!(fs::read_to_string(copy.join("a.md")).unwrap(), "a");
        assert_eq!(
            fs::read_to_string(copy.join("nested").join("b.md")).unwrap(),
            "b"
        );
    }

    #[test]
    fn ensure_within_workspace_rejects_escape() {
        let root = temp_dir();
        let inside = root.join("notes.md");
        fs::write(&inside, "").unwrap();
        assert!(ensure_within_workspace(&inside, &root).is_ok());

        let outside_root = temp_dir();
        assert!(ensure_within_workspace(&outside_root, &root).is_err());
    }

    #[test]
    fn import_file_copies_and_avoids_collisions() {
        let source_dir = temp_dir();
        let dest_dir = temp_dir();
        let source = source_dir.join("notes.md");
        fs::write(&source, "imported content").unwrap();

        let first = import_file(&source, &dest_dir).unwrap();
        assert_eq!(first.file_name().unwrap().to_str().unwrap(), "notes.md");
        assert_eq!(fs::read_to_string(&first).unwrap(), "imported content");

        let second = import_file(&source, &dest_dir).unwrap();
        assert_eq!(
            second.file_name().unwrap().to_str().unwrap(),
            "notes copy.md"
        );
    }

    #[test]
    fn ensure_within_workspace_resolves_nonexistent_create_target() {
        let root = temp_dir();
        let target = root.join("new-note.md");
        let resolved = ensure_within_workspace(&target, &root).unwrap();
        assert_eq!(resolved.file_name().unwrap(), "new-note.md");
    }

    #[test]
    fn containment_rejects_sibling_prefix_and_parent_traversal() {
        let root = temp_dir();
        let sibling = root.with_file_name(format!(
            "{}-archive",
            root.file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir_all(&sibling).unwrap();
        assert!(ensure_within_workspace(&sibling.join("note.md"), &root).is_err());

        let outside = root.parent().unwrap().join("outside-note.md");
        assert!(ensure_within_workspace(&root.join("..").join("outside-note.md"), &root).is_err());
        assert_eq!(outside.file_name().unwrap(), "outside-note.md");
    }

    #[test]
    fn ensure_within_workspace_rejects_symlinks() {
        // Regression test: a symlink (dangling or not) inside the workspace
        // used to bypass containment — the "doesn't exist yet" branch only
        // canonicalized the parent and reused the link's own file name,
        // never resolving (or rejecting) the link itself.
        let root = temp_dir();
        let outside_target = temp_dir().join("secret.md");
        fs::write(&outside_target, "secret").unwrap();
        let link = root.join("link.md");

        #[cfg(unix)]
        let created = std::os::unix::fs::symlink(&outside_target, &link).is_ok();
        #[cfg(windows)]
        let created = std::os::windows::fs::symlink_file(&outside_target, &link).is_ok();

        if !created {
            // Creating symlinks on Windows needs a privilege (Developer Mode
            // or admin) that may not be available in this environment —
            // skip rather than report a spurious failure.
            return;
        }
        assert!(ensure_within_workspace(&link, &root).is_err());
    }

    #[test]
    fn read_file_strips_a_leading_byte_order_mark() {
        let dir = temp_dir();
        let path = dir.join("bom.md");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"# Heading");
        fs::write(&path, bytes).unwrap();

        assert_eq!(read_file(&path).unwrap(), "# Heading");
    }

    #[test]
    fn read_file_rejects_non_utf8_with_a_clear_message() {
        let dir = temp_dir();
        let path = dir.join("latin1.md");
        fs::write(&path, [0x48, 0x65, 0x6C, 0x6C, 0xF6]).unwrap();

        let err = read_file(&path).unwrap_err();
        assert!(
            format!("{err}").contains("not valid UTF-8"),
            "unexpected message: {err}"
        );
    }
}
