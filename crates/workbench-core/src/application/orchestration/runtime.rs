//! orchestration 런타임(041): AW orchestration command 18개의 흐름을 core로 옮겼다. 작업 영역은 창 label 대신
//! 작업대 묶임으로 찾는다(`bench_id`). 저장소 호출은 동기 파일 입출력이므로 `spawn_blocking`에서 하고
//! (research R1), 엔진·전달을 기다리는 동안에는 어떤 lock도 쥐지 않는다(research R2 — 서비스·명령·알림
//! 전달의 각 단계가 독립 transaction이다). 오류 문구는 오늘 command와 같다: 도메인 오류는
//! `OrchestrationError`, command가 직접 만들던 문구는 평문이다.

use std::sync::Arc;

use crate::{
    application::{
        bench_service::BenchServices,
        orchestration::{
            binding::OrchestrationBindings,
            command_service::{DeliverTaskCommandRequest, OrchestrationCommandService},
            notification_dispatcher::CoordinatorNotificationDispatcher,
            scheduler::{LeaseOutcome, OrchestrationScheduler},
            service::{
                BindMainRunRequest, CoordinatorHandoffRequest, DelegateGoalOutcome,
                DelegateGoalRequest, DispatchPromptRequest, MainRunBindingState,
                OrchestrationService, SetPresentationRequest, TaskActionRequest,
            },
        },
    },
    domain::agent_orchestration::{
        AccessPolicy, AgentNodeKind, OrchestrationError, OrchestrationSession, OrchestrationTask,
        PromptDelivery, PromptDispatch, PromptDispatchTargetStatus, TaskCommand, TaskCommandKind,
        TaskCommandSource, TaskCommandStatus, TaskReport, TaskReportType, TaskStatus,
        WorkerRuntimeProfile, MAIN_AGENT_NODE_ID,
    },
    infrastructure::{
        fs::orchestration_store::JsonOrchestrationRepository,
        orchestration::{
            bound_repository::BoundOrchestrationRepository,
            delivery_sink::DeliveryOrchestrationSink,
            engine_agent_worker::EngineAgentWorker,
            worktree_guard::{verify_worktree_unchanged, WorktreeGuards},
        },
    },
    ports::{
        agent_worker::{AgentWorkerPort, StartWorkerOutcome, WorkerAssignment, WorkerBinding},
        desktop_bridge::RunTerminalHook,
        orchestration_event_sink::{OrchestrationEvent, OrchestrationEventSink},
        orchestration_repository::OrchestrationRepository,
    },
};

pub type Repository = BoundOrchestrationRepository<JsonOrchestrationRepository>;
pub type Service = OrchestrationService<Repository, DeliveryOrchestrationSink>;

pub const MESSAGE_WORKSPACE_UNAVAILABLE: &str = "Orchestration workspace is unavailable.";
pub const MESSAGE_NOT_BOOTSTRAPPED: &str = "Orchestration workspace is not bootstrapped.";
pub const MESSAGE_MAIN_RUN_UNAVAILABLE: &str = "Main Coordinator run is unavailable.";
pub const MESSAGE_RUN_OWNED_BY_OTHER_BENCH: &str = "run is owned by another bench.";

/// 동시 자식 수와 자식 agent 프로필(오늘 환경 변수 규칙).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationConfig {
    pub max_concurrent_children: usize,
    pub child_agent_profile: String,
}

impl OrchestrationConfig {
    /// `ACP_MAX_RUNS`(없으면 `ACP_WORKBENCH_MAX_RUNS`, 기본 4) − 1, 최소 1. 프로필 `AW_ORCHESTRATION_AGENT_PROFILE`(기본 codex).
    pub fn from_env() -> Self {
        Self {
            max_concurrent_children: std::env::var("ACP_MAX_RUNS")
                .or_else(|_| std::env::var("ACP_WORKBENCH_MAX_RUNS"))
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(4)
                .saturating_sub(1)
                .max(1),
            child_agent_profile: std::env::var("AW_ORCHESTRATION_AGENT_PROFILE")
                .unwrap_or_else(|_| "codex".into()),
        }
    }
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_children: 3,
            child_agent_profile: "codex".into(),
        }
    }
}

