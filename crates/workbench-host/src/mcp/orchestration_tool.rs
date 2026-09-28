//! Coordinator·자식 orchestration MCP 도구(041 US2). 도구 16개는 토큰이 인증한 run의 agent principal로 대응
//! `orchestration.*` agent operation을 부른다 — 역할 판정·상태 변경·대기는 서버에 있다(research R7·R12). 결과와
//! 오류는 오늘 도구 결과 형태(`structuredContent`)로 되돌린다.

use std::sync::Arc;

use serde_json::{Value, json};
use workbench_core::application::workbench_runtime::WorkbenchRuntime;
use workbench_protocol::{AuthenticatedPrincipal, CallRequest, OperationId, Workbench};

use crate::mcp::capability_registry::CapabilityPrincipal;

pub const CREATE_CHILD_TASK_TOOL: &str = "aw_create_child_task";
pub const ASSIGN_CHILD_TASK_TOOL: &str = "aw_assign_child_task";
pub const LIST_CHILD_TASKS_TOOL: &str = "aw_list_child_tasks";
pub const SEND_CHILD_MESSAGE_TOOL: &str = "aw_send_child_message";
pub const WAIT_CHILD_TASKS_TOOL: &str = "aw_wait_child_tasks";
pub const COLLECT_CHILD_RESULTS_TOOL: &str = "aw_collect_child_results";
pub const INTERRUPT_CHILD_TASK_TOOL: &str = "aw_interrupt_child_task";
pub const CANCEL_CHILD_TASK_TOOL: &str = "aw_cancel_child_task";
pub const RETRY_CHILD_TASK_TOOL: &str = "aw_retry_child_task";
pub const REASSIGN_CHILD_TASK_TOOL: &str = "aw_reassign_child_task";
pub const GET_OWN_TASK_TOOL: &str = "aw_get_own_task";
pub const REPORT_PROGRESS_TOOL: &str = "aw_report_progress";
pub const REPORT_RESULT_TOOL: &str = "aw_report_result";
pub const REQUEST_PARENT_INPUT_TOOL: &str = "aw_request_parent_input";
pub const REPORT_BLOCKED_TOOL: &str = "aw_report_blocked";
pub const SEND_PARENT_MESSAGE_TOOL: &str = "aw_send_parent_message";

const COORDINATOR_TOOLS: &[&str] = &[
    CREATE_CHILD_TASK_TOOL,
    ASSIGN_CHILD_TASK_TOOL,
    LIST_CHILD_TASKS_TOOL,
    SEND_CHILD_MESSAGE_TOOL,
    WAIT_CHILD_TASKS_TOOL,
    COLLECT_CHILD_RESULTS_TOOL,
    INTERRUPT_CHILD_TASK_TOOL,
    CANCEL_CHILD_TASK_TOOL,
    RETRY_CHILD_TASK_TOOL,
    REASSIGN_CHILD_TASK_TOOL,
];

const CHILD_TOOLS: &[&str] = &[
    GET_OWN_TASK_TOOL,
    REPORT_PROGRESS_TOOL,
    REPORT_RESULT_TOOL,
    REQUEST_PARENT_INPUT_TOOL,
    REPORT_BLOCKED_TOOL,
    SEND_PARENT_MESSAGE_TOOL,
];

/// 서버가 판정한 역할(`orchestration.getAgentRole`의 `role`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentToolRole {
    Coordinator,
    Child,
}

pub fn is_orchestration_tool(name: &str) -> bool {
    COORDINATOR_TOOLS.contains(&name) || CHILD_TOOLS.contains(&name)
}

/// 요청 시점 역할의 도구 목록. 역할이 없으면(오늘 `LegacyRun`) 없다.
pub fn tool_definitions(role: Option<AgentToolRole>) -> Vec<Value> {
    let names = match role {
        Some(AgentToolRole::Coordinator) => COORDINATOR_TOOLS,
        Some(AgentToolRole::Child) => CHILD_TOOLS,
        None => return Vec::new(),
    };
    names.iter().map(|name| tool_definition(name)).collect()
}

