use std::net::Ipv4Addr;

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::TcpListener;
use uuid::Uuid;

use crate::{
    application::mcp_title_control_service::McpTitleControlService,
    domain::{
        mcp_title_control::{TitleChangeFailureCode, TitleChangeResult},
        run::{AgentMcpHttpHeader, AgentMcpServerConfig},
    },
    infrastructure::{
        agent_session_registry::AppState,
        mcp::{
            protocol::{JsonRpcResponse, initialize_result, parse_request},
            title_tool::{
                SET_WINDOW_TITLE_TOOL, is_authorized, origin_allowed, parse_title_change_request,
                tool_result, tools_list_result, unsupported_tool_result,
            },
        },
    },
};

pub mod protocol;
pub mod title_tool;

pub const AW_MCP_URL_ENV: &str = "AW_MCP_URL";
pub const AW_MCP_TOKEN_ENV: &str = "AW_MCP_TOKEN";
pub const AW_MCP_RUN_ID_ENV: &str = "AW_MCP_RUN_ID";
pub const MCP_WINDOW_TITLE_EVENT: &str = "workspace://mcp-window-title";
pub const MCP_WINDOW_TITLE_FALLBACK_EVENT: &str = "mcp-window-title-fallback";
pub const AW_MCP_SERVER_NAME: &str = "agentic_workbench";

#[derive(Clone)]
pub struct McpServerState {
    base_url: String,
    token: String,
}

#[derive(Clone)]
struct McpRouterState {
    app: AppHandle,
    registry: AppState,
    token: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowTitleEvent {
    pub title: String,
}

#[derive(Clone, Debug)]
pub struct McpLaunchEnv {
    pub url: String,
    pub token: String,
    pub run_id: String,
}

impl McpLaunchEnv {
    pub fn server_config(&self) -> AgentMcpServerConfig {
        AgentMcpServerConfig::Http {
            name: AW_MCP_SERVER_NAME.to_string(),
            url: self.url.clone(),
            headers: vec![AgentMcpHttpHeader {
                name: "Authorization".to_string(),
                value: format!("Bearer {}", self.token),
            }],
        }
    }

    pub fn agent_instructions(&self) -> String {
        format!(
            r#"## Agentic Workbench MCP tools

You are running inside an Agentic Workbench Worktree Session.
The local MCP server named `{AW_MCP_SERVER_NAME}` is available for controlling this session UI.

Available tool:
- `set_window_title`: change only the current Worktree Session window title.

When the user asks to change, label, rename, or summarize the current session/window title, call `set_window_title` with:
- `runId`: `{run_id}`
- `title`: a readable title, 80 characters or fewer, without control characters.

Do not use this MCP server for file edits, Git operations, permission approval, or reading source files. If the title tool fails, report the failure instead of claiming the title changed.
"#,
            run_id = self.run_id
        )
    }
}

impl McpServerState {
    pub fn start(app: AppHandle, registry: AppState) -> Result<Self> {
        let token = Uuid::new_v4().to_string();
        let std_listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .context("failed to bind MCP server to localhost")?;
        std_listener
            .set_nonblocking(true)
            .context("failed to configure MCP listener")?;
        let address = std_listener
            .local_addr()
            .context("failed to read MCP server address")?;
        let router_state = McpRouterState {
            app,
            registry,
            token: token.clone(),
        };
        let router = Router::new()
            .route("/mcp", post(handle_post).get(handle_get))
            .with_state(router_state);

        tauri::async_runtime::spawn(async move {
            let listener = match TcpListener::from_std(std_listener) {
                Ok(listener) => listener,
                Err(error) => {
                    eprintln!("failed to create Tokio MCP listener: {error}");
                    return;
                }
            };

            if let Err(error) = axum::serve(listener, router).await {
                eprintln!("MCP server stopped: {error}");
            }
        });

        Ok(Self {
            base_url: format!("http://{address}/mcp"),
            token,
        })
    }

    pub fn launch_env(&self, run_id: &str) -> McpLaunchEnv {
        McpLaunchEnv {
            url: self.base_url.clone(),
            token: self.token.clone(),
            run_id: run_id.to_string(),
        }
    }
}

async fn handle_get() -> Response {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        Json(json!({
            "error": "MCP streaming GET is not supported for title-control-only service"
        })),
    )
        .into_response()
}