/// command 결과 오류. 도메인 오류는 오늘처럼 JSON으로, 나머지는 평문으로 화면에 간다.
#[derive(Debug, Clone, PartialEq)]
pub enum OrchestrationFailure {
    Domain(OrchestrationError),
    Plain(String),
    /// 작업대 소유가 아닌 run을 작업 영역에 넣으려 함(research R18).
    Forbidden(String),
}

impl From<OrchestrationError> for OrchestrationFailure {
    fn from(error: OrchestrationError) -> Self {
        Self::Domain(error)
    }
}

pub type OrchestrationResult<T> = Result<T, OrchestrationFailure>;

pub struct OrchestrationRuntime {
    repository: Repository,
    sink: DeliveryOrchestrationSink,
    worker: EngineAgentWorker,
    scheduler: OrchestrationScheduler,
    config: OrchestrationConfig,
    benches: Arc<BenchServices>,
    guards: Arc<WorktreeGuards>,
}

impl OrchestrationRuntime {
    pub fn new(
        repository: Repository,
        sink: DeliveryOrchestrationSink,
        benches: Arc<BenchServices>,
        guards: Arc<WorktreeGuards>,
        config: OrchestrationConfig,
    ) -> Self {
        Self {
            worker: EngineAgentWorker::new(Arc::clone(&benches), Arc::clone(&guards)),
            scheduler: OrchestrationScheduler::new(config.max_concurrent_children),
            repository,
            sink,
            config,
            benches,
            guards,
        }
    }

    pub fn bindings(&self) -> &Arc<OrchestrationBindings> {
        self.repository.bindings()
    }

    pub fn repository(&self) -> &Repository {
        &self.repository
    }

    pub fn service(&self) -> Service {
        OrchestrationService::new(self.repository.clone(), self.sink.clone())
    }

    pub fn command_service(&self) -> OrchestrationCommandService<Repository, EngineAgentWorker> {
        OrchestrationCommandService::new(self.repository.clone(), self.worker.clone())
    }

    pub fn dispatcher(&self) -> CoordinatorNotificationDispatcher<Repository, EngineAgentWorker> {
        CoordinatorNotificationDispatcher::new(self.repository.clone(), self.worker.clone())
    }

    pub fn worker(&self) -> &EngineAgentWorker {
        &self.worker
    }

    pub fn scheduler(&self) -> &OrchestrationScheduler {
        &self.scheduler
    }

    pub fn config(&self) -> &OrchestrationConfig {
        &self.config
    }

    /// 서비스 호출(동기 파일 입출력)을 blocking pool에서.
    pub async fn blocking<T, F>(&self, f: F) -> OrchestrationResult<T>
    where
        T: Send + 'static,
        F: FnOnce(Service) -> Result<T, OrchestrationError> + Send + 'static,
    {
        let service = self.service();
        tokio::task::spawn_blocking(move || f(service))
            .await
            .map_err(|error| OrchestrationFailure::Plain(error.to_string()))?
            .map_err(OrchestrationFailure::Domain)
    }

    async fn snapshot_for(
        &self,
        bench_id: &str,
        missing: &'static str,
    ) -> OrchestrationResult<OrchestrationSession> {
        let bench_id = bench_id.to_owned();
        self.blocking(move |service| service.get_for_bench(&bench_id))
            .await?
            .ok_or_else(|| OrchestrationFailure::Plain(missing.into()))
    }

    async fn require_owned_run(&self, bench_id: &str, run_id: &str) -> OrchestrationResult<()> {
        if self.benches.engine.active_owner_of(run_id).await.as_deref() == Some(bench_id) {
            Ok(())
        } else {
            Err(OrchestrationFailure::Forbidden(
                MESSAGE_RUN_OWNED_BY_OTHER_BENCH.into(),
            ))
        }
    }

    fn emit_runtime_update(&self, session: &OrchestrationSession, reason: &str) {
        if let Some(bench_id) = session.bound_bench_id.as_deref() {
            let _ = self.sink.emit(
                bench_id,
                OrchestrationEvent {
                    workspace_id: session.id.clone(),
                    revision: session.revision,
                    reason: reason.into(),
                    task_id: None,
                    node_id: None,
                },
            );
        }
    }