fn operation_for(name: &str) -> Option<OperationId> {
    Some(match name {
        CREATE_CHILD_TASK_TOOL => OperationId::OrchestrationCreateChildTask,
        ASSIGN_CHILD_TASK_TOOL => OperationId::OrchestrationAssignChildTask,
        LIST_CHILD_TASKS_TOOL => OperationId::OrchestrationListChildTasks,
        SEND_CHILD_MESSAGE_TOOL => OperationId::OrchestrationSendChildMessage,
        WAIT_CHILD_TASKS_TOOL => OperationId::OrchestrationWaitChildTasks,
        COLLECT_CHILD_RESULTS_TOOL => OperationId::OrchestrationCollectChildResults,
        INTERRUPT_CHILD_TASK_TOOL => OperationId::OrchestrationInterruptChildTask,
        CANCEL_CHILD_TASK_TOOL => OperationId::OrchestrationCancelChildTask,
        RETRY_CHILD_TASK_TOOL => OperationId::OrchestrationRetryChildTask,
        REASSIGN_CHILD_TASK_TOOL => OperationId::OrchestrationReassignChildTask,
        GET_OWN_TASK_TOOL => OperationId::OrchestrationGetOwnTask,
        REPORT_PROGRESS_TOOL => OperationId::OrchestrationReportProgress,
        REPORT_RESULT_TOOL => OperationId::OrchestrationReportResult,
        REQUEST_PARENT_INPUT_TOOL => OperationId::OrchestrationRequestParentInput,
        REPORT_BLOCKED_TOOL => OperationId::OrchestrationReportBlocked,
        SEND_PARENT_MESSAGE_TOOL => OperationId::OrchestrationSendParentMessage,
        _ => return None,
    })
}

/// 변경 도구는 인자의 `requestId`로 재시도를 식별한다(`retry_identity`).
fn request(operation: OperationId, input: Value, rpc_id: Option<&Value>) -> CallRequest {
    let key = (!matches!(
        workbench_protocol::operations::spec_for(operation).kind,
        workbench_protocol::OperationKind::Query
    ))
    .then(|| {
        super::retry_identity::tool_idempotency_key(
            input["runId"].as_str().unwrap_or_default(),
            operation,
            input.get("arguments"),
            rpc_id,
        )
    });
    let mut call = CallRequest::query(operation, input);
    call.idempotency_key = key;
    call
}

/// 이 run의 서버 역할. 조회 실패는 역할 없음으로 본다(도구 목록이 비고, 호출은 서버가 다시 거절한다).
pub async fn agent_role(
    runtime: &Arc<WorkbenchRuntime>,
    principal: &CapabilityPrincipal,
) -> Option<AgentToolRole> {
    let reply = runtime
        .call(
            AuthenticatedPrincipal::agent(&principal.run_id),
            request(
                OperationId::OrchestrationGetAgentRole,
                json!({ "runId": principal.run_id }),
                None,
            ),
        )
        .await
        .ok()?;
    match reply.output()?.get("role")?.as_str()? {
        "coordinator" => Some(AgentToolRole::Coordinator),
        "child" => Some(AgentToolRole::Child),
        _ => None,
    }
}

pub async fn handle_tool(
    runtime: &Arc<WorkbenchRuntime>,
    principal: &CapabilityPrincipal,
    name: &str,
    arguments: Option<&Value>,
    rpc_id: Option<&Value>,
) -> Value {
    let Some(operation) = operation_for(name) else {
        return tool_error(
            "unsupportedTool",
            format!("Unsupported tool: {name}"),
            false,
        );
    };
    let input = json!({
        "runId": principal.run_id,
        "arguments": arguments.cloned().unwrap_or_else(|| json!({})),
    });
    match runtime
        .call(
            AuthenticatedPrincipal::agent(&principal.run_id),
            request(operation, input, rpc_id),
        )
        .await
    {
        Ok(reply) => tool_success(reply.output().cloned().unwrap_or(Value::Null)),
        Err(fault) => fault_tool_error(&fault),
    }
}

/// fault → 도구 오류. handler가 만든 도구 오류(`details.toolError`)는 그 코드를 쓰고, 입구 판정 fault(비우기 `draining`,
/// 정지 `unavailable` 등)는 fault 코드를 그대로 싣는다(044 OCR 구현 리뷰 — `internalError`로 뭉개면 agent가 "비우는 중,
/// 다시 시도 가능"과 버그를 구별하지 못한다). 내부 오류만 오늘의 `internalError` 이름을 쓴다.
fn fault_tool_error(fault: &workbench_protocol::WorkbenchFault) -> Value {
    let fallback = fault_code_name(fault.code);
    match fault
        .details
        .as_ref()
        .and_then(|details| details.get("toolError"))
    {
        Some(error) => tool_error(
            error["code"].as_str().unwrap_or(fallback).to_owned(),
            error["message"]
                .as_str()
                .unwrap_or(&fault.message)
                .to_owned(),
            error["retryable"].as_bool().unwrap_or(fault.retryable),
        ),
        None => tool_error(fallback, fault.message.clone(), fault.retryable),
    }
}

