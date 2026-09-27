use serde::Deserialize;
use serde_json::{Value, json};

use std::sync::Arc;

use workbench_core::{
    application::workbench_runtime::WorkbenchRuntime,
    domain::agent_exchange::{AgentExchangeDelivery, AgentExchangeError},
};
use workbench_protocol::{
    AuthenticatedPrincipal, CallRequest, OperationId, OperationKind, Workbench, WorkbenchFault,
    operations::spec_for,
};

use crate::infrastructure::mcp::capability_registry::CapabilityPrincipal;

pub const LIST_PEER_AGENTS_TOOL: &str = "list_peer_agents";
pub const SEND_MESSAGE_TO_AGENT_TOOL: &str = "send_message_to_agent";
pub const GET_AGENT_EXCHANGE_STATUS_TOOL: &str = "get_agent_exchange_status";

pub fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": LIST_PEER_AGENTS_TOOL,
            "description": "List other agent-run panels in the same Worktree Session window.",
            "inputSchema": {
                "type": "object",
                "properties": { "runId": { "type": "string" } },
                "required": ["runId"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": SEND_MESSAGE_TO_AGENT_TOOL,
            "description": "Send a scoped text message to another agent-run panel.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "runId": { "type": "string" },
                    "requestId": { "type": "string" },
                    "targetPanelId": { "type": "string" },
                    "targetRunId": { "type": ["string", "null"] },
                    "message": { "type": "string", "maxLength": 16384 },
                    "delivery": { "type": "string", "enum": ["send", "queue", "draft"] }
                },
                "required": ["runId", "requestId", "targetPanelId", "message", "delivery"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": GET_AGENT_EXCHANGE_STATUS_TOOL,
            "description": "Read the delivery status of an exchange created by the current agent run.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "runId": { "type": "string" },
                    "requestId": { "type": "string" }
                },
                "required": ["runId", "requestId"],
                "additionalProperties": false
            }
        }),
    ]
}

