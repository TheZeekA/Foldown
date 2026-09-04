use std::collections::HashSet;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub(crate) const MAX_ACTION_CONTENT: usize = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AiAction {
    Create { path: String, content: String },
    Replace { path: String, content: String },
    Delete { path: String },
}

impl AiAction {
    pub fn path(&self) -> &str {
        match self {
            Self::Create { path, .. } | Self::Replace { path, .. } | Self::Delete { path } => path,
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Create { .. } => "create",
            Self::Replace { .. } => "replace",
            Self::Delete { .. } => "delete",
        }
    }
    pub fn content(&self) -> Option<&str> {
        match self {
            Self::Create { content, .. } | Self::Replace { content, .. } => Some(content),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedAssistantResponse {
    pub message: String,
    pub actions: Vec<AiAction>,
}

pub fn validate_relative_markdown_path(value: &str) -> AppResult<()> {
    if value.contains('\\') || value.contains(':') || value.trim() != value {
        return Err(AppError::Message(
            "AI action contains an invalid path".to_string(),
        ));
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(AppError::Message(
            "AI action path must remain inside the workspace".to_string(),
        ));
    }
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    {
        return Err(AppError::Message(
            "AI actions may only target Markdown files".to_string(),
        ));
    }
    Ok(())
}

fn repair_unescaped_content_quotes(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some(relative_key) = input[cursor..].find("\"content\"") {
        let key = cursor + relative_key;
        let Some(relative_colon) = input[key + 9..].find(':') else {
            break;
        };
        let colon = key + 9 + relative_colon;
        let Some(relative_open) = input[colon + 1..].find('"') else {
            break;
        };
        let open = colon + 1 + relative_open;
        output.push_str(&input[cursor..open + 1]);
        let bytes = input.as_bytes();
        let mut position = open + 1;
        let mut segment = position;
        while position < bytes.len() {
            if bytes[position] == b'"' {
                let mut slashes = 0;
                let mut previous = position;
                while previous > open + 1 && bytes[previous - 1] == b'\\' {
                    slashes += 1;
                    previous -= 1;
                }
                if slashes % 2 == 0 {
                    let mut next = position + 1;
                    while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                        next += 1;
                    }
                    if next < bytes.len() && bytes[next] == b'}' {
                        output.push_str(&input[segment..position + 1]);
                        cursor = position + 1;
                        break;
                    }
                    output.push_str(&input[segment..position]);
                    output.push_str("\\\"");
                    segment = position + 1;
                }
            }
            position += 1;
        }
        if position >= bytes.len() {
            output.push_str(&input[segment..]);
            cursor = input.len();
            break;
        }
    }
    output.push_str(&input[cursor..]);
    output
}

fn repair_raw_control_chars(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut characters = input.chars().peekable();
    while let Some(character) = characters.next() {
        if !in_string {
            output.push(character);
            if character == '"' {
                in_string = true;
            }
            continue;
        }
        if escaped {
            output.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' => {
                let valid_escape = characters.peek().is_some_and(|next| {
                    matches!(next, '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u')
                });
                if valid_escape {
                    output.push(character);
                    escaped = true;
                } else {
                    output.push_str("\\\\");
                }
            }
            '"' => {
                output.push(character);
                in_string = false;
            }
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32))
            }
            _ => output.push(character),
        }
    }
    output
}

