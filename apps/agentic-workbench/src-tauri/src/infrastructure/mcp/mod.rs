use std::net::Ipv4Addr;

use crate::{
    domain::{
        mcp_title_control::{TitleChangeFailureCode, TitleChangeResult},
        run::{AgentMcpHttpHeader, AgentMcpServerConfig},
    },
    infrastructure::mcp::{
        agent_exchange_tool::{handle_tool as handle_exchange_tool, is_exchange_tool},
        capability_registry::{CapabilityPrincipal, CapabilityRegistry},
        orchestration_tool::{
            agent_role, handle_tool as handle_orchestration_tool, is_orchestration_tool,
            tool_definitions as orchestration_tool_definitions,
        },
        protocol::{JsonRpcResponse, initialize_result, parse_request},
        title_tool::{
            SET_WINDOW_TITLE_TOOL, origin_allowed, parse_title_change_request, tool_result,
            tools_list_result, unsupported_tool_result,
        },
    },
};
use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};
use tokio::net::TcpListener;

pub mod agent_exchange_tool;
pub mod capability_registry;
pub mod orchestration_tool;
pub mod protocol;
pub mod title_tool;

pub const AW_MCP_URL_ENV: &str = "AW_MCP_URL";
pub const AW_MCP_TOKEN_ENV: &str = "AW_MCP_TOKEN";
pub const AW_MCP_RUN_ID_ENV: &str = "AW_MCP_RUN_ID";
pub const AW_MCP_SERVER_NAME: &str = "agentic_workbench";

#[derive(Clone)]
pub struct McpServerState {
    base_url: String,
    capability_registry: CapabilityRegistry,
    /// 042 R17: 받아들인 도구 호출. 연결과 무관하게 끝까지 실행되고, 앱 종료는 이것이 비기를 기다린다.
    calls: std::sync::Arc<workbench_server::drain::DetachedCalls>,
}

#[derive(Clone)]
struct McpRouterState {
    app: AppHandle,
    mcp_state: McpServerState,
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

Available tools:
- `set_window_title`: change only the current Worktree Session window title.
- `list_peer_agents`: list other agent-run panels in this same session window.
- `send_message_to_agent`: send a scoped `send`, `queue`, or `draft` message to a listed peer.
- `get_agent_exchange_status`: inspect the final delivery status of a sent message.
- Main Coordinator runs can create, monitor, wait for, and collect direct Child tasks with `aw_*_child_*` tools.
- Child runs can inspect their own task and report progress, input requests, blocked state, and the final result with `aw_report_*` tools.

When the user asks to change, label, rename, or summarize the current session/window title, call `set_window_title` with:
- `runId`: `{run_id}`
- `title`: a readable title, 80 characters or fewer, without control characters.

For agent-to-agent messages, first call `list_peer_agents`, use the returned stable panel/run ids, generate a unique request id, and report the returned delivery status accurately.

Do not use this MCP server for file edits, Git operations, permission approval, or reading source files. If the title tool fails, report the failure instead of claiming the title changed. Apply the same rule to the agent exchange tools.
"#,
            run_id = self.run_id
        )
    }
}

impl McpServerState {
    pub fn start(app: AppHandle) -> Result<Self> {
        let std_listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .context("failed to bind MCP server to localhost")?;
        std_listener
            .set_nonblocking(true)
            .context("failed to configure MCP listener")?;
        let address = std_listener
            .local_addr()
            .context("failed to read MCP server address")?;
        let server_state = Self {
            base_url: format!("http://{address}/mcp"),
            capability_registry: CapabilityRegistry::default(),
            calls: std::sync::Arc::default(),
        };
        let router_state = McpRouterState {
            app,
            mcp_state: server_state.clone(),
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

        Ok(server_state)
    }

    /// run에 묶인 MCP 토큰을 만든다(041: 역할 주장 없음).
    pub fn launch_env(&self, run_id: &str) -> McpLaunchEnv {
        McpLaunchEnv {
            url: self.base_url.clone(),
            token: self.capability_registry.issue(run_id),
            run_id: run_id.to_owned(),
        }
    }

    /// 받아들인 도구 호출 추적기(앱 종료 drain용).
    pub fn detached_calls(&self) -> std::sync::Arc<workbench_server::drain::DetachedCalls> {
        std::sync::Arc::clone(&self.calls)
    }

    pub fn revoke_run_capability(&self, run_id: &str) {
        self.capability_registry.revoke_run(run_id);
    }

    fn resolve_capability(&self, token: &str) -> Option<CapabilityPrincipal> {
        self.capability_registry.resolve(token)
    }
}

fn workbench_runtime(
    app: &AppHandle,
) -> std::sync::Arc<workbench_core::application::workbench_runtime::WorkbenchRuntime> {
    app.state::<std::sync::Arc<workbench_core::application::workbench_runtime::WorkbenchRuntime>>()
        .inner()
        .clone()
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
        Err(response) => return (StatusCode::BAD_REQUEST, Json(*response)).into_response(),
    };
    let id = request.id.clone();

