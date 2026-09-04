use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

fn user_facing_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = value.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        value.into_owned()
    }
}

fn normalize_recent_workspace_paths(conn: &mut Connection) -> AppResult<()> {
    let entries = {
        let mut stmt = conn.prepare("SELECT path, last_opened FROM recent_workspaces")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        rows.filter_map(Result::ok).collect::<Vec<_>>()
    };
    let tx = conn.transaction()?;
    for (stored, last_opened) in entries {
        let normalized = user_facing_path(Path::new(&stored));
        if normalized == stored {
            continue;
        }
        tx.execute(
            "INSERT INTO recent_workspaces(path, last_opened) VALUES(?1, ?2)
             ON CONFLICT(path) DO UPDATE SET last_opened = MAX(last_opened, excluded.last_opened)",
            params![normalized, last_opened],
        )?;
        tx.execute(
            "DELETE FROM recent_workspaces WHERE path = ?1",
            params![stored],
        )?;
    }
    tx.commit()?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

fn default_embedding_document_prefix() -> String {
    "search_document: ".to_string()
}
fn default_embedding_query_prefix() -> String {
    "search_query: ".to_string()
}
fn default_retrieval_candidate_count() -> u32 {
    20
}
fn default_retrieval_final_count() -> u32 {
    8
}
fn default_retrieval_max_chars() -> u32 {
    12_000
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorFont {
    pub family: String,
    pub size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AiProvider {
    Local,
    Openai,
    Anthropic,
    Gemini,
}

impl Default for AiProvider {
    fn default() -> Self {
        AiProvider::Local
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub base_url: String,
    pub chat_model: String,
    pub api_key: Option<String>,
}

fn default_local_config() -> ProviderConfig {
    ProviderConfig {
        base_url: "http://localhost:11434/v1".to_string(),
        chat_model: String::new(),
        api_key: None,
    }
}
fn default_openai_config() -> ProviderConfig {
    ProviderConfig {
        base_url: "https://api.openai.com/v1".to_string(),
        chat_model: String::new(),
        api_key: None,
    }
}
fn default_anthropic_config() -> ProviderConfig {
    ProviderConfig {
        base_url: "https://api.anthropic.com/v1".to_string(),
        chat_model: String::new(),
        api_key: None,
    }
}
fn default_gemini_config() -> ProviderConfig {
    ProviderConfig {
        base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        chat_model: String::new(),
        api_key: None,
    }
}

/// `base_url` is editable and meaningful for `local`. For the three cloud
/// providers it exists only for struct symmetry — each cloud provider module
/// uses its own hardcoded endpoint constant unconditionally and never reads
/// this field when actually sending a request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    #[serde(default)]
    pub provider: AiProvider,
    #[serde(default = "default_local_config")]
    pub local: ProviderConfig,
    #[serde(default = "default_openai_config")]
    pub openai: ProviderConfig,
    #[serde(default = "default_anthropic_config")]
    pub anthropic: ProviderConfig,
    #[serde(default = "default_gemini_config")]
    pub gemini: ProviderConfig,

    // Unchanged from the RAG overhaul — not touched by this plan. RAG always
    // runs locally regardless of which chat provider is selected.
    pub embedding_model: Option<String>,
    #[serde(default)]
    pub embedding_base_url: Option<String>,
    #[serde(default = "default_embedding_document_prefix")]
    pub embedding_document_prefix: String,
    #[serde(default = "default_embedding_query_prefix")]
    pub embedding_query_prefix: String,
    #[serde(default = "default_retrieval_candidate_count")]
    pub retrieval_candidate_count: u32,
    #[serde(default = "default_retrieval_final_count")]
    pub retrieval_final_count: u32,
    #[serde(default = "default_retrieval_max_chars")]
    pub retrieval_max_chars: u32,
    #[serde(default)]
    pub reranker_enabled: bool,
    #[serde(default)]
    pub reranker_base_url: Option<String>,
    #[serde(default)]
    pub reranker_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecentWorkspace {
    pub path: String,
    pub name: String,
    pub last_opened: i64,
    pub available: bool,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: AiProvider::default(),
            local: default_local_config(),
            openai: default_openai_config(),
            anthropic: default_anthropic_config(),
            gemini: default_gemini_config(),
            embedding_model: None,
            embedding_base_url: None,
            embedding_document_prefix: default_embedding_document_prefix(),
            embedding_query_prefix: default_embedding_query_prefix(),
            retrieval_candidate_count: default_retrieval_candidate_count(),
            retrieval_final_count: default_retrieval_final_count(),
            retrieval_max_chars: default_retrieval_max_chars(),
            reranker_enabled: false,
            reranker_base_url: None,
            reranker_model: None,
        }
    }
}

/// Rewrites a pre-multi-provider `AiSettings` JSON blob (flat `baseUrl`/
/// `chatModel`/`apiKey` at the top level) into the new nested shape — those
/// three values moved under `"local"` — before `serde_json::from_str` ever
/// sees it. A blob that already has a `"local"` key (already-migrated, or a
/// brand new user) is returned unchanged. Pure — never touches Credential
/// Manager; the caller (`get_ai_settings`) is responsible for migrating a
/// folded-in `apiKey` into Credential Manager afterward.
fn migrate_flat_ai_settings_json(raw: &str) -> AppResult<String> {
    let mut value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| AppError::Message(format!("Corrupt AI settings: {e}")))?;
    let Some(object) = value.as_object_mut() else {
        return Ok(raw.to_string());
    };
    if object.contains_key("local") || !object.contains_key("baseUrl") {
        return Ok(raw.to_string());
    }
    let base_url = object.remove("baseUrl").unwrap_or(serde_json::Value::Null);
    let chat_model = object
        .remove("chatModel")
        .unwrap_or(serde_json::Value::Null);
    let api_key = object.remove("apiKey").unwrap_or(serde_json::Value::Null);
    let mut local = serde_json::Map::new();
    local.insert("baseUrl".to_string(), base_url);
    local.insert("chatModel".to_string(), chat_model);
    local.insert("apiKey".to_string(), api_key);
    object.insert("local".to_string(), serde_json::Value::Object(local));
    object.insert(
        "provider".to_string(),
        serde_json::Value::String("local".to_string()),
    );
    serde_json::to_string(&value)
        .map_err(|e| AppError::Message(format!("Could not migrate AI settings: {e}")))
}

/// SQLite-backed store for app settings (recent/pinned files, window state,
/// search index — added in later steps). Never holds the user's note content;
/// `.md` files on disk remain the source of truth.
pub struct SettingsStore {
    conn: Mutex<Connection>,
    /// `None` in production (real Credential Manager target names). Set to
    /// `Some(unique-per-test-run string)` only by this module's own tests, so
    /// `cargo test` never reads, writes, or deletes the developer's real
    /// stored API keys under `Foldown/ai-key/{local,openai,anthropic,gemini}`.
    credential_namespace: Option<String>,
}

impl SettingsStore {
    pub fn open(db_path: PathBuf) -> AppResult<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kv_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS window_state (
                id INTEGER PRIMARY KEY CHECK (id = 0),
                x INTEGER NOT NULL,
                y INTEGER NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                maximized INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS recent_workspaces (
                path TEXT PRIMARY KEY,
                last_opened INTEGER NOT NULL
            );",
        )?;
        normalize_recent_workspace_paths(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            credential_namespace: None,
        })
    }

    #[cfg(test)]
    fn with_credential_namespace(mut self, namespace: String) -> Self {
        self.credential_namespace = Some(namespace);
        self
    }

    fn credential_provider_name(&self, provider: &str) -> String {
        match &self.credential_namespace {
            Some(namespace) => format!("{namespace}-{provider}"),
            None => provider.to_string(),
        }
    }

    const AI_PROVIDER_NAMES: [&'static str; 4] = ["local", "openai", "anthropic", "gemini"];

    fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM kv_settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO kv_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn recent_workspaces(&self, limit: usize) -> AppResult<Vec<RecentWorkspace>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT path, last_opened FROM recent_workspaces
             ORDER BY last_opened DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let path: String = row.get(0)?;
            let name = Path::new(&path)
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| path.clone());
            Ok(RecentWorkspace {
                available: Path::new(&path).is_dir(),
                path,
                name,
                last_opened: row.get(1)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn touch_recent_workspace(&self, path: &Path) -> AppResult<String> {
        let canonical = path.canonicalize()?;
        if !canonical.is_dir() {
            return Err(AppError::Message(format!(
                "\"{}\" is not a folder that exists",
                path.display()
            )));
        }
        let path = user_facing_path(&canonical);
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let next: i64 = tx.query_row(
            "SELECT COALESCE(MAX(last_opened), 0) + 1 FROM recent_workspaces",
            [],
            |row| row.get(0),
        )?;
        tx.execute(
            "INSERT INTO recent_workspaces(path, last_opened) VALUES(?1, ?2)
             ON CONFLICT(path) DO UPDATE SET last_opened = excluded.last_opened",
            params![path, next],
        )?;
        tx.execute(
            "DELETE FROM recent_workspaces WHERE path NOT IN (
                SELECT path FROM recent_workspaces ORDER BY last_opened DESC LIMIT 10
             )",
            [],
        )?;
        tx.commit()?;
        Ok(path)
    }

    pub fn remove_recent_workspace(&self, path: &str) -> AppResult<()> {
        self.conn.lock().unwrap().execute(
            "DELETE FROM recent_workspaces WHERE path = ?1",
            params![path],
        )?;
        Ok(())
    }

    pub fn get_window_state(&self) -> AppResult<Option<WindowState>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT x, y, width, height, maximized FROM window_state WHERE id = 0")?;
        let mut rows = stmt.query([])?;
        match rows.next()? {
            Some(row) => Ok(Some(WindowState {
                x: row.get(0)?,
                y: row.get(1)?,
                width: row.get(2)?,
                height: row.get(3)?,
                maximized: row.get::<_, i64>(4)? != 0,
            })),
            None => Ok(None),
        }
    }

    pub fn set_window_state(&self, state: &WindowState) -> AppResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO window_state (id, x, y, width, height, maximized)
             VALUES (0, ?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                x = excluded.x, y = excluded.y, width = excluded.width,
                height = excluded.height, maximized = excluded.maximized",
            params![
                state.x,
                state.y,
                state.width,
                state.height,
                state.maximized as i64
            ],
        )?;
        Ok(())
    }

    /// "system" | "light" | "dark". `None` means never explicitly set — the
    /// caller should follow the OS theme in that case.
    pub fn get_theme(&self) -> AppResult<Option<String>> {
        self.get_setting("theme")
    }

    pub fn set_theme(&self, theme: &str) -> AppResult<()> {
        self.set_setting("theme", theme)
    }

    /// Stored as one JSON blob rather than two separate keys so a partial
    /// write can never leave a mismatched family/size pair.
    pub fn get_editor_font(&self) -> AppResult<Option<EditorFont>> {
        match self.get_setting("editor_font")? {
            Some(json) => serde_json::from_str(&json)
                .map(Some)
                .map_err(|e| AppError::Message(format!("Corrupt editor_font setting: {e}"))),
            None => Ok(None),
        }
    }

    pub fn set_editor_font(&self, font: &EditorFont) -> AppResult<()> {
        let json = serde_json::to_string(font)
            .map_err(|e| AppError::Message(format!("Could not save editor font: {e}")))?;
        self.set_setting("editor_font", &json)
    }

    pub fn get_ai_settings(&self) -> AppResult<AiSettings> {
        let mut settings = match self.get_setting("ai_settings")? {
            Some(raw) => {
                let migrated_json = migrate_flat_ai_settings_json(&raw)?;
                let was_migrated = migrated_json != raw;
                let mut settings: AiSettings = serde_json::from_str(&migrated_json)
                    .map_err(|e| AppError::Message(format!("Corrupt AI settings: {e}")))?;
                if was_migrated {
                    if let Some(key) = settings.local.api_key.take().filter(|k| !k.is_empty()) {
                        crate::native::credentials::store_api_key(
                            &self.credential_provider_name("local"),
                            &key,
                        )?;
                    }
                    // Persist the migrated, now key-scrubbed shape immediately, so a
                    // plaintext key from the old flat blob doesn't linger in SQLite
                    // just because the user never re-opens AI Settings.
                    let scrubbed = serde_json::to_string(&settings).map_err(|e| {
                        AppError::Message(format!("Could not save AI settings: {e}"))
                    })?;
                    self.set_setting("ai_settings", &scrubbed)?;
                }
                settings
            }
            None => AiSettings::default(),
        };
        for provider in Self::AI_PROVIDER_NAMES {
            let key =
                crate::native::credentials::read_api_key(&self.credential_provider_name(provider))?;
            match provider {
                "local" => settings.local.api_key = key,
                "openai" => settings.openai.api_key = key,
                "anthropic" => settings.anthropic.api_key = key,
                "gemini" => settings.gemini.api_key = key,
                _ => unreachable!(),
            }
        }
        Ok(settings)
    }

    pub fn set_ai_settings(&self, settings: &AiSettings) -> AppResult<()> {
        let providers: [(&str, &Option<String>); 4] = [
            ("local", &settings.local.api_key),
            ("openai", &settings.openai.api_key),
            ("anthropic", &settings.anthropic.api_key),
            ("gemini", &settings.gemini.api_key),
        ];
        for (name, api_key) in providers {
            let target = self.credential_provider_name(name);
            match api_key.as_deref().filter(|k| !k.trim().is_empty()) {
                Some(key) => crate::native::credentials::store_api_key(&target, key)?,
                None => crate::native::credentials::delete_api_key(&target)?,
            }
        }
        let mut value = serde_json::to_value(settings)
            .map_err(|e| AppError::Message(format!("Could not save AI settings: {e}")))?;
        for provider in Self::AI_PROVIDER_NAMES {
            if let Some(block) = value.get_mut(provider).and_then(|v| v.as_object_mut()) {
                block.insert("apiKey".to_string(), serde_json::Value::Null);
            }
        }
        let json = serde_json::to_string(&value)
            .map_err(|e| AppError::Message(format!("Could not save AI settings: {e}")))?;
        self.set_setting("ai_settings", &json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_store() -> SettingsStore {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("foldown-test-{}-{}", std::process::id(), n));
        let namespace = format!("test-{}-{}", std::process::id(), n);
        SettingsStore::open(dir.join("settings.db"))
            .unwrap()
            .with_credential_namespace(namespace)
    }

    #[test]
    fn window_state_roundtrip() {
        let store = temp_store();
        assert!(store.get_window_state().unwrap().is_none());

        let state = WindowState {
            x: 10,
            y: 20,
            width: 1280,
            height: 800,
            maximized: false,
        };
        store.set_window_state(&state).unwrap();

        let loaded = store.get_window_state().unwrap().unwrap();
        assert_eq!(loaded.x, 10);
        assert_eq!(loaded.y, 20);
        assert_eq!(loaded.width, 1280);
        assert_eq!(loaded.height, 800);
        assert!(!loaded.maximized);

        store
            .set_window_state(&WindowState {
                maximized: true,
                ..state
            })
            .unwrap();
        assert!(store.get_window_state().unwrap().unwrap().maximized);
    }

    #[test]
    fn theme_roundtrip() {
        let store = temp_store();
        assert_eq!(store.get_theme().unwrap(), None);

        store.set_theme("dark").unwrap();
        assert_eq!(store.get_theme().unwrap(), Some("dark".to_string()));

        store.set_theme("light").unwrap();
        assert_eq!(store.get_theme().unwrap(), Some("light".to_string()));
    }

    #[test]
    fn editor_font_roundtrip() {
        let store = temp_store();
        assert_eq!(store.get_editor_font().unwrap(), None);

        let font = EditorFont {
            family: "Fira Code".to_string(),
            size: 16,
        };
        store.set_editor_font(&font).unwrap();
        assert_eq!(store.get_editor_font().unwrap(), Some(font));
    }

    #[test]
    fn ai_settings_roundtrip() {
        let store = temp_store();
        assert_eq!(store.get_ai_settings().unwrap(), AiSettings::default());

        let settings = AiSettings {
            provider: AiProvider::Anthropic,
            local: ProviderConfig {
                base_url: "http://localhost:11434/v1".to_string(),
                chat_model: "qwen3:8b".to_string(),
                api_key: None,
            },
            openai: ProviderConfig {
                base_url: "https://api.openai.com/v1".to_string(),
                chat_model: "gpt-4.1".to_string(),
                api_key: Some("sk-openai-test".to_string()),
            },
            anthropic: ProviderConfig {
                base_url: "https://api.anthropic.com/v1".to_string(),
                chat_model: "claude-opus-4".to_string(),
                api_key: Some("sk-anthropic-test".to_string()),
            },
            gemini: ProviderConfig {
                base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
                chat_model: "gemini-2.0-flash".to_string(),
                api_key: None,
            },
            embedding_model: Some("nomic-embed-text".to_string()),
            embedding_base_url: Some("http://127.0.0.1:9932/v1".to_string()),
            embedding_document_prefix: "search_document: ".to_string(),
            embedding_query_prefix: "search_query: ".to_string(),
            retrieval_candidate_count: 20,
            retrieval_final_count: 8,
            retrieval_max_chars: 12_000,
            reranker_enabled: true,
            reranker_base_url: Some("http://127.0.0.1:9933".to_string()),
            reranker_model: Some("bge-reranker-v2-m3".to_string()),
        };
        store.set_ai_settings(&settings).unwrap();
        assert_eq!(store.get_ai_settings().unwrap(), settings);

        // API keys must never reach the raw SQLite blob.
        let raw = store.get_setting("ai_settings").unwrap().unwrap();
        assert!(!raw.contains("sk-openai-test"));
        assert!(!raw.contains("sk-anthropic-test"));

        crate::native::credentials::delete_api_key(&store.credential_provider_name("openai"))
            .unwrap();
        crate::native::credentials::delete_api_key(&store.credential_provider_name("anthropic"))
            .unwrap();
    }

    #[test]
    fn ai_settings_defaults_fill_in_for_a_blob_saved_before_rag_fields_existed() {
        let store = temp_store();
        // Simulates a real user's pre-multi-provider database: the stored JSON
        // only ever had the four original flat fields.
        store.set_setting(
            "ai_settings",
            r#"{"baseUrl":"http://localhost:11434/v1","chatModel":"qwen3:8b","embeddingModel":null,"apiKey":null}"#,
        ).unwrap();
        let settings = store.get_ai_settings().unwrap();
        assert_eq!(settings.provider, AiProvider::Local);
        assert_eq!(settings.local.base_url, "http://localhost:11434/v1");
        assert_eq!(settings.local.chat_model, "qwen3:8b");
        assert_eq!(settings.openai.base_url, "https://api.openai.com/v1");
        assert_eq!(settings.anthropic.base_url, "https://api.anthropic.com/v1");
        assert_eq!(
            settings.gemini.base_url,
            "https://generativelanguage.googleapis.com/v1beta"
        );
        assert_eq!(settings.embedding_document_prefix, "search_document: ");
        assert_eq!(settings.embedding_query_prefix, "search_query: ");
        assert_eq!(settings.retrieval_candidate_count, 20);
        assert_eq!(settings.retrieval_final_count, 8);
        assert_eq!(settings.retrieval_max_chars, 12_000);
        assert!(!settings.reranker_enabled);
        assert_eq!(settings.reranker_base_url, None);
    }

    #[test]
    fn migrating_a_legacy_blob_moves_its_api_key_into_credential_manager_and_scrubs_it_from_the_json(
    ) {
        let store = temp_store();
        store.set_setting(
            "ai_settings",
            r#"{"baseUrl":"http://localhost:11434/v1","chatModel":"qwen3:8b","embeddingModel":null,"apiKey":"legacy-secret"}"#,
        ).unwrap();

        let settings = store.get_ai_settings().unwrap();
        assert_eq!(settings.provider, AiProvider::Local);
        assert_eq!(settings.local.base_url, "http://localhost:11434/v1");
        assert_eq!(settings.local.chat_model, "qwen3:8b");
        assert_eq!(settings.local.api_key, Some("legacy-secret".to_string()));

        // The plaintext key must not linger in the SQLite blob after migration.
        let raw = store.get_setting("ai_settings").unwrap().unwrap();
        assert!(!raw.contains("legacy-secret"));

        crate::native::credentials::delete_api_key(&store.credential_provider_name("local"))
            .unwrap();
    }

    #[test]
    fn a_blob_that_already_has_a_local_block_is_left_alone_by_migration() {
        let store = temp_store();
        store.set_setting(
            "ai_settings",
            r#"{"provider":"openai","local":{"baseUrl":"http://localhost:11434/v1","chatModel":"qwen3:8b","apiKey":null},"openai":{"baseUrl":"https://api.openai.com/v1","chatModel":"gpt-4.1","apiKey":null},"anthropic":{"baseUrl":"https://api.anthropic.com/v1","chatModel":"","apiKey":null},"gemini":{"baseUrl":"https://generativelanguage.googleapis.com/v1beta","chatModel":"","apiKey":null},"embeddingModel":null}"#,
        ).unwrap();
        let settings = store.get_ai_settings().unwrap();
        assert_eq!(settings.provider, AiProvider::Openai);
        assert_eq!(settings.openai.chat_model, "gpt-4.1");
    }

    #[test]
    fn recent_workspaces_are_promoted_capped_and_removable() {
        let store = temp_store();
        let root = std::env::temp_dir().join(format!(
            "foldown-recents-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&root).unwrap();

        for index in 0..12 {
            let path = root.join(format!("Workspace {index}"));
            std::fs::create_dir(&path).unwrap();
            store.touch_recent_workspace(&path).unwrap();
        }

        let recents = store.recent_workspaces(10).unwrap();
        assert_eq!(recents.len(), 10);
        assert_eq!(recents[0].name, "Workspace 11");
        assert_eq!(recents[9].name, "Workspace 2");
        assert!(recents.iter().all(|entry| entry.available));
        assert!(recents.iter().all(|entry| !entry.path.starts_with(r"\\?\")));

        let promoted = root.join("Workspace 5");
        store.touch_recent_workspace(&promoted).unwrap();
        assert_eq!(store.recent_workspaces(10).unwrap()[0].name, "Workspace 5");

        std::fs::remove_dir(&promoted).unwrap();
        let missing = store
            .recent_workspaces(10)
            .unwrap()
            .into_iter()
            .find(|entry| entry.name == "Workspace 5")
            .unwrap();
        assert!(!missing.available);

        store.remove_recent_workspace(&missing.path).unwrap();
        assert!(store
            .recent_workspaces(10)
            .unwrap()
            .iter()
            .all(|entry| entry.name != "Workspace 5"));
    }

    #[test]
    fn migrates_existing_windows_verbatim_recent_paths() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE recent_workspaces (
                path TEXT PRIMARY KEY,
                last_opened INTEGER NOT NULL
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO recent_workspaces(path, last_opened) VALUES(?1, 7)",
            params![r"\\?\C:\MD-Files"],
        )
        .unwrap();

        normalize_recent_workspace_paths(&mut conn).unwrap();

        let stored: String = conn
            .query_row("SELECT path FROM recent_workspaces", [], |row| row.get(0))
            .unwrap();
        assert_eq!(stored, r"C:\MD-Files");
    }
}