    async fn emit_runtime_update_for(&self, bench_id: &str, reason: &str) {
        let bench_id = bench_id.to_owned();
        if let Ok(Some(session)) = self
            .blocking(move |service| service.get_for_bench(&bench_id))
            .await
        {
            self.emit_runtime_update(&session, reason);
        }
    }

    // ---- 데스크톱 동작 18개 ----

    pub async fn bootstrap(
        &self,
        bench_id: &str,
        worktree_path: &str,
        resume_workspace_id: Option<String>,
    ) -> OrchestrationResult<OrchestrationSession> {
        let canonical = canonical_directory(worktree_path)?;
        let bench_id = bench_id.to_owned();
        self.blocking(move |service| {
            service.bootstrap(&canonical, &bench_id, resume_workspace_id.as_deref())
        })
        .await
    }

    pub async fn get(&self, bench_id: &str) -> OrchestrationResult<Option<OrchestrationSession>> {
        let bench_id = bench_id.to_owned();
        self.blocking(move |service| service.get_for_bench(&bench_id))
            .await
    }

    pub async fn list_recoverable(
        &self,
        worktree_path: &str,
    ) -> OrchestrationResult<Vec<OrchestrationSession>> {
        let canonical = canonical_directory(worktree_path)?;
        self.blocking(move |service| service.list_recoverable(&canonical))
            .await
    }

    pub async fn bind_coordinator(
        &self,
        bench_id: &str,
        request: BindMainRunRequest,
    ) -> OrchestrationResult<OrchestrationSession> {
        // research R18: 활성 연결은 이 작업대가 소유한 살아 있는 run만 넣는다. 작업대 하나에 작업 영역 하나이므로
        // 한 run이 두 작업 영역에 들어가는 일도 없다.
        if request.state == MainRunBindingState::Active {
            self.require_owned_run(bench_id, &request.run_id).await?;
        }
        let bench_id = bench_id.to_owned();
        self.blocking(move |service| service.bind_main_run(&bench_id, request))
            .await
    }

    pub async fn delegate_goal(
        &self,
        bench_id: &str,
        request: DelegateGoalRequest,
    ) -> OrchestrationResult<DelegateGoalOutcome> {
        let goal = request.goal.clone();
        let bench = bench_id.to_owned();
        let outcome = self
            .blocking(move |service| service.delegate_goal(&bench, request))
            .await?;
        let snapshot = self
            .snapshot_for(bench_id, MESSAGE_WORKSPACE_UNAVAILABLE)
            .await?;
        let run_id = snapshot
            .nodes
            .iter()
            .find(|node| node.id == MAIN_AGENT_NODE_ID)
            .and_then(|node| node.current_run_id.clone())
            .ok_or_else(|| OrchestrationFailure::Plain(MESSAGE_MAIN_RUN_UNAVAILABLE.into()))?;
        self.benches
            .engine
            .send_prompt(&run_id, goal, self.benches.run_sink(bench_id))
            .await
            .map_err(|error| OrchestrationFailure::Plain(error.message))?;
        Ok(outcome)
    }

    pub async fn adopt_manual_child(
        &self,
        bench_id: &str,
        panel_id: String,
        title: String,
    ) -> OrchestrationResult<OrchestrationSession> {
        let bench_id = bench_id.to_owned();
        self.blocking(move |service| service.adopt_manual_child(&bench_id, &panel_id, &title))
            .await
    }

    pub async fn list_tasks(
        &self,
        bench_id: &str,
        generation_id: String,
    ) -> OrchestrationResult<Vec<OrchestrationTask>> {
        let bench_id = bench_id.to_owned();
        self.blocking(move |service| service.list_child_tasks(&bench_id, &generation_id))
            .await
    }

    pub async fn collect_reports(&self, bench_id: &str) -> OrchestrationResult<Vec<TaskReport>> {
        Ok(self
            .get(bench_id)
            .await?
            .map(|session| session.reports)
            .unwrap_or_default())
    }

    pub async fn set_presentation(
        &self,
        bench_id: &str,
        request: SetPresentationRequest,
    ) -> OrchestrationResult<OrchestrationSession> {
        let bench_id = bench_id.to_owned();
        self.blocking(move |service| service.set_presentation(&bench_id, request))
            .await
    }