    let principal = if request.method == "initialize" {
        None
    } else {
        let Some(token) = bearer_token(&headers) else {
            let result = tool_result(TitleChangeResult::failure(
                TitleChangeFailureCode::Unauthorized,
                "MCP request is unauthorized.",
            ));
            return (
                StatusCode::UNAUTHORIZED,
                Json(JsonRpcResponse::result(id, result)),
            )
                .into_response();
        };
        match state.mcp_state.resolve_capability(token) {
            Some(principal) => Some(principal),
            None => {
                let result = tool_result(TitleChangeResult::failure(
                    TitleChangeFailureCode::Unauthorized,
                    "MCP capability is invalid or expired.",
                ));
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(JsonRpcResponse::result(id, result)),
                )
                    .into_response();
            }
        }
    };

    let response = match request.method.as_str() {
        "initialize" => JsonRpcResponse::result(id, initialize_result()),
        "tools/list" => {
            let mut result = tools_list_result();
            // 041: 도구 목록은 요청 시점의 서버 역할에서 고른다(research R7).
            let role = agent_role(
                &workbench_runtime(&state.app),
                principal.as_ref().expect("authenticated tools list"),
            )
            .await;
            if let Some(tools) = result.get_mut("tools").and_then(Value::as_array_mut) {
                tools.extend(orchestration_tool_definitions(role));
            }
            JsonRpcResponse::result(id, result)
        }
        "tools/call" => {
            // 042 R17: 도구 호출은 서버 소유 task에서 실행한다 — agent 연결이 끊겨도 효과와 멱등 기록은 끝난다.
            let Some(guard) = state.mcp_state.calls.accept() else {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(JsonRpcResponse::error(
                        id,
                        -32000,
                        workbench_server::drain::MESSAGE_SHUTTING_DOWN,
                    )),
                )
                    .into_response();
            };
            let principal = principal.expect("authenticated tool call");
            let task_state = state.clone();
            match workbench_server::drain::spawn_accepted(guard, async move {
                handle_tool_call(&task_state, &principal, request.params).await
            })
            .await
            {
                Ok(result) => JsonRpcResponse::result(id, result),
                Err(_) => JsonRpcResponse::error(id, -32603, "tool call execution failed"),
            }
        }
        method => JsonRpcResponse::error(id, -32601, format!("Unsupported MCP method: {method}")),
    };

    (StatusCode::OK, Json(response)).into_response()
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
}