fn fault_code_name(code: workbench_protocol::FaultCode) -> &'static str {
    match code {
        workbench_protocol::FaultCode::Internal => "internalError",
        other => other.as_str(),
    }
}

fn tool_definition(name: &str) -> Value {
    let (description, properties, required) = match name {
        CREATE_CHILD_TASK_TOOL => (
            "Create and schedule a direct child task under Main.",
            json!({
                "requestId": { "type": "string" },
                "title": { "type": "string", "maxLength": 120 },
                "role": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "responsibility": { "type": "string" },
                        "expectedOutput": { "type": "string" }
                    },
                    "required": ["name", "responsibility", "expectedOutput"]
                },
                "objective": { "type": "string" },
                "constraints": { "type": "array", "items": { "type": "string" } },
                "expectedResult": { "type": "string" },
                "dependencyTaskIds": { "type": "array", "items": { "type": "string" } },
                "preferredNodeId": { "type": ["string", "null"] }
            }),
            json!([
                "requestId",
                "title",
                "role",
                "objective",
                "constraints",
                "expectedResult"
            ]),
        ),
        REPORT_PROGRESS_TOOL => (
            "Report progress for the authenticated child task.",
            report_properties(false),
            json!(["requestId", "summary"]),
        ),
        REPORT_RESULT_TOOL => (
            "Submit the explicit structured result that completes the authenticated child task.",
            report_properties(true),
            json!(["requestId", "summary"]),
        ),
        REQUEST_PARENT_INPUT_TOOL => (
            "Request input from Main or the user without focusing a panel.",
            json!({
                "requestId": { "type": "string" },
                "summary": { "type": "string" },
                "question": { "type": "string" },
                "options": { "type": "array", "items": { "type": "string" } }
            }),
            json!(["requestId", "summary", "question"]),
        ),
        REPORT_BLOCKED_TOOL => (
            "Report a blocked authenticated child task.",
            report_properties(false),
            json!(["requestId", "summary"]),
        ),
        SEND_PARENT_MESSAGE_TOOL => (
            "Send a status-neutral message to Main.",
            json!({
                "requestId": { "type": "string" },
                "summary": { "type": "string" }
            }),
            json!(["requestId", "summary"]),
        ),
        SEND_CHILD_MESSAGE_TOOL => (
            "Send a durable message to the exact current Child task run.",
            json!({
                "requestId": { "type": "string" },
                "taskId": { "type": "string" },
                "message": { "type": "string" }
            }),
            json!(["requestId", "taskId", "message"]),
        ),
        REASSIGN_CHILD_TASK_TOOL => (
            "Fence the previous worker and launch the task on another direct Child.",
            json!({
                "requestId": { "type": "string" },
                "taskId": { "type": "string" },
                "targetNodeId": { "type": "string" }
            }),
            json!(["requestId", "taskId", "targetNodeId"]),
        ),
        INTERRUPT_CHILD_TASK_TOOL | CANCEL_CHILD_TASK_TOOL | RETRY_CHILD_TASK_TOOL => (
            "Control the exact current Child task runtime.",
            json!({
                "requestId": { "type": "string" },
                "taskId": { "type": "string" }
            }),
            json!(["requestId", "taskId"]),
        ),
        _ => (
            "Operate on direct child tasks owned by the active Main generation.",
            json!({
                "requestId": { "type": "string" },
                "taskId": { "type": "string" },
                "taskIds": { "type": "array", "items": { "type": "string" } },
                "message": { "type": "string" },
                "targetNodeId": { "type": "string" },
                "timeoutMs": { "type": "integer", "minimum": 0, "maximum": 30000 },
                "includePartial": { "type": "boolean" }
            }),
            json!([]),
        ),
    };
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        }
    })
}

