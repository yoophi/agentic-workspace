//! agent orchestration operation 17개(041 US2): MCP 도구 16개 + 역할 조회. principal run이 입력 `runId`와 같아야 하고
//! (도구 쪽 검사와 같은 문구), 역할은 서버 상태로 판정한다(research R7). 도구 오류는 `details.toolError`에 오늘
//! `structuredContent`(`{code, message, retryable}`)를 싣는다 — AW MCP가 그대로 되돌린다.

use std::sync::Arc;

use workbench_protocol::{
    operations::orchestration::{AgentRoleDto, AgentRoleInput, AgentRoleKindDto, AgentToolInput},
    FaultCode, OperationId, RequestId, WorkbenchFault,
};

use crate::application::{
    bench_service::BenchServices,
    handlers::{
        epoch::{async_query_handler, epoch_handler, to_json, Scope},
        exchange::MESSAGE_RUN_MISMATCH,
    },
    orchestration::{
        agent_tools::{self, forbidden_actor, not_bound, AgentRole, RoleLookup, ToolError},
        runtime::OrchestrationRuntime,
    },
    registry::Registry,
};

pub fn tool_fault(request_id: &RequestId, error: ToolError) -> WorkbenchFault {
    let code = match error.code.as_str() {
        "forbiddenActor" | "scopeMismatch" | "unauthorized" => FaultCode::Forbidden,
        "invalidInput" | "invalidTopology" => FaultCode::InvalidArgument,
        "unknownTask" | "unknownNode" | "notFound" | "workspaceNotBootstrapped" => {
            FaultCode::NotFound
        }
        "revisionConflict" | "duplicateConflict" | "invalidTransition" => FaultCode::Conflict,
        "capacityExceeded" => FaultCode::RateLimited,
        "draining" => FaultCode::Draining,
        _ => FaultCode::Unavailable,
    };
    let details = serde_json::json!({ "toolError": &error });
    WorkbenchFault::new(code, request_id.clone(), error.message.clone())
        .with_retryable(error.retryable)
        .with_details(details)
}

fn ensure_run(
    request_id: &RequestId,
    principal_run: Option<&str>,
    run_id: &str,
) -> Result<(), WorkbenchFault> {
    if principal_run == Some(run_id) {
        Ok(())
    } else {
        Err(WorkbenchFault::new(
            FaultCode::Forbidden,
            request_id.clone(),
            MESSAGE_RUN_MISMATCH,
        ))
    }
}

/// 역할을 찾고 이 도구를 부를 수 있는지 본다(오늘 순서: 역할 불일치 → forbiddenActor, 작업 영역 없음 → scopeMismatch).
async fn role_for(
    runtime: &OrchestrationRuntime,
    run_id: &str,
    tool: &str,
) -> Result<AgentRole, ToolError> {
    match runtime.agent_role(run_id).await {
        RoleLookup::Role(role) if role.allows(tool) => Ok(role),
        RoleLookup::Role(_) | RoleLookup::None => Err(forbidden_actor()),
        RoleLookup::Unbound => Err(not_bound()),
    }
}

