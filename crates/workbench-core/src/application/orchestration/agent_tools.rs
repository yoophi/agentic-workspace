//! agent orchestration 도구(041 US2): AW MCP `orchestration_tool.rs`의 16개 도구 로직을 core로 옮겼다. 달라진 점은
//! 하나다 — 부른 run의 역할을 토큰 주장이 아니라 **서버 상태**로 판정한다(research R7). 결과·오류는 오늘 도구의
//! `structuredContent`와 같다(`ToolError{code, message, retryable}`).
//!
//! 어떤 lock도 await를 사이에 두고 쥐지 않는다(research R2): 저장소는 단계마다 `blocking`, 결과 대기는 작업 영역
//! revision watch로 깨어난다(research R12 — 대기 중 자식 보고가 막히지 않고 즉시 깨운다, 설계 리뷰 H2).

use std::sync::Arc;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    application::orchestration::{
        command_service::DeliverTaskCommandRequest,
        runtime::{OrchestrationFailure, OrchestrationRuntime},
        scheduler::LeaseOutcome,
        service::{CreateChildTaskRequest, ReportTaskRequest, TaskActionRequest},
    },
    domain::agent_orchestration::{
        AgentNodeCreator, AgentNodeKind, AgentRoleProfile, ArtifactReference,
        CoordinatorGenerationStatus, OrchestrationError, OrchestrationSession, PromptDelivery,
        TaskCommandKind, TaskCommandSource, TaskFinding, TaskReportType, TaskStatus,
    },
    ports::{
        agent_worker::{AgentWorkerPort, StartWorkerOutcome, WorkerAssignment},
        orchestration_repository::OrchestrationRepository,
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

pub const COORDINATOR_TOOLS: &[&str] = &[
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

pub const CHILD_TOOLS: &[&str] = &[
    GET_OWN_TASK_TOOL,
    REPORT_PROGRESS_TOOL,
    REPORT_RESULT_TOOL,
    REQUEST_PARENT_INPUT_TOOL,
    REPORT_BLOCKED_TOOL,
    SEND_PARENT_MESSAGE_TOOL,
];

/// 오늘 도구 오류(`structuredContent`)와 같은 모양.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ToolError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl ToolError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }
}

impl From<OrchestrationError> for ToolError {
    /// 오늘 `domain_error`: 코드는 도메인 코드의 camelCase 이름.
    fn from(error: OrchestrationError) -> Self {
        let code = serde_json::to_value(error.code)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "internalError".into());
        Self::new(code, error.message, error.retryable)
    }
}

impl From<OrchestrationFailure> for ToolError {
    fn from(failure: OrchestrationFailure) -> Self {
        match failure {
            OrchestrationFailure::Domain(error) => error.into(),
            OrchestrationFailure::Plain(message) | OrchestrationFailure::Forbidden(message) => {
                Self::new("workerUnavailable", message, true)
            }
        }
    }
}

pub fn forbidden_actor() -> ToolError {
    ToolError::new(
        "forbiddenActor",
        "The authenticated agent role cannot call this tool.",
        false,
    )
}

pub fn not_bound() -> ToolError {
    ToolError::new(
        "scopeMismatch",
        "This run is not bound to an orchestration workspace.",
        false,
    )
}

/// 서버 상태로 도출한 역할(research R7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRole {
    Coordinator {
        bench_id: String,
        workspace_id: String,
        generation_id: String,
    },
    Child {
        bench_id: String,
        workspace_id: String,
        node_id: String,
        task_id: String,
    },
}

impl AgentRole {
    pub fn bench_id(&self) -> &str {
        match self {
            Self::Coordinator { bench_id, .. } | Self::Child { bench_id, .. } => bench_id,
        }
    }

    pub fn allows(&self, tool: &str) -> bool {
        match self {
            Self::Coordinator { .. } => COORDINATOR_TOOLS.contains(&tool),
            Self::Child { .. } => CHILD_TOOLS.contains(&tool),
        }
    }
}

/// 역할 판정 결과. 작업 영역에 속하지만 그 작업 영역이 어느 작업대에도 묶이지 않았으면 `Unbound`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleLookup {
    Role(AgentRole),
    Unbound,
    None,
}