    pub async fn send_child_command(
        &self,
        bench_id: &str,
        request: DeliverTaskCommandRequest,
    ) -> OrchestrationResult<TaskCommand> {
        let command = self.command_service().deliver(bench_id, request).await?;
        self.emit_runtime_update_for(bench_id, "taskCommandDelivery")
            .await;
        Ok(command)
    }

    pub async fn respond_input(
        &self,
        bench_id: &str,
        request: TaskActionRequest,
    ) -> OrchestrationResult<TaskCommand> {
        let snapshot = self
            .snapshot_for(bench_id, MESSAGE_WORKSPACE_UNAVAILABLE)
            .await?;
        let input_report_id = snapshot
            .reports
            .iter()
            .rev()
            .find(|report| {
                report.task_id == request.task_id
                    && report.report_type == TaskReportType::InputRequest
            })
            .map(|report| report.id.clone());
        let task_revision = snapshot
            .tasks
            .iter()
            .find(|task| task.id == request.task_id)
            .map(|task| task.revision);
        Ok(self
            .command_service()
            .deliver(
                bench_id,
                DeliverTaskCommandRequest {
                    request_id: request.request_id,
                    task_id: request.task_id,
                    kind: TaskCommandKind::InputResponse,
                    message: request.message,
                    input_report_id,
                    delivery: PromptDelivery::Queue,
                    source: TaskCommandSource::User,
                    expected_task_revision: task_revision,
                },
            )
            .await?)
    }

    pub async fn cancel_task(
        &self,
        bench_id: &str,
        request: TaskActionRequest,
    ) -> OrchestrationResult<OrchestrationSession> {
        let snapshot = self
            .snapshot_for(bench_id, MESSAGE_WORKSPACE_UNAVAILABLE)
            .await?;
        let task = snapshot
            .tasks
            .iter()
            .find(|task| task.id == request.task_id);
        let task_revision = task.map(|task| task.revision);
        let has_active_run = task
            .and_then(|task| task.assigned_node_id.as_ref())
            .and_then(|node_id| snapshot.nodes.iter().find(|node| node.id == *node_id))
            .and_then(|node| node.current_run_id.as_ref())
            .is_some();
        if !has_active_run {
            let bench = bench_id.to_owned();
            return self
                .blocking(move |service| service.cancel_task(&bench, request))
                .await;
        }
        let task_id = request.task_id.clone();
        self.command_service()
            .deliver(
                bench_id,
                DeliverTaskCommandRequest {
                    request_id: request.request_id,
                    task_id: request.task_id,
                    kind: TaskCommandKind::Cancel,
                    message: None,
                    input_report_id: None,
                    delivery: PromptDelivery::Queue,
                    source: TaskCommandSource::User,
                    expected_task_revision: task_revision,
                },
            )
            .await?;
        let _ = self.scheduler.release(&task_id);
        self.snapshot_for(bench_id, MESSAGE_WORKSPACE_UNAVAILABLE)
            .await
    }

    pub async fn retry_task(
        &self,
        bench_id: &str,
        request: TaskActionRequest,
    ) -> OrchestrationResult<OrchestrationSession> {
        self.stop_existing_task_worker(bench_id, &request.task_id)
            .await?;
        let task_id = request.task_id.clone();
        let bench = bench_id.to_owned();
        self.blocking(move |service| service.retry_task(&bench, request))
            .await?;
        self.launch_task_for_ui(bench_id, &task_id).await
    }

    pub async fn reassign_task(
        &self,
        bench_id: &str,
        request: TaskActionRequest,
    ) -> OrchestrationResult<OrchestrationSession> {
        self.stop_existing_task_worker(bench_id, &request.task_id)
            .await?;
        let task_id = request.task_id.clone();
        let bench = bench_id.to_owned();
        self.blocking(move |service| service.reassign_task(&bench, request))
            .await?;
        self.launch_task_for_ui(bench_id, &task_id).await
    }

