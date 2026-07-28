//! Coordinator and child orchestration MCP tools.

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::AppHandle;

use crate::{
    application::coordinator_notification_dispatcher::CoordinatorNotificationDispatcher,
    application::orchestration_command_service::{
        DeliverTaskCommandRequest, OrchestrationCommandService,
    },
    application::orchestration_scheduler::LeaseOutcome,
    application::orchestration_service::{
        CreateChildTaskRequest, OrchestrationService, ReportTaskRequest, TaskActionRequest,
    },
    domain::agent_orchestration::{
        AccessPolicy, AgentRoleProfile, OrchestrationError, PromptDelivery, TaskCommandKind,
        TaskCommandSource, TaskReportType, WorkerRuntimeProfile,
    },
    infrastructure::{
        acp_agent_worker_adapter::{AcpAgentWorkerAdapter, TauriAcpWorkerRuntime},
        agent_session_registry::AppState,
        json_orchestration_repository::JsonOrchestrationRepository,
        mcp::McpServerState,
        mcp::capability_registry::{CapabilityActorKind, CapabilityPrincipal},
        tauri_orchestration_event_sink::TauriOrchestrationEventSink,
    },
    ports::{
        agent_worker::{AgentWorkerPort, StartWorkerOutcome, WorkerAssignment},
        orchestration_event_sink::{OrchestrationEvent, OrchestrationEventSink},
    },
};

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

pub fn is_orchestration_tool(name: &str) -> bool {
    COORDINATOR_TOOLS.contains(&name) || CHILD_TOOLS.contains(&name)
}

