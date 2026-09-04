use tauri::{Manager, State};

use crate::error::{AppError, AppResult};
use crate::settings::store::{
    AiSettings, EditorFont, ProviderConfig, RecentWorkspace, SettingsStore,
};
use crate::workspace_authority::ActiveWorkspace;

#[tauri::command]
pub fn get_recent_workspaces(store: State<SettingsStore>) -> AppResult<Vec<RecentWorkspace>> {
    store.recent_workspaces(10)
}

#[tauri::command]
pub fn open_workspace(
    app: tauri::AppHandle,
    store: State<SettingsStore>,
    active: State<ActiveWorkspace>,
    path: String,
) -> AppResult<String> {
    let root = active.activate(std::path::Path::new(&path))?;
    let touched = store.touch_recent_workspace(&root)?;
    let sync_root = root.clone();
    tauri::async_runtime::spawn(async move {
        use tauri::Emitter;
        let _ = app.emit(
            crate::ai::commands::AI_INDEX_STATUS_EVENT,
            crate::ai::commands::IndexStatusEvent {
                status: "indexing".to_string(),
                detail: None,
            },
        );
        let index = app.state::<crate::ai::index::KnowledgeIndex>();
        let result = index.sync_workspace(&sync_root);
        let event = match result {
            Ok(()) => crate::ai::commands::IndexStatusEvent {
                status: "ready".to_string(),
                detail: None,
            },
            Err(error) => crate::ai::commands::IndexStatusEvent {
                status: "error".to_string(),
                detail: Some(error.to_string()),
            },
        };
        let _ = app.emit(crate::ai::commands::AI_INDEX_STATUS_EVENT, event);
    });
    Ok(touched)
}

#[tauri::command]
pub fn remove_recent_workspace(store: State<SettingsStore>, path: String) -> AppResult<()> {
    store.remove_recent_workspace(&path)
}

#[tauri::command]
pub fn get_theme(store: State<SettingsStore>) -> AppResult<Option<String>> {
    store.get_theme()
}

#[tauri::command]
pub fn set_theme(store: State<SettingsStore>, theme: String) -> AppResult<()> {
    store.set_theme(&theme)
}

#[tauri::command]
pub fn get_editor_font(store: State<SettingsStore>) -> AppResult<Option<EditorFont>> {
    store.get_editor_font()
}

#[tauri::command]
pub fn set_editor_font(store: State<SettingsStore>, family: String, size: u32) -> AppResult<()> {
    store.set_editor_font(&EditorFont { family, size })
}