    async fn stop_existing_task_worker(
        &self,
        bench_id: &str,
        task_id: &str,
    ) -> OrchestrationResult<()> {
        let Some(snapshot) = self.get(bench_id).await? else {
            return Ok(());
        };
        let Some(task) = snapshot.tasks.iter().find(|task| task.id == task_id) else {
            return Ok(());
        };
        let Some(node) = task
            .assigned_node_id
            .as_ref()
            .and_then(|node_id| snapshot.nodes.iter().find(|node| node.id == *node_id))
        else {
            return Ok(());
        };
        let Some(run_id) = node.current_run_id.as_ref() else {
            return Ok(());
        };
        let binding = WorkerBinding {
            workspace_id: snapshot.id.clone(),
            bench_id: bench_id.to_owned(),
            node_id: node.id.clone(),
            task_id: task.id.clone(),
            run_id: run_id.clone(),
        };
        if self.worker.is_active(&binding).await {
            let _ = self.worker.cancel_worker(&binding).await;
        }
        if let Some(decorator) = &self.benches.launch_decorator {
            decorator.revoke_run(run_id);
        }
        Ok(())
    }

    /// 과제 하나를 scheduler 자리를 얻어 기동한다(오늘 `launch_orchestration_task_for_ui`).
    pub async fn launch_task_for_ui(
        &self,
        bench_id: &str,
        task_id: &str,
    ) -> OrchestrationResult<OrchestrationSession> {
        if let LeaseOutcome::Queued { .. } = self.scheduler.acquire(task_id)? {
            return self
                .snapshot_for(bench_id, MESSAGE_WORKSPACE_UNAVAILABLE)
                .await;
        }
        let snapshot = self
            .snapshot_for(bench_id, MESSAGE_WORKSPACE_UNAVAILABLE)
            .await?;
        let task = snapshot
            .tasks
            .iter()
            .find(|task| task.id == task_id)
            .ok_or_else(|| OrchestrationFailure::Plain("Task is unavailable.".into()))?;
        let node = task
            .assigned_node_id
            .as_ref()
            .and_then(|node_id| snapshot.nodes.iter().find(|node| node.id == *node_id))
            .ok_or_else(|| OrchestrationFailure::Plain("Assigned Child is unavailable.".into()))?;
        let outcome = self
            .worker
            .start_worker(WorkerAssignment {
                workspace_id: snapshot.id.clone(),
                bench_id: bench_id.to_owned(),
                worktree_path: snapshot.worktree_path.clone(),
                node_id: node.id.clone(),
                task_id: task.id.clone(),
                attempt: task.attempt,
                planned_run_id: uuid::Uuid::new_v4().to_string(),
                role: node.role.clone(),
                objective: task.objective.clone(),
                constraints: task.constraints.clone(),
                expected_result: task.expected_result.clone(),
                runtime_profile: self.child_runtime_profile(),
                mcp_capability: String::new(),
            })
            .await?;
        match outcome {
            StartWorkerOutcome::Started { run_id } => {
                let bench = bench_id.to_owned();
                let task_id = task.id.clone();
                let node_id = node.id.clone();
                self.blocking(move |service| {
                    service.bind_child_run(&bench, &task_id, &node_id, &run_id)
                })
                .await
            }
            StartWorkerOutcome::Queued { .. } => {
                self.snapshot_for(bench_id, MESSAGE_WORKSPACE_UNAVAILABLE)
                    .await
            }
            StartWorkerOutcome::Failed { message, .. } => {
                let _ = self.scheduler.release(task_id);
                Err(OrchestrationFailure::Plain(message))
            }
        }
    }

    pub fn child_runtime_profile(&self) -> WorkerRuntimeProfile {
        WorkerRuntimeProfile {
            agent_profile_id: self.config.child_agent_profile.clone(),
            provider_id: "acp".into(),
            model_id: None,
            access_policy: AccessPolicy::ReadOnly,
            supports_read_only: true,
        }
    }

    pub async fn handoff_coordinator(
        &self,
        bench_id: &str,
        request: CoordinatorHandoffRequest,
    ) -> OrchestrationResult<OrchestrationSession> {
        // research R18: 후계 run도 이 작업대가 소유한 살아 있는 run이어야 한다.
        self.require_owned_run(bench_id, &request.successor_run_id)
            .await?;
        let previous_generation = self
            .get(bench_id)
            .await?
            .and_then(|session| session.active_coordinator_generation_id);
        let bench = bench_id.to_owned();
        let session = self
            .blocking(move |service| service.handoff_coordinator(&bench, request))
            .await?;
        if let (Some(generation_id), Some(decorator)) =
            (previous_generation, &self.benches.launch_decorator)
        {
            decorator.revoke_generation(&session.id, &generation_id);
        }
        Ok(session)
    }

