use axum::http::HeaderMap;
use serde_json::{Value, json};

use crate::domain::mcp_title_control::{
    TitleChangeFailureCode, TitleChangeRequest, TitleChangeResult,
};

pub const SET_WINDOW_TITLE_TOOL: &str = "set_window_title";

pub fn tools_list_result() -> Value {
    let mut tools = vec![json!({
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
    })];
    tools.extend(crate::infrastructure::mcp::agent_exchange_tool::tool_definitions());
    json!({ "tools": tools })
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
        SET_WINDOW_TITLE_TOOL, origin_allowed, parse_title_change_request, tools_list_result,
        unsupported_tool_result,
    };
    use axum::http::{HeaderMap, HeaderValue};
    use serde_json::json;

    #[test]
    fn tools_list_exposes_title_and_agent_exchange_tools() {
        let result = tools_list_result();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
        assert_eq!(tools[0]["name"], SET_WINDOW_TITLE_TOOL);
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
    fn origin_validation_rejects_untrusted_browser_origin() {
        let mut headers = HeaderMap::new();
        assert!(origin_allowed(&headers));
        headers.insert("origin", HeaderValue::from_static("https://example.com"));
        assert!(!origin_allowed(&headers));
        headers.insert("origin", HeaderValue::from_static("http://127.0.0.1:1420"));
        assert!(origin_allowed(&headers));
    }
}