const TOOLS: &[(OperationId, &str, bool)] = &[
    (
        OperationId::OrchestrationCreateChildTask,
        agent_tools::CREATE_CHILD_TASK_TOOL,
        true,
    ),
    (
        OperationId::OrchestrationAssignChildTask,
        agent_tools::ASSIGN_CHILD_TASK_TOOL,
        true,
    ),
    (
        OperationId::OrchestrationListChildTasks,
        agent_tools::LIST_CHILD_TASKS_TOOL,
        false,
    ),
    (
        OperationId::OrchestrationSendChildMessage,
        agent_tools::SEND_CHILD_MESSAGE_TOOL,
        true,
    ),
    (
        OperationId::OrchestrationWaitChildTasks,
        agent_tools::WAIT_CHILD_TASKS_TOOL,
        false,
    ),
    (
        OperationId::OrchestrationCollectChildResults,
        agent_tools::COLLECT_CHILD_RESULTS_TOOL,
        true,
    ),
    (
        OperationId::OrchestrationInterruptChildTask,
        agent_tools::INTERRUPT_CHILD_TASK_TOOL,
        true,
    ),
    (
        OperationId::OrchestrationCancelChildTask,
        agent_tools::CANCEL_CHILD_TASK_TOOL,
        true,
    ),
    (
        OperationId::OrchestrationRetryChildTask,
        agent_tools::RETRY_CHILD_TASK_TOOL,
        true,
    ),
    (
        OperationId::OrchestrationReassignChildTask,
        agent_tools::REASSIGN_CHILD_TASK_TOOL,
        true,
    ),
    (
        OperationId::OrchestrationGetOwnTask,
        agent_tools::GET_OWN_TASK_TOOL,
        false,
    ),
    (
        OperationId::OrchestrationReportProgress,
        agent_tools::REPORT_PROGRESS_TOOL,
        true,
    ),
    (
        OperationId::OrchestrationReportResult,
        agent_tools::REPORT_RESULT_TOOL,
        true,
    ),
    (
        OperationId::OrchestrationRequestParentInput,
        agent_tools::REQUEST_PARENT_INPUT_TOOL,
        true,
    ),
    (
        OperationId::OrchestrationReportBlocked,
        agent_tools::REPORT_BLOCKED_TOOL,
        true,
    ),
    (
        OperationId::OrchestrationSendParentMessage,
        agent_tools::SEND_PARENT_MESSAGE_TOOL,
        true,
    ),
];

pub fn register(
    registry: &mut Registry,
    services: &Arc<BenchServices>,
    runtime: &Arc<OrchestrationRuntime>,
) {
    for &(operation, tool, command) in TOOLS {
        let runtime = Arc::clone(runtime);
        let run = move |_: Arc<BenchServices>,
                        ctx: crate::application::registry::CallContext,
                        input: AgentToolInput| {
            let runtime = Arc::clone(&runtime);
            async move {
                ensure_run(&ctx.request_id, ctx.principal.agent_run_id(), &input.run_id)?;
                let role = role_for(&runtime, &input.run_id, tool)
                    .await
                    .map_err(|error| tool_fault(&ctx.request_id, error))?;
                agent_tools::handle_tool(&runtime, &input.run_id, role, tool, &input.arguments)
                    .await
                    .map_err(|error| tool_fault(&ctx.request_id, error))
            }
        };
        let handler = if command {
            epoch_handler(
                operation,
                services,
                |input: &AgentToolInput| Scope::RunOwner(input.run_id.clone()),
                run,
            )
        } else {
            async_query_handler(services, run)
        };
        registry.register(operation, handler);
    }
    let runtime = Arc::clone(runtime);
    registry.register(
        OperationId::OrchestrationGetAgentRole,
        async_query_handler(services, move |_, ctx, input: AgentRoleInput| {
            let runtime = Arc::clone(&runtime);
            async move {
                ensure_run(&ctx.request_id, ctx.principal.agent_run_id(), &input.run_id)?;
                let dto = match runtime.agent_role(&input.run_id).await {
                    RoleLookup::Role(AgentRole::Coordinator { workspace_id, .. }) => AgentRoleDto {
                        role: Some(AgentRoleKindDto::Coordinator),
                        workspace_id: Some(workspace_id),
                        task_id: None,
                    },
                    RoleLookup::Role(AgentRole::Child {
                        workspace_id,
                        task_id,
                        ..
                    }) => AgentRoleDto {
                        role: Some(AgentRoleKindDto::Child),
                        workspace_id: Some(workspace_id),
                        task_id: Some(task_id),
                    },
                    RoleLookup::Unbound | RoleLookup::None => AgentRoleDto {
                        role: None,
                        workspace_id: None,
                        task_id: None,
                    },
                };
                Ok(to_json(dto))
            }
        }),
    );
}
