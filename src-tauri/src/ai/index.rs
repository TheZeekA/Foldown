use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::ai::context::{chunk_markdown, content_hash};
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextChunk {
    pub path: String,
    pub heading: String,
    pub text: String,
    pub score: f64,
    pub ordinal: usize,
}

pub struct KnowledgeIndex {
    conn: Mutex<Connection>,
}

impl KnowledgeIndex {
    pub fn open(path: PathBuf) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        migrate_schema(&conn)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ai_documents (
                workspace TEXT NOT NULL, path TEXT NOT NULL, hash TEXT NOT NULL,
                PRIMARY KEY(workspace, path)
             );
             CREATE VIRTUAL TABLE IF NOT EXISTS ai_chunks USING fts5(
                workspace UNINDEXED, path UNINDEXED, heading, content, ordinal UNINDEXED
             );
             CREATE TABLE IF NOT EXISTS ai_embedding_cache (
                hash TEXT NOT NULL, model TEXT NOT NULL, vector TEXT NOT NULL,
                PRIMARY KEY(hash, model)
             );
             CREATE TABLE IF NOT EXISTS ai_embedding_meta (
                model TEXT NOT NULL PRIMARY KEY, dimension INTEGER NOT NULL
             );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn sync_workspace(&self, root: &Path) -> AppResult<()> {
        let root = root.canonicalize()?;
        let workspace = root.to_string_lossy().into_owned();
        let mut docs = Vec::new();
        walk_markdown(&root, &root, &mut docs)?;
        let mut conn = self.conn.lock().unwrap();
        let existing = {
            let mut stmt =
                conn.prepare("SELECT path, hash FROM ai_documents WHERE workspace = ?1")?;
            let rows = stmt.query_map(params![workspace], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.filter_map(Result::ok).collect::<HashMap<_, _>>()
        };
        let tx = conn.transaction()?;
        let discovered = docs
            .iter()
            .map(|(path, _)| path.as_str())
            .collect::<HashSet<_>>();
        for stale in existing
            .keys()
            .filter(|path| !discovered.contains(path.as_str()))
        {
            tx.execute(
                "DELETE FROM ai_chunks WHERE workspace=?1 AND path=?2",
                params![workspace, stale],
            )?;
            tx.execute(
                "DELETE FROM ai_documents WHERE workspace=?1 AND path=?2",
                params![workspace, stale],
            )?;
        }
        for (relative, content) in docs {
            let hash = content_hash(&content);
            if existing.get(&relative) == Some(&hash) {
                continue;
            }
            tx.execute(
                "DELETE FROM ai_chunks WHERE workspace=?1 AND path=?2",
                params![workspace, relative],
            )?;
            tx.execute(
                "INSERT INTO ai_documents(workspace,path,hash) VALUES(?1,?2,?3)
                        ON CONFLICT(workspace,path) DO UPDATE SET hash=excluded.hash",
                params![workspace, relative, hash],
            )?;
            for chunk in chunk_markdown(&relative, &content) {
                tx.execute("INSERT INTO ai_chunks(workspace,path,heading,content,ordinal) VALUES(?1,?2,?3,?4,?5)",
                    params![workspace, chunk.path, chunk.heading, chunk.text, chunk.ordinal as i64])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn rebuild_workspace(&self, root: &Path) -> AppResult<()> {
        let workspace = root.canonicalize()?.to_string_lossy().into_owned();
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM ai_documents WHERE workspace=?1",
            params![workspace],
        )?;
        tx.execute(
            "DELETE FROM ai_chunks WHERE workspace=?1",
            params![workspace],
        )?;
        tx.commit()?;
        drop(conn);
        self.sync_workspace(root)
    }

    pub fn refresh_path(&self, root: &Path, path: &Path) -> AppResult<()> {
        let root = root.canonicalize()?;
        let requested = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        };
        let target = if requested.exists() {
            requested.canonicalize()?
        } else {
            let parent = requested.parent().ok_or_else(|| {
                crate::error::AppError::Message("Index path has no parent".to_string())
            })?;
            parent
                .canonicalize()?
                .join(requested.file_name().ok_or_else(|| {
                    crate::error::AppError::Message("Index path has no file name".to_string())
                })?)
        };
        if !target.starts_with(&root) {
            return Err(crate::error::AppError::Message(
                "Index path must remain inside the workspace".to_string(),
            ));
        }
        let relative = target
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if relative.split('/').any(|part| part.starts_with('.'))
            || !target
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            return Ok(());
        }
        let workspace = root.to_string_lossy().into_owned();
        let content = if target.is_file() {
            fs::read_to_string(&target).ok()
        } else {
            None
        };
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM ai_chunks WHERE workspace=?1 AND path=?2",
            params![workspace, relative],
        )?;
        tx.execute(
            "DELETE FROM ai_documents WHERE workspace=?1 AND path=?2",
            params![workspace, relative],
        )?;
        if let Some(content) = content {
            tx.execute(
                "INSERT INTO ai_documents(workspace,path,hash) VALUES(?1,?2,?3)",
                params![workspace, relative, content_hash(&content)],
            )?;
            for chunk in chunk_markdown(&relative, &content) {
                tx.execute("INSERT INTO ai_chunks(workspace,path,heading,content,ordinal) VALUES(?1,?2,?3,?4,?5)",
                    params![workspace, chunk.path, chunk.heading, chunk.text, chunk.ordinal as i64])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn search_candidates(
        &self,
        root: &Path,
        query: &str,
        limit: usize,
    ) -> AppResult<Vec<ContextChunk>> {
        let workspace = root.canonicalize()?.to_string_lossy().into_owned();
        let terms = query
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| !w.is_empty())
            .map(|w| format!("\"{}\"*", w.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" OR ");
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT path, heading, content, bm25(ai_chunks), ordinal FROM ai_chunks
             WHERE ai_chunks MATCH ?1 AND workspace = ?2 ORDER BY bm25(ai_chunks) LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![terms, workspace, limit as i64], |row| {
            Ok(ContextChunk {
                path: row.get(0)?,
                heading: row.get(1)?,
                text: row.get(2)?,
                score: -row.get::<_, f64>(3)?,
                ordinal: row.get::<_, i64>(4)? as usize,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    // Kept as a thin wrapper over `search_candidates` so every pre-existing
    // test of `search` keeps working unchanged; `ai::commands::retrieve_context`
    // now calls `search_candidates` + `truncate_to_char_budget` directly, so
    // this is exercised only from tests, which is why it's gated to test builds.
    #[cfg(test)]
    pub fn search(
        &self,
        root: &Path,
        query: &str,
        limit: usize,
        max_chars: usize,
    ) -> AppResult<Vec<ContextChunk>> {
        Ok(truncate_to_char_budget(
            self.search_candidates(root, query, limit)?,
            max_chars,
        ))
    }

    /// Every indexed Markdown file's relative path, sorted. Unlike
    /// `search_candidates` (which only surfaces content *relevant to a
    /// query*) or `all_chunks` (which returns every chunk's full text), this
    /// is a lightweight listing
    /// so the model can be told what exists across the whole workspace —
    /// including files and folders that never matched any retrieval query —
    /// without spending its context budget on their content.
    pub fn all_document_paths(&self, root: &Path) -> AppResult<Vec<String>> {
        let workspace = root.canonicalize()?.to_string_lossy().into_owned();
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT path FROM ai_documents WHERE workspace=?1 ORDER BY path")?;
        let rows = stmt.query_map(params![workspace], |row| row.get::<_, String>(0))?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn all_chunks(&self, root: &Path) -> AppResult<Vec<ContextChunk>> {
        let workspace = root.canonicalize()?.to_string_lossy().into_owned();
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT path,heading,content,ordinal FROM ai_chunks WHERE workspace=?1")?;
        let rows = stmt.query_map(params![workspace], |row| {
            Ok(ContextChunk {
                path: row.get(0)?,
                heading: row.get(1)?,
                text: row.get(2)?,
                score: 0.0,
                ordinal: row.get::<_, i64>(3)? as usize,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn cached_embedding(&self, model: &str, text: &str) -> AppResult<Option<Vec<f32>>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT vector FROM ai_embedding_cache WHERE hash=?1 AND model=?2")?;
        let mut rows = stmt.query(params![content_hash(text), model])?;
        match rows.next()? {
            Some(row) => {
                let json: String = row.get(0)?;
                Ok(serde_json::from_str(&json).ok())
            }
            None => Ok(None),
        }
    }

    pub fn store_embedding(&self, model: &str, text: &str, vector: &[f32]) -> AppResult<()> {
        let conn = self.conn.lock().unwrap();
        let known_dimension: Option<i64> = conn
            .query_row(
                "SELECT dimension FROM ai_embedding_meta WHERE model=?1",
                params![model],
                |row| row.get(0),
            )
            .ok();
        if known_dimension.is_some_and(|d| d != vector.len() as i64) {
            conn.execute(
                "DELETE FROM ai_embedding_cache WHERE model=?1",
                params![model],
            )?;
        }
        conn.execute(
            "INSERT OR REPLACE INTO ai_embedding_meta(model,dimension) VALUES(?1,?2)",
            params![model, vector.len() as i64],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO ai_embedding_cache(hash,model,vector) VALUES(?1,?2,?3)",
            params![
                content_hash(text),
                model,
                serde_json::to_string(vector).unwrap_or_default()
            ],
        )?;
        Ok(())
    }

    /// The vector dimension this model was last seen producing, if any chunk
    /// has ever been embedded with it. `None` means this model has never been
    /// used before (the legitimate first-time-embedding case) — callers
    /// should proceed normally rather than treat that as a mismatch.
    pub fn embedding_dimension(&self, model: &str) -> AppResult<Option<usize>> {
        let conn = self.conn.lock().unwrap();
        let dimension: Option<i64> = conn
            .query_row(
                "SELECT dimension FROM ai_embedding_meta WHERE model=?1",
                params![model],
                |row| row.get(0),
            )
            .ok();
        Ok(dimension.map(|d| d as usize))
    }
}

fn is_indexable_markdown_entry(path: &Path, is_file: bool, is_symlink: bool) -> bool {
    is_file
        && !is_symlink
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

fn walk_markdown(root: &Path, dir: &Path, output: &mut Vec<(String, String)>) -> AppResult<()> {
    for entry in fs::read_dir(dir)?.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let Ok(resolved) = path.canonicalize() else {
            continue;
        };
        if resolved.strip_prefix(root).is_err() {
            continue;
        }
        if file_type.is_dir() {
            walk_markdown(root, &resolved, output)?;
        } else if is_indexable_markdown_entry(
            &resolved,
            file_type.is_file(),
            file_type.is_symlink(),
        ) {
            if let Ok(content) = fs::read_to_string(&resolved) {
                let relative = resolved
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                output.push((relative, content));
            }
        }
    }
    Ok(())
}

/// `ai_chunks` gained an `ordinal` column after initial release. An existing
/// database predates it — rather than a numbered-migration framework, follow
/// this file's existing convention (see settings::store's normalize-on-open)
/// of an imperative fixup run unconditionally on every open. `ai_chunks` is a
/// fully regenerable cache, so the fixup is just: drop the stale table and
/// forget every document's stored hash, so the next sync_workspace call sees
/// every file as "new" and re-chunks it under the current schema.
fn migrate_schema(conn: &Connection) -> AppResult<()> {
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='ai_chunks'",
        [],
        |row| row.get::<_, i64>(0),
    )? > 0;
    if !table_exists {
        return Ok(());
    }
    let has_ordinal = {
        let mut stmt = conn.prepare("SELECT name FROM pragma_table_info('ai_chunks')")?;
        let mut rows = stmt.query([])?;
        let mut found = false;
        while let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            if name == "ordinal" {
                found = true;
                break;
            }
        }
        found
    };
    if !has_ordinal {
        conn.execute_batch("DROP TABLE ai_chunks; DELETE FROM ai_documents;")?;
    }
    Ok(())
}

/// Ranked chunks arrive largest-first-relevance; walk them in order, keeping
/// whole chunks until the budget would be exceeded, then truncate only the
/// one chunk that crosses the line. Shared by `KnowledgeIndex::search` (now
/// `#[cfg(test)]`-only — kept as a thin wrapper so existing tests keep
/// working) and `ai::commands::retrieve_context` (the real no-embedding-model
/// default path today is `KnowledgeIndex::search_candidates`, called via
/// `retrieve_context`'s final selection step, after any reranking).
pub fn truncate_to_char_budget(chunks: Vec<ContextChunk>, max_chars: usize) -> Vec<ContextChunk> {
    let mut result = Vec::new();
    let mut used = 0usize;
    for mut chunk in chunks {
        let remaining = max_chars.saturating_sub(used);
        if remaining == 0 {
            break;
        }
        if chunk.text.chars().count() > remaining {
            chunk.text = chunk.text.chars().take(remaining).collect();
        }
        used += chunk.text.chars().count();
        result.push(chunk);
    }
    result
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    let dot: f32 = left.iter().zip(right).map(|(a, b)| a * b).sum();
    let l = left.iter().map(|v| v * v).sum::<f32>().sqrt();
    let r = right.iter().map(|v| v * v).sum::<f32>().sqrt();
    if l == 0.0 || r == 0.0 {
        0.0
    } else {
        dot / (l * r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn fixture() -> (std::path::PathBuf, KnowledgeIndex) {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root =
            std::env::temp_dir().join(format!("foldown-ai-index-{}-{n}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let index = KnowledgeIndex::open(root.join("index.db")).unwrap();
        (root, index)
    }

    #[test]
    fn indexes_markdown_and_returns_bounded_context() {
        let (root, index) = fixture();
        fs::write(
            root.join("orchard.md"),
            "# Apples\nOrchards contain apple trees.",
        )
        .unwrap();
        fs::write(root.join("ignore.txt"), "apple").unwrap();
        index.sync_workspace(&root).unwrap();
        let results = index.search(&root, "apple", 8, 12_000).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "orchard.md");
        assert!(results[0].text.contains("apple trees"));
    }

    #[test]
    fn all_document_paths_lists_every_indexed_file_including_nested_and_unmatched_ones() {
        // Regression test: retrieval only ever surfaces content relevant to a
        // query, so the model previously had no way to know a file existed at
        // all unless it happened to match. This lightweight listing covers the
        // whole workspace, including subfolders and files no query would match.
        let (root, index) = fixture();
        fs::write(root.join("Root.md"), "# Root").unwrap();
        fs::create_dir_all(root.join("Journal")).unwrap();
        fs::write(
            root.join("Journal").join("2024-01-01.md"),
            "Nothing relevant here.",
        )
        .unwrap();
        index.sync_workspace(&root).unwrap();

        let paths = index.all_document_paths(&root).unwrap();
        assert_eq!(paths, vec!["Journal/2024-01-01.md", "Root.md"]);
    }

    #[test]
    fn skips_dotfolders_and_removes_stale_files() {
        let (root, index) = fixture();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::write(root.join(".hidden").join("secret.md"), "needle").unwrap();
        let visible = root.join("visible.md");
        fs::write(&visible, "needle").unwrap();
        index.sync_workspace(&root).unwrap();
        assert_eq!(index.search(&root, "needle", 8, 12_000).unwrap().len(), 1);
        fs::remove_file(visible).unwrap();
        index.sync_workspace(&root).unwrap();
        assert!(index.search(&root, "needle", 8, 12_000).unwrap().is_empty());
    }

    #[test]
    fn cosine_similarity_ranks_related_vectors() {
        assert!(
            cosine_similarity(&[1.0, 0.0], &[0.9, 0.1])
                > cosine_similarity(&[1.0, 0.0], &[0.0, 1.0])
        );
    }

    #[test]
    fn embedding_cache_keys_on_the_exact_string_that_was_embedded() {
        let (_, index) = fixture();
        index
            .store_embedding("m", "search_document: Hello", &[1.0, 0.0])
            .unwrap();
        assert_eq!(
            index
                .cached_embedding("m", "search_document: Hello")
                .unwrap(),
            Some(vec![1.0, 0.0])
        );
        // The same raw text without the prefix is a different cache entry —
        // proves the cache is keyed on the literal (possibly prefixed) string
        // handed to it, not on some prefix-stripped canonical form.
        assert_eq!(index.cached_embedding("m", "Hello").unwrap(), None);
    }

    #[test]
    fn sync_preserves_unchanged_chunks_and_replaces_only_changed_files() {
        let (root, index) = fixture();
        fs::write(root.join("stable.md"), "stable needle").unwrap();
        fs::write(root.join("changed.md"), "old phrase").unwrap();
        index.sync_workspace(&root).unwrap();
        let stable_rowid: i64 = index
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT rowid FROM ai_chunks WHERE path='stable.md'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        fs::write(root.join("changed.md"), "new phrase").unwrap();
        index.sync_workspace(&root).unwrap();

        let stable_after: i64 = index
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT rowid FROM ai_chunks WHERE path='stable.md'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stable_rowid, stable_after);
        assert!(index.search(&root, "old", 8, 12_000).unwrap().is_empty());
        assert_eq!(index.search(&root, "new", 8, 12_000).unwrap().len(), 1);
    }

    #[test]
    fn refresh_path_updates_and_removes_one_document() {
        let (root, index) = fixture();
        let note = root.join("note.md");
        fs::write(&note, "first marker").unwrap();
        index.refresh_path(&root, &note).unwrap();
        assert_eq!(index.search(&root, "first", 8, 12_000).unwrap().len(), 1);

        fs::write(&note, "second marker").unwrap();
        index.refresh_path(&root, &note).unwrap();
        assert!(index.search(&root, "first", 8, 12_000).unwrap().is_empty());
        assert_eq!(index.search(&root, "second", 8, 12_000).unwrap().len(), 1);

        fs::remove_file(&note).unwrap();
        index.refresh_path(&root, &note).unwrap();
        assert!(index.search(&root, "second", 8, 12_000).unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn sync_skips_markdown_symlinks_that_point_outside_workspace() {
        let (root, index) = fixture();
        let outside = root.parent().unwrap().join(format!(
            "foldown-outside-{}-{}.md",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        fs::write(&outside, "external-secret-marker").unwrap();
        let link = root.join("linked-secret.md");

        std::os::unix::fs::symlink(&outside, &link).unwrap();

        index.sync_workspace(&root).unwrap();

        assert!(index
            .search(&root, "external-secret-marker", 8, 12_000)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn markdown_symlinks_are_never_index_candidates() {
        assert!(!is_indexable_markdown_entry(
            Path::new("linked-secret.md"),
            true,
            true,
        ));
        assert!(is_indexable_markdown_entry(
            Path::new("ordinary-note.md"),
            true,
            false,
        ));
    }

    #[test]
    fn chunks_carry_their_ordinal_within_the_document() {
        let (root, index) = fixture();
        std::fs::write(
            root.join("multi.md"),
            "# One\nfirst\n\n# Two\nsecond\n\n# Three\nthird",
        )
        .unwrap();
        index.sync_workspace(&root).unwrap();
        let mut chunks = index.all_chunks(&root).unwrap();
        chunks.sort_by_key(|c| c.ordinal);
        assert_eq!(
            chunks.iter().map(|c| c.ordinal).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(chunks[0].heading, "One");
        assert_eq!(chunks[2].heading, "Three");
    }

    #[test]
    fn opening_an_index_with_a_pre_ordinal_schema_forces_one_clean_reindex() {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "foldown-ai-index-migrate-{}-{n}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("note.md"), "# Note\nbody").unwrap();
        let db_path = root.join("index.db");

        // Build the *old* schema by hand (no ordinal column), matching what
        // every installed copy of Foldown before this change actually wrote,
        // then index one document into it the old way.
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE ai_documents (workspace TEXT NOT NULL, path TEXT NOT NULL, hash TEXT NOT NULL, PRIMARY KEY(workspace, path));
                 CREATE VIRTUAL TABLE ai_chunks USING fts5(workspace UNINDEXED, path UNINDEXED, heading, content);
                 CREATE TABLE ai_embedding_cache (hash TEXT NOT NULL, model TEXT NOT NULL, vector TEXT NOT NULL, PRIMARY KEY(hash, model));"
            ).unwrap();
            let workspace = root.canonicalize().unwrap().to_string_lossy().into_owned();
            conn.execute(
                "INSERT INTO ai_documents(workspace,path,hash) VALUES(?1,?2,?3)",
                params![workspace, "note.md", "stale-hash"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO ai_chunks(workspace,path,heading,content) VALUES(?1,?2,?3,?4)",
                params![workspace, "note.md", "Note", "old body"],
            )
            .unwrap();
        }

        // Opening through the real API must detect the missing `ordinal`
        // column, drop the stale table, and clear ai_documents so the next
        // sync treats every file as new rather than crashing on the old shape.
        let index = KnowledgeIndex::open(db_path).unwrap();
        index.sync_workspace(&root).unwrap();
        let chunks = index.all_chunks(&root).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].ordinal, 0);
        assert!(chunks[0].text.contains("body"));
    }

    #[test]
    fn truncate_to_char_budget_stops_partway_through_a_chunk_that_would_overflow() {
        let make = |text: &str| ContextChunk {
            path: "a.md".into(),
            heading: "H".into(),
            text: text.into(),
            score: 0.0,
            ordinal: 0,
        };
        let chunks = vec![make("12345"), make("67890")];
        let result = truncate_to_char_budget(chunks, 7);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "12345");
        assert_eq!(result[1].text, "67"); // only 2 of the 5 remaining chars fit the 7-char budget
    }

    #[test]
    fn search_candidates_returns_untruncated_text_for_reranking() {
        // NOTE: adapted from the task brief's literal version, which wrote one
        // document and asserted a single candidate's text exceeds 12,000 chars
        // with limit=5. That's unsatisfiable here: chunk_markdown (context.rs)
        // caps every chunk at MAX_CHUNK_CHARS (2,000), so 5 candidates can sum
        // to at most 10,000 chars regardless of how the fixture document is
        // written. This version proves the same property — search_candidates
        // does not apply a running/global char budget across what it returns,
        // unlike `search` — by summing several full-size chunks past 12,000.
        let (root, index) = fixture();
        for i in 0..7 {
            let body = "word ".repeat(400); // trims to 1999 chars: one whole chunk, just under MAX_CHUNK_CHARS
            fs::write(root.join(format!("long-{i}.md")), format!("# Long\n{body}")).unwrap();
        }
        index.sync_workspace(&root).unwrap();
        let candidates = index.search_candidates(&root, "word", 10).unwrap();
        assert_eq!(candidates.len(), 7);
        let total_chars: usize = candidates.iter().map(|c| c.text.chars().count()).sum();
        assert!(
            total_chars > 12_000,
            "search_candidates must not apply the final char budget"
        );
    }

    #[test]
    fn storing_an_embedding_at_a_new_dimension_for_the_same_model_clears_the_old_cache() {
        let (_, index) = fixture();
        index.store_embedding("m", "a", &[1.0, 0.0]).unwrap();
        index.store_embedding("m", "b", &[0.0, 1.0]).unwrap();
        assert!(index.cached_embedding("m", "a").unwrap().is_some());

        // Same model string, different vector length — e.g. the user pointed
        // "m" at a different server that happens to answer to the same name.
        index.store_embedding("m", "c", &[1.0, 0.0, 0.0]).unwrap();

        // The old 2-dimensional entries must be gone; a mismatched dimension
        // would otherwise silently score as unrelated via cosine_similarity's
        // length-mismatch guard rather than surfacing the real problem.
        assert!(index.cached_embedding("m", "a").unwrap().is_none());
        assert!(index.cached_embedding("m", "b").unwrap().is_none());
        assert_eq!(
            index.cached_embedding("m", "c").unwrap(),
            Some(vec![1.0, 0.0, 0.0])
        );
    }

    #[test]
    fn embedding_dimension_reports_none_for_an_unseen_model_and_the_recorded_size_after() {
        let (_, index) = fixture();
        assert_eq!(index.embedding_dimension("never-used").unwrap(), None);
        index.store_embedding("m", "a", &[1.0, 0.0, 0.0]).unwrap();
        assert_eq!(index.embedding_dimension("m").unwrap(), Some(3));
    }
}
