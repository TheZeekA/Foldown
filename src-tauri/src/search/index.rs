use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SearchResult {
    pub path: String,
    pub name: String,
    pub snippet: String,
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .map(|e| e.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

fn is_reindexable_markdown(path: &Path, is_file: bool, is_symlink: bool) -> bool {
    is_file && !is_symlink && is_markdown(path)
}

pub fn create_index_table(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS docs USING fts5(path UNINDEXED, name, content);",
    )?;
    Ok(())
}

/// Full (re)build: clears the table and walks the workspace, indexing every
/// `.md` file. Dotfiles/dot-folders are always skipped, matching the sidebar tree.
pub fn index_workspace(conn: &Connection, root: &Path) -> AppResult<()> {
    conn.execute("DELETE FROM docs", [])?;
    walk_and_index(conn, root)
}

fn walk_and_index(conn: &Connection, dir: &Path) -> AppResult<()> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            walk_and_index(conn, &path)?;
        } else if file_type.is_file() && is_markdown(&path) {
            if let Ok(content) = fs::read_to_string(&path) {
                insert_doc(conn, &path, &name, &content)?;
            }
        }
    }
    Ok(())
}

fn insert_doc(conn: &Connection, path: &Path, name: &str, content: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO docs (path, name, content) VALUES (?1, ?2, ?3)",
        params![path.to_string_lossy().to_string(), name, content],
    )?;
    Ok(())
}

/// Re-indexes a single path after a filesystem-watcher event: drops any
/// existing row for it, then re-inserts if it's still a markdown file on
/// disk (a no-op re-insert for deletes, since the path no longer exists).
pub fn reindex_path(conn: &Connection, path: &Path) -> AppResult<()> {
    let path_str = path.to_string_lossy().to_string();
    conn.execute("DELETE FROM docs WHERE path = ?1", params![path_str])?;
    let file_type = fs::symlink_metadata(path)
        .ok()
        .map(|metadata| metadata.file_type());
    if file_type
        .as_ref()
        .is_some_and(|kind| is_reindexable_markdown(path, kind.is_file(), kind.is_symlink()))
    {
        if let Ok(content) = fs::read_to_string(path) {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            insert_doc(conn, path, &name, &content)?;
        }
    }
    Ok(())
}

fn build_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|word| format!("{}*", word.replace(['"', '*'], "")))
        .filter(|w| *w != "*")
        .collect::<Vec<_>>()
        .join(" ")
}

/// Marks match boundaries with control characters (not visible punctuation
/// like `[`/`]`) so the frontend can safely split and highlight them without
/// risking collision with characters that legitimately appear in note text.
const MARK_START: &str = "\u{1}";
const MARK_END: &str = "\u{2}";

pub fn query_index(conn: &Connection, query: &str) -> AppResult<Vec<SearchResult>> {
    let fts_query = build_fts_query(query);
    if fts_query.is_empty() {
        return Ok(vec![]);
    }
    let mut stmt = conn.prepare(
        "SELECT path, name, snippet(docs, 2, ?2, ?3, '…', 12)
         FROM docs WHERE docs MATCH ?1 ORDER BY rank LIMIT 50",
    )?;
    let rows = stmt.query_map(params![fts_query, MARK_START, MARK_END], |row| {
        Ok(SearchResult {
            path: row.get(0)?,
            name: row.get(1)?,
            snippet: row.get(2)?,
        })
    })?;
    let mut results = Vec::new();
    for r in rows {
        results.push(r?);
    }
    Ok(results)
}

struct IndexState {
    conn: Arc<Mutex<Connection>>,
    _watcher: RecommendedWatcher,
}

/// Holds the current workspace's in-memory FTS5 index. Rebuilt from scratch
/// whenever the workspace changes; kept current afterwards by a recursive
/// filesystem watcher. Nothing here ever touches the user's files — this is
/// purely a rebuildable cache.
pub struct SearchIndex {
    inner: Mutex<Option<IndexState>>,
}

impl SearchIndex {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    pub fn build(&self, root: PathBuf) -> AppResult<()> {
        let conn = Connection::open_in_memory()?;
        create_index_table(&conn)?;
        index_workspace(&conn, &root)?;
        let conn = Arc::new(Mutex::new(conn));