async fn handle_tool_call(
    state: &McpRouterState,
    principal: &CapabilityPrincipal,
    params: Option<Value>,
) -> Value {
    let name = params
        .as_ref()
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if is_orchestration_tool(name) {
        let arguments = params.as_ref().and_then(|value| value.get("arguments"));
        return handle_orchestration_tool(
            &workbench_runtime(&state.app),
            principal,
            name,
            arguments,
        )
        .await;
    }
    if is_exchange_tool(name) {
        let arguments = params.as_ref().and_then(|value| value.get("arguments"));
        return handle_exchange_tool(&workbench_runtime(&state.app), principal, name, arguments)
            .await;
    }
    if name != SET_WINDOW_TITLE_TOOL {
        return unsupported_tool_result(name);
    }

    let arguments = params.as_ref().and_then(|value| value.get("arguments"));
    let request = match parse_title_change_request(arguments) {
        Ok(request) => request,
        Err(result) => return tool_result(result),
    };
    if request.run_id != principal.run_id {
        return tool_result(TitleChangeResult::failure(
            TitleChangeFailureCode::Unauthorized,
            "The requested run does not match the authenticated capability.",
        ));
    }
    // 040 US3(ADR 0006·0007): 제목 요청은 agent principal로 `bench.requestTitle`을 부른다. 서버는 작업대 알림
    // 스트림에 발행하고, 데스크톱 bridge가 그 작업대의 창에 적용한다(네이티브 방송 없음).
    let runtime = workbench_runtime(&state.app);
    let mut call = workbench_protocol::CallRequest::query(
        workbench_protocol::OperationId::BenchRequestTitle,
        serde_json::json!({ "runId": request.run_id, "title": request.title }),
    );
    call.idempotency_key = Some(workbench_protocol::IdempotencyKey::random());
    use workbench_protocol::Workbench as _;
    match runtime
        .call(
            workbench_protocol::AuthenticatedPrincipal::agent(&principal.run_id),
            call,
        )
        .await
    {
        Ok(reply) => {
            let applied = reply
                .output()
                .and_then(|output| output.get("appliedTitle"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            tool_result(TitleChangeResult::success(applied))
        }
        Err(fault) => tool_result(TitleChangeResult::failure(
            title_failure_code(&fault),
            fault.message.clone(),
        )),
    }
}

/// fault의 `details.titleCode` → 오늘 `TitleChangeFailureCode`.
fn title_failure_code(fault: &workbench_protocol::WorkbenchFault) -> TitleChangeFailureCode {
    match fault
        .details
        .as_ref()
        .and_then(|details| details.get("titleCode"))
        .and_then(serde_json::Value::as_str)
    {
        Some("invalidTitle") => TitleChangeFailureCode::InvalidTitle,
        Some("unknownRun") => TitleChangeFailureCode::UnknownRun,
        Some("unauthorized") => TitleChangeFailureCode::Unauthorized,
        _ => TitleChangeFailureCode::InternalError,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AW_MCP_RUN_ID_ENV, AW_MCP_SERVER_NAME, AW_MCP_TOKEN_ENV, AW_MCP_URL_ENV, McpServerState,
        bearer_token,
    };
    use crate::domain::run::{AgentMcpHttpHeader, AgentMcpServerConfig};
    use crate::infrastructure::mcp::capability_registry::CapabilityRegistry;
    use axum::http::{HeaderMap, HeaderValue};

    fn test_state() -> McpServerState {
        McpServerState {
            base_url: "http://127.0.0.1:1/mcp".into(),
            capability_registry: CapabilityRegistry::default(),
            calls: std::sync::Arc::default(),
        }
    }

    #[test]
    fn launch_env_uses_app_reserved_keys() {
        assert_eq!(AW_MCP_URL_ENV, "AW_MCP_URL");
        assert_eq!(AW_MCP_TOKEN_ENV, "AW_MCP_TOKEN");
        assert_eq!(AW_MCP_RUN_ID_ENV, "AW_MCP_RUN_ID");
    }

    #[test]
    fn bearer_token_is_required() {
        let mut headers = HeaderMap::new();
        assert_eq!(bearer_token(&headers), None);
        headers.insert("authorization", HeaderValue::from_static("Bearer secret"));
        assert_eq!(bearer_token(&headers), Some("secret"));
    }

    #[test]
    fn launch_env_carries_run_id() {
        let state = test_state();
        let env = state.launch_env("run-1");
        assert_eq!(env.url, "http://127.0.0.1:1/mcp");
        assert!(env.token.starts_with("awcap_"));
        assert_eq!(env.run_id, "run-1");
    }

    #[test]
    fn launch_env_builds_http_mcp_server_config() {
        let state = test_state();
        let env = state.launch_env("run-1");
        let token = env.token.clone();
        let config = env.server_config();

        assert_eq!(
            config,
            AgentMcpServerConfig::Http {
                name: AW_MCP_SERVER_NAME.to_string(),
                url: "http://127.0.0.1:1/mcp".to_string(),
                headers: vec![AgentMcpHttpHeader {
                    name: "Authorization".to_string(),
                    value: format!("Bearer {token}")
                }]
            }
        );
    }

    #[test]
    fn launch_env_builds_agent_instructions() {
        let state = test_state();

        let instructions = state.launch_env("run-1").agent_instructions();

        assert!(instructions.contains(AW_MCP_SERVER_NAME));
        assert!(instructions.contains("set_window_title"));
        assert!(instructions.contains("list_peer_agents"));
        assert!(instructions.contains("send_message_to_agent"));
        assert!(instructions.contains("runId`: `run-1`"));
        assert!(instructions.contains("If the title tool fails"));
    }
}
