use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::settings::store::AiSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
}

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: &'a [String],
}

pub fn endpoint(base: &str, resource: &str) -> AppResult<String> {
    let base = base.trim_end_matches('/');
    let parsed = url::Url::parse(&format!("{base}/{}", resource.trim_start_matches('/')))
        .map_err(|_| AppError::Message("AI endpoint is not valid".to_string()))?;
    Ok(parsed.to_string())
}

pub fn parse_completion(json: &str) -> AppResult<String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|_| AppError::Message("The model returned invalid JSON".to_string()))?;
    value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            AppError::Message("The model response did not contain a message".to_string())
        })
}

pub fn parse_sse(input: &str) -> AppResult<Vec<String>> {
    let mut deltas = Vec::new();
    for line in input.lines().map(str::trim) {
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" {
            break;
        }
        let value: serde_json::Value = serde_json::from_str(data)
            .map_err(|_| AppError::Message("The model returned an invalid stream".to_string()))?;
        if let Some(delta) = value
            .pointer("/choices/0/delta/content")
            .and_then(|v| v.as_str())
        {
            deltas.push(delta.to_string());
        }
    }
    Ok(deltas)
}

pub fn parse_embeddings(json: &str) -> AppResult<Vec<Vec<f32>>> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|_| AppError::Message("The embedding model returned invalid JSON".to_string()))?;
    let data = value
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            AppError::Message("The embedding response contained no vectors".to_string())
        })?;
    let mut indexed = data
        .iter()
        .map(|item| {
            let index = item.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let vector = item
                .get("embedding")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    AppError::Message(
                        "The embedding response contained an invalid vector".to_string(),
                    )
                })?
                .iter()
                .map(|v| {
                    v.as_f64().map(|n| n as f32).ok_or_else(|| {
                        AppError::Message(
                            "The embedding response contained an invalid number".to_string(),
                        )
                    })
                })
                .collect::<AppResult<Vec<_>>>()?;
            Ok((index, vector))
        })
        .collect::<AppResult<Vec<_>>>()?;
    indexed.sort_by_key(|(index, _)| *index);
    Ok(indexed.into_iter().map(|(_, vector)| vector).collect())
}

pub fn parse_models(json: &str) -> AppResult<Vec<String>> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|_| AppError::Message("The AI server returned invalid model data".to_string()))?;
    let data = value
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Message("The AI server returned no model list".to_string()))?;
    let mut models = data
        .iter()
        .filter_map(|item| item.get("id").and_then(|v| v.as_str()).map(str::to_string))
        .collect::<Vec<_>>();
    models.sort();
    models.dedup();
    Ok(models)
}

async fn fetch_models(
    base_url: &str,
    api_key: Option<&str>,
    timeout: Option<std::time::Duration>,
) -> AppResult<Vec<String>> {
    let mut builder = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());
    if let Some(timeout) = timeout {
        builder = builder.timeout(timeout);
    }
    let client = builder
        .build()
        .map_err(|e| AppError::Message(format!("Could not configure AI client: {e}")))?;
    let mut request = client.get(endpoint(base_url, "models")?);
    if let Some(key) = api_key.filter(|key| !key.trim().is_empty()) {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| AppError::Message(format!("Could not connect to the AI server: {e}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::Message(format!("Could not read the model list: {e}")))?;
    if !status.is_success() {
        return Err(AppError::Message(format!(
            "AI server returned HTTP {}",
            status.as_u16()
        )));
    }
    parse_models(&body)
}

pub async fn list_models(base_url: &str, api_key: Option<&str>) -> AppResult<Vec<String>> {
    fetch_models(base_url, api_key, None).await
}

/// Same as `list_models` but with a short timeout — used for probing several
/// candidate local ports/endpoints (server discovery, connection status
/// checks) where most candidates are expected to be unreachable and must
/// fail fast rather than hang on the OS's default TCP connect timeout.
pub async fn probe_models(base_url: &str, api_key: Option<&str>) -> AppResult<Vec<String>> {
    fetch_models(
        base_url,
        api_key,
        Some(std::time::Duration::from_millis(800)),
    )
    .await
}

/// The embedding server is configurable separately from the chat server (e.g.
/// a dedicated Nomic embeddings server at `127.0.0.1:9932`) — mirrors the
/// identical fallback pattern used by `ai::reranker::rerank` for its own
/// separately-configurable server.
fn embedding_endpoint_base(settings: &AiSettings) -> &str {
    settings
        .embedding_base_url
        .as_deref()
        .unwrap_or(&settings.local.base_url)
}

pub async fn create_embeddings(
    settings: &AiSettings,
    model: &str,
    input: &[String],
) -> AppResult<Vec<Vec<f32>>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Message(format!("Could not configure AI client: {e}")))?;
    let mut request = client
        .post(endpoint(embedding_endpoint_base(settings), "embeddings")?)
        .json(&EmbeddingRequest { model, input });
    if let Some(key) = settings.local.api_key.as_deref() {
        request = request.bearer_auth(key);
    }
    let response = request.send().await.map_err(|e| {
        AppError::Message(format!("Could not connect to the embedding endpoint: {e}"))
    })?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::Message(format!("Could not read the embedding response: {e}")))?;
    if !status.is_success() {
        return Err(AppError::Message(format!(
            "Embedding endpoint returned HTTP {}",
            status.as_u16()
        )));
    }
    parse_embeddings(&body)
}

