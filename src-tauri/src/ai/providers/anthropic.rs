use std::collections::HashMap;

use futures_util::StreamExt;
use serde::Serialize;
use serde_json::Value;

use crate::ai::client::ChatMessage;
use crate::error::{AppError, AppResult};
use crate::settings::store::ProviderConfig;

use super::{
    action_tool_parameters_schema, actions_from_tool_arguments, ChatOutcome,
    ACTION_TOOL_DESCRIPTION, ACTION_TOOL_NAME,
};

pub const BASE_URL: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 8192;

#[derive(Serialize)]
struct AnthropicTool<'a> {
    name: &'a str,
    description: &'a str,
    input_schema: Value,
}
#[derive(Serialize)]
struct ContentBlock<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    text: &'a str,
}
#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: Vec<ContentBlock<'a>>,
}
#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<AnthropicMessage<'a>>,
    stream: bool,
    tools: [AnthropicTool<'a>; 1],
}

fn tool_definition() -> AnthropicTool<'static> {
    AnthropicTool {
        name: ACTION_TOOL_NAME,
        description: ACTION_TOOL_DESCRIPTION,
        input_schema: action_tool_parameters_schema(),
    }
}

/// Claude has no system-role message — Foldown's already-built system prompt
/// (workspace files, active document, retrieved context, grounding
/// instructions) becomes the top-level `system` field instead of a
/// `messages` entry.
fn split_system_and_messages(messages: &[ChatMessage]) -> (String, Vec<AnthropicMessage<'_>>) {
    let system = messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let rest = messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| AnthropicMessage {
            role: if m.role == "assistant" {
                "assistant"
            } else {
                "user"
            },
            content: vec![ContentBlock {
                kind: "text",
                text: &m.content,
            }],
        })
        .collect();
    (system, rest)
}

pub async fn list_models(api_key: Option<&str>) -> AppResult<Vec<String>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Message(format!("Could not configure AI client: {e}")))?;
    let mut request = client
        .get(format!("{BASE_URL}/models"))
        .header("anthropic-version", ANTHROPIC_VERSION);
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        request = request.header("x-api-key", key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| AppError::Message(format!("Could not connect to Claude: {e}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::Message(format!("Could not read Claude's model list: {e}")))?;
    if !status.is_success() {
        return Err(AppError::Message(format!(
            "Claude returned HTTP {}",
            status.as_u16()
        )));
    }
    crate::ai::client::parse_models(&body)
}

struct ContentBlockAccumulator {
    is_tool_use: bool,
    json: String,
}

pub async fn send_chat<F>(
    config: &ProviderConfig,
    messages: &[ChatMessage],
    mut on_delta: F,
) -> AppResult<ChatOutcome>
where
    F: FnMut(&str),
{
    let (system, rest) = split_system_and_messages(messages);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Message(format!("Could not configure AI client: {e}")))?;
    let mut request = client
        .post(format!("{BASE_URL}/messages"))
        .header("anthropic-version", ANTHROPIC_VERSION)
        .json(&ChatRequest {
            model: &config.chat_model,
            max_tokens: MAX_TOKENS,
            system: &system,
            messages: rest,
            stream: true,
            tools: [tool_definition()],
        });
    if let Some(key) = config.api_key.as_deref().filter(|k| !k.trim().is_empty()) {
        request = request.header("x-api-key", key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| AppError::Message(format!("Could not connect to Claude: {e}")))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Message(format!(
            "Claude returned HTTP {}: {body}",
            status.as_u16()
        )));
    }
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut text = String::new();
    let mut blocks: HashMap<u32, ContentBlockAccumulator> = HashMap::new();
    while let Some(part) = stream.next().await {
        let part =
            part.map_err(|e| AppError::Message(format!("Claude response stream failed: {e}")))?;
        buffer.push_str(&String::from_utf8_lossy(&part));
        while let Some((end, delimiter_len)) = buffer
            .find("\r\n\r\n")
            .map(|i| (i, 4))
            .or_else(|| buffer.find("\n\n").map(|i| (i, 2)))
        {
            let event = buffer[..end].to_string();
            buffer.drain(..end + delimiter_len);
            apply_event(&event, &mut text, &mut blocks, &mut on_delta)?;
        }
    }
    if !buffer.trim().is_empty() {
        apply_event(&buffer, &mut text, &mut blocks, &mut on_delta)?;
    }
    let mut indices: Vec<&u32> = blocks.keys().collect();
    indices.sort();
    let mut actions = Vec::new();
    for index in indices {
        let block = &blocks[index];
        if block.is_tool_use {
            actions.extend(parse_tool_call_actions(&block.json)?);
        }
    }
    Ok(ChatOutcome { text, actions })
}