pub fn tool_definitions(actor_kind: CapabilityActorKind) -> Vec<Value> {
    let names = match actor_kind {
        CapabilityActorKind::Coordinator => COORDINATOR_TOOLS,
        CapabilityActorKind::Child => CHILD_TOOLS,
        CapabilityActorKind::LegacyRun => return Vec::new(),
    };
    names.iter().map(|name| tool_definition(name)).collect()
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RoleInput {
    name: String,
    responsibility: String,
    expected_output: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaskInput {
    request_id: String,
    title: String,
    role: RoleInput,
    objective: String,
    #[serde(default)]
    constraints: Vec<String>,
    expected_result: String,
    #[serde(default)]
    dependency_task_ids: Vec<String>,
    preferred_node_id: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportInput {
    request_id: String,
    progress_percent: Option<u8>,
    summary: String,
    #[serde(default)]
    findings: Vec<crate::domain::agent_orchestration::TaskFinding>,
    #[serde(default)]
    artifact_refs: Vec<crate::domain::agent_orchestration::ArtifactReference>,
    #[serde(default)]
    unresolved: Vec<String>,
    confidence: Option<f64>,
    question: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CoordinatorTaskInput {
    request_id: Option<String>,
    task_id: Option<String>,
    #[serde(default)]
    task_ids: Vec<String>,
    message: Option<String>,
    timeout_ms: Option<u64>,
    target_node_id: Option<String>,
}

pub async fn handle_tool(
    app: &AppHandle,
    registry: &AppState,
    mcp_state: &McpServerState,
    principal: &CapabilityPrincipal,
    name: &str,
    arguments: Option<&Value>,
) -> Value {
    if !is_allowed(principal.actor_kind, name) {
        return tool_error(
            "forbiddenActor",
            "The authenticated agent role cannot call this tool.",
            false,
        );
    }
    let Some(window_label) = principal.window_label.as_deref() else {
        return tool_error(
            "scopeMismatch",
            "This run is not bound to an orchestration workspace.",
            false,
        );
    };
    let repository = match JsonOrchestrationRepository::from_app(app) {
        Ok(repository) => repository,
        Err(error) => return tool_error("persistenceFailed", error, true),
    };
    let service = OrchestrationService::new(
        repository.clone(),
        TauriOrchestrationEventSink::new(app.clone()),
    );

    match name {
        CREATE_CHILD_TASK_TOOL => {
            let input: CreateTaskInput = match parse(arguments) {
                Ok(input) => input,
                Err(error) => return domain_error(error),
            };
            let Some(generation_id) = principal.generation_id.as_deref() else {
                return tool_error(
                    "staleCoordinatorGeneration",
                    "The Coordinator generation is unavailable.",
                    false,
                );
            };
            let role_id = input.role.name.to_lowercase().replace(' ', "-");
            let role = match AgentRoleProfile::new(
                role_id,
                input.role.name,
                input.role.responsibility,
                input.role.expected_output,
            ) {
                Ok(role) => role,
                Err(error) => return domain_error(error),
            };
            match service.create_child_task(
                window_label,
                generation_id,
                CreateChildTaskRequest {
                    request_id: input.request_id,
                    title: input.title,
                    role,
                    objective: input.objective,
                    constraints: input.constraints,
                    expected_result: input.expected_result,
                    dependency_task_ids: input.dependency_task_ids,
                    preferred_node_id: input.preferred_node_id,
                },
            ) {
                Ok(outcome) => {
                    let snapshot = match service.get_for_window(window_label) {
                        Ok(Some(snapshot)) => snapshot,
                        Ok(None) => {
                            return tool_error(
                                "workspaceNotBootstrapped",
                                "Workspace is unavailable.",
                                true,
                            );
                        }
                        Err(error) => return domain_error(error),
                    };
                    let Some(task) = snapshot
                        .tasks
                        .iter()
                        .find(|task| task.id == outcome.task_id)
                    else {
                        return tool_error("unknownTask", "Created task is unavailable.", true);
                    };
                    let Some(node) = snapshot
                        .nodes
                        .iter()
                        .find(|node| node.id == outcome.node_id)
                    else {
                        return tool_error(
                            "unknownNode",
                            "Created child node is unavailable.",
                            true,
                        );
                    };
                    match mcp_state.orchestration_scheduler().acquire(&task.id) {
                        Ok(LeaseOutcome::Queued { position }) => {
                            return tool_success(json!({
                                "taskId": outcome.task_id,
                                "nodeId": outcome.node_id,
                                "status": outcome.status,
                                "executionStatus": "starting",
                                "queued": true,
                                "queuePosition": position
                            }));
                        }
                        Ok(LeaseOutcome::Acquired) => {}
                        Err(error) => return domain_error(error),
                    }
                    let planned_run_id = uuid::Uuid::new_v4().to_string();
                    let runtime_profile = WorkerRuntimeProfile {
                        agent_profile_id: std::env::var("AW_ORCHESTRATION_AGENT_PROFILE")
                            .unwrap_or_else(|_| "codex".into()),
                        provider_id: "acp".into(),
                        model_id: None,
                        access_policy: AccessPolicy::ReadOnly,
                        supports_read_only: true,
                    };
                    let adapter = AcpAgentWorkerAdapter::new(TauriAcpWorkerRuntime::new(
                        app.clone(),
                        registry.clone(),
                        mcp_state.clone(),
                    ));
                    let launch = adapter
                        .start_worker(WorkerAssignment {
                            workspace_id: snapshot.id.clone(),
                            window_label: window_label.into(),
                            worktree_path: snapshot.worktree_path.clone(),
                            node_id: node.id.clone(),
                            task_id: task.id.clone(),
                            attempt: task.attempt,
                            planned_run_id,
                            role: node.role.clone(),
                            objective: task.objective.clone(),
                            constraints: task.constraints.clone(),
                            expected_result: task.expected_result.clone(),
                            runtime_profile,
                            mcp_capability: String::new(),
                        })
                        .await;
                    match launch {
                        Ok(StartWorkerOutcome::Started { run_id }) => {
                            if let Err(error) =
                                service.bind_child_run(window_label, &task.id, &node.id, &run_id)
                            {
                                return domain_error(error);
                            }
                            tool_success(json!({
                                "taskId": outcome.task_id,
                                "nodeId": outcome.node_id,
                                "status": outcome.status,
                                "executionStatus": "active",
                                "runId": run_id
                            }))
                        }
                        Ok(other) => tool_success(json!({
                            "taskId": outcome.task_id,
                            "nodeId": outcome.node_id,
                            "status": outcome.status,
                            "executionStatus": "starting",
                            "launch": other
                        })),
                        Err(error) => {
                            let _ = mcp_state.orchestration_scheduler().release(&task.id);
                            domain_error(error)
                        }
                    }
                }
                Err(error) => domain_error(error),
            }
        }
        LIST_CHILD_TASKS_TOOL | WAIT_CHILD_TASKS_TOOL | COLLECT_CHILD_RESULTS_TOOL => {
            let Some(generation_id) = principal.generation_id.as_deref() else {
                return tool_error(
                    "staleCoordinatorGeneration",
                    "Coordinator generation is unavailable.",
                    false,
                );
            };
            let input: CoordinatorTaskInput = parse(arguments).unwrap_or_default();
            if name == WAIT_CHILD_TASKS_TOOL {
                let timeout_ms = input.timeout_ms.unwrap_or(30_000).min(30_000);
                let started = std::time::Instant::now();
                loop {
                    let tasks = match service.list_child_tasks(window_label, generation_id) {
                        Ok(tasks) => tasks,
                        Err(error) => return domain_error(error),
                    };
                    let selected = tasks.iter().filter(|task| {
                        input.task_ids.is_empty() || input.task_ids.contains(&task.id)
                    });
                    if selected.clone().all(|task| {
                        task.status.is_terminal()
                            || matches!(
                                task.status,
                                crate::domain::agent_orchestration::TaskStatus::Failed
                            )
                    }) {
                        break;
                    }
                    if started.elapsed().as_millis() >= timeout_ms as u128 {
                        return tool_success(json!({ "timedOut": true, "tasks": tasks }));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
            match service.list_child_tasks(window_label, generation_id) {
                Ok(tasks) => {
                    let reports = if name == COLLECT_CHILD_RESULTS_TOOL {
                        match service.collect_child_results(
                            window_label,
                            generation_id,
                            &input.task_ids,
                        ) {
                            Ok(reports) => reports,
                            Err(error) => return domain_error(error),
                        }
                    } else {
                        Vec::new()
                    };
                    tool_success(json!({
                        "timedOut": false,
                        "tasks": tasks,
                        "reports": reports
                    }))
                }
                Err(error) => domain_error(error),
            }
        }
        ASSIGN_CHILD_TASK_TOOL => {
            let input: CoordinatorTaskInput = match parse(arguments) {
                Ok(input) => input,
                Err(error) => return domain_error(error),
            };
            let Some(task_id) = input.task_id else {
                return tool_error("invalidInput", "taskId is required.", false);
            };
            match mcp_state.orchestration_scheduler().acquire(&task_id) {
                Ok(LeaseOutcome::Queued { position }) => tool_success(json!({
                    "taskId": task_id,
                    "queued": true,
                    "queuePosition": position
                })),
                Ok(LeaseOutcome::Acquired) => match launch_existing_task(
                    app,
                    registry,
                    mcp_state,
                    &service,
                    window_label,
                    &task_id,
                )
                .await
                {
                    Ok(value) => tool_success(value),
                    Err(error) => {
                        let _ = mcp_state.orchestration_scheduler().release(&task_id);
                        domain_error(error)
                    }
                },
                Err(error) => domain_error(error),
            }
        }
        SEND_CHILD_MESSAGE_TOOL
        | INTERRUPT_CHILD_TASK_TOOL
        | CANCEL_CHILD_TASK_TOOL
        | RETRY_CHILD_TASK_TOOL
        | REASSIGN_CHILD_TASK_TOOL => {
            let input: CoordinatorTaskInput = match parse(arguments) {
                Ok(input) => input,
                Err(error) => return domain_error(error),
            };
            let Some(task_id) = input.task_id.clone() else {
                return tool_error("invalidInput", "taskId is required.", false);
            };
            let snapshot = match service.get_for_window(window_label) {
                Ok(Some(snapshot)) => snapshot,
                Ok(None) => {
                    return tool_error(
                        "workspaceNotBootstrapped",
                        "Workspace is unavailable.",
                        true,
                    );
                }
                Err(error) => return domain_error(error),
            };
            let Some(task) = snapshot.tasks.iter().find(|task| task.id == task_id) else {
                return tool_error("unknownTask", "Task was not found.", false);
            };
            let Some(node) = task
                .assigned_node_id
                .as_ref()
                .and_then(|node_id| snapshot.nodes.iter().find(|node| node.id == *node_id))
            else {
                return tool_error("unknownNode", "Assigned child node was not found.", false);
            };
            let request = TaskActionRequest {
                request_id: input
                    .request_id
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                task_id: task_id.clone(),
                expected_revision: snapshot.revision,
                message: input.message.clone(),
                target_node_id: input.target_node_id,
            };
            if matches!(name, RETRY_CHILD_TASK_TOOL | REASSIGN_CHILD_TASK_TOOL)
                && let Some(run_id) = node.current_run_id.as_ref()
            {
                let adapter = AcpAgentWorkerAdapter::new(TauriAcpWorkerRuntime::new(
                    app.clone(),
                    registry.clone(),
                    mcp_state.clone(),
                ));
                let binding = crate::ports::agent_worker::WorkerBinding {
                    workspace_id: snapshot.id.clone(),
                    window_label: window_label.into(),
                    node_id: node.id.clone(),
                    task_id: task.id.clone(),
                    run_id: run_id.clone(),
                };
                if adapter.is_active(&binding).await {
                    let _ = adapter.cancel_worker(&binding).await;
                }
                if let Err(error) = mcp_state.revoke_run_capability(run_id) {
                    return domain_error(error);
                }
            }
            if name == RETRY_CHILD_TASK_TOOL {
                let session = match service.retry_task(window_label, request) {
                    Ok(session) => session,
                    Err(error) => return domain_error(error),
                };
                return match mcp_state.orchestration_scheduler().acquire(&task_id) {
                    Ok(LeaseOutcome::Queued { position }) => tool_success(json!({
                        "workspace": session,
                        "accepted": true,
                        "queued": true,
                        "queuePosition": position
                    })),
                    Ok(LeaseOutcome::Acquired) => match launch_existing_task(
                        app,
                        registry,
                        mcp_state,
                        &service,
                        window_label,
                        &task_id,
                    )
                    .await
                    {
                        Ok(launch) => tool_success(json!({
                            "workspace": session,
                            "accepted": true,
                            "launch": launch
                        })),
                        Err(error) => domain_error(error),
                    },
                    Err(error) => domain_error(error),
                };
            }
            if name == REASSIGN_CHILD_TASK_TOOL {
                let session = match service.reassign_task(window_label, request) {
                    Ok(session) => session,
                    Err(error) => return domain_error(error),
                };
                return match mcp_state.orchestration_scheduler().acquire(&task_id) {
                    Ok(LeaseOutcome::Queued { position }) => tool_success(json!({
                        "workspace": session,
                        "accepted": true,
                        "queued": true,
                        "queuePosition": position
                    })),
                    Ok(LeaseOutcome::Acquired) => match launch_existing_task(
                        app,
                        registry,
                        mcp_state,
                        &service,
                        window_label,
                        &task_id,
                    )
                    .await
                    {
                        Ok(launch) => tool_success(json!({
                            "workspace": session,
                            "accepted": true,
                            "launch": launch
                        })),
                        Err(error) => domain_error(error),
                    },
                    Err(error) => domain_error(error),
                };
            }
            if name == CANCEL_CHILD_TASK_TOOL && node.current_run_id.is_none() {
                return match service.cancel_task(window_label, request) {
                    Ok(session) => tool_success(json!({
                        "workspace": session,
                        "accepted": true,
                        "runtimeCommand": null
                    })),
                    Err(error) => domain_error(error),
                };
            }
            let adapter = AcpAgentWorkerAdapter::new(TauriAcpWorkerRuntime::new(
                app.clone(),
                registry.clone(),
                mcp_state.clone(),
            ));
            let kind = if name == SEND_CHILD_MESSAGE_TOOL {
                if input.message.as_deref().is_none() {
                    return tool_error("invalidInput", "message is required.", false);
                }
                if task.status == crate::domain::agent_orchestration::TaskStatus::InputRequired {
                    TaskCommandKind::InputResponse
                } else {
                    TaskCommandKind::Message
                }
            } else if name == INTERRUPT_CHILD_TASK_TOOL {
                TaskCommandKind::Interrupt
            } else {
                TaskCommandKind::Cancel
            };
            let command_service = OrchestrationCommandService::new(repository.clone(), adapter);
            match command_service
                .deliver(
                    window_label,
                    DeliverTaskCommandRequest {
                        request_id: request.request_id,
                        task_id: task_id.clone(),
                        kind,
                        message: input.message,
                        input_report_id: (kind == TaskCommandKind::InputResponse)
                            .then(|| {
                                snapshot
                                    .reports
                                    .iter()
                                    .rev()
                                    .find(|report| {
                                        report.task_id == task_id
                                            && report.report_type == TaskReportType::InputRequest
                                    })
                                    .map(|report| report.id.clone())
                            })
                            .flatten(),
                        delivery: PromptDelivery::Queue,
                        source: TaskCommandSource::Coordinator,
                        expected_task_revision: Some(task.revision),
                    },
                )
                .await
            {
                Ok(command) if name == CANCEL_CHILD_TASK_TOOL => {
                    emit_latest_runtime_event(app, &service, window_label, "taskCommandDelivery");
                    let next_task_id = mcp_state
                        .orchestration_scheduler()
                        .release(&task_id)
                        .ok()
                        .flatten();
                    tool_success(json!({
                        "command": command,
                        "nextReadyTaskId": next_task_id
                    }))
                }
                Ok(command) => {
                    emit_latest_runtime_event(app, &service, window_label, "taskCommandDelivery");
                    tool_success(json!(command))
                }
                Err(error) => domain_error(error),
            }
        }
        GET_OWN_TASK_TOOL => {
            let Some(task_id) = principal.task_id.as_deref() else {
                return tool_error("unknownTask", "No task is bound to this run.", false);
            };
            match service.get_for_window(window_label) {
                Ok(Some(session)) => {
                    match session.tasks.into_iter().find(|task| task.id == task_id) {
                        Some(task) => tool_success(json!(task)),
                        None => tool_error("unknownTask", "Assigned task was not found.", false),
                    }
                }
                Ok(None) => tool_error(
                    "workspaceNotBootstrapped",
                    "Workspace is unavailable.",
                    true,
                ),
                Err(error) => domain_error(error),
            }
        }
        REPORT_PROGRESS_TOOL
        | REPORT_RESULT_TOOL
        | REQUEST_PARENT_INPUT_TOOL
        | REPORT_BLOCKED_TOOL
        | SEND_PARENT_MESSAGE_TOOL => {
            let input: ReportInput = match parse(arguments) {
                Ok(input) => input,
                Err(error) => return domain_error(error),
            };
            let (Some(task_id), Some(node_id)) =
                (principal.task_id.clone(), principal.node_id.clone())
            else {
                return tool_error("unknownTask", "No task is bound to this run.", false);
            };
            let report_type = match name {
                REPORT_RESULT_TOOL => TaskReportType::Result,
                REQUEST_PARENT_INPUT_TOOL => TaskReportType::InputRequest,
                REPORT_BLOCKED_TOOL => TaskReportType::Blocked,
                SEND_PARENT_MESSAGE_TOOL => TaskReportType::Message,
                _ => TaskReportType::Progress,
            };
            let summary = if let Some(question) = input.question {
                format!("{}\n\n{}", input.summary, question)
            } else {
                input.summary
            };
            match service.report_task(
                window_label,
                ReportTaskRequest {
                    request_id: input.request_id,
                    task_id,
                    reporter_node_id: node_id,
                    reporter_run_id: principal.run_id.clone(),
                    report_type,
                    progress_percent: input.progress_percent,
                    summary,
                    findings: input.findings,
                    artifact_refs: input.artifact_refs,
                    unresolved: input.unresolved,
                    confidence: input.confidence,
                },
            ) {
                Ok(report) => {
                    let notifications = service
                        .get_for_window(window_label)
                        .ok()
                        .flatten()
                        .map(|session| {
                            session
                                .coordinator_notifications
                                .into_iter()
                                .filter(|notification| notification.report_id == report.id)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let next_task_id = (report.report_type == TaskReportType::Result)
                        .then(|| {
                            mcp_state
                                .orchestration_scheduler()
                                .release(&report.task_id)
                                .ok()
                                .flatten()
                        })
                        .flatten();
                    let dispatch_app = app.clone();
                    let dispatch_registry = registry.clone();
                    let dispatch_mcp_state = mcp_state.clone();
                    let dispatch_repository = repository.clone();
                    let dispatch_window_label = window_label.to_string();
                    tokio::spawn(async move {
                        let adapter = AcpAgentWorkerAdapter::new(TauriAcpWorkerRuntime::new(
                            dispatch_app.clone(),
                            dispatch_registry,
                            dispatch_mcp_state,
                        ));
                        let dispatcher = CoordinatorNotificationDispatcher::new(
                            dispatch_repository.clone(),
                            adapter,
                        );
                        let _ = dispatcher.dispatch_pending(&dispatch_window_label).await;
                        let service = OrchestrationService::new(
                            dispatch_repository,
                            TauriOrchestrationEventSink::new(dispatch_app.clone()),
                        );
                        emit_latest_runtime_event(
                            &dispatch_app,
                            &service,
                            &dispatch_window_label,
                            "notificationDelivery",
                        );
                    });
                    tool_success(json!({
                        "report": report,
                        "notifications": notifications,
                        "nextReadyTaskId": next_task_id
                    }))
                }
                Err(error) => domain_error(error),
            }
        }
        _ => tool_error(
            "unsupportedTool",
            "This orchestration operation is not available yet.",
            true,
        ),
    }
}

fn emit_latest_runtime_event(
    app: &AppHandle,
    service: &OrchestrationService<JsonOrchestrationRepository, TauriOrchestrationEventSink>,
    window_label: &str,
    reason: &str,
) {
    if let Ok(Some(session)) = service.get_for_window(window_label) {
        let _ = TauriOrchestrationEventSink::new(app.clone()).emit(
            window_label,
            OrchestrationEvent {
                workspace_id: session.id,
                revision: session.revision,
                reason: reason.into(),
                task_id: None,
                node_id: None,
            },
        );
    }
}

async fn launch_existing_task(
    app: &AppHandle,
    registry: &AppState,
    mcp_state: &McpServerState,
    service: &OrchestrationService<JsonOrchestrationRepository, TauriOrchestrationEventSink>,
    window_label: &str,
    task_id: &str,
) -> Result<Value, OrchestrationError> {
    let snapshot = service.get_for_window(window_label)?.ok_or_else(|| {
        OrchestrationError::new(
            crate::domain::agent_orchestration::OrchestrationErrorCode::NotFound,
            "Workspace is unavailable.",
        )
    })?;
    let task = snapshot
        .tasks
        .iter()
        .find(|task| task.id == task_id)
        .ok_or_else(|| {
            OrchestrationError::new(
                crate::domain::agent_orchestration::OrchestrationErrorCode::NotFound,
                "Task was not found.",
            )
        })?;
    if !matches!(
        task.status,
        crate::domain::agent_orchestration::TaskStatus::Ready
            | crate::domain::agent_orchestration::TaskStatus::Running
    ) {
        return Err(OrchestrationError::new(
            crate::domain::agent_orchestration::OrchestrationErrorCode::InvalidTransition,
            "Only a ready task can be assigned to a worker.",
        ));
    }
    let node = task
        .assigned_node_id
        .as_ref()
        .and_then(|node_id| snapshot.nodes.iter().find(|node| node.id == *node_id))
        .ok_or_else(|| {
            OrchestrationError::new(
                crate::domain::agent_orchestration::OrchestrationErrorCode::NotFound,
                "Assigned child node was not found.",
            )
        })?;
    if let Some(run_id) = node.current_run_id.as_ref() {
        return Ok(json!({
            "taskId": task.id,
            "nodeId": node.id,
            "runId": run_id,
            "alreadyAssigned": true
        }));
    }
    let adapter = AcpAgentWorkerAdapter::new(TauriAcpWorkerRuntime::new(
        app.clone(),
        registry.clone(),
        mcp_state.clone(),
    ));
    let launch = adapter
        .start_worker(WorkerAssignment {
            workspace_id: snapshot.id.clone(),
            window_label: window_label.into(),
            worktree_path: snapshot.worktree_path.clone(),
            node_id: node.id.clone(),
            task_id: task.id.clone(),
            attempt: task.attempt,
            planned_run_id: uuid::Uuid::new_v4().to_string(),
            role: node.role.clone(),
            objective: task.objective.clone(),
            constraints: task.constraints.clone(),
            expected_result: task.expected_result.clone(),
            runtime_profile: WorkerRuntimeProfile {
                agent_profile_id: std::env::var("AW_ORCHESTRATION_AGENT_PROFILE")
                    .unwrap_or_else(|_| "codex".into()),
                provider_id: "acp".into(),
                model_id: None,
                access_policy: AccessPolicy::ReadOnly,
                supports_read_only: true,
            },
            mcp_capability: String::new(),
        })
        .await?;
    match launch {
        StartWorkerOutcome::Started { run_id } => {
            service.bind_child_run(window_label, &task.id, &node.id, &run_id)?;
            Ok(json!({
                "taskId": task.id,
                "nodeId": node.id,
                "runId": run_id,
                "executionStatus": "active"
            }))
        }
        other => Ok(json!({
            "taskId": task.id,
            "nodeId": node.id,
            "launch": other
        })),
    }
}

fn is_allowed(actor_kind: CapabilityActorKind, name: &str) -> bool {
    match actor_kind {
        CapabilityActorKind::Coordinator => COORDINATOR_TOOLS.contains(&name),
        CapabilityActorKind::Child => CHILD_TOOLS.contains(&name),
        CapabilityActorKind::LegacyRun => false,
    }
}

fn parse<T: for<'de> Deserialize<'de>>(arguments: Option<&Value>) -> Result<T, OrchestrationError> {
    serde_json::from_value(arguments.cloned().unwrap_or(Value::Null)).map_err(|error| {
        OrchestrationError::new(
            crate::domain::agent_orchestration::OrchestrationErrorCode::InvalidInput,
            format!("Invalid tool arguments: {error}"),
        )
    })
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

fn domain_error(error: OrchestrationError) -> Value {
    let code = serde_json::to_value(error.code)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "internalError".into());
    tool_error(code, error.message, error.retryable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::mcp::capability_registry::CapabilityActorKind;

    #[test]
    fn exposes_role_specific_tool_sets() {
        let coordinator = tool_definitions(CapabilityActorKind::Coordinator);
        let child = tool_definitions(CapabilityActorKind::Child);

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
    fn structured_errors_do_not_claim_success() {
        let result = tool_error("forbiddenActor", "Only Main can create child tasks.", false);
        assert_eq!(result["isError"], true);
        assert_eq!(result["structuredContent"]["code"], "forbiddenActor");
    }

    #[test]
    fn child_report_and_coordinator_send_contracts_are_explicit() {
        let coordinator = tool_definitions(CapabilityActorKind::Coordinator);
        let send = coordinator
            .iter()
            .find(|tool| tool["name"] == SEND_CHILD_MESSAGE_TOOL)
            .unwrap();
        assert_eq!(
            send["inputSchema"]["required"],
            json!(["requestId", "taskId", "message"])
        );

        let child = tool_definitions(CapabilityActorKind::Child);
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