/// coordinator = 활성 세대(`activeCoordinatorGenerationId`)가 Active이고 그 run이 주체 run. 자식 = coordinator가
/// 만든 자식 노드의 현재 run(기동 중 예정 run 포함)이고 배정 과제가 있음. 수동 채택 자식은 역할이 없다(오늘처럼).
pub fn resolve_role(
    sessions: &[OrchestrationSession],
    run_id: &str,
    launching: Option<(String, String, String)>,
) -> RoleLookup {
    for session in sessions {
        let coordinator = session
            .active_coordinator_generation_id
            .as_ref()
            .and_then(|id| {
                session
                    .generations
                    .iter()
                    .find(|generation| &generation.id == id)
            })
            .filter(|generation| {
                generation.status == CoordinatorGenerationStatus::Active
                    && generation.run_id == run_id
            });
        if let Some(generation) = coordinator {
            return match &session.bound_bench_id {
                Some(bench_id) => RoleLookup::Role(AgentRole::Coordinator {
                    bench_id: bench_id.clone(),
                    workspace_id: session.id.clone(),
                    generation_id: generation.id.clone(),
                }),
                None => RoleLookup::Unbound,
            };
        }
        let child = session.nodes.iter().find(|node| {
            node.kind == AgentNodeKind::Child
                && node.created_by == AgentNodeCreator::Coordinator
                && node.assigned_task_id.is_some()
                && (node.current_run_id.as_deref() == Some(run_id)
                    || launching.as_ref().is_some_and(|(workspace, node_id, _)| {
                        workspace == &session.id && node_id == &node.id
                    }))
        });
        if let Some(node) = child {
            let task_id = node.assigned_task_id.clone().unwrap_or_default();
            return match &session.bound_bench_id {
                Some(bench_id) => RoleLookup::Role(AgentRole::Child {
                    bench_id: bench_id.clone(),
                    workspace_id: session.id.clone(),
                    node_id: node.id.clone(),
                    task_id,
                }),
                None => RoleLookup::Unbound,
            };
        }
    }
    RoleLookup::None
}

