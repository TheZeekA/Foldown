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

pub const BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Serialize)]
struct ToolFunction<'a> {
    name: &'a str,
    description: &'a str,
    parameters: Value,
}
#[derive(Serialize)]
struct Tool<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    function: ToolFunction<'a>,
}
#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
    tools: [Tool<'a>; 1],
    tool_choice: &'a str,
}

fn tool_definition() -> Tool<'static> {
    Tool {
        kind: "function",
        function: ToolFunction {
            name: ACTION_TOOL_NAME,
            description: ACTION_TOOL_DESCRIPTION,
            parameters: action_tool_parameters_schema(),
        },
    }
}

pub async fn list_models(api_key: Option<&str>) -> AppResult<Vec<String>> {
    crate::ai::client::list_models(BASE_URL, api_key).await
}

#[derive(Default)]
struct ToolCallAccumulator {
    arguments: String,
}

pub async fn send_chat<F>(
    config: &ProviderConfig,
    messages: &[ChatMessage],
    mut on_delta: F,
) -> AppResult<ChatOutcome>
where
    F: FnMut(&str),
{
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Message(format!("Could not configure AI client: {e}")))?;
    let mut request = client
        .post(format!("{BASE_URL}/chat/completions"))
        .json(&ChatRequest {
            model: &config.chat_model,
            messages,
            stream: true,
            tools: [tool_definition()],
            tool_choice: "auto",
        });
    if let Some(key) = config.api_key.as_deref().filter(|k| !k.trim().is_empty()) {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| AppError::Message(format!("Could not connect to ChatGPT: {e}")))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Message(format!(
            "ChatGPT returned HTTP {}: {body}",
            status.as_u16()
        )));
    }
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut text = String::new();
    let mut calls: HashMap<u32, ToolCallAccumulator> = HashMap::new();
    while let Some(part) = stream.next().await {
        let part =
            part.map_err(|e| AppError::Message(format!("ChatGPT response stream failed: {e}")))?;
        buffer.push_str(&String::from_utf8_lossy(&part));
        while let Some((end, delimiter_len)) = buffer
            .find("\r\n\r\n")
            .map(|i| (i, 4))
            .or_else(|| buffer.find("\n\n").map(|i| (i, 2)))
        {
            let event = buffer[..end].to_string();
            buffer.drain(..end + delimiter_len);
            apply_event(&event, &mut text, &mut calls, &mut on_delta)?;
        }
    }
    if !buffer.trim().is_empty() {
        apply_event(&buffer, &mut text, &mut calls, &mut on_delta)?;
    }
    let actions = parse_tool_calls(&calls)?;
    Ok(ChatOutcome { text, actions })
}

fn apply_event<F: FnMut(&str)>(
    event: &str,
    text: &mut String,
    calls: &mut HashMap<u32, ToolCallAccumulator>,
    on_delta: &mut F,
) -> AppResult<()> {
    for line in event.lines().map(str::trim) {
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" {
            continue;
        }
        let value: Value = serde_json::from_str(data)
            .map_err(|_| AppError::Message("ChatGPT returned an invalid stream".to_string()))?;
        if let Some(delta) = value
            .pointer("/choices/0/delta/content")
            .and_then(|v| v.as_str())
        {
            on_delta(delta);
            text.push_str(delta);
        }
        if let Some(tool_calls) = value
            .pointer("/choices/0/delta/tool_calls")
            .and_then(|v| v.as_array())
        {
            for call in tool_calls {
                let index = call.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let entry = calls.entry(index).or_default();
                if let Some(fragment) = call.pointer("/function/arguments").and_then(|v| v.as_str())
                {
                    entry.arguments.push_str(fragment);
                }
            }
        }
    }
    Ok(())
}

fn parse_tool_calls(
    calls: &HashMap<u32, ToolCallAccumulator>,
) -> AppResult<Vec<crate::ai::operations::AiAction>> {
    let mut indices: Vec<&u32> = calls.keys().collect();
    indices.sort();
    let mut actions = Vec::new();
    for index in indices {
        actions.extend(parse_tool_call_actions(&calls[index].arguments)?);
    }
    Ok(actions)
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
    fn tool_definition_wraps_the_shared_schema_in_the_function_envelope() {
        let tool = tool_definition();
        assert_eq!(tool.kind, "function");
        assert_eq!(tool.function.name, "propose_file_actions");
        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["type"], "function");
        assert_eq!(json["function"]["parameters"]["required"][0], "actions");
    }

    #[test]
    fn request_body_includes_tools_and_tool_choice_auto() {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "hi".to_string(),
        }];
        let request = ChatRequest {
            model: "gpt-4.1",
            messages: &messages,
            stream: true,
            tools: [tool_definition()],
            tool_choice: "auto",
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "gpt-4.1");
        assert_eq!(json["stream"], true);
        assert_eq!(json["tool_choice"], "auto");
        assert_eq!(json["tools"][0]["function"]["name"], "propose_file_actions");
    }

    #[test]
    fn streams_text_deltas_and_ignores_the_done_marker() {
        let mut text = String::new();
        let mut calls = HashMap::new();
        let mut collected = Vec::new();
        let mut on_delta = |d: &str| collected.push(d.to_string());
        apply_event(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\ndata: [DONE]\n\n",
            &mut text, &mut calls, &mut on_delta,
        ).unwrap();
        assert_eq!(text, "Hello");
        assert_eq!(collected, vec!["Hel", "lo"]);
    }

    #[test]
    fn accumulates_fragmented_tool_call_arguments_across_chunks_by_index() {
        let mut text = String::new();
        let mut calls = HashMap::new();
        let mut on_delta = |_: &str| {};
        apply_event(
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"name":"propose_file_actions","arguments":"{\"actions\":[{\"type\""}}]}}]}"#,
            &mut text, &mut calls, &mut on_delta,
        ).unwrap();
        apply_event(
            r##"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":":\"create\",\"path\":\"a.md\",\"content\":\"# A\"}]}"}}]}}]}"##,
            &mut text, &mut calls, &mut on_delta,
        ).unwrap();
        let actions = parse_tool_calls(&calls).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].kind(), "create");
        assert_eq!(actions[0].path(), "a.md");
    }

    #[test]
    fn malformed_tool_arguments_return_an_error_instead_of_silently_dropping_the_action() {
        assert!(parse_tool_call_actions("not json").is_err());
    }

    #[test]
    fn no_tool_calls_means_no_actions() {
        let calls: HashMap<u32, ToolCallAccumulator> = HashMap::new();
        assert!(parse_tool_calls(&calls).unwrap().is_empty());
    }
}
