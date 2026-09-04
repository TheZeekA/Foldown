use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub workspace_root: String,
    pub relative_path: String,
    pub created_at: i64,
    pub byte_length: i64,
}

pub struct HistoryStore {
    conn: Mutex<Connection>,
}

fn workspace_key(path: &Path) -> AppResult<String> {
    Ok(path.canonicalize()?.to_string_lossy().to_lowercase())
}

fn relative_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

impl HistoryStore {
    pub fn open(db_path: PathBuf) -> AppResult<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace_root TEXT NOT NULL,
                relative_path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                content TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_history_file
                ON history_entries(workspace_root, relative_path, created_at DESC, id DESC);",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn record_snapshot(
        &self,
        workspace_root: &Path,
        relative_path: &Path,
        content: &str,
        retention: usize,
    ) -> AppResult<()> {
        let workspace = workspace_key(workspace_root)?;
        let relative = relative_key(relative_path);
        let retention = retention.max(1) as i64;
        let conn = self.conn.lock().unwrap();
        let latest: Option<String> = conn
            .query_row(
                "SELECT content FROM history_entries
                 WHERE workspace_root = ?1 AND relative_path = ?2
                 ORDER BY created_at DESC, id DESC LIMIT 1",
                params![workspace, relative],
                |row| row.get(0),
            )
            .optional()?;
        if latest.as_deref() == Some(content) {
            return Ok(());
        }
        conn.execute(
            "INSERT INTO history_entries(workspace_root, relative_path, created_at, content)
             VALUES(?1, ?2, ?3, ?4)",
            params![workspace, relative, now(), content],
        )?;
        conn.execute(
            "DELETE FROM history_entries WHERE workspace_root = ?1 AND relative_path = ?2
             AND id NOT IN (
                SELECT id FROM history_entries
                WHERE workspace_root = ?1 AND relative_path = ?2
                ORDER BY created_at DESC, id DESC LIMIT ?3
             )",
            params![workspace, relative, retention],
        )?;
        Ok(())
    }

    pub fn list_snapshots(&self, workspace_root: &Path, relative_path: &Path) -> AppResult<Vec<HistoryEntry>> {
        let workspace = workspace_key(workspace_root)?;
        let relative = relative_key(relative_path);
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, workspace_root, relative_path, created_at, length(content)
             FROM history_entries WHERE workspace_root = ?1 AND relative_path = ?2
             ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map(params![workspace, relative], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                workspace_root: row.get(1)?,
                relative_path: row.get(2)?,
                created_at: row.get(3)?,
                byte_length: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn snapshot(&self, id: i64) -> AppResult<Option<(String, String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT workspace_root, relative_path, content FROM history_entries WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?))),
            None => Ok(None),
        }
    }

    pub fn delete_snapshot(&self, id: i64) -> AppResult<()> {
        self.conn.lock().unwrap().execute("DELETE FROM history_entries WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_snapshots(&self, workspace_root: &Path, relative_path: Option<&Path>) -> AppResult<()> {
        let workspace = workspace_key(workspace_root)?;
        let conn = self.conn.lock().unwrap();
        match relative_path {
            Some(path) => {
                conn.execute(
                    "DELETE FROM history_entries WHERE workspace_root = ?1 AND relative_path = ?2",
                    params![workspace, relative_key(path)],
                )?;
            }
            None => {
                conn.execute("DELETE FROM history_entries WHERE workspace_root = ?1", params![workspace])?;
            }
        }
        Ok(())
    }
}

use rusqlite::OptionalExtension;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn store() -> (HistoryStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!("foldown-history-{}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&dir).unwrap();
        let root = dir.join("workspace");
        std::fs::create_dir(&root).unwrap();
        (HistoryStore::open(dir.join("history.db")).unwrap(), root)
    }

    #[test]
    fn records_deduplicates_and_prunes_snapshots() {
        let (store, root) = store();
        store.record_snapshot(&root, Path::new("note.md"), "one", 2).unwrap();
        store.record_snapshot(&root, Path::new("note.md"), "one", 2).unwrap();
        store.record_snapshot(&root, Path::new("note.md"), "two", 2).unwrap();
        store.record_snapshot(&root, Path::new("note.md"), "three", 2).unwrap();
        let entries = store.list_snapshots(&root, Path::new("note.md")).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(store.snapshot(entries[0].id).unwrap().unwrap().2, "three");
    }

    #[test]
    fn isolates_workspaces_and_paths() {
        let (store, root) = store();
        let other = root.parent().unwrap().join("other");
        std::fs::create_dir(&other).unwrap();
        store.record_snapshot(&root, Path::new("note.md"), "root", 50).unwrap();
        store.record_snapshot(&other, Path::new("note.md"), "other", 50).unwrap();
        store.record_snapshot(&root, Path::new("other.md"), "other path", 50).unwrap();
        assert_eq!(store.list_snapshots(&root, Path::new("note.md")).unwrap().len(), 1);
        assert_eq!(store.list_snapshots(&other, Path::new("note.md")).unwrap().len(), 1);
        assert_eq!(store.list_snapshots(&root, Path::new("other.md")).unwrap().len(), 1);
    }

    #[test]
    fn deletes_and_clears_snapshots() {
        let (store, root) = store();
        store.record_snapshot(&root, Path::new("note.md"), "one", 50).unwrap();
        store.record_snapshot(&root, Path::new("other.md"), "two", 50).unwrap();
        let id = store.list_snapshots(&root, Path::new("note.md")).unwrap()[0].id;
        store.delete_snapshot(id).unwrap();
        assert!(store.list_snapshots(&root, Path::new("note.md")).unwrap().is_empty());
        store.clear_snapshots(&root, None).unwrap();
        assert!(store.list_snapshots(&root, Path::new("other.md")).unwrap().is_empty());
    }
}