impl OrchestrationRuntime {
    pub fn agent_role(&self, run_id: &str) -> RoleLookup {
        let launching = self.launching_child(run_id);
        match self.repository().snapshot() {
            Ok(sessions) => resolve_role(&sessions, run_id, launching),
            Err(_) => RoleLookup::None,
        }
    }
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
    findings: Vec<TaskFinding>,
    #[serde(default)]
    artifact_refs: Vec<ArtifactReference>,
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

fn parse<T: for<'de> Deserialize<'de>>(arguments: &Value) -> Result<T, ToolError> {
    serde_json::from_value(arguments.clone()).map_err(|error| {
        OrchestrationError::new(
            crate::domain::agent_orchestration::OrchestrationErrorCode::InvalidInput,
            format!("Invalid tool arguments: {error}"),
        )
        .into()
    })
}

fn unavailable_workspace() -> ToolError {
    ToolError::new(
        "workspaceNotBootstrapped",
        "Workspace is unavailable.",
        true,
    )
}

/// 도구 하나를 실행한다. `role`은 이미 `allows(tool)`를 통과했다.
pub async fn handle_tool(
    runtime: &Arc<OrchestrationRuntime>,
    run_id: &str,
    role: AgentRole,
    tool: &str,
    arguments: &Value,
) -> Result<Value, ToolError> {
    let bench = role.bench_id().to_owned();
    match (tool, &role) {
        (CREATE_CHILD_TASK_TOOL, AgentRole::Coordinator { generation_id, .. }) => {
            let input: CreateTaskInput = parse(arguments)?;
            let role_id = input.role.name.to_lowercase().replace(' ', "-");
            let profile = AgentRoleProfile::new(
                role_id,
                input.role.name,
                input.role.responsibility,
                input.role.expected_output,
            )?;
            let (b, generation) = (bench.clone(), generation_id.clone());
            let outcome = runtime
                .blocking(move |service| {
                    service.create_child_task(
                        &b,
                        &generation,
                        CreateChildTaskRequest {
                            request_id: input.request_id,
                            title: input.title,
                            role: profile,
                            objective: input.objective,
                            constraints: input.constraints,
                            expected_result: input.expected_result,
                            dependency_task_ids: input.dependency_task_ids,
                            preferred_node_id: input.preferred_node_id,
                        },
                    )
                })
                .await?;
            let snapshot = runtime
                .get(&bench)
                .await?
                .ok_or_else(unavailable_workspace)?;
            let task = snapshot
                .tasks
                .iter()
                .find(|task| task.id == outcome.task_id)
                .ok_or_else(|| {
                    ToolError::new("unknownTask", "Created task is unavailable.", true)
                })?;
            let node = snapshot
                .nodes
                .iter()
                .find(|node| node.id == outcome.node_id)
                .ok_or_else(|| {
                    ToolError::new("unknownNode", "Created child node is unavailable.", true)
                })?;
            if let LeaseOutcome::Queued { position } = runtime.scheduler().acquire(&task.id)? {
                return Ok(json!({
                    "taskId": outcome.task_id,
                    "nodeId": outcome.node_id,
                    "status": outcome.status,
                    "executionStatus": "starting",
                    "queued": true,
                    "queuePosition": position
                }));
            }
            match runtime
                .start_child(&bench, &snapshot, &task.id, &node.id)
                .await
            {
                Ok(StartWorkerOutcome::Started { run_id }) => Ok(json!({
                    "taskId": outcome.task_id,
                    "nodeId": outcome.node_id,
                    "status": outcome.status,
                    "executionStatus": "active",
                    "runId": run_id
                })),
                Ok(other) => Ok(json!({
                    "taskId": outcome.task_id,
                    "nodeId": outcome.node_id,
                    "status": outcome.status,
                    "executionStatus": "starting",
                    "launch": other
                })),
                Err(error) => {
                    let _ = runtime.scheduler().release(&task.id);
                    Err(error)
                }
            }
        }
        (
            LIST_CHILD_TASKS_TOOL | WAIT_CHILD_TASKS_TOOL | COLLECT_CHILD_RESULTS_TOOL,
            AgentRole::Coordinator { generation_id, .. },
        ) => {
            let input: CoordinatorTaskInput = parse(arguments).unwrap_or_default();
            let list = |runtime: &Arc<OrchestrationRuntime>| {
                let (b, generation) = (bench.clone(), generation_id.clone());
                let runtime = Arc::clone(runtime);
                async move {
                    runtime
                        .blocking(move |service| service.list_child_tasks(&b, &generation))
                        .await
                }
            };
            if tool == WAIT_CHILD_TASKS_TOOL {
                // research R12: 먼저 구독하고 그다음 읽는다 — 읽은 뒤·기다리기 전에 온 보고도 깨운다. lock은 쥐지 않는다.
                let AgentRole::Coordinator { workspace_id, .. } = &role else {
                    return Err(forbidden_actor());
                };
                let mut changes = runtime.revisions().subscribe(workspace_id);
                let timeout = std::time::Duration::from_millis(
                    input.timeout_ms.unwrap_or(30_000).min(30_000),
                );
                let deadline = tokio::time::Instant::now() + timeout;
                loop {
                    let tasks = list(runtime).await?;
                    let done = tasks
                        .iter()
                        .filter(|task| {
                            input.task_ids.is_empty() || input.task_ids.contains(&task.id)
                        })
                        .all(|task| task.status.is_terminal() || task.status == TaskStatus::Failed);
                    if done {
                        break;
                    }
                    match tokio::time::timeout_at(deadline, changes.changed()).await {
                        Ok(Ok(())) => continue,
                        // 채널이 닫혔으면 다시 구독한다(구독자가 없어 정리된 경우).
                        Ok(Err(_)) => changes = runtime.revisions().subscribe(workspace_id),
                        Err(_) => return Ok(json!({ "timedOut": true, "tasks": tasks })),
                    }
                }
            }
            let tasks = list(runtime).await?;
            let reports = if tool == COLLECT_CHILD_RESULTS_TOOL {
                let (b, generation, ids) =
                    (bench.clone(), generation_id.clone(), input.task_ids.clone());
                runtime
                    .blocking(move |service| service.collect_child_results(&b, &generation, &ids))
                    .await?
            } else {
                Vec::new()
            };
            Ok(json!({ "timedOut": false, "tasks": tasks, "reports": reports }))
        }
        (ASSIGN_CHILD_TASK_TOOL, AgentRole::Coordinator { .. }) => {
            let input: CoordinatorTaskInput = parse(arguments)?;
            let task_id = input
                .task_id
                .ok_or_else(|| ToolError::new("invalidInput", "taskId is required.", false))?;
            match runtime.scheduler().acquire(&task_id)? {
                LeaseOutcome::Queued { position } => Ok(json!({
                    "taskId": task_id,
                    "queued": true,
                    "queuePosition": position
                })),
                LeaseOutcome::Acquired => {
                    match launch_existing_task(runtime, &bench, &task_id).await {
                        Ok(value) => Ok(value),
                        Err(error) => {
                            let _ = runtime.scheduler().release(&task_id);
                            Err(error)
                        }
                    }
                }
            }
        }
        (
            SEND_CHILD_MESSAGE_TOOL
            | INTERRUPT_CHILD_TASK_TOOL
            | CANCEL_CHILD_TASK_TOOL
            | RETRY_CHILD_TASK_TOOL
            | REASSIGN_CHILD_TASK_TOOL,
            AgentRole::Coordinator { .. },
        ) => coordinator_task_command(runtime, &bench, tool, arguments).await,
        (GET_OWN_TASK_TOOL, AgentRole::Child { task_id, .. }) => {
            let session = runtime
                .get(&bench)
                .await?
                .ok_or_else(unavailable_workspace)?;
            session
                .tasks
                .into_iter()
                .find(|task| &task.id == task_id)
                .map(|task| json!(task))
                .ok_or_else(|| ToolError::new("unknownTask", "Assigned task was not found.", false))
        }
        (
            REPORT_PROGRESS_TOOL
            | REPORT_RESULT_TOOL
            | REQUEST_PARENT_INPUT_TOOL
            | REPORT_BLOCKED_TOOL
            | SEND_PARENT_MESSAGE_TOOL,
            AgentRole::Child {
                task_id, node_id, ..
            },
        ) => {
            let input: ReportInput = parse(arguments)?;
            let report_type = match tool {
                REPORT_RESULT_TOOL => TaskReportType::Result,
                REQUEST_PARENT_INPUT_TOOL => TaskReportType::InputRequest,
                REPORT_BLOCKED_TOOL => TaskReportType::Blocked,
                SEND_PARENT_MESSAGE_TOOL => TaskReportType::Message,
                _ => TaskReportType::Progress,
            };
            let summary = match input.question {
                Some(question) => format!("{}\n\n{}", input.summary, question),
                None => input.summary,
            };
            // research R18: 보고자 run은 입력이 아니라 principal run이다.
            let request = ReportTaskRequest {
                request_id: input.request_id,
                task_id: task_id.clone(),
                reporter_node_id: node_id.clone(),
                reporter_run_id: run_id.to_owned(),
                report_type,
                progress_percent: input.progress_percent,
                summary,
                findings: input.findings,
                artifact_refs: input.artifact_refs,
                unresolved: input.unresolved,
                confidence: input.confidence,
            };
            let b = bench.clone();
            let report = runtime
                .blocking(move |service| service.report_task(&b, request))
                .await?;
            let notifications = runtime
                .get(&bench)
                .await
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
                .then(|| runtime.scheduler().release(&report.task_id).ok().flatten())
                .flatten();
            let dispatcher = runtime.dispatcher();
            let dispatch_runtime = Arc::clone(runtime);
            let dispatch_bench = bench.clone();
            tokio::spawn(async move {
                let _ = dispatcher.dispatch_pending(&dispatch_bench).await;
                dispatch_runtime
                    .emit_runtime_update_for(&dispatch_bench, "notificationDelivery")
                    .await;
            });
            Ok(json!({
                "report": report,
                "notifications": notifications,
                "nextReadyTaskId": next_task_id
            }))
        }
        _ => Err(forbidden_actor()),
    }
}

async fn coordinator_task_command(
    runtime: &Arc<OrchestrationRuntime>,
    bench: &str,
    tool: &str,
    arguments: &Value,
) -> Result<Value, ToolError> {
    let input: CoordinatorTaskInput = parse(arguments)?;
    let task_id = input
        .task_id
        .clone()
        .ok_or_else(|| ToolError::new("invalidInput", "taskId is required.", false))?;
    let snapshot = runtime
        .get(bench)
        .await?
        .ok_or_else(unavailable_workspace)?;
    let task = snapshot
        .tasks
        .iter()
        .find(|task| task.id == task_id)
        .ok_or_else(|| ToolError::new("unknownTask", "Task was not found.", false))?;
    let node = task
        .assigned_node_id
        .as_ref()
        .and_then(|node_id| snapshot.nodes.iter().find(|node| node.id == *node_id))
        .ok_or_else(|| {
            ToolError::new("unknownNode", "Assigned child node was not found.", false)
        })?;
    let request = TaskActionRequest {
        request_id: input
            .request_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        task_id: task_id.clone(),
        expected_revision: snapshot.revision,
        message: input.message.clone(),
        target_node_id: input.target_node_id.clone(),
    };
    if matches!(tool, RETRY_CHILD_TASK_TOOL | REASSIGN_CHILD_TASK_TOOL)
        && node.current_run_id.is_some()
    {
        runtime.stop_existing_task_worker(bench, &task_id).await?;
    }
    if matches!(tool, RETRY_CHILD_TASK_TOOL | REASSIGN_CHILD_TASK_TOOL) {
        let b = bench.to_owned();
        let retry = tool == RETRY_CHILD_TASK_TOOL;
        let session = runtime
            .blocking(move |service| {
                if retry {
                    service.retry_task(&b, request)
                } else {
                    service.reassign_task(&b, request)
                }
            })
            .await?;
        return match runtime.scheduler().acquire(&task_id)? {
            LeaseOutcome::Queued { position } => Ok(json!({
                "workspace": session,
                "accepted": true,
                "queued": true,
                "queuePosition": position
            })),
            LeaseOutcome::Acquired => {
                let launch = launch_existing_task(runtime, bench, &task_id).await?;
                Ok(json!({ "workspace": session, "accepted": true, "launch": launch }))
            }
        };
    }
    if tool == CANCEL_CHILD_TASK_TOOL && node.current_run_id.is_none() {
        let b = bench.to_owned();
        let session = runtime
            .blocking(move |service| service.cancel_task(&b, request))
            .await?;
        return Ok(json!({ "workspace": session, "accepted": true, "runtimeCommand": null }));
    }
    let kind = if tool == SEND_CHILD_MESSAGE_TOOL {
        if input.message.is_none() {
            return Err(ToolError::new(
                "invalidInput",
                "message is required.",
                false,
            ));
        }
        if task.status == TaskStatus::InputRequired {
            TaskCommandKind::InputResponse
        } else {
            TaskCommandKind::Message
        }
    } else if tool == INTERRUPT_CHILD_TASK_TOOL {
        TaskCommandKind::Interrupt
    } else {
        TaskCommandKind::Cancel
    };
    let input_report_id = (kind == TaskCommandKind::InputResponse)
        .then(|| {
            snapshot
                .reports
                .iter()
                .rev()
                .find(|report| {
                    report.task_id == task_id && report.report_type == TaskReportType::InputRequest
                })
                .map(|report| report.id.clone())
        })
        .flatten();
    let command = runtime
        .command_service()
        .deliver(
            bench,
            DeliverTaskCommandRequest {
                request_id: request.request_id,
                task_id: task_id.clone(),
                kind,
                message: input.message,
                input_report_id,
                delivery: PromptDelivery::Queue,
                source: TaskCommandSource::Coordinator,
                expected_task_revision: Some(task.revision),
            },
        )
        .await?;
    runtime
        .emit_runtime_update_for(bench, "taskCommandDelivery")
        .await;
    if tool == CANCEL_CHILD_TASK_TOOL {
        let next_task_id = runtime.scheduler().release(&task_id).ok().flatten();
        return Ok(json!({ "command": command, "nextReadyTaskId": next_task_id }));
    }
    Ok(json!(command))
}

/// 오늘 `launch_existing_task`.
async fn launch_existing_task(
    runtime: &Arc<OrchestrationRuntime>,
    bench: &str,
    task_id: &str,
) -> Result<Value, ToolError> {
    use crate::domain::agent_orchestration::OrchestrationErrorCode as C;
    let snapshot = runtime.get(bench).await?.ok_or_else(|| {
        ToolError::from(OrchestrationError::new(
            C::NotFound,
            "Workspace is unavailable.",
        ))
    })?;
    let task = snapshot
        .tasks
        .iter()
        .find(|task| task.id == task_id)
        .ok_or_else(|| {
            ToolError::from(OrchestrationError::new(C::NotFound, "Task was not found."))
        })?;
    if !matches!(task.status, TaskStatus::Ready | TaskStatus::Running) {
        return Err(OrchestrationError::new(
            C::InvalidTransition,
            "Only a ready task can be assigned to a worker.",
        )
        .into());
    }
    let node = task
        .assigned_node_id
        .as_ref()
        .and_then(|node_id| snapshot.nodes.iter().find(|node| node.id == *node_id))
        .ok_or_else(|| {
            ToolError::from(OrchestrationError::new(
                C::NotFound,
                "Assigned child node was not found.",
            ))
        })?;
    if let Some(run_id) = node.current_run_id.as_ref() {
        return Ok(json!({
            "taskId": task.id,
            "nodeId": node.id,
            "runId": run_id,
            "alreadyAssigned": true
        }));
    }
    match runtime
        .start_child(bench, &snapshot, &task.id, &node.id)
        .await?
    {
        StartWorkerOutcome::Started { run_id } => Ok(json!({
            "taskId": task.id,
            "nodeId": node.id,
            "runId": run_id,
            "executionStatus": "active"
        })),
        other => Ok(json!({ "taskId": task.id, "nodeId": node.id, "launch": other })),
    }
}

impl OrchestrationRuntime {
    /// 자식 run을 기동하고(예정 run id를 먼저 기억해 자식의 첫 턴 도구 호출을 허용 — 설계 리뷰 H6) 성공하면 묶는다.
    pub async fn start_child(
        self: &Arc<Self>,
        bench: &str,
        snapshot: &OrchestrationSession,
        task_id: &str,
        node_id: &str,
    ) -> Result<StartWorkerOutcome, ToolError> {
        let task = snapshot
            .tasks
            .iter()
            .find(|task| task.id == task_id)
            .ok_or_else(|| ToolError::new("unknownTask", "Task was not found.", false))?;
        let node = snapshot
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .ok_or_else(|| {
                ToolError::new("unknownNode", "Assigned child node was not found.", false)
            })?;
        let planned_run_id = uuid::Uuid::new_v4().to_string();
        self.remember_launching(&planned_run_id, &snapshot.id, &node.id, &task.id);
        let outcome = self
            .worker()
            .start_worker(WorkerAssignment {
                workspace_id: snapshot.id.clone(),
                bench_id: bench.to_owned(),
                worktree_path: snapshot.worktree_path.clone(),
                node_id: node.id.clone(),
                task_id: task.id.clone(),
                attempt: task.attempt,
                planned_run_id: planned_run_id.clone(),
                role: node.role.clone(),
                objective: task.objective.clone(),
                constraints: task.constraints.clone(),
                expected_result: task.expected_result.clone(),
                runtime_profile: self.child_runtime_profile(),
                mcp_capability: String::new(),
            })
            .await;
        let result = match outcome {
            Ok(StartWorkerOutcome::Started { run_id }) => {
                let (b, t, n, r) = (
                    bench.to_owned(),
                    task.id.clone(),
                    node.id.clone(),
                    run_id.clone(),
                );
                self.blocking(move |service| service.bind_child_run(&b, &t, &n, &r))
                    .await
                    .map(|_| StartWorkerOutcome::Started { run_id })
                    .map_err(ToolError::from)
            }
            Ok(other) => Ok(other),
            Err(error) => Err(error.into()),
        };
        self.forget_launching(&planned_run_id);
        result
    }
}