pub fn validate_ai_base_url(value: &str) -> AppResult<()> {
    let parsed = url::Url::parse(value.trim())
        .map_err(|_| AppError::Message("AI base URL is not valid".to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(AppError::Message(
            "AI base URL must use http or https".to_string(),
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn get_ai_settings(store: State<SettingsStore>) -> AppResult<AiSettings> {
    store.get_ai_settings()
}

pub(crate) fn normalize_ai_settings(settings: AiSettings) -> AppResult<AiSettings> {
    let normalize = |value: Option<String>| {
        value
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let normalize_url = |value: Option<String>| -> AppResult<Option<String>> {
        match normalize(value) {
            Some(url) => {
                let url = url.trim_end_matches('/').to_string();
                validate_ai_base_url(&url)?;
                Ok(Some(url))
            }
            None => Ok(None),
        }
    };
    let normalize_provider =
        |config: ProviderConfig, validate_url: bool| -> AppResult<ProviderConfig> {
            let base_url = config.base_url.trim().trim_end_matches('/').to_string();
            if validate_url {
                validate_ai_base_url(&base_url)?;
            }
            Ok(ProviderConfig {
                base_url,
                chat_model: config.chat_model.trim().to_string(),
                api_key: normalize(config.api_key),
            })
        };
    let embedding_base_url = normalize_url(settings.embedding_base_url)?;
    let reranker_base_url = normalize_url(settings.reranker_base_url)?;
    Ok(AiSettings {
        provider: settings.provider,
        // Local Server's endpoint is user-edited and load-bearing — validate it
        // like any other configured server URL.
        local: normalize_provider(settings.local, true)?,
        // The three cloud providers' base_url is present only for struct
        // symmetry (each provider module uses its own hardcoded endpoint
        // constant, never this field) — normalize but don't validate it as a
        // live endpoint.
        openai: normalize_provider(settings.openai, false)?,
        anthropic: normalize_provider(settings.anthropic, false)?,
        gemini: normalize_provider(settings.gemini, false)?,
        embedding_model: normalize(settings.embedding_model),
        embedding_base_url,
        embedding_document_prefix: settings.embedding_document_prefix,
        embedding_query_prefix: settings.embedding_query_prefix,
        retrieval_candidate_count: settings.retrieval_candidate_count.max(1),
        retrieval_final_count: settings.retrieval_final_count.max(1),
        retrieval_max_chars: settings.retrieval_max_chars.max(500),
        reranker_enabled: settings.reranker_enabled,
        reranker_base_url,
        reranker_model: normalize(settings.reranker_model),
    })
}

#[tauri::command]
pub fn set_ai_settings(store: State<SettingsStore>, settings: AiSettings) -> AppResult<()> {
    store.set_ai_settings(&normalize_ai_settings(settings)?)
}

#[cfg(test)]
mod ai_tests {
    use super::*;

    #[test]
    fn accepts_only_http_or_https_endpoints() {
        assert!(validate_ai_base_url("http://localhost:11434/v1").is_ok());
        assert!(validate_ai_base_url("https://models.internal/v1").is_ok());
        assert!(validate_ai_base_url("file:///C:/notes").is_err());
        assert!(validate_ai_base_url("ftp://models.internal").is_err());
        assert!(validate_ai_base_url("not a url").is_err());
    }

    #[test]
    fn rejects_an_invalid_embedding_or_reranker_url_while_accepting_a_valid_local_base_url() {
        let store_result = |embedding_base_url: Option<&str>| AiSettings {
            local: ProviderConfig {
                base_url: "http://localhost:11434/v1".to_string(),
                chat_model: "qwen3:8b".to_string(),
                ..AiSettings::default().local
            },
            embedding_base_url: embedding_base_url.map(str::to_string),
            ..AiSettings::default()
        };
        assert!(normalize_ai_settings(store_result(Some("not a url"))).is_err());
        let good = normalize_ai_settings(store_result(Some("http://127.0.0.1:9932/v1"))).unwrap();
        assert_eq!(
            good.embedding_base_url,
            Some("http://127.0.0.1:9932/v1".to_string())
        );
    }

    #[test]
    fn cloud_provider_base_urls_are_normalized_but_not_validated_as_live_endpoints() {
        // A cloud provider's base_url is never actually used to send a request
        // (each provider module hardcodes its own endpoint), so a stray or even
        // malformed value there must not block saving settings the way an
        // invalid Local Server endpoint would.
        let mut settings = AiSettings::default();
        settings.local = ProviderConfig {
            base_url: "http://localhost:11434/v1".to_string(),
            chat_model: "qwen3:8b".to_string(),
            api_key: None,
        };
        settings.openai.base_url = "  not a url  ".to_string();
        settings.openai.chat_model = "  gpt-4.1  ".to_string();
        settings.openai.api_key = Some("  sk-test  ".to_string());

        let normalized = normalize_ai_settings(settings).unwrap();
        assert_eq!(normalized.openai.base_url, "not a url");
        assert_eq!(normalized.openai.chat_model, "gpt-4.1");
        assert_eq!(normalized.openai.api_key, Some("sk-test".to_string()));
    }

    #[test]
    fn an_invalid_local_base_url_is_rejected() {
        let mut settings = AiSettings::default();
        settings.local.base_url = "not a url".to_string();
        assert!(normalize_ai_settings(settings).is_err());
    }
}