pub fn parse_action_block(response: &str) -> AppResult<ParsedAssistantResponse> {
    let canonical = response
        .find("```foldown-actions")
        .map(|start| (start, "```foldown-actions"));
    let generic = response.find("```json").map(|start| (start, "```json"));
    let Some((start, marker)) = canonical.or(generic) else {
        return Ok(ParsedAssistantResponse {
            message: response.trim().to_string(),
            actions: Vec::new(),
        });
    };
    let json_start = start + marker.len();
    if marker == "```json" {
        let payload = response[json_start..]
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();
        let looks_like_actions = payload.contains("\"actions\"")
            || payload.contains("\"action\"")
            || ["create", "replace", "delete"]
                .iter()
                .any(|kind| payload.contains(&format!("\"type\":\"{kind}\"")));
        if !looks_like_actions {
            return Ok(ParsedAssistantResponse {
                message: response.trim().to_string(),
                actions: Vec::new(),
            });
        }
    }
    let end = response[json_start..]
        .find("```")
        .map(|relative_end| json_start + relative_end)
        .unwrap_or(response.len());
    let action_json = response[json_start..end].trim();
    let mut value: serde_json::Value = serde_json::from_str(action_json)
        .or_else(|_| {
            let repaired_quotes = repair_unescaped_content_quotes(action_json);
            serde_json::from_str(&repair_raw_control_chars(&repaired_quotes))
        })
        .map_err(|_| {
            AppError::Message("The model returned malformed Foldown actions".to_string())
        })?;
    let values = if let Some(array) = value.as_array_mut() {
        array
    } else {
        value
            .get_mut("actions")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or_else(|| {
                AppError::Message("The model returned malformed Foldown actions".to_string())
            })?
    };
    for item in values.iter_mut() {
        if let Some(object) = item.as_object_mut() {
            if !object.contains_key("type") {
                if let Some(action) = object.remove("action") {
                    object.insert("type".to_string(), action);
                }
            }
        }
    }
    let actions: Vec<AiAction> = serde_json::from_value(serde_json::Value::Array(values.clone()))
        .map_err(|_| {
        AppError::Message("The model returned malformed Foldown actions".to_string())
    })?;
    let mut targets = HashSet::new();
    for action in &actions {
        validate_relative_markdown_path(action.path())?;
        if action
            .content()
            .is_some_and(|c| c.len() > MAX_ACTION_CONTENT)
        {
            return Err(AppError::Message("An AI action is too large".to_string()));
        }
        if !targets.insert(action.path().to_ascii_lowercase()) {
            return Err(AppError::Message(
                "The model proposed duplicate file actions".to_string(),
            ));
        }
    }
    let trailing = if end < response.len() {
        &response[end + 3..]
    } else {
        ""
    };
    let message = format!("{}{}", &response[..start], trailing)
        .trim()
        .to_string();
    Ok(ParsedAssistantResponse { message, actions })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_prose_from_valid_actions() {
        let text = "I can update it.\n```foldown-actions\n{\"actions\":[{\"type\":\"create\",\"path\":\"new.md\",\"content\":\"# New\"}]}\n```";
        let parsed = parse_action_block(text).unwrap();
        assert_eq!(parsed.message, "I can update it.");
        assert_eq!(parsed.actions.len(), 1);
    }

    #[test]
    fn rejects_traversal_absolute_and_non_markdown_targets() {
        for path in ["../escape.md", "C:/escape.md", "/escape.md", "image.png"] {
            assert!(
                validate_relative_markdown_path(path).is_err(),
                "accepted {path}"
            );
        }
        assert!(validate_relative_markdown_path("notes/safe.md").is_ok());
    }

    #[test]
    fn rejects_duplicate_targets() {
        let text = "```foldown-actions\n{\"actions\":[{\"type\":\"delete\",\"path\":\"a.md\"},{\"type\":\"delete\",\"path\":\"a.md\"}]}\n```";
        assert!(parse_action_block(text).is_err());
    }

    #[test]
    fn accepts_generic_json_array_with_action_key() {
        let text = "```json\n[{\"action\":\"replace\",\"path\":\"Foldown Plan.md\",\"content\":\"## Hello\"}]\n```";
        let parsed = parse_action_block(text).unwrap();
        assert_eq!(parsed.message, "");
        assert_eq!(parsed.actions.len(), 1);
        assert_eq!(parsed.actions[0].kind(), "replace");
    }

    #[test]
    fn leaves_unrelated_json_blocks_in_assistant_prose() {
        let text = "A reviewer checklist can use this example:\n```json\n{\"approved\":true,\"comments\":[]}\n```";
        let parsed = parse_action_block(text).unwrap();
        assert_eq!(parsed.message, text);
        assert!(parsed.actions.is_empty());
    }

    #[test]
    fn repairs_unescaped_quotes_inside_action_content() {
        let text = "Done.\n```json\n{\"actions\":[{\"type\":\"create\",\"path\":\"Code Review.md\",\"content\":\"Explain why (not just *what\") matters.\"}]}\n```";
        let parsed = parse_action_block(text).unwrap();
        assert_eq!(
            parsed.actions[0].content(),
            Some("Explain why (not just *what\") matters.")
        );
    }

    #[test]
    fn repairs_raw_newlines_inside_action_content() {
        let text = r##"```json
{"actions":[{"type":"create","path":"guide.md","content":"# Guide
## Rules
Use a formatter.
"}]}
```"##;
        let parsed = parse_action_block(text).unwrap();
        assert_eq!(
            parsed.actions[0].content(),
            Some("# Guide\n## Rules\nUse a formatter.\n")
        );
    }

    #[test]
    fn accepts_a_complete_action_block_without_a_closing_fence() {
        let text = "Done.\n```json\n{\"actions\":[{\"type\":\"create\",\"path\":\"guide.md\",\"content\":\"# Guide\"}]}";
        let parsed = parse_action_block(text).unwrap();
        assert_eq!(parsed.actions.len(), 1);
        assert_eq!(parsed.message, "Done.");
    }

    #[test]
    fn repairs_invalid_backslash_escapes_inside_action_content() {
        let text = r##"```json
{"actions":[{"type":"create","path":"guide.md","content":"Use the pattern \q+ and C:\Go\src."}]}
```"##;
        let parsed = parse_action_block(text).unwrap();
        assert_eq!(
            parsed.actions[0].content(),
            Some(r"Use the pattern \q+ and C:\Go\src.")
        );
    }
}
