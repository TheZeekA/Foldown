use futures_util::StreamExt;
use serde::Serialize;
use serde_json::Value;

use crate::ai::client::ChatMessage;
use crate::ai::operations::AiAction;
use crate::error::{AppError, AppResult};
use crate::settings::store::ProviderConfig;

use super::{
    action_tool_parameters_schema, actions_from_tool_arguments, ChatOutcome,
    ACTION_TOOL_DESCRIPTION, ACTION_TOOL_NAME,
};

pub const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

#[derive(Serialize)]
struct FunctionDeclaration<'a> {
    name: &'a str,
    description: &'a str,
    parameters: Value,
}
#[derive(Serialize)]
struct Tool<'a> {
    #[serde(rename = "functionDeclarations")]
    function_declarations: [FunctionDeclaration<'a>; 1],
}
#[derive(Serialize)]
struct Part<'a> {
    text: &'a str,
}
#[derive(Serialize)]
struct Content<'a> {
    role: &'a str,
    parts: Vec<Part<'a>>,
}
#[derive(Serialize)]
struct SystemInstruction<'a> {
    parts: [Part<'a>; 1],
}
#[derive(Serialize)]
struct ChatRequest<'a> {
    contents: Vec<Content<'a>>,
    #[serde(rename = "systemInstruction")]
    system_instruction: SystemInstruction<'a>,
    tools: [Tool<'a>; 1],
}

fn tool_definition() -> Tool<'static> {
    Tool {
        function_declarations: [FunctionDeclaration {
            name: ACTION_TOOL_NAME,
            description: ACTION_TOOL_DESCRIPTION,
            parameters: action_tool_parameters_schema(),
        }],
    }
}

/// Foldown's internal `"assistant"` role maps to Gemini's `"model"`; the
/// leading system-role message becomes `systemInstruction` instead of a
/// `contents` entry, matching Gemini having no system role either.
fn build_request<'a>(messages: &'a [ChatMessage]) -> ChatRequest<'a> {
    let system_text = messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.as_str())
        .unwrap_or("");
    let contents = messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| Content {
            role: if m.role == "assistant" {
                "model"
            } else {
                "user"
            },
            parts: vec![Part { text: &m.content }],
        })
        .collect();
    ChatRequest {
        contents,
        system_instruction: SystemInstruction {
            parts: [Part { text: system_text }],
        },
        tools: [tool_definition()],
    }
}

fn chat_url(model: &str, api_key: &str) -> String {
    format!("{BASE_URL}/models/{model}:streamGenerateContent?alt=sse&key={api_key}")
}

pub async fn send_chat<F>(
    config: &ProviderConfig,
    messages: &[ChatMessage],
    mut on_delta: F,
) -> AppResult<ChatOutcome>
where
    F: FnMut(&str),
{
    let api_key = config.api_key.as_deref().unwrap_or("");
    let request_body = build_request(messages);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Message(format!("Could not configure AI client: {e}")))?;
    let response = client
        .post(chat_url(&config.chat_model, api_key))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| AppError::Message(format!("Could not connect to Gemini: {e}")))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Message(format!(
            "Gemini returned HTTP {}: {body}",
            status.as_u16()
        )));
    }
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut text = String::new();
    let mut actions = Vec::new();
    while let Some(part) = stream.next().await {
        let part =
            part.map_err(|e| AppError::Message(format!("Gemini response stream failed: {e}")))?;
        buffer.push_str(&String::from_utf8_lossy(&part));
        while let Some((end, delimiter_len)) = buffer
            .find("\r\n\r\n")
            .map(|i| (i, 4))
            .or_else(|| buffer.find("\n\n").map(|i| (i, 2)))
        {
            let event = buffer[..end].to_string();
            buffer.drain(..end + delimiter_len);
            apply_event(&event, &mut text, &mut actions, &mut on_delta)?;
        }
    }
    if !buffer.trim().is_empty() {
        apply_event(&buffer, &mut text, &mut actions, &mut on_delta)?;
    }
    Ok(ChatOutcome { text, actions })
}

/// Unlike OpenAI/Anthropic, Gemini's function-call arguments arrive as a
/// complete, already-parsed JSON object in one chunk (`args` is an object,
/// not a string fragment) — no partial-JSON accumulation is needed.
fn apply_event<F: FnMut(&str)>(
    event: &str,
    text: &mut String,
    actions: &mut Vec<AiAction>,
    on_delta: &mut F,
) -> AppResult<()> {
    for line in event.lines().map(str::trim) {
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(data)
            .map_err(|_| AppError::Message("Gemini returned an invalid stream".to_string()))?;
        let parts = value
            .pointer("/candidates/0/content/parts")
            .and_then(|v| v.as_array());
        for part in parts.into_iter().flatten() {
            if let Some(delta) = part.get("text").and_then(|v| v.as_str()) {
                on_delta(delta);
                text.push_str(delta);
            }
            if let Some(call) = part.get("functionCall") {
                let args = call
                    .get("args")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(Default::default()));
                actions.extend(actions_from_tool_arguments(args)?);
            }
        }
    }
    Ok(())
}

