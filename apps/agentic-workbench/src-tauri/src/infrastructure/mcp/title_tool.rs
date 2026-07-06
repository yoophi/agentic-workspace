use axum::http::HeaderMap;
use serde_json::{Value, json};

use crate::domain::agent_tool_candidate::{
    AgentToolCandidate, AgentToolCandidateScope, AgentToolCandidateSource,
};
use crate::domain::mcp_title_control::{
    TitleChangeFailureCode, TitleChangeRequest, TitleChangeResult,
};

pub const SET_WINDOW_TITLE_TOOL: &str = "set_window_title";

pub fn tools_list_result() -> Value {
    json!({
        "tools": [
            {
                "name": SET_WINDOW_TITLE_TOOL,
                "description": "Change the current Worktree Session window title for the active agent run.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "runId": {
                            "type": "string",
                            "description": "The active agent run id provided by AW_MCP_RUN_ID."
                        },
                        "title": {
                            "type": "string",
                            "description": "Readable window title to apply to the owning Worktree Session window."
                        }
                    },
                    "required": ["runId", "title"],
                    "additionalProperties": false
                }
            }
        ]
    })
}

pub fn tool_command_candidates(
    run_id: Option<&str>,
    agent_id: &str,
    working_directory: &str,
) -> Vec<AgentToolCandidate> {
    vec![AgentToolCandidate {
        id: format!("session:{SET_WINDOW_TITLE_TOOL}"),
        name: SET_WINDOW_TITLE_TOOL.to_string(),
        description: Some(
            "Change the current Worktree Session window title for the active agent run."
                .to_string(),
        ),
        insert_text: format!("${SET_WINDOW_TITLE_TOOL}"),
        source: AgentToolCandidateSource::SessionTool,
        scope: AgentToolCandidateScope {
            run_id: run_id.map(str::to_string),
            agent_id: Some(agent_id.to_string()),
            working_directory: Some(working_directory.to_string()),
        },
    }]
}

pub fn parse_title_change_request(
    arguments: Option<&Value>,
) -> Result<TitleChangeRequest, TitleChangeResult> {
    let Some(arguments) = arguments else {
        return Err(TitleChangeResult::failure(
            TitleChangeFailureCode::InvalidTitle,
            "Tool arguments are required.",
        ));
    };
    serde_json::from_value::<TitleChangeRequest>(arguments.clone()).map_err(|error| {
        TitleChangeResult::failure(
            TitleChangeFailureCode::InvalidTitle,
            format!("Invalid set_window_title arguments: {error}"),
        )
    })
}

pub fn tool_result(result: TitleChangeResult) -> Value {
    let text = if result.ok {
        format!(
            "Window title changed to {}.",
            result.applied_title.as_deref().unwrap_or_default()
        )
    } else {
        result
            .reason
            .clone()
            .unwrap_or_else(|| "Window title was not changed.".to_string())
    };
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "structuredContent": result,
        "isError": !result.ok
    })
}

pub fn unsupported_tool_result(name: &str) -> Value {
    tool_result(TitleChangeResult::failure(
        TitleChangeFailureCode::UnsupportedTool,
        format!("Unsupported MCP tool: {name}"),
    ))
}

pub fn is_authorized(headers: &HeaderMap, expected_token: &str) -> bool {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| !token.is_empty() && token == expected_token)
}

pub fn origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get("origin").and_then(|value| value.to_str().ok()) else {
        return true;
    };
    origin == "tauri://localhost"
        || origin.starts_with("http://127.0.0.1")
        || origin.starts_with("http://localhost")
}

#[cfg(test)]
mod tests {
    use super::{
        SET_WINDOW_TITLE_TOOL, is_authorized, origin_allowed, parse_title_change_request,
        tool_command_candidates, tools_list_result, unsupported_tool_result,
    };
    use axum::http::{HeaderMap, HeaderValue};
    use serde_json::json;

    #[test]
    fn tools_list_exposes_only_set_window_title() {
        let result = tools_list_result();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], SET_WINDOW_TITLE_TOOL);
    }

    #[test]
    fn tool_command_candidates_expose_prompt_insert_token() {
        let candidates = tool_command_candidates(Some("run-1"), "codex", "/repo");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, SET_WINDOW_TITLE_TOOL);
        assert_eq!(candidates[0].insert_text, "$set_window_title");
        assert_eq!(candidates[0].scope.run_id.as_deref(), Some("run-1"));
    }

    #[test]
    fn unsupported_tool_returns_error_payload() {
        let result = unsupported_tool_result("read_file");
        assert_eq!(result["isError"], true);
        assert_eq!(
            result["structuredContent"]["code"],
            json!("unsupportedTool")
        );
    }

    #[test]
    fn parses_title_change_arguments() {
        let request = parse_title_change_request(Some(&json!({
            "runId": "run-1",
            "title": "New title"
        })))
        .unwrap();
        assert_eq!(request.run_id, "run-1");
        assert_eq!(request.title, "New title");
    }

    #[test]
    fn bearer_token_is_required() {
        let mut headers = HeaderMap::new();
        assert!(!is_authorized(&headers, "secret"));
        headers.insert("authorization", HeaderValue::from_static("Bearer secret"));
        assert!(is_authorized(&headers, "secret"));
        assert!(!is_authorized(&headers, "other"));
    }

    #[test]
    fn origin_validation_rejects_untrusted_browser_origin() {
        let mut headers = HeaderMap::new();
        assert!(origin_allowed(&headers));
        headers.insert("origin", HeaderValue::from_static("https://example.com"));
        assert!(!origin_allowed(&headers));
        headers.insert("origin", HeaderValue::from_static("http://127.0.0.1:1420"));
        assert!(origin_allowed(&headers));
    }
}