fn report_properties(include_result_fields: bool) -> Value {
    let mut value = json!({
        "requestId": { "type": "string" },
        "progressPercent": { "type": ["integer", "null"], "minimum": 0, "maximum": 100 },
        "summary": { "type": "string" },
        "findings": { "type": "array" }
    });
    if include_result_fields {
        let properties = value.as_object_mut().expect("report properties");
        properties.insert("artifactRefs".into(), json!({ "type": "array" }));
        properties.insert(
            "unresolved".into(),
            json!({ "type": "array", "items": { "type": "string" } }),
        );
        properties.insert(
            "confidence".into(),
            json!({ "type": ["number", "null"], "minimum": 0, "maximum": 1 }),
        );
    }
    value
}

pub fn tool_success(structured: Value) -> Value {
    json!({
        "content": [{ "type": "text", "text": structured.to_string() }],
        "structuredContent": structured,
        "isError": false
    })
}

pub fn tool_error(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Value {
    let code = code.into();
    let message = message.into();
    json!({
        "content": [{ "type": "text", "text": message }],
        "structuredContent": {
            "code": code,
            "message": message,
            "retryable": retryable
        },
        "isError": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fault(code: workbench_protocol::FaultCode) -> workbench_protocol::WorkbenchFault {
        workbench_protocol::WorkbenchFault::new(
            code,
            workbench_protocol::RequestId::random(),
            "message",
        )
    }

    #[test]
    fn entry_faults_keep_their_code_and_internal_keeps_the_tool_name() {
        use workbench_protocol::FaultCode;
        for (code, expected) in [
            (FaultCode::Draining, "draining"),
            (FaultCode::Unavailable, "unavailable"),
            (FaultCode::Forbidden, "forbidden"),
            (FaultCode::Internal, "internalError"),
        ] {
            let fault = fault(code);
            let error = fault_tool_error(&fault);
            assert_eq!(error["structuredContent"]["code"], expected, "{code:?}");
            assert_eq!(error["structuredContent"]["retryable"], fault.retryable);
        }
        let mut with_tool_error = fault(FaultCode::Conflict);
        with_tool_error.details =
            Some(json!({ "toolError": { "code": "taskNotReady", "message": "m" } }));
        assert_eq!(
            fault_tool_error(&with_tool_error)["structuredContent"]["code"],
            "taskNotReady"
        );
    }

    #[test]
    fn exposes_role_specific_tool_sets() {
        let coordinator = tool_definitions(Some(AgentToolRole::Coordinator));
        let child = tool_definitions(Some(AgentToolRole::Child));

        assert!(
            coordinator
                .iter()
                .any(|tool| { tool["name"] == CREATE_CHILD_TASK_TOOL })
        );
        assert!(
            !coordinator
                .iter()
                .any(|tool| { tool["name"] == REPORT_RESULT_TOOL })
        );
        assert!(
            child
                .iter()
                .any(|tool| { tool["name"] == REPORT_RESULT_TOOL })
        );
        assert!(
            !child
                .iter()
                .any(|tool| { tool["name"] == CREATE_CHILD_TASK_TOOL })
        );
    }

    #[test]
    fn runs_without_a_role_see_no_orchestration_tools() {
        assert!(tool_definitions(None).is_empty());
        assert!(
            COORDINATOR_TOOLS
                .iter()
                .chain(CHILD_TOOLS)
                .all(|name| operation_for(name).is_some())
        );
    }

    #[test]
    fn structured_errors_do_not_claim_success() {
        let result = tool_error("forbiddenActor", "Only Main can create child tasks.", false);
        assert_eq!(result["isError"], true);
        assert_eq!(result["structuredContent"]["code"], "forbiddenActor");
    }

    #[test]
    fn child_report_and_coordinator_send_contracts_are_explicit() {
        let coordinator = tool_definitions(Some(AgentToolRole::Coordinator));
        let send = coordinator
            .iter()
            .find(|tool| tool["name"] == SEND_CHILD_MESSAGE_TOOL)
            .unwrap();
        assert_eq!(
            send["inputSchema"]["required"],
            json!(["requestId", "taskId", "message"])
        );

        let child = tool_definitions(Some(AgentToolRole::Child));
        for name in [
            REPORT_PROGRESS_TOOL,
            REPORT_RESULT_TOOL,
            REQUEST_PARENT_INPUT_TOOL,
            REPORT_BLOCKED_TOOL,
        ] {
            let report = child.iter().find(|tool| tool["name"] == name).unwrap();
            assert!(
                report["inputSchema"]["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("requestId"))
            );
        }
    }
}
