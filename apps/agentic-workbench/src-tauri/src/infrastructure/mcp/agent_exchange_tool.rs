use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    application::agent_exchange_service::AgentExchangeService,
    domain::agent_exchange::{AgentExchangeDelivery, AgentExchangeError, SendAgentExchangeRequest},
    infrastructure::{
        agent_session_registry::AppState,
        in_memory_agent_workspace_registry::{
            InMemoryAgentWorkspaceRegistry, TauriAgentExchangeEventSink,
        },
        mcp::capability_registry::CapabilityPrincipal,
    },
};

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

type ExchangeService =
    AgentExchangeService<InMemoryAgentWorkspaceRegistry, AppState, TauriAgentExchangeEventSink>;

pub async fn handle_tool(
    service: &ExchangeService,
    principal: &CapabilityPrincipal,
    name: &str,
    arguments: Option<&Value>,
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
            service
                .list_peers_for_run(&principal.run_id)
                .await
                .map(|peers| json!({ "peers": peers }))
        }
        SEND_MESSAGE_TO_AGENT_TOOL => {
            let request: SendRequest = match parse(arguments) {
                Ok(request) => request,
                Err(error) => return tool_error(error),
            };
            if let Err(error) = require_authenticated_run(principal, &request.run_id) {
                return tool_error(error);
            }
            service
                .send_agent_exchange(
                    &principal.run_id,
                    SendAgentExchangeRequest {
                        request_id: request.request_id,
                        source_panel_id: String::new(),
                        source_run_id: Some(principal.run_id.clone()),
                        target_panel_id: request.target_panel_id,
                        target_run_id: request.target_run_id,
                        message: request.message,
                        delivery: request.delivery,
                    },
                )
                .await
                .map(|exchange| serde_json::to_value(exchange).unwrap_or(Value::Null))
        }
        GET_AGENT_EXCHANGE_STATUS_TOOL => {
            let request: StatusRequest = match parse(arguments) {
                Ok(request) => request,
                Err(error) => return tool_error(error),
            };
            if let Err(error) = require_authenticated_run(principal, &request.run_id) {
                return tool_error(error);
            }
            service
                .exchange_for_source_run(&principal.run_id, &request.request_id)
                .await
                .map(|exchange| serde_json::to_value(exchange).unwrap_or(Value::Null))
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