pub async fn list_models(api_key: Option<&str>) -> AppResult<Vec<String>> {
    let api_key = api_key.unwrap_or("");
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Message(format!("Could not configure AI client: {e}")))?;
    let response = client
        .get(format!("{BASE_URL}/models?key={api_key}"))
        .send()
        .await
        .map_err(|e| AppError::Message(format!("Could not connect to Gemini: {e}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::Message(format!("Could not read Gemini's model list: {e}")))?;
    if !status.is_success() {
        return Err(AppError::Message(format!(
            "Gemini returned HTTP {}",
            status.as_u16()
        )));
    }
    parse_models(&body)
}

/// Filters to entries whose `supportedGenerationMethods` includes
/// `"generateContent"` and strips the `"models/"` prefix before use in the
/// chat URL path.
fn parse_models(json: &str) -> AppResult<Vec<String>> {
    let value: Value = serde_json::from_str(json)
        .map_err(|_| AppError::Message("Gemini returned invalid model data".to_string()))?;
    let models = value
        .get("models")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Message("Gemini returned no model list".to_string()))?;
    let mut names: Vec<String> = models
        .iter()
        .filter_map(|item| {
            let methods = item
                .get("supportedGenerationMethods")
                .and_then(|v| v.as_array())?;
            let supports_generate = methods
                .iter()
                .any(|m| m.as_str() == Some("generateContent"));
            if !supports_generate {
                return None;
            }
            item.get("name")
                .and_then(|v| v.as_str())
                .map(|name| name.trim_start_matches("models/").to_string())
        })
        .collect();
    names.sort();
    names.dedup();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_url_uses_streaming_path_with_query_param_key() {
        let url = chat_url("gemini-2.0-flash", "abc123");
        assert_eq!(url, "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:streamGenerateContent?alt=sse&key=abc123");
    }

    #[test]
    fn request_maps_assistant_role_to_model_and_extracts_system_instruction() {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are Foldown.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: "hello".to_string(),
            },
        ];
        let request = build_request(&messages);
        assert_eq!(request.system_instruction.parts[0].text, "You are Foldown.");
        assert_eq!(request.contents.len(), 2);
        assert_eq!(request.contents[0].role, "user");
        assert_eq!(request.contents[1].role, "model");
    }

    #[test]
    fn request_body_wraps_the_shared_schema_in_function_declarations() {
        let input = [ChatMessage {
            role: "user".to_string(),
            content: "hi".to_string(),
        }];
        let request = build_request(&input);
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(
            json["tools"][0]["functionDeclarations"][0]["name"],
            "propose_file_actions"
        );
    }

    #[test]
    fn streams_text_deltas_from_candidate_parts() {
        let mut text = String::new();
        let mut actions = Vec::new();
        let mut collected = Vec::new();
        let mut on_delta = |d: &str| collected.push(d.to_string());
        apply_event(
            r#"data: {"candidates":[{"content":{"parts":[{"text":"Hello"}]},"finishReason":null}]}"#,
            &mut text, &mut actions, &mut on_delta,
        ).unwrap();
        assert_eq!(text, "Hello");
        assert_eq!(collected, vec!["Hello"]);
    }

    #[test]
    fn a_function_call_chunk_arrives_as_a_complete_object_with_no_accumulation_needed() {
        let mut text = String::new();
        let mut actions = Vec::new();
        let mut on_delta = |_: &str| {};
        apply_event(
            r##"data: {"candidates":[{"content":{"parts":[{"functionCall":{"name":"propose_file_actions","args":{"actions":[{"type":"create","path":"a.md","content":"# A"}]}}}]}}]}"##,
            &mut text, &mut actions, &mut on_delta,
        ).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].kind(), "create");
        assert_eq!(actions[0].path(), "a.md");
    }

    #[test]
    fn parse_models_filters_to_generate_content_and_strips_the_models_prefix() {
        let json = r#"{"models":[
            {"name":"models/gemini-2.0-flash","supportedGenerationMethods":["generateContent"]},
            {"name":"models/embedding-001","supportedGenerationMethods":["embedContent"]}
        ]}"#;
        let models = parse_models(json).unwrap();
        assert_eq!(models, vec!["gemini-2.0-flash"]);
    }
}
