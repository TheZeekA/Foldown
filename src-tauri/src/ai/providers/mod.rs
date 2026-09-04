pub mod anthropic;
pub mod gemini;
pub mod openai;

use std::collections::HashSet;

use crate::ai::operations::{validate_relative_markdown_path, AiAction, MAX_ACTION_CONTENT};
use crate::error::{AppError, AppResult};

pub const ACTION_TOOL_NAME: &str = "propose_file_actions";
pub const ACTION_TOOL_DESCRIPTION: &str =
    "Create, replace, or delete a Markdown file in the active workspace.";

/// The one internal tool definition, expressed once as plain JSON Schema.
/// Each provider module wraps this in its own envelope (OpenAI's
/// `{type:"function", function:{name, description, parameters}}`; Claude's
/// flat `{name, description, input_schema}`; Gemini's
/// `{functionDeclarations:[{name, description, parameters}]}`).
pub fn action_tool_parameters_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "actions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "type": { "type": "string", "enum": ["create", "replace", "delete"] },
                        "path": { "type": "string", "description": "Relative .md path inside the workspace." },
                        "content": { "type": "string", "description": "Complete file content. Required for create/replace, omitted for delete." }
                    },
                    "required": ["type", "path"]
                }
            }
        },
        "required": ["actions"]
    })
}

/// The result of one provider's chat turn: whatever text streamed (already
/// delivered live via `on_delta`, and repeated here for building the final
/// `AiChatResult`), plus any actions the model invoked the
/// `propose_file_actions` tool with.
pub struct ChatOutcome {
    pub text: String,
    pub actions: Vec<AiAction>,
}

/// Shared validation core for every provider's `parse_tool_call_actions`: each
/// provider module is responsible only for turning its own wire format into
/// one `serde_json::Value` shaped like `{"actions": [...]}` (this is exactly
/// what a `propose_file_actions` tool-call's arguments look like once
/// parsed), then hands it here for the same validation
/// `ai::operations::parse_action_block` already applies to Local Server's
/// text-based actions.
pub fn actions_from_tool_arguments(value: serde_json::Value) -> AppResult<Vec<AiAction>> {
    let malformed =
        || AppError::Message("The model returned malformed Foldown actions".to_string());
    let actions_value = value
        .get("actions")
        .and_then(|v| v.as_array())
        .ok_or_else(malformed)?;
    let actions: Vec<AiAction> =
        serde_json::from_value(serde_json::Value::Array(actions_value.clone()))
            .map_err(|_| malformed())?;
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
    Ok(actions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_well_formed_actions_object() {
        let value = serde_json::json!({
            "actions": [{"type": "create", "path": "new.md", "content": "# New"}]
        });
        let actions = actions_from_tool_arguments(value).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].kind(), "create");
    }

    #[test]
    fn rejects_a_missing_actions_key() {
        let value = serde_json::json!({ "notActions": [] });
        assert!(actions_from_tool_arguments(value).is_err());
    }

    #[test]
    fn rejects_a_path_traversal_attempt() {
        let value = serde_json::json!({
            "actions": [{"type": "delete", "path": "../escape.md"}]
        });
        assert!(actions_from_tool_arguments(value).is_err());
    }

    #[test]
    fn rejects_duplicate_targets() {
        let value = serde_json::json!({
            "actions": [
                {"type": "delete", "path": "a.md"},
                {"type": "delete", "path": "a.md"}
            ]
        });
        assert!(actions_from_tool_arguments(value).is_err());
    }

    #[test]
    fn the_shared_schema_matches_the_spec_exactly() {
        let schema = action_tool_parameters_schema();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["required"][0], "actions");
        let item_props = &schema["properties"]["actions"]["items"]["properties"];
        assert_eq!(
            item_props["type"]["enum"],
            serde_json::json!(["create", "replace", "delete"])
        );
    }
}