    pub async fn dispatch_prompt(
        &self,
        bench_id: &str,
        request: DispatchPromptRequest,
    ) -> OrchestrationResult<PromptDispatch> {
        let bench = bench_id.to_owned();
        let mut dispatch = self
            .blocking(move |service| service.record_prompt_dispatch(&bench, request))
            .await?;
        let snapshot = self
            .snapshot_for(bench_id, MESSAGE_WORKSPACE_UNAVAILABLE)
            .await?;
        for target in dispatch.targets.clone() {
            let Some(node) = snapshot
                .nodes
                .iter()
                .find(|node| node.id == target.panel_id && node.kind == AgentNodeKind::Child)
            else {
                continue;
            };
            let Some(task) = node
                .assigned_task_id
                .as_ref()
                .and_then(|task_id| snapshot.tasks.iter().find(|task| task.id == *task_id))
            else {
                dispatch = self
                    .update_dispatch_target(
                        bench_id,
                        &dispatch.id,
                        &target.request_id,
                        PromptDispatchTargetStatus::Rejected,
                        Some(("unknownTask".into(), "Child has no assigned task.".into())),
                    )
                    .await?;
                continue;
            };
            let command = self
                .command_service()
                .deliver(
                    bench_id,
                    DeliverTaskCommandRequest {
                        request_id: target.request_id.clone(),
                        task_id: task.id.clone(),
                        kind: TaskCommandKind::Message,
                        message: Some(dispatch.message.clone()),
                        input_report_id: None,
                        delivery: dispatch.delivery,
                        source: TaskCommandSource::User,
                        expected_task_revision: Some(task.revision),
                    },
                )
                .await;
            let (status, failure) = match command {
                Ok(command) if command.status == TaskCommandStatus::Accepted => {
                    (PromptDispatchTargetStatus::Delivered, None)
                }
                Ok(command) => (
                    PromptDispatchTargetStatus::Failed,
                    command
                        .failure
                        .map(|failure| (format!("{:?}", failure.code), failure.message)),
                ),
                Err(error) => (
                    PromptDispatchTargetStatus::Failed,
                    Some((format!("{:?}", error.code), error.message)),
                ),
            };
            dispatch = self
                .update_dispatch_target(bench_id, &dispatch.id, &target.request_id, status, failure)
                .await?;
        }
        Ok(dispatch)
    }

    async fn update_dispatch_target(
        &self,
        bench_id: &str,
        dispatch_id: &str,
        request_id: &str,
        status: PromptDispatchTargetStatus,
        failure: Option<(String, String)>,
    ) -> OrchestrationResult<PromptDispatch> {
        let (bench, dispatch_id, request_id) = (
            bench_id.to_owned(),
            dispatch_id.to_owned(),
            request_id.to_owned(),
        );
        self.blocking(move |service| {
            service.update_prompt_dispatch_target(
                &bench,
                &dispatch_id,
                &request_id,
                status,
                failure,
            )
        })
        .await
    }