pub fn is_exchange_tool(name: &str) -> bool {
    matches!(
        name,
        LIST_PEER_AGENTS_TOOL | SEND_MESSAGE_TO_AGENT_TOOL | GET_AGENT_EXCHANGE_STATUS_TOOL
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunRequest {
    run_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusRequest {
    run_id: String,
    request_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendRequest {
    run_id: String,
    request_id: String,
    target_panel_id: String,
    target_run_id: Option<String>,
    message: String,
    delivery: AgentExchangeDelivery,
}

/// 040(ADR 0006): 교환 도구는 run에 묶인 agent principal로 `Workbench.call`을 거친다. 작업대는 run의 소유로 서버가 찾는다.
async fn call_as_agent(
    runtime: &Arc<WorkbenchRuntime>,
    run_id: &str,
    operation: OperationId,
    input: Value,
    rpc_id: Option<&Value>,
) -> Result<Value, AgentExchangeError> {
    // 교환 요청의 `requestId`(`input.request.requestId`)로 재시도를 식별한다(`retry_identity`).
    let key = matches!(spec_for(operation).kind, OperationKind::Command).then(|| {
        super::retry_identity::tool_idempotency_key(run_id, operation, input.get("request"), rpc_id)
    });
    let mut request = CallRequest::query(operation, input);
    request.idempotency_key = key;
    runtime
        .call(AuthenticatedPrincipal::agent(run_id), request)
        .await
        .map(|reply| reply.output().cloned().unwrap_or(Value::Null))
        .map_err(|fault| exchange_error_of(&fault))
}

/// fault → 오늘 도구 결과의 `{code, message}`. 교환 도메인 코드는 `details.exchangeCode`에 있다.
pub fn exchange_error_of(fault: &WorkbenchFault) -> AgentExchangeError {
    let code = fault
        .details
        .as_ref()
        .and_then(|details| details.get("exchangeCode"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| fault.code.as_str().to_owned());
    AgentExchangeError::new(code, fault.message.clone())
}

pub async fn handle_tool(
    runtime: &Arc<WorkbenchRuntime>,
    principal: &CapabilityPrincipal,
    name: &str,
    arguments: Option<&Value>,
    rpc_id: Option<&Value>,
) -> Value {
    let result = match name {
        LIST_PEER_AGENTS_TOOL => {
            let request: RunRequest = match parse(arguments) {
                Ok(request) => request,
                Err(error) => return tool_error(error),
            };
            if let Err(error) = require_authenticated_run(principal, &request.run_id) {
                return tool_error(error);
            }
            call_as_agent(
                runtime,
                &principal.run_id,
                OperationId::ExchangeListPeers,
                json!({ "runId": principal.run_id }),
                rpc_id,
            )
            .await
        }
        SEND_MESSAGE_TO_AGENT_TOOL => {
            let request: SendRequest = match parse(arguments) {
                Ok(request) => request,
                Err(error) => return tool_error(error),
            };
            if let Err(error) = require_authenticated_run(principal, &request.run_id) {
                return tool_error(error);
            }
            call_as_agent(
                runtime,
                &principal.run_id,
                OperationId::ExchangeSendFromRun,
                json!({
                    "runId": principal.run_id,
                    "request": {
                        "requestId": request.request_id,
                        "sourcePanelId": "",
                        "sourceRunId": principal.run_id,
                        "targetPanelId": request.target_panel_id,
                        "targetRunId": request.target_run_id,
                        "message": request.message,
                        "delivery": request.delivery,
                    },
                }),
                rpc_id,
            )
            .await
        }
        GET_AGENT_EXCHANGE_STATUS_TOOL => {
            let request: StatusRequest = match parse(arguments) {
                Ok(request) => request,
                Err(error) => return tool_error(error),
            };
            if let Err(error) = require_authenticated_run(principal, &request.run_id) {
                return tool_error(error);
            }
            call_as_agent(
                runtime,
                &principal.run_id,
                OperationId::ExchangeGetForRun,
                json!({ "runId": principal.run_id, "requestId": request.request_id }),
                rpc_id,
            )
            .await
        }
        _ => Err(AgentExchangeError::new(
            "unsupportedTool",
            format!("Unsupported MCP tool: {name}"),
        )),
    };

    match result {
        Ok(structured) => tool_success(structured),
        Err(error) => tool_error(error),
    }
}

fn require_authenticated_run(
    principal: &CapabilityPrincipal,
    requested_run_id: &str,
) -> Result<(), AgentExchangeError> {
    if principal.run_id == requested_run_id {
        return Ok(());
    }
    Err(AgentExchangeError::new(
        "forbiddenActor",
        "The requested run does not match the authenticated capability.",
    ))
}

fn parse<T: for<'de> Deserialize<'de>>(arguments: Option<&Value>) -> Result<T, AgentExchangeError> {
    serde_json::from_value(arguments.cloned().unwrap_or(Value::Null)).map_err(|error| {
        AgentExchangeError::new(
            "invalidArguments",
            format!("Invalid tool arguments: {error}"),
        )
    })
}

fn tool_success(structured: Value) -> Value {
    json!({
        "content": [{ "type": "text", "text": structured.to_string() }],
        "structuredContent": structured,
        "isError": false
    })
}

fn tool_error(error: AgentExchangeError) -> Value {
    json!({
        "content": [{ "type": "text", "text": error.message }],
        "structuredContent": error,
        "isError": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fault_details_restore_the_exchange_code() {
        let fault = WorkbenchFault::new(
            workbench_protocol::FaultCode::NotFound,
            workbench_protocol::RequestId::random(),
            "Source agent run is not active.",
        )
        .with_details(json!({ "exchangeCode": "unknownSource" }));
        let error = exchange_error_of(&fault);
        assert_eq!(error.code, "unknownSource");
        assert_eq!(error.message, "Source agent run is not active.");
        let plain = WorkbenchFault::new(
            workbench_protocol::FaultCode::Forbidden,
            workbench_protocol::RequestId::random(),
            "denied",
        );
        assert_eq!(exchange_error_of(&plain).code, "forbidden");
    }

    #[test]
    fn exposes_peer_send_and_status_tool_schemas() {
        let tools = tool_definitions();
        assert_eq!(tools.len(), 3);
        assert_eq!(tools[0]["name"], LIST_PEER_AGENTS_TOOL);
        assert_eq!(tools[1]["name"], SEND_MESSAGE_TO_AGENT_TOOL);
        assert_eq!(tools[2]["name"], GET_AGENT_EXCHANGE_STATUS_TOOL);
        assert_eq!(
            tools[1]["inputSchema"]["properties"]["delivery"]["enum"],
            json!(["send", "queue", "draft"])
        );
    }
}