async fn handle_post(
    State(state): State<McpRouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    if !origin_allowed(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(JsonRpcResponse::error(None, -32000, "Forbidden origin")),
        )
            .into_response();
    }

    let request = match parse_request(payload) {
        Ok(request) => request,
        Err(response) => return (StatusCode::BAD_REQUEST, Json(response)).into_response(),
    };
    let id = request.id.clone();

    if request.method != "initialize" && !is_authorized(&headers, &state.token) {
        let result = tool_result(TitleChangeResult::failure(
            TitleChangeFailureCode::Unauthorized,
            "MCP request is unauthorized.",
        ));
        return (
            StatusCode::UNAUTHORIZED,
            Json(JsonRpcResponse::result(id, result)),
        )
            .into_response();
    }

    let response = match request.method.as_str() {
        "initialize" => JsonRpcResponse::result(id, initialize_result()),
        "tools/list" => JsonRpcResponse::result(id, tools_list_result()),
        "tools/call" => {
            let result = handle_tool_call(&state, request.params).await;
            JsonRpcResponse::result(id, result)
        }
        method => JsonRpcResponse::error(id, -32601, format!("Unsupported MCP method: {method}")),
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn handle_tool_call(state: &McpRouterState, params: Option<Value>) -> Value {
    let name = params
        .as_ref()
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if name != SET_WINDOW_TITLE_TOOL {
        return unsupported_tool_result(name);
    }

    let arguments = params.as_ref().and_then(|value| value.get("arguments"));
    let request = match parse_title_change_request(arguments) {
        Ok(request) => request,
        Err(result) => return tool_result(result),
    };
    let service = McpTitleControlService::new(state.registry.clone());
    let command = match service.build_command(request).await {
        Ok(command) => command,
        Err(result) => return tool_result(result),
    };
    let Some(window) = state.app.get_webview_window(&command.window_label) else {
        return tool_result(TitleChangeResult::failure(
            TitleChangeFailureCode::WindowUnavailable,
            "Owner Worktree Session window is unavailable.",
        ));
    };

    let payload = WindowTitleEvent {
        title: command.title.clone(),
    };
    if let Err(error) = window.set_title(&command.title) {
        return tool_result(TitleChangeResult::failure(
            TitleChangeFailureCode::WindowUnavailable,
            format!("Owner Worktree Session window title could not be changed: {error}"),
        ));
    }
    let _ = window.emit(MCP_WINDOW_TITLE_EVENT, &payload);
    if let Ok(serialized) = serde_json::to_string(&payload) {
        let script = format!(
            "window.dispatchEvent(new CustomEvent('{MCP_WINDOW_TITLE_FALLBACK_EVENT}', {{ detail: {serialized} }}));"
        );
        let _ = window.eval(&script);
    }

    tool_result(TitleChangeResult::success(command.title))
}

#[cfg(test)]
mod tests {
    use super::{
        AW_MCP_RUN_ID_ENV, AW_MCP_SERVER_NAME, AW_MCP_TOKEN_ENV, AW_MCP_URL_ENV, McpServerState,
    };
    use crate::domain::run::{AgentMcpHttpHeader, AgentMcpServerConfig};

    #[test]
    fn launch_env_uses_app_reserved_keys() {
        assert_eq!(AW_MCP_URL_ENV, "AW_MCP_URL");
        assert_eq!(AW_MCP_TOKEN_ENV, "AW_MCP_TOKEN");
        assert_eq!(AW_MCP_RUN_ID_ENV, "AW_MCP_RUN_ID");
    }

    #[test]
    fn launch_env_carries_run_id() {
        let state = McpServerState {
            base_url: "http://127.0.0.1:1/mcp".into(),
            token: "token".into(),
        };
        let env = state.launch_env("run-1");
        assert_eq!(env.url, "http://127.0.0.1:1/mcp");
        assert_eq!(env.token, "token");
        assert_eq!(env.run_id, "run-1");
    }

    #[test]
    fn launch_env_builds_http_mcp_server_config() {
        let state = McpServerState {
            base_url: "http://127.0.0.1:1/mcp".into(),
            token: "token".into(),
        };

        let config = state.launch_env("run-1").server_config();

        assert_eq!(
            config,
            AgentMcpServerConfig::Http {
                name: AW_MCP_SERVER_NAME.to_string(),
                url: "http://127.0.0.1:1/mcp".to_string(),
                headers: vec![AgentMcpHttpHeader {
                    name: "Authorization".to_string(),
                    value: "Bearer token".to_string()
                }]
            }
        );
    }

    #[test]
    fn launch_env_builds_agent_instructions() {
        let state = McpServerState {
            base_url: "http://127.0.0.1:1/mcp".into(),
            token: "token".into(),
        };

        let instructions = state.launch_env("run-1").agent_instructions();

        assert!(instructions.contains(AW_MCP_SERVER_NAME));
        assert!(instructions.contains("set_window_title"));
        assert!(instructions.contains("runId`: `run-1`"));
        assert!(instructions.contains("If the title tool fails"));
    }
}