fn apply_event<F: FnMut(&str)>(
    event: &str,
    text: &mut String,
    blocks: &mut HashMap<u32, ContentBlockAccumulator>,
    on_delta: &mut F,
) -> AppResult<()> {
    let mut event_name = None;
    let mut data = None;
    for line in event.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix("event:") {
            event_name = Some(name.trim().to_string());
        } else if let Some(payload) = line.strip_prefix("data:") {
            data = Some(payload.trim().to_string());
        }
    }
    let (Some(event_name), Some(data)) = (event_name, data) else {
        return Ok(());
    };
    let value: Value = serde_json::from_str(&data)
        .map_err(|_| AppError::Message("Claude returned an invalid stream".to_string()))?;
    match event_name.as_str() {
        "content_block_start" => {
            let index = value.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let block_type = value
                .pointer("/content_block/type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            blocks.insert(
                index,
                ContentBlockAccumulator {
                    is_tool_use: block_type == "tool_use",
                    json: String::new(),
                },
            );
        }
        "content_block_delta" => {
            let index = value.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            if let Some(text_delta) = value.pointer("/delta/text").and_then(|v| v.as_str()) {
                on_delta(text_delta);
                text.push_str(text_delta);
            }
            if let Some(partial) = value
                .pointer("/delta/partial_json")
                .and_then(|v| v.as_str())
            {
                if let Some(block) = blocks.get_mut(&index) {
                    block.json.push_str(partial);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn parse_tool_call_actions(
    arguments_json: &str,
) -> AppResult<Vec<crate::ai::operations::AiAction>> {
    let value: Value = serde_json::from_str(arguments_json).map_err(|_| {
        AppError::Message("The model returned malformed Foldown actions".to_string())
    })?;
    actions_from_tool_arguments(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uses_the_top_level_system_field_not_a_system_message() {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are Foldown.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
            },
        ];
        let (system, rest) = split_system_and_messages(&messages);
        assert_eq!(system, "You are Foldown.");
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].role, "user");
    }

    #[test]
    fn request_body_uses_flat_tool_envelope_and_fixed_max_tokens() {
        let input = [ChatMessage {
            role: "user".to_string(),
            content: "hi".to_string(),
        }];
        let (_, messages) = split_system_and_messages(&input);
        let request = ChatRequest {
            model: "claude-opus-4",
            max_tokens: MAX_TOKENS,
            system: "sys",
            messages,
            stream: true,
            tools: [tool_definition()],
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["max_tokens"], 8192);
        assert_eq!(json["tools"][0]["name"], "propose_file_actions");
        assert!(
            json["tools"][0].get("function").is_none(),
            "Claude's tool envelope is flat, not nested under function"
        );
    }

    #[test]
    fn streams_text_deltas_from_content_block_delta_events() {
        let mut text = String::new();
        let mut blocks = HashMap::new();
        let mut collected = Vec::new();
        let mut on_delta = |d: &str| collected.push(d.to_string());
        apply_event(
            "event: content_block_start\ndata: {\"index\":0,\"content_block\":{\"type\":\"text\"}}",
            &mut text,
            &mut blocks,
            &mut on_delta,
        )
        .unwrap();
        apply_event(
            "event: content_block_delta\ndata: {\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}",
            &mut text, &mut blocks, &mut on_delta,
        ).unwrap();
        assert_eq!(text, "Hello");
        assert_eq!(collected, vec!["Hello"]);
    }

    #[test]
    fn accumulates_fragmented_tool_use_partial_json_by_index() {
        let mut text = String::new();
        let mut blocks = HashMap::new();
        let mut on_delta = |_: &str| {};
        apply_event(
            "event: content_block_start\ndata: {\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"name\":\"propose_file_actions\"}}",
            &mut text, &mut blocks, &mut on_delta,
        ).unwrap();
        apply_event(
            r#"event: content_block_delta
data: {"index":1,"delta":{"type":"input_json_delta","partial_json":"{\"actions\":[{\"type\":\"create\","}}"#,
            &mut text, &mut blocks, &mut on_delta,
        ).unwrap();
        apply_event(
            r##"event: content_block_delta
data: {"index":1,"delta":{"type":"input_json_delta","partial_json":"\"path\":\"a.md\",\"content\":\"# A\"}]}"}}"##,
            &mut text, &mut blocks, &mut on_delta,
        ).unwrap();
        let block = &blocks[&1];
        assert!(block.is_tool_use);
        let actions = parse_tool_call_actions(&block.json).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].kind(), "create");
    }

    #[test]
    fn a_plain_text_block_is_never_treated_as_a_tool_use() {
        let mut text = String::new();
        let mut blocks = HashMap::new();
        let mut on_delta = |_: &str| {};
        apply_event(
            "event: content_block_start\ndata: {\"index\":0,\"content_block\":{\"type\":\"text\"}}",
            &mut text,
            &mut blocks,
            &mut on_delta,
        )
        .unwrap();
        assert!(!blocks[&0].is_tool_use);
    }

    #[test]
    fn malformed_tool_arguments_return_an_error() {
        assert!(parse_tool_call_actions("not json").is_err());
    }
}