    /// 묶인 작업 영역을 재조정한다(오늘 `recover_orchestration_workspace`): 살아 있는 run 반영, scheduler 재구성,
    /// 중단된 명령·알림 복구, 대기 알림은 백그라운드로 전달.
    pub async fn recover(
        self: &Arc<Self>,
        bench_id: &str,
    ) -> OrchestrationResult<OrchestrationSession> {
        let snapshot = self
            .snapshot_for(bench_id, MESSAGE_NOT_BOOTSTRAPPED)
            .await?;
        let mut live_run_ids = Vec::new();
        for run_id in snapshot
            .nodes
            .iter()
            .filter_map(|node| node.current_run_id.as_ref())
        {
            if self.benches.engine.active_owner_of(run_id).await.as_deref() == Some(bench_id) {
                live_run_ids.push(run_id.clone());
            }
        }
        let bench = bench_id.to_owned();
        let live = live_run_ids.clone();
        let reconciled = self
            .blocking(move |service| service.reconcile_runtime(&bench, &live))
            .await?;
        let active_task_ids = reconciled
            .tasks
            .iter()
            .filter(|task| {
                task.status == TaskStatus::Running
                    && task
                        .assigned_node_id
                        .as_ref()
                        .and_then(|node_id| {
                            reconciled.nodes.iter().find(|node| node.id == *node_id)
                        })
                        .and_then(|node| node.current_run_id.as_ref())
                        .is_some_and(|run_id| live_run_ids.contains(run_id))
            })
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let ready_task_ids = reconciled
            .tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Ready)
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        self.scheduler
            .reconcile(&active_task_ids, &ready_task_ids)?;
        let commands = self.command_service();
        let bench = bench_id.to_owned();
        tokio::task::spawn_blocking(move || commands.reconcile_pending(&bench))
            .await
            .map_err(|error| OrchestrationFailure::Plain(error.to_string()))??;
        let dispatcher = self.dispatcher();
        let bench = bench_id.to_owned();
        let recovering = self.dispatcher();
        tokio::task::spawn_blocking(move || recovering.recover_interrupted(&bench))
            .await
            .map_err(|error| OrchestrationFailure::Plain(error.to_string()))??;
        let runtime = Arc::clone(self);
        let bench = bench_id.to_owned();
        tokio::spawn(async move {
            let _ = dispatcher.dispatch_pending(&bench).await;
            runtime
                .emit_runtime_update_for(&bench, "notificationRecovery")
                .await;
        });
        self.snapshot_for(bench_id, MESSAGE_NOT_BOOTSTRAPPED).await
    }

    /// 작업대 닫기(research R3): 오늘 창 닫힘의 `release_window`와 같은 규칙으로 복구 가능 전환. 묶임 표 제거는
    /// 묶임 저장소 commit이 한다.
    pub fn release(&self, bench_id: &str) {
        let _ = self.service().release_bench(bench_id);
    }

    /// run 종료 뒤 worktree 감시(research R8). lock을 기다리지 않는 조건부 갱신 한 번.
    pub fn on_run_terminal(&self, run_id: &str) {
        let Some(guard) = self.guards.take(run_id) else {
            return;
        };
        let service = self.service();
        let check = move || {
            let Err(violation) = verify_worktree_unchanged(&guard.worktree_path, &guard.baseline)
            else {
                return;
            };
            let _ = service.fail_task_for_runtime(
                &guard.bench_id,
                &guard.task_id,
                &guard.node_id,
                violation.code,
                &violation.message,
            );
        };
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.spawn_blocking(check);
            }
            Err(_) => check(),
        }
    }
}

/// run 종료 → orchestration worktree 감시(core 안). AW의 040 과도기 hook을 대신한다.
pub struct OrchestrationTerminalHook {
    runtime: std::sync::OnceLock<std::sync::Weak<OrchestrationRuntime>>,
}

impl OrchestrationTerminalHook {
    pub fn new() -> Self {
        Self {
            runtime: std::sync::OnceLock::new(),
        }
    }

    pub fn attach(&self, runtime: &Arc<OrchestrationRuntime>) {
        let _ = self.runtime.set(Arc::downgrade(runtime));
    }
}

impl Default for OrchestrationTerminalHook {
    fn default() -> Self {
        Self::new()
    }
}

impl RunTerminalHook for OrchestrationTerminalHook {
    fn on_terminal(&self, run_id: &str) {
        if let Some(runtime) = self.runtime.get().and_then(std::sync::Weak::upgrade) {
            runtime.on_run_terminal(run_id);
        }
    }
}

fn canonical_directory(path: &str) -> OrchestrationResult<String> {
    let canonical = std::fs::canonicalize(path).map_err(|error| {
        OrchestrationFailure::Plain(format!("Failed to resolve workspace path: {error}"))
    })?;
    if !canonical.is_dir() {
        return Err(OrchestrationFailure::Plain(
            "Workspace path must be a directory.".into(),
        ));
    }
    Ok(canonical.to_string_lossy().to_string())
}