pub async fn send_chat<F>(
    settings: &AiSettings,
    messages: &[ChatMessage],
    mut on_delta: F,
) -> AppResult<String>
where
    F: FnMut(&str),
{
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Message(format!("Could not configure AI client: {e}")))?;
    let mut request = client
        .post(endpoint(&settings.local.base_url, "chat/completions")?)
        .json(&ChatRequest {
            model: &settings.local.chat_model,
            messages,
            stream: true,
        });
    if let Some(key) = settings.local.api_key.as_deref() {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| AppError::Message(format!("Could not connect to the AI endpoint: {e}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::Message(format!(
            "AI endpoint returned HTTP {}",
            status.as_u16()
        )));
    }
    let is_stream = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("text/event-stream"));
    if !is_stream {
        let body = response
            .text()
            .await
            .map_err(|e| AppError::Message(format!("Could not read the AI response: {e}")))?;
        let content = parse_completion(&body)?;
        on_delta(&content);
        return Ok(content);
    }
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut full = String::new();
    while let Some(part) = stream.next().await {
        let part =
            part.map_err(|e| AppError::Message(format!("AI response stream failed: {e}")))?;
        buffer.push_str(&String::from_utf8_lossy(&part));
        while let Some((end, delimiter_len)) = buffer
            .find("\r\n\r\n")
            .map(|i| (i, 4))
            .or_else(|| buffer.find("\n\n").map(|i| (i, 2)))
        {
            let event = buffer[..end].to_string();
            buffer.drain(..end + delimiter_len);
            for delta in parse_sse(&event)? {
                on_delta(&delta);
                full.push_str(&delta);
            }
        }
    }
    if !buffer.trim().is_empty() {
        for delta in parse_sse(&buffer)? {
            on_delta(&delta);
            full.push_str(&delta);
        }
    }
    Ok(full)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::store::ProviderConfig;

    #[test]
    fn joins_openai_endpoint_without_double_slashes() {
        assert_eq!(
            endpoint("http://localhost:11434/v1/", "chat/completions").unwrap(),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[test]
    fn parses_chat_completion_content() {
        let json = r#"{"choices":[{"message":{"content":"Hello"}}]}"#;
        assert_eq!(parse_completion(json).unwrap(), "Hello");
    }

    #[test]
    fn parses_sse_deltas_and_done_marker() {
        let input = "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\ndata: [DONE]\n\n";
        assert_eq!(parse_sse(input).unwrap(), vec!["Hel", "lo"]);
    }

    #[test]
    fn parses_embedding_vectors_in_index_order() {
        let json =
            r#"{"data":[{"index":1,"embedding":[0.0,1.0]},{"index":0,"embedding":[1.0,0.0]}]}"#;
        assert_eq!(
            parse_embeddings(json).unwrap(),
            vec![vec![1.0, 0.0], vec![0.0, 1.0]]
        );
    }

    #[test]
    fn parses_available_model_ids() {
        let json = r#"{"data":[{"id":"qwen3:8b"},{"id":"llama3.2:latest"}]}"#;
        assert_eq!(
            parse_models(json).unwrap(),
            vec!["llama3.2:latest", "qwen3:8b"]
        );
    }

    #[test]
    fn embedding_requests_post_to_the_dedicated_embedding_server_when_configured() {
        // The headline "separate local embedding server" feature (e.g. Nomic
        // embeddings at 127.0.0.1:9932, distinct from the chat server) only
        // works if create_embeddings actually targets embedding_base_url
        // instead of always posting to the local server's base_url.
        let settings = AiSettings {
            local: ProviderConfig {
                base_url: "http://localhost:11434/v1".to_string(),
                ..AiSettings::default().local
            },
            embedding_base_url: Some("http://127.0.0.1:9932/v1".to_string()),
            ..AiSettings::default()
        };
        assert_eq!(
            embedding_endpoint_base(&settings),
            "http://127.0.0.1:9932/v1"
        );
        assert_eq!(
            endpoint(embedding_endpoint_base(&settings), "embeddings").unwrap(),
            "http://127.0.0.1:9932/v1/embeddings"
        );
    }

    #[test]
    fn embedding_requests_fall_back_to_the_local_base_url_when_unset() {
        let settings = AiSettings {
            local: ProviderConfig {
                base_url: "http://localhost:11434/v1".to_string(),
                ..AiSettings::default().local
            },
            embedding_base_url: None,
            ..AiSettings::default()
        };
        assert_eq!(
            embedding_endpoint_base(&settings),
            "http://localhost:11434/v1"
        );
        assert_eq!(
            endpoint(embedding_endpoint_base(&settings), "embeddings").unwrap(),
            "http://localhost:11434/v1/embeddings"
        );
    }
}