        let watcher_conn = Arc::clone(&conn);
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let Ok(event) = res else { return };
            if !matches!(
                event.kind,
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
            ) {
                return;
            }
            if let Ok(conn) = watcher_conn.lock() {
                for path in &event.paths {
                    let _ = reindex_path(&conn, path);
                }
            }
        })
        .map_err(|e| AppError::Message(format!("Could not watch workspace: {e}")))?;
        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| AppError::Message(format!("Could not watch workspace: {e}")))?;

        *self.inner.lock().unwrap() = Some(IndexState {
            conn,
            _watcher: watcher,
        });
        Ok(())
    }

    pub fn search(&self, query: &str) -> AppResult<Vec<SearchResult>> {
        let guard = self.inner.lock().unwrap();
        match guard.as_ref() {
            Some(state) => {
                let conn = state.conn.lock().unwrap();
                query_index(&conn, query)
            }
            None => Ok(vec![]),
        }
    }
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("foldown-search-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn temp_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_index_table(&conn).unwrap();
        conn
    }

    #[test]
    fn indexes_and_finds_matching_files() {
        let dir = temp_dir();
        fs::write(dir.join("apple.md"), "Notes about apples and orchards.").unwrap();
        fs::write(dir.join("banana.md"), "Notes about bananas.").unwrap();
        let conn = temp_conn();
        index_workspace(&conn, &dir).unwrap();

        let results = query_index(&conn, "apple").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "apple.md");
    }

    #[test]
    fn prefix_matches_partial_words() {
        let dir = temp_dir();
        fs::write(dir.join("notes.md"), "This mentions orchestration heavily.").unwrap();
        let conn = temp_conn();
        index_workspace(&conn, &dir).unwrap();

        let results = query_index(&conn, "orches").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn skips_non_markdown_and_dotfiles() {
        let dir = temp_dir();
        fs::write(dir.join("notes.md"), "findme content").unwrap();
        fs::write(dir.join("image.png"), "findme content").unwrap();
        fs::create_dir_all(dir.join(".obsidian")).unwrap();
        fs::write(dir.join(".obsidian").join("hidden.md"), "findme content").unwrap();

        let conn = temp_conn();
        index_workspace(&conn, &dir).unwrap();

        let results = query_index(&conn, "findme").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "notes.md");
    }

    #[test]
    fn reindex_path_updates_and_removes_entries() {
        let dir = temp_dir();
        let path = dir.join("notes.md");
        fs::write(&path, "original content").unwrap();
        let conn = temp_conn();
        index_workspace(&conn, &dir).unwrap();
        assert_eq!(query_index(&conn, "original").unwrap().len(), 1);

        fs::write(&path, "updated content").unwrap();
        reindex_path(&conn, &path).unwrap();
        assert_eq!(query_index(&conn, "original").unwrap().len(), 0);
        assert_eq!(query_index(&conn, "updated").unwrap().len(), 1);

        fs::remove_file(&path).unwrap();
        reindex_path(&conn, &path).unwrap();
        assert_eq!(query_index(&conn, "updated").unwrap().len(), 0);
    }

    #[test]
    fn snippet_marks_match_boundaries() {
        let dir = temp_dir();
        fs::write(dir.join("notes.md"), "The quick brown fox jumps.").unwrap();
        let conn = temp_conn();
        index_workspace(&conn, &dir).unwrap();

        let results = query_index(&conn, "quick").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].snippet.contains(MARK_START));
        assert!(results[0].snippet.contains(MARK_END));
    }

    #[test]
    fn empty_query_returns_no_results() {
        let conn = temp_conn();
        assert_eq!(query_index(&conn, "   ").unwrap().len(), 0);
    }

    #[test]
    fn incremental_index_rejects_markdown_symlinks() {
        assert!(!is_reindexable_markdown(Path::new("linked.md"), true, true,));
        assert!(is_reindexable_markdown(
            Path::new("ordinary.md"),
            true,
            false,
        ));
    }
}
