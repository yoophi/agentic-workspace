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
        runtime::{
            LaunchInFlight, LaunchPoint, OrchestrationFailure, OrchestrationResult,
            OrchestrationRuntime, PendingRevert, StorePoint, StoreProbe,
        },
        scheduler::{HoldOutcome, SlotHold},
        service::{
            ChildRunReservation, CreateChildTaskRequest, ReportTaskRequest, TaskActionRequest,
        },
    },
    application::work_gate::{LaunchCancel, LaunchCancelled, LaunchState, MESSAGE_STOPPING},
    domain::agent_orchestration::{
        AgentNodeCreator, AgentNodeKind, AgentRoleProfile, ArtifactReference,
        CoordinatorGenerationStatus, OrchestrationError, OrchestrationSession, PromptDelivery,
        TaskCommandKind, TaskCommandSource, TaskFinding, TaskReportType, TaskStatus,
    },
    ports::{
        agent_worker::{StartWorkerOutcome, WorkerAssignment},
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
    /// 서버 상태의 역할(research R7). 041 Codex 리뷰: 영속 기록만 보면 작업대를 닫고 다른 작업대가 재개했을 때
    /// 끝난 이전 run(폐기되지 않은 토큰)이 역할을 되찾는다. 그래서 역할은 **지금 묶인 작업대가 소유한 살아 있는 run**
    /// 이거나 그 작업대가 기동 중인 자식 run일 때만 준다.
    pub async fn agent_role(&self, run_id: &str) -> RoleLookup {
        let launching = self.launching_child(run_id);
        let lookup = match self.repository().snapshot() {
            Ok(sessions) => resolve_role(&sessions, run_id, launching.clone()),
            Err(_) => RoleLookup::None,
        };
        let RoleLookup::Role(role) = &lookup else {
            return lookup;
        };
        if launching.is_some() {
            return lookup;
        }
        let live_owner = self.benches.engine.active_owner_of(run_id).await;
        if live_owner.as_deref() == Some(role.bench_id()) {
            lookup
        } else {
            RoleLookup::None
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
            // 같은 requestId의 재시도는 이미 첫 turn에서 끝난 task를 다시 기동하지 않는다. 첫 create 호출도
            // bind 전에 terminal 보고가 끝난 경우 아래 start_child 정리 뒤 같은 성공 모양으로 돌아온다.
            if task.status.is_terminal() {
                return Ok(json!({
                    "taskId": outcome.task_id,
                    "nodeId": outcome.node_id,
                    "status": task.status,
                    "executionStatus": node.execution_status
                }));
            }
            let hold = match runtime.scheduler().acquire_hold(&task.id)? {
                HoldOutcome::Queued { position } => {
                    return Ok(json!({
                        "taskId": outcome.task_id,
                        "nodeId": outcome.node_id,
                        "status": outcome.status,
                        "executionStatus": "starting",
                        "queued": true,
                        "queuePosition": position
                    }));
                }
                HoldOutcome::Acquired(hold) => hold,
            };
            // 이 시도의 보유는 `start_child`가 끝까지 책임진다(Codex r8).
            match runtime
                .start_child(&bench, &snapshot, &task.id, &node.id, hold)
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
                Err(error) if error.code == "invalidTransition" => {
                    // 시작 장벽 직후의 빠른 결과는 task·report·알림을 이미 커밋했다. terminal task에 늦게
                    // bind하려던 worker는 start_child가 취소·정리했으므로, 적용된 create를 실패로 돌려
                    // 호출자가 중복 task를 만들게 하지 말고 저장된 최종 상태를 성공으로 반환한다.
                    let current = runtime
                        .get(&bench)
                        .await?
                        .ok_or_else(unavailable_workspace)?;
                    let task = current.tasks.iter().find(|task| task.id == outcome.task_id);
                    let node = current.nodes.iter().find(|node| node.id == outcome.node_id);
                    match (task, node) {
                        (Some(task), Some(node)) if task.status.is_terminal() => Ok(json!({
                            "taskId": outcome.task_id,
                            "nodeId": outcome.node_id,
                            "status": task.status,
                            "executionStatus": node.execution_status
                        })),
                        _ => Err(error),
                    }
                }
                Err(error) => Err(error),
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
            ensure_assign_continues(runtime, &bench, &task_id).await?;
            match runtime.scheduler().acquire_hold(&task_id)? {
                HoldOutcome::Queued { position } => Ok(json!({
                    "taskId": task_id,
                    "queued": true,
                    "queuePosition": position
                })),
                // 이 시도의 보유는 `launch_existing_task`가 끝까지 책임진다(Codex r8).
                HoldOutcome::Acquired(hold) => {
                    launch_existing_task(runtime, &bench, &task_id, hold).await
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
            // R14 표 6': 보고 호출이 돌아가기 전에 N-notify를 잡아 전달기로 넘긴다 — 전달기가 처음 돌기 전에 보고
            // 호출과 자식 turn이 끝나도 활동이 0이 되지 않는다.
            let first = runtime.begin_notify_attempt();
            runtime.spawn_notification_pass_with(&bench, "notificationDelivery", first);
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
        return match runtime.scheduler().acquire_hold(&task_id)? {
            HoldOutcome::Queued { position } => Ok(json!({
                "workspace": session,
                "accepted": true,
                "queued": true,
                "queuePosition": position
            })),
            HoldOutcome::Acquired(hold) => {
                let launch = launch_existing_task(runtime, bench, &task_id, hold).await?;
                Ok(json!({ "workspace": session, "accepted": true, "launch": launch }))
            }
        };
    }
    // 044 R14 표 5: 노드에 예정 run이 있으면 기동 중일 수 있다. 토큰이 아직 `Pending`이면 여기서 실행을 막고 task만
    // 취소한다(기동 경로가 준비한 run을 취소한다). 이미 실행이 허용됐으면 아래 오늘 경로가 실제 run을 취소한다.
    if tool == CANCEL_CHILD_TASK_TOOL
        && node.current_run_id.is_some()
        && runtime.prevent_task_launch(&task_id) == LaunchCancel::Prevented
    {
        let b = bench.to_owned();
        let session = runtime
            .blocking(move |service| service.cancel_launching_task(&b, request))
            .await?;
        let next_task_id = runtime.scheduler().release(&task_id).ok().flatten();
        return Ok(json!({
            "workspace": session, "accepted": true, "runtimeCommand": null,
            "nextReadyTaskId": next_task_id
        }));
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

/// 오늘 `launch_existing_task`. `hold`(이 시도의 scheduler 보유)는 여기서 끝까지 책임진다(Codex r8): 기동으로 넘기거나,
/// 기동하지 않고 끝나면 놓는다.
async fn launch_existing_task(
    runtime: &Arc<OrchestrationRuntime>,
    bench: &str,
    task_id: &str,
    hold: SlotHold,
) -> Result<Value, ToolError> {
    use crate::domain::agent_orchestration::OrchestrationErrorCode as C;
    runtime
        .launch_probe(LaunchPoint::BeforeAssignSnapshot)
        .await;
    let located = async {
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
        let node_id = task
            .assigned_node_id
            .as_ref()
            .and_then(|node_id| snapshot.nodes.iter().find(|node| node.id == *node_id))
            .map(|node| node.id.clone())
            .ok_or_else(|| {
                ToolError::from(OrchestrationError::new(
                    C::NotFound,
                    "Assigned child node was not found.",
                ))
            })?;
        Ok::<_, ToolError>((snapshot, node_id))
    }
    .await;
    let (snapshot, node_id) = match located {
        Ok(located) => located,
        Err(error) => {
            let _ = runtime.scheduler().release_hold(hold);
            return Err(error);
        }
    };
    let node = snapshot
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .expect("located node");
    // 앞 기동을 되돌리는 중이면(Codex r7) 노드에 아직 남은 run은 곧 취소된다: 그 id를 `alreadyAssigned`로 주지 않는다.
    // 저장되지 않은 되돌리기가 있으면 한 번 다시 시도한다(Codex r8).
    if runtime.task_launch_rolling_back(task_id) {
        let _ = runtime.scheduler().release_hold(hold);
        runtime.retry_pending_revert(task_id).await;
        return Ok(
            json!({ "taskId": task_id, "nodeId": node.id, "launch": rolling_back_launch() }),
        );
    }
    if let Some(run_id) = node.current_run_id.as_ref() {
        // 이미 실행 중인 run(그 run의 자리가 따로 있다): 이 시도의 보유는 놓는다.
        let _ = runtime.scheduler().release_hold(hold);
        return Ok(json!({
            "taskId": task_id,
            "nodeId": node.id,
            "runId": run_id,
            "alreadyAssigned": true
        }));
    }
    match runtime
        .start_child(bench, &snapshot, task_id, &node_id, hold)
        .await?
    {
        StartWorkerOutcome::Started { run_id } => Ok(json!({
            "taskId": task_id,
            "nodeId": node_id,
            "runId": run_id,
            "executionStatus": "active"
        })),
        other => Ok(json!({ "taskId": task_id, "nodeId": node_id, "launch": other })),
    }
}

impl OrchestrationRuntime {
    /// 자식 run을 기동하고(예정 run id를 먼저 기억해 자식의 첫 턴 도구 호출을 허용 — 설계 리뷰 H6) 성공하면 묶는다.
    ///
    /// 044 R14 표 4·5·5'·6(시작 장벽): 기동 토큰(`Pending`)과 T-start를 G 아래에서 만들고, 같은 task의 기동은 하나로 묶는다
    /// (단일 비행). 저장소 RMW로 노드를 예약한 뒤 엔진을 **준비**만 하고(run은 registry에 있고 취소할 수 있음), G 아래에서
    /// 토큰을 `Registered`로 옮기며 T-start를 A-turn으로 인계한 다음에만 시작 장벽을 연다. 그 전에 온 취소는 토큰을
    /// `Cancelled`로 바꿔 실행을 막고, 이 경로는 준비한 run을 취소한다. future가 어느 지점에서 drop되어도 장벽은 닫힌 채
    /// drop되고(실행 0), 예약은 guard가 되돌린다.
    ///
    /// Codex r8: `hold`는 호출자가 이 시도로 얻은 scheduler 보유다. 이 함수(기동 guard)가 끝까지 책임진다 — 자기 run을
    /// 실행하면 실행 중 자리로 넘기고, 그 밖의 모든 끝(오류·실패 결과·다른 기동의 run 반환·abort 정리)에서는 이 보유만
    /// 놓는다. 호출자는 결과와 무관하게 보유를 다시 다루지 않는다.
    pub async fn start_child(
        self: &Arc<Self>,
        bench: &str,
        snapshot: &OrchestrationSession,
        task_id: &str,
        node_id: &str,
        hold: SlotHold,
    ) -> Result<StartWorkerOutcome, ToolError> {
        let give_back = |hold: SlotHold| {
            let _ = self.scheduler().release_hold(hold);
        };
        let Some(task) = snapshot.tasks.iter().find(|task| task.id == task_id) else {
            give_back(hold);
            return Err(ToolError::new("unknownTask", "Task was not found.", false));
        };
        let Some(node) = snapshot.nodes.iter().find(|node| node.id == node_id) else {
            give_back(hold);
            return Err(ToolError::new(
                "unknownNode",
                "Assigned child node was not found.",
                false,
            ));
        };
        let gate = Arc::clone(self.work_gate());
        let ticket = match gate.issue_launch() {
            Ok(ticket) => ticket,
            Err(_) => {
                give_back(hold);
                return Err(ToolError::new("serverStopping", MESSAGE_STOPPING, true));
            }
        };
        let token = ticket.token();
        let planned_run_id = uuid::Uuid::new_v4().to_string();
        match self.begin_task_launch(&task.id, token, &planned_run_id) {
            Ok(()) => {}
            // 같은 task의 기동이 진행 중이다: 그 run을 돌려준다(이 토큰은 drop으로 `Failed`, T-start 해제). 그 기동이 자기
            // 보유를 쥐므로 이 시도의 보유는 놓는다.
            Err(LaunchInFlight::Launching(existing)) => {
                give_back(hold);
                return Ok(StartWorkerOutcome::Started { run_id: existing });
            }
            // 앞 기동을 되돌리는 중이다(Codex r7): 곧 취소될 run id를 주지 않고 다시 시도하게 한다. 저장되지 않은 되돌리기가
            // 있으면 한 번 다시 시도한다(Codex r8). 앞 기동은 자기 보유를 쥐므로 이 시도의 보유만 놓는다.
            Err(LaunchInFlight::RollingBack) => {
                give_back(hold);
                self.retry_pending_revert(&task.id).await;
                return Ok(rolling_back_launch());
            }
        }
        let mut cleanup = LaunchCleanup {
            runtime: Arc::clone(self),
            bench: bench.to_owned(),
            workspace_id: snapshot.id.clone(),
            node_id: node.id.clone(),
            task_id: task.id.clone(),
            planned_run_id: planned_run_id.clone(),
            token,
            state: CleanupState::Reserving,
            pending: None,
            hold: Some(hold),
            handed_off: false,
        };
        // 엔진을 부르기 전에 예정 run을 노드의 현재 run으로 예약한다(비교 후 변경) — 첫 턴 보고가 현재 run의 보고로
        // 반영된다. 다른 배정이 먼저 예약했으면 그 run을 돌려준다.
        // 저장소 커밋 단계(예약·바인딩)는 blocking 작업이라 이 future가 await 중에 drop돼도 끝까지 커밋한다(Codex r6·r7).
        // 그래서 각 단계를 이 future와 따로 도는 소유 task로 돌리고 그 handle을 guard가 쥔다: 결과를 여기서 받으면 guard
        // 상태를 같은 poll에서 옮기고, 받기 전에 drop되면 guard가 handle을 넘겨받아 커밋을 기다린 뒤 되돌린다.
        cleanup.pending = Some(Pending::Reserve({
            let (b, t, n, r) = (
                bench.to_owned(),
                task.id.clone(),
                node.id.clone(),
                planned_run_id.clone(),
            );
            let probe = self.store_probe();
            self.spawn_store(move |service| {
                store_point(&probe, StorePoint::ReserveBeforeCommit);
                let reserved = service.reserve_child_run(&b, &t, &n, &r);
                store_point(&probe, StorePoint::ReserveAfterCommit);
                reserved
            })
        }));
        let joined = match cleanup.pending.as_mut() {
            Some(Pending::Reserve(handle)) => handle.await,
            _ => unreachable!("the reservation task was just spawned"),
        };
        cleanup.pending = None;
        match joined {
            Ok(Ok(ChildRunReservation::Reserved)) => cleanup.state = CleanupState::Reserved,
            Ok(Ok(ChildRunReservation::Existing(run_id))) => {
                cleanup.settled();
                return Ok(StartWorkerOutcome::Started { run_id });
            }
            Ok(Err(error)) => {
                cleanup.settled();
                return Err(error.into());
            }
            Err(error) => {
                // 예약 task가 끝나지 못했다(panic 등): 커밋 여부를 모르니 조건부로 되돌린다(이 기동의 run일 때만 푼다).
                cleanup.state = CleanupState::Reserved;
                cleanup.fail().await;
                return Err(OrchestrationFailure::Plain(error.to_string()).into());
            }
        }
        self.remember_launching(&planned_run_id, &snapshot.id, &node.id, &task.id);
        self.launch_probe(LaunchPoint::BeforePrepare).await;
        if gate.launch_state(token) == Some(LaunchState::Cancelled) {
            drop(ticket);
            cleanup.fail().await;
            return Ok(cancelled_launch());
        }
        let (open_gate, start_gate) = tokio::sync::oneshot::channel();
        let outcome = self
            .worker()
            .prepare_worker(
                WorkerAssignment {
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
                },
                start_gate,
            )
            .await;
        let run_id = match outcome {
            Ok(StartWorkerOutcome::Started { run_id }) => run_id,
            Ok(other) => {
                drop(ticket);
                cleanup.fail().await;
                return Ok(other);
            }
            Err(error) => {
                drop(ticket);
                cleanup.fail().await;
                return Err(error.into());
            }
        };
        cleanup.state = CleanupState::Prepared;
        self.launch_probe(LaunchPoint::AfterPrepare).await;
        // 선형화 지점(G 아래): `Pending`이면 `Registered`로 바꾸고 T-start를 A-turn으로 인계한다.
        let turn = match gate.register_launch(ticket, &run_id) {
            Ok(turn) => turn,
            Err(LaunchCancelled) => {
                // 5': 그 전에 취소됐다 — 장벽을 닫은 채 drop하고 준비한 run을 취소한다(실행 0).
                drop(open_gate);
                cleanup.fail().await;
                return Ok(cancelled_launch());
            }
        };
        self.launch_probe(LaunchPoint::AfterRegister).await;
        // 엔진의 초기 순서 guard(A-turn)가 실행을 덮으므로 인계받은 예약은 장벽을 연 뒤 놓는다.
        let _ = open_gate.send(());
        drop(turn);
        self.launch_probe(LaunchPoint::AfterOpen).await;
        // 바인딩(task `Running` + 노드 run)도 소유 task로 커밋한다(Codex r7): 결과를 받기 전에 drop되면 guard가 커밋을
        // 기다린 뒤 run을 취소하고 task·노드를 한 트랜잭션에서 되돌린다.
        cleanup.state = CleanupState::Binding;
        cleanup.pending = Some(Pending::Bind({
            let (b, t, n, r) = (
                bench.to_owned(),
                task.id.clone(),
                node.id.clone(),
                run_id.clone(),
            );
            let probe = self.store_probe();
            self.spawn_store(move |service| {
                store_point(&probe, StorePoint::BindBeforeCommit);
                let bound = service.bind_child_run(&b, &t, &n, &r);
                store_point(&probe, StorePoint::BindAfterCommit);
                bound
            })
        }));
        let bound = match cleanup.pending.as_mut() {
            Some(Pending::Bind(handle)) => handle.await,
            _ => unreachable!("the bind task was just spawned"),
        };
        cleanup.pending = None;
        match bound {
            Ok(Ok(_)) => {
                cleanup.launched();
                Ok(StartWorkerOutcome::Started { run_id })
            }
            Ok(Err(error)) => {
                // 6: 기동 중에 취소된 task(또는 사라진 작업 영역)에는 묶지 않고 run을 취소한다.
                cleanup.state = CleanupState::Prepared;
                cleanup.fail().await;
                Err(error.into())
            }
            Err(error) => {
                // 바인딩 task가 끝나지 못했다: 커밋 여부를 모르니 run을 취소하고 조건부로 되돌린다.
                cleanup.state = CleanupState::Prepared;
                cleanup.fail().await;
                Err(OrchestrationFailure::Plain(error.to_string()).into())
            }
        }
    }

    /// 자식 기동의 저장소 커밋 한 단계를 이 호출 future와 따로 도는 소유 task로 돌린다(abort돼도 커밋은 끝까지 가고, 그
    /// 결과는 handle을 쥔 쪽이 받는다).
    fn spawn_store<T, F>(self: &Arc<Self>, f: F) -> tokio::task::JoinHandle<OrchestrationResult<T>>
    where
        T: Send + 'static,
        F: FnOnce(super::runtime::Service) -> Result<T, OrchestrationError> + Send + 'static,
    {
        let runtime = Arc::clone(self);
        tokio::spawn(async move { runtime.blocking(f).await })
    }
}

fn store_point(probe: &Option<StoreProbe>, point: StorePoint) {
    if let Some(probe) = probe {
        probe(point);
    }
}

/// 같은 task의 앞 기동을 되돌리는 중(재시도 가능). scheduler 자리는 되돌리는 쪽이 반납한다.
pub(crate) const LAUNCH_ROLLING_BACK: &str = "launchRollingBack";

fn rolling_back_launch() -> StartWorkerOutcome {
    StartWorkerOutcome::Failed {
        code: LAUNCH_ROLLING_BACK.into(),
        message: "The previous launch of this task is being rolled back; retry the assignment."
            .into(),
        retryable: true,
    }
}

fn cancelled_launch() -> StartWorkerOutcome {
    StartWorkerOutcome::Failed {
        code: "taskCancelled".into(),
        message: "The task was cancelled before its worker started.".into(),
        retryable: false,
    }
}

/// 자식 기동의 수명 상태(Codex r7·r8 — research R14 "기동 수명과 취소 책임" 표). 어느 상태에서 future가 drop돼도 되돌리기와
/// 이 시도의 scheduler 보유는 [`LaunchCleanup`] 하나가 책임진다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CleanupState {
    /// 노드 예약 커밋 중(소유 task — drop되면 그 결과를 기다려 되돌린다).
    Reserving,
    /// 노드 예약 뒤·엔진 준비 전.
    Reserved,
    /// 엔진 준비 뒤(registry에 run이 있다).
    Prepared,
    /// 바인딩 커밋 중(소유 task — drop되면 커밋을 기다린 뒤 run 취소 + 한 트랜잭션 되돌리기).
    Binding,
    /// 실패 정리 중(소유 task — drop돼도 끝까지 간다).
    RollingBack,
    /// 끝났다(바인딩 성공, 되돌리기 완료, 되돌릴 것 없음, 또는 저장되지 않은 되돌리기를 런타임에 넘김).
    Done,
}

/// guard가 쥔 진행 중 소유 task.
enum Pending {
    Reserve(tokio::task::JoinHandle<OrchestrationResult<ChildRunReservation>>),
    Bind(tokio::task::JoinHandle<OrchestrationResult<OrchestrationSession>>),
    Rollback(tokio::task::JoinHandle<Result<(), String>>),
}

/// 자식 기동의 정리 guard(R14 표 4 취소·abort 열, Codex r6·r7·r8). 이 시도의 scheduler 보유를 쥐고, 자기 run을 실행하면 실행
/// 중 자리로 넘기고(`transfer`), 그 밖의 끝에서는 자기 보유만 놓는다. 끝나지 않고 drop되면(future abort) 진행 중 커밋을
/// 기다려 준비한 run 취소·task·노드 되돌리기를 뒤에서 하고, 그 동안 단일 비행 자리를 "되돌리는 중"으로 쥔다. 되돌리기
/// 저장이 실패하면 완료로 보지 않고 런타임의 재시도 목록으로 넘긴다(자리·보유 유지, 정지 판정에 활동으로 보임).
struct LaunchCleanup {
    runtime: Arc<OrchestrationRuntime>,
    bench: String,
    /// 되돌리기는 작업 영역 id로 한다(작업대가 닫혀도 끝낼 수 있게, Codex r9).
    workspace_id: String,
    node_id: String,
    task_id: String,
    planned_run_id: String,
    token: u64,
    state: CleanupState,
    pending: Option<Pending>,
    hold: Option<SlotHold>,
    /// 저장되지 않은 되돌리기를 런타임에 넘겼다(단일 비행 자리는 재시도가 끝낸다).
    handed_off: bool,
}

impl LaunchCleanup {
    /// 자기 run이 실행된다: 보유를 실행 중 자리로 넘긴다.
    fn launched(&mut self) {
        if let Some(hold) = self.hold.take() {
            self.runtime.scheduler().transfer(hold);
        }
        self.state = CleanupState::Done;
    }

    /// 되돌릴 것 없이 끝났다(다른 기동의 run을 돌려줌, 예약 거절): 자기 보유만 놓는다.
    fn settled(&mut self) {
        if let Some(hold) = self.hold.take() {
            let _ = self.runtime.scheduler().release_hold(hold);
        }
        self.state = CleanupState::Done;
    }

    /// 기동 실패·취소: 준비한 run을 취소하고 task·노드를 되돌린다. 되돌리기는 소유 task로 돌리고 guard가 handle을 쥔다 —
    /// 그 await 중에 drop돼도 되돌리기는 끝까지 가고 guard가 끝을 기다려 정리한다. 저장이 실패하면 재시도 목록으로 넘긴다.
    async fn fail(&mut self) {
        let state = std::mem::replace(&mut self.state, CleanupState::RollingBack);
        if !matches!(state, CleanupState::Reserved | CleanupState::Prepared) {
            self.settled();
            return;
        }
        self.runtime
            .mark_task_launch_rolling_back(&self.task_id, self.token);
        let handle = tokio::spawn(rollback(
            Arc::clone(&self.runtime),
            self.bench.clone(),
            self.workspace_id.clone(),
            self.task_id.clone(),
            self.node_id.clone(),
            self.planned_run_id.clone(),
            state,
        ));
        self.pending = Some(Pending::Rollback(handle));
        let result = match self.pending.as_mut() {
            Some(Pending::Rollback(handle)) => handle.await,
            _ => unreachable!("the rollback task was just spawned"),
        };
        self.pending = None;
        let result = result.unwrap_or_else(|error| Err(error.to_string()));
        self.handed_off = finish_undo(
            &self.runtime,
            UndoTarget {
                bench: self.bench.clone(),
                workspace_id: self.workspace_id.clone(),
                node_id: self.node_id.clone(),
                task_id: self.task_id.clone(),
                planned_run_id: self.planned_run_id.clone(),
                token: self.token,
            },
            self.hold.take(),
            result,
            false,
        );
        self.state = CleanupState::Done;
    }
}

/// 되돌린 기동의 식별.
struct UndoTarget {
    bench: String,
    workspace_id: String,
    node_id: String,
    task_id: String,
    planned_run_id: String,
    token: u64,
}

/// 되돌리기 결과를 반영한다. 저장됐으면 이 시도의 보유를 놓는다(`clear_slot`이면 단일 비행 자리도 지운다). 저장되지
/// 않았으면 완료로 보지 않고 런타임의 재시도 목록으로 넘긴다(단일 비행 자리 "되돌리는 중"·보유 유지). 넘겼으면 true.
fn finish_undo(
    runtime: &Arc<OrchestrationRuntime>,
    target: UndoTarget,
    hold: Option<SlotHold>,
    result: Result<(), String>,
    clear_slot: bool,
) -> bool {
    runtime.forget_launching(&target.planned_run_id);
    match result {
        Ok(()) => {
            if clear_slot {
                runtime.end_task_launch(&target.task_id, target.token);
            }
            if let Some(hold) = hold {
                let _ = runtime.scheduler().release_hold(hold);
            }
            false
        }
        Err(error) => {
            runtime.defer_revert(
                &target.task_id,
                PendingRevert {
                    workspace_id: target.workspace_id,
                    node_id: target.node_id,
                    planned_run_id: target.planned_run_id,
                    token: target.token,
                    hold,
                    attempts: 1,
                    last_error: error,
                    in_flight: false,
                },
            );
            true
        }
    }
}

/// 되돌리기: 준비한 run이면 엔진에서 취소하고, 노드가 아직 이 기동의 run을 가리키면 task·노드를 한 트랜잭션에서 되돌린다.
/// 저장 실패는 결과로 돌려준다(Codex r8 — 성공처럼 끝내지 않는다).
async fn rollback(
    runtime: Arc<OrchestrationRuntime>,
    bench: String,
    workspace_id: String,
    task_id: String,
    node_id: String,
    planned_run_id: String,
    state: CleanupState,
) -> Result<(), String> {
    if state == CleanupState::Prepared {
        runtime
            .benches
            .engine
            .cancel(&planned_run_id, runtime.benches.run_sink(&bench))
            .await;
        runtime
            .benches
            .hub
            .release_run_claim(&planned_run_id, &bench);
    }
    if !matches!(state, CleanupState::Reserved | CleanupState::Prepared) {
        return Ok(());
    }
    let probe = runtime.store_probe();
    let faults = Arc::clone(&runtime);
    runtime
        .blocking(move |service| {
            store_point(&probe, StorePoint::RevertBeforeCommit);
            if faults.take_revert_fault() {
                return Err(OrchestrationError::new(
                    crate::domain::agent_orchestration::OrchestrationErrorCode::WorkerUnavailable,
                    "injected rollback store failure",
                ));
            }
            service.revert_child_launch(&workspace_id, &task_id, &node_id, &planned_run_id)
        })
        .await
        .map_err(|error| error.text())
}

impl Drop for LaunchCleanup {
    fn drop(&mut self) {
        let pending = self.pending.take();
        let hold = self.hold.take();
        // 되돌릴 것이 없으면 자리를 바로 지운다(끝났거나, 예약 결과를 받아 끝남). 넘긴 되돌리기는 재시도가 지운다.
        let nothing_to_undo = self.state == CleanupState::Done
            || (self.state == CleanupState::Reserving && pending.is_none());
        let handle = match tokio::runtime::Handle::try_current() {
            Ok(handle) if !nothing_to_undo => handle,
            _ => {
                if !self.handed_off {
                    self.runtime.end_task_launch(&self.task_id, self.token);
                }
                self.runtime.forget_launching(&self.planned_run_id);
                if let Some(hold) = hold {
                    let _ = self.runtime.scheduler().release_hold(hold);
                }
                return;
            }
        };
        // future가 끝나지 않고 drop됐다(abort). 시작 장벽 sender는 이미 닫혔거나(장벽 전) 실행이 시작됐다(바인딩 중).
        // 진행 중 커밋을 기다려 되돌리고, 끝날 때까지 단일 비행 자리를 "되돌리는 중"으로 쥔다.
        self.runtime
            .mark_task_launch_rolling_back(&self.task_id, self.token);
        let runtime = Arc::clone(&self.runtime);
        let target = UndoTarget {
            bench: self.bench.clone(),
            workspace_id: self.workspace_id.clone(),
            node_id: self.node_id.clone(),
            task_id: self.task_id.clone(),
            planned_run_id: self.planned_run_id.clone(),
            token: self.token,
        };
        let state = self.state;
        handle.spawn(async move {
            let undo = |state| {
                rollback(
                    Arc::clone(&runtime),
                    target.bench.clone(),
                    target.workspace_id.clone(),
                    target.task_id.clone(),
                    target.node_id.clone(),
                    target.planned_run_id.clone(),
                    state,
                )
            };
            let result = match (state, pending) {
                (CleanupState::Reserving, Some(Pending::Reserve(reserving))) => {
                    match reserving.await {
                        // 다른 배정의 run이다: 되돌릴 것이 없다(자기 보유만 놓는다).
                        Ok(Ok(ChildRunReservation::Existing(_))) | Ok(Err(_)) => Ok(()),
                        Ok(Ok(ChildRunReservation::Reserved)) | Err(_) => {
                            undo(CleanupState::Reserved).await
                        }
                    }
                }
                (CleanupState::Binding, Some(Pending::Bind(binding))) => {
                    // 바인딩이 커밋했든 아니든 이 배정은 끝나지 않았다: run을 취소하고 한 트랜잭션에서 되돌린다.
                    let _ = binding.await;
                    undo(CleanupState::Prepared).await
                }
                (CleanupState::RollingBack, Some(Pending::Rollback(rolling_back))) => rolling_back
                    .await
                    .unwrap_or_else(|error| Err(error.to_string())),
                (CleanupState::Reserved | CleanupState::Prepared, _) => undo(state).await,
                // 상태와 맞지 않는 소유 task는 없다(보수적으로 run 취소 + 조건부 되돌리기).
                (CleanupState::Binding | CleanupState::RollingBack, _) => {
                    undo(CleanupState::Prepared).await
                }
                (CleanupState::Reserving | CleanupState::Done, _) => Ok(()),
            };
            finish_undo(&runtime, target, hold, result, true);
        });
    }
}

/// 비우기 중 대기 task 배정의 이어 가기(K) 판정(044 T040, contracts/drain-classification.md): 대상이 이 작업대 세션의 아직
/// 시작하지 않은 task(`pending`·`ready`)이고 비우기 시작 **전에** 만들어졌으면 받는다. 아니면 새 작업(N)과 같이 `draining`.
/// 서빙 중이면 아무것도 보지 않는다.
async fn ensure_assign_continues(
    runtime: &OrchestrationRuntime,
    bench: &str,
    task_id: &str,
) -> Result<(), ToolError> {
    let gate = runtime.work_gate();
    if !matches!(
        gate.state(),
        crate::application::work_gate::GateState::Draining(_)
    ) {
        return Ok(());
    }
    let refused = || {
        ToolError::new(
            "draining",
            crate::application::drain::MESSAGE_DRAINING,
            true,
        )
    };
    let drain_started_at = gate.drain_started_at().ok_or_else(refused)?;
    let session = runtime.get(bench).await?.ok_or_else(refused)?;
    let waiting = session.tasks.iter().any(|task| {
        task.id == task_id
            && matches!(task.status, TaskStatus::Pending | TaskStatus::Ready)
            && chrono::DateTime::parse_from_rfc3339(&task.created_at)
                .is_ok_and(|created| created < drain_started_at)
    });
    if waiting {
        Ok(())
    } else {
        Err(refused())
    }
}
