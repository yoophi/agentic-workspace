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
        agent_worker::{AgentWorkerPort, StartWorkerOutcome, WorkerBinding},
        desktop_bridge::{OrchestrationLaunchRole, RunTerminalHook},
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

/// 알림 재전달 backoff(첫 대기와 상한).
const NOTIFICATION_RETRY_BASE: std::time::Duration = std::time::Duration::from_millis(50);
const NOTIFICATION_RETRY_MAX: std::time::Duration = std::time::Duration::from_secs(5);

/// 자식 기동의 시작 장벽 지점(044 research R14, 시험용 관측 지점). 운영에서는 probe가 없어 아무것도 하지 않는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchPoint {
    /// 저장소 예약 뒤·엔진 준비(registry 예약) 전.
    BeforePrepare,
    /// 엔진 준비(registry 예약·spawn·attach) 뒤·G 아래 전이 전.
    AfterPrepare,
    /// G 아래 전이 뒤·시작 장벽을 열기 전.
    AfterRegister,
    /// 시작 장벽을 연 뒤·바인딩 전(자식 첫 턴이 바인딩 전에 도구를 부르는 경우를 결정적으로 재현).
    AfterOpen,
}

/// 자식 기동의 저장소 커밋 단계(blocking 스레드) 전후 지점(Codex r6·r7 abort 재현).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorePoint {
    /// 노드 예약(`reserve_child_run`, RMW 커밋) 직전.
    ReserveBeforeCommit,
    /// 노드 예약이 끝난 뒤.
    ReserveAfterCommit,
    /// run 바인딩(`bind_child_run`) 커밋 직전.
    BindBeforeCommit,
    /// run 바인딩이 끝난 뒤.
    BindAfterCommit,
    /// 기동 되돌리기(노드 예약 해제) 커밋 직전.
    RevertBeforeCommit,
}

/// blocking 스레드에서 동기로 불린다(await 없음).
pub type StoreProbe = std::sync::Arc<dyn Fn(StorePoint) + Send + Sync>;

/// 같은 task의 진행 중 기동(단일 비행 표 값).
#[derive(Debug, Clone)]
pub(crate) struct LaunchSlot {
    token: u64,
    planned_run_id: String,
    /// 기동이 실패·abort로 되돌리는 중이다(되돌리기가 끝나면 자리를 지운다).
    rolling_back: bool,
}

/// [`OrchestrationRuntime::begin_task_launch`]가 거절한 이유.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LaunchInFlight {
    /// 같은 task가 기동 중이다(그 예정 run id).
    Launching(String),
    /// 같은 task의 앞 기동을 되돌리는 중이다(재시도 가능).
    RollingBack,
}

pub type LaunchProbe = std::sync::Arc<
    dyn Fn(LaunchPoint) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

pub struct OrchestrationRuntime {
    repository: Repository,
    sink: DeliveryOrchestrationSink,
    worker: EngineAgentWorker,
    scheduler: OrchestrationScheduler,
    config: OrchestrationConfig,
    pub(crate) benches: Arc<BenchServices>,
    guards: Arc<WorktreeGuards>,
    /// 기동 중인 자식: 예정 run id → (작업 영역, 노드, 과제). 엔진이 run을 등록하고 노드에 묶기 전에 자식의 첫 턴이
    /// 도구를 불러도 자식 역할을 인정한다(research R7, 설계 리뷰 H6). 메모리 상태.
    launching: std::sync::Mutex<std::collections::HashMap<String, (String, String, String)>>,
    launch_probe: std::sync::Mutex<Option<LaunchProbe>>,
    store_probe: std::sync::Mutex<Option<StoreProbe>>,
    dispatch_probe: std::sync::Mutex<Option<super::notification_dispatcher::DispatchProbe>>,
    /// 마지막으로 띄운 알림 전달 한 바퀴(시험이 abort한다).
    last_notification_pass: std::sync::Mutex<Option<tokio::task::AbortHandle>>,
    /// 작업대별 알림 재전달 상태(연속 재시도 수, 예약됨). 재시도 가능 실패 뒤 서버가 backoff로 다시 돈다(R14 표 6').
    notification_retries: std::sync::Mutex<std::collections::HashMap<String, (u32, bool)>>,
    /// 전달 한 바퀴의 서버 알림(회수·재전달)이 이 런타임을 부르기 위한 약한 참조.
    weak_self: std::sync::OnceLock<std::sync::Weak<Self>>,
    /// 기동 중인 task → (기동 토큰, 예정 run id). 같은 task의 기동을 하나로 묶고(단일 비행), 취소가 토큰으로 기동을
    /// 막을 수 있게 한다(044 R14 표 4·5). 메모리 상태 — 토큰은 이 프로세스의 작업 관문에서만 뜻이 있다.
    launch_tokens: std::sync::Mutex<std::collections::HashMap<String, LaunchSlot>>,
    /// 작업대 서비스에 관문이 없을 때(관문 없는 조립) 쓰는 자체 관문.
    fallback_gate: std::sync::OnceLock<std::sync::Arc<crate::application::work_gate::WorkGate>>,
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
            launching: std::sync::Mutex::default(),
            launch_probe: std::sync::Mutex::default(),
            store_probe: std::sync::Mutex::default(),
            dispatch_probe: std::sync::Mutex::default(),
            last_notification_pass: std::sync::Mutex::default(),
            notification_retries: std::sync::Mutex::default(),
            weak_self: std::sync::OnceLock::new(),
            launch_tokens: std::sync::Mutex::default(),
            fallback_gate: std::sync::OnceLock::new(),
        }
    }

    /// 자식 기동 토큰을 발급하는 작업 관문.
    pub(crate) fn work_gate(&self) -> &std::sync::Arc<crate::application::work_gate::WorkGate> {
        match self.benches.work_gate() {
            Some(gate) => gate,
            None => self
                .fallback_gate
                .get_or_init(crate::application::work_gate::WorkGate::new),
        }
    }

    /// 같은 task의 기동이 진행 중이면 그 예정 run id를(되돌리는 중이면 [`LaunchInFlight::RollingBack`]) 돌려주고, 아니면
    /// 이 기동을 등록한다.
    pub(crate) fn begin_task_launch(
        &self,
        task_id: &str,
        token: u64,
        planned_run_id: &str,
    ) -> Result<(), LaunchInFlight> {
        let mut tokens = self
            .launch_tokens
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(slot) = tokens.get(task_id) {
            return Err(if slot.rolling_back {
                LaunchInFlight::RollingBack
            } else {
                LaunchInFlight::Launching(slot.planned_run_id.clone())
            });
        }
        tokens.insert(
            task_id.to_owned(),
            LaunchSlot {
                token,
                planned_run_id: planned_run_id.to_owned(),
                rolling_back: false,
            },
        );
        Ok(())
    }

    /// 이 기동이 되돌리기에 들어갔다(Codex r7): 되돌리기가 끝날 때까지 단일 비행 자리를 쥔 채, 같은 task의 새 배정에는 곧
    /// 취소될 예정 run id 대신 재시도 가능 거절을 준다.
    pub(crate) fn mark_task_launch_rolling_back(&self, task_id: &str, token: u64) {
        let mut tokens = self
            .launch_tokens
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(slot) = tokens.get_mut(task_id).filter(|slot| slot.token == token) {
            slot.rolling_back = true;
        }
    }

    /// 같은 task의 앞 기동을 되돌리는 중인가.
    pub(crate) fn task_launch_rolling_back(&self, task_id: &str) -> bool {
        self.launch_tokens
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(task_id)
            .is_some_and(|slot| slot.rolling_back)
    }

    pub(crate) fn end_task_launch(&self, task_id: &str, token: u64) {
        let mut tokens = self
            .launch_tokens
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if tokens.get(task_id).is_some_and(|slot| slot.token == token) {
            tokens.remove(task_id);
        }
    }

    /// task 취소(R14 표 5): 기동 중이면 토큰을 `Pending→Cancelled`로 바꿔 실행을 막는다(`Prevented`). 이미 실행이
    /// 허용됐으면 그 run(`Registered`), 기동 중이 아니면 `Unknown`.
    pub(crate) fn prevent_task_launch(
        &self,
        task_id: &str,
    ) -> crate::application::work_gate::LaunchCancel {
        let token = self
            .launch_tokens
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(task_id)
            .map(|slot| slot.token);
        match token {
            Some(token) => self.work_gate().cancel_launch(token),
            None => crate::application::work_gate::LaunchCancel::Unknown,
        }
    }

    /// 시험: 시작 장벽 지점마다 부를 probe를 건다.
    #[cfg(feature = "test-hooks")]
    pub fn set_launch_probe(&self, probe: Option<LaunchProbe>) {
        *self
            .launch_probe
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = probe;
    }

    /// 시험: 자식 기동의 저장소 커밋 단계 전후 지점 probe를 건다.
    #[cfg(feature = "test-hooks")]
    pub fn set_store_probe(&self, probe: Option<StoreProbe>) {
        *self
            .store_probe
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = probe;
    }

    pub(crate) fn store_probe(&self) -> Option<StoreProbe> {
        self.store_probe
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// 시험: 알림 전달 한 바퀴의 지점 probe를 건다.
    #[cfg(feature = "test-hooks")]
    pub fn set_dispatch_probe(&self, probe: Option<super::notification_dispatcher::DispatchProbe>) {
        *self
            .dispatch_probe
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = probe;
    }

    /// 시험: 마지막으로 띄운 알림 전달 한 바퀴.
    #[cfg(feature = "test-hooks")]
    pub fn last_notification_pass(&self) -> Option<tokio::task::AbortHandle> {
        self.last_notification_pass
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// 조립이 `Arc`로 감싼 직후 한 번 부른다(전달 한 바퀴의 서버 알림이 이 런타임을 부른다).
    pub fn attach_self(self: &std::sync::Arc<Self>) {
        let _ = self.weak_self.set(std::sync::Arc::downgrade(self));
    }

    /// 알림 전달 시도 하나를 시작한다(보고 호출이 C-call을 놓기 전에 N-notify를 잡아 전달기로 넘긴다, R14 표 6').
    pub(crate) fn begin_notify_attempt(
        &self,
    ) -> Option<super::notification_dispatcher::NotifyAttempt> {
        super::notification_dispatcher::NotifyAttempt::begin(Some(self.work_gate()))
    }

    /// 알림 전달 한 바퀴를 뒤에서 띄운다.
    pub(crate) fn spawn_notification_pass(
        self: &std::sync::Arc<Self>,
        bench_id: &str,
        reason: &'static str,
    ) {
        self.spawn_notification_pass_with(bench_id, reason, None);
    }

    pub(crate) fn spawn_notification_pass_with(
        self: &std::sync::Arc<Self>,
        bench_id: &str,
        reason: &'static str,
        first: Option<super::notification_dispatcher::NotifyAttempt>,
    ) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let dispatcher = self.dispatcher();
        let runtime = std::sync::Arc::clone(self);
        let bench = bench_id.to_owned();
        let task = handle.spawn(async move {
            let _ = dispatcher.dispatch_pending_with(&bench, first).await;
            runtime.emit_runtime_update_for(&bench, reason).await;
        });
        *self
            .last_notification_pass
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(task.abort_handle());
    }

    /// 회수 한 바퀴: 살아 있는 시도가 없는 `Dispatching{attemptId}` 알림을 `Failed(retryable)`로 되돌린다(R14 표 6'').
    pub async fn reclaim_notifications(&self, bench_id: &str) -> OrchestrationResult<usize> {
        let dispatcher = self.dispatcher();
        let bench = bench_id.to_owned();
        Ok(
            tokio::task::spawn_blocking(move || dispatcher.reclaim_orphaned(&bench))
                .await
                .map_err(|error| OrchestrationFailure::Plain(error.to_string()))??,
        )
    }

    fn dispatch_events(&self) -> Option<super::notification_dispatcher::DispatchEvents> {
        use super::notification_dispatcher::DispatchEvent;
        let weak = self.weak_self.get()?.clone();
        Some(std::sync::Arc::new(move |bench: &str, event| {
            let Some(runtime) = weak.upgrade() else {
                return;
            };
            match event {
                // 결과를 저장하지 못한 시도: 새 한 바퀴가 시작할 때 회수하고 다시 전달한다.
                DispatchEvent::Orphaned => {
                    runtime.spawn_notification_pass(bench, "notificationRecovery")
                }
                DispatchEvent::RetryableFailures => runtime.schedule_notification_retry(bench),
                DispatchEvent::Settled => runtime.settle_notification_retry(bench),
            }
        }))
    }

    /// 재시도 가능 실패 뒤 backoff로 다시 전달한다. 대상 coordinator run이 살아 있을 때만 돈다(R14: 미전달 알림은
    /// coordinator run이 살아 있을 때만 활동이다). 이미 예약돼 있으면 더 예약하지 않는다.
    fn schedule_notification_retry(self: &std::sync::Arc<Self>, bench_id: &str) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let delay = {
            let mut retries = self
                .notification_retries
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let entry = retries.entry(bench_id.to_owned()).or_default();
            if entry.1 {
                return;
            }
            entry.1 = true;
            let delay = NOTIFICATION_RETRY_BASE
                .saturating_mul(1 << entry.0.min(7))
                .min(NOTIFICATION_RETRY_MAX);
            entry.0 = entry.0.saturating_add(1);
            delay
        };
        let runtime = std::sync::Arc::clone(self);
        let bench = bench_id.to_owned();
        handle.spawn(async move {
            tokio::time::sleep(delay).await;
            if let Some(entry) = runtime
                .notification_retries
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get_mut(&bench)
            {
                entry.1 = false;
            }
            if runtime.coordinator_alive(&bench).await {
                runtime.spawn_notification_pass(&bench, "notificationRetry");
            } else {
                runtime.settle_notification_retry(&bench);
            }
        });
    }

    fn settle_notification_retry(&self, bench_id: &str) {
        if let Some(entry) = self
            .notification_retries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get_mut(bench_id)
        {
            entry.0 = 0;
        }
    }

    async fn coordinator_alive(&self, bench_id: &str) -> bool {
        let Ok(Some(session)) = self.get(bench_id).await else {
            return false;
        };
        let Some(run_id) = session
            .nodes
            .iter()
            .find(|node| node.id == crate::domain::agent_orchestration::MAIN_AGENT_NODE_ID)
            .and_then(|node| node.current_run_id.clone())
        else {
            return false;
        };
        self.benches
            .engine
            .active_owner_of(&run_id)
            .await
            .as_deref()
            == Some(bench_id)
    }

    pub(crate) async fn launch_probe(&self, point: LaunchPoint) {
        let probe = self
            .launch_probe
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        if let Some(probe) = probe {
            probe(point).await;
        }
    }

    pub(crate) fn remember_launching(
        &self,
        run_id: &str,
        workspace_id: &str,
        node_id: &str,
        task_id: &str,
    ) {
        self.launching
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(
                run_id.to_owned(),
                (
                    workspace_id.to_owned(),
                    node_id.to_owned(),
                    task_id.to_owned(),
                ),
            );
    }

    pub(crate) fn forget_launching(&self, run_id: &str) {
        self.launching
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(run_id);
    }

    pub(crate) fn launching_child(&self, run_id: &str) -> Option<(String, String, String)> {
        self.launching
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(run_id)
            .cloned()
    }

    pub fn bindings(&self) -> &Arc<OrchestrationBindings> {
        self.repository.bindings()
    }

    pub fn repository(&self) -> &Repository {
        &self.repository
    }

    pub fn revisions(
        &self,
    ) -> &Arc<crate::application::orchestration::revision_watch::RevisionWatch> {
        self.repository.revisions()
    }

    pub fn service(&self) -> Service {
        OrchestrationService::new(self.repository.clone(), self.sink.clone())
    }

    pub fn command_service(&self) -> OrchestrationCommandService<Repository, EngineAgentWorker> {
        OrchestrationCommandService::new(self.repository.clone(), self.worker.clone())
    }

    pub fn dispatcher(&self) -> CoordinatorNotificationDispatcher<Repository, EngineAgentWorker> {
        let dispatcher =
            CoordinatorNotificationDispatcher::new(self.repository.clone(), self.worker.clone());
        let dispatcher = match self.dispatch_events() {
            Some(events) => dispatcher.with_server(std::sync::Arc::clone(self.work_gate()), events),
            None => dispatcher,
        };
        dispatcher.with_probe(
            self.dispatch_probe
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
        )
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

    /// research R18: 활성 연결에 넣을 수 있는 run — 이 작업대가 소유한 살아 있는 run, 또는 **아직 어디에도 흔적이
    /// 없는 계획 run id**(화면은 Main run을 띄우기 전에 계획한 id로 먼저 묶는다: `onBeforeRunStart`). 계획 id는 여기서
    /// 이 작업대 소유로 claim하므로, 다른 작업대는 그 id로 run을 띄우거나 묶을 수 없다(`run.start` 재사용 검사).
    /// 돌려주는 값: 이 호출이 계획 id를 **새로** claim했는가(묶기가 실패하면 그 claim만 되돌린다).
    async fn require_bindable_run(
        &self,
        bench_id: &str,
        run_id: &str,
    ) -> OrchestrationResult<bool> {
        let forbidden = || OrchestrationFailure::Forbidden(MESSAGE_RUN_OWNED_BY_OTHER_BENCH.into());
        if self.benches.engine.active_owner_of(run_id).await.as_deref() == Some(bench_id) {
            return Ok(false);
        }
        let hub = &self.benches.hub;
        if self.benches.engine.owner_of(run_id).await.is_some() || hub.has_run_history(run_id) {
            return Err(forbidden());
        }
        let existing = hub.run_owner(run_id);
        if existing.as_deref().is_some_and(|owner| owner != bench_id) {
            return Err(forbidden());
        }
        let recorded_elsewhere = self.repository.snapshot().is_ok_and(|sessions| {
            sessions.iter().any(|session| {
                session.bound_bench_id.as_deref() != Some(bench_id)
                    && session_records_run(session, run_id)
            })
        });
        if recorded_elsewhere || !hub.claim_run(run_id, bench_id) {
            return Err(forbidden());
        }
        Ok(existing.is_none())
    }

    /// 묶기 실패 뒤: 이 호출이 새로 만든 계획 id claim을 되돌린다(발행 이력이 생겼거나 작업 영역이 기록했으면 둔다).
    fn release_failed_prebind(&self, bench_id: &str, run_id: &str, newly_claimed: bool) {
        if newly_claimed && !self.references_run(run_id) {
            self.benches.hub.release_run_claim(run_id, bench_id);
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

    pub(crate) async fn emit_runtime_update_for(&self, bench_id: &str, reason: &str) {
        let bench_id = bench_id.to_owned();
        if let Ok(Some(session)) = self
            .blocking(move |service| service.get_for_bench(&bench_id))
            .await
        {
            self.emit_runtime_update(&session, reason);
        }
    }

    /// 어떤 작업 영역(묶임과 무관)이라도 이 run을 노드·세대로 기록하고 있는가(research R18).
    pub fn references_run(&self, run_id: &str) -> bool {
        self.repository.snapshot().is_ok_and(|sessions| {
            sessions
                .iter()
                .any(|session| session_records_run(session, run_id))
        })
    }

    /// 이 run을 기록한 작업 영역이 지금 묶인 작업대(research R17 허용 조건 2: 노드 `currentRunId`, 세대 `runId`, 과제
    /// 명령·시도의 run id). 재생·구독 권한이 같은 규칙을 쓴다.
    pub fn bench_with_workspace_run(&self, run_id: &str) -> Option<String> {
        let sessions = self.repository.snapshot().ok()?;
        sessions
            .iter()
            .find(|session| {
                session.bound_bench_id.is_some() && session_records_run(session, run_id)
            })
            .and_then(|session| session.bound_bench_id.clone())
    }

    /// Main 패널 run을 띄우기 전 검사(오늘 AW `resolve_agent_run_launch_principal`의 문구 그대로). 작업대에 묶인
    /// 작업 영역의 활성 세대가 이 run이어야 coordinator 역할을 준다.
    pub fn coordinator_launch_role(
        &self,
        bench_id: &str,
        run_id: &str,
    ) -> Result<OrchestrationLaunchRole, String> {
        let sessions = self.repository.snapshot().map_err(|error| error.message)?;
        let session = sessions
            .iter()
            .find(|session| session.bound_bench_id.as_deref() == Some(bench_id))
            .ok_or_else(|| "Main Coordinator workspace is unavailable.".to_string())?;
        let generation_id = session
            .active_coordinator_generation_id
            .clone()
            .ok_or_else(|| {
                "Main Coordinator generation must be bound before launch.".to_string()
            })?;
        let generation = session
            .generations
            .iter()
            .find(|generation| generation.id == generation_id)
            .ok_or_else(|| "Active Main Coordinator generation is unavailable.".to_string())?;
        if generation.run_id != run_id {
            return Err(
                "Main Coordinator generation does not match the run being launched.".to_string(),
            );
        }
        Ok(OrchestrationLaunchRole::Coordinator {
            workspace_id: session.id.clone(),
            generation_id,
        })
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
        let newly_claimed = if request.state == MainRunBindingState::Active {
            self.require_bindable_run(bench_id, &request.run_id).await?
        } else {
            false
        };
        let run_id = request.run_id.clone();
        let bench = bench_id.to_owned();
        let result = self
            .blocking(move |service| service.bind_main_run(&bench, request))
            .await;
        if result.is_err() {
            self.release_failed_prebind(bench_id, &run_id, newly_claimed);
        }
        result
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
        // 044 R14 표 5: 기동 토큰이 아직 `Pending`이면 실행을 막고 task만 취소한다(기동 경로가 준비한 run을 취소한다).
        if self.prevent_task_launch(&request.task_id)
            == crate::application::work_gate::LaunchCancel::Prevented
        {
            let (bench, task_id) = (bench_id.to_owned(), request.task_id.clone());
            let session = self
                .blocking(move |service| service.cancel_launching_task(&bench, request))
                .await?;
            let _ = self.scheduler.release(&task_id);
            return Ok(session);
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
        self: &Arc<Self>,
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
        self: &Arc<Self>,
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

    pub(crate) async fn stop_existing_task_worker(
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
        self: &Arc<Self>,
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
        let node_id = task
            .assigned_node_id
            .as_ref()
            .and_then(|node_id| snapshot.nodes.iter().find(|node| node.id == *node_id))
            .map(|node| node.id.clone())
            .ok_or_else(|| OrchestrationFailure::Plain("Assigned Child is unavailable.".into()))?;
        match self
            .start_child(bench_id, &snapshot, task_id, &node_id)
            .await
        {
            Ok(StartWorkerOutcome::Failed { code, message, .. }) => {
                // 앞 기동을 되돌리는 중이면 그 자리는 되돌리는 쪽이 반납한다(Codex r7).
                if code != super::agent_tools::LAUNCH_ROLLING_BACK {
                    let _ = self.scheduler.release(task_id);
                }
                Err(OrchestrationFailure::Plain(message))
            }
            Ok(_) => {
                self.snapshot_for(bench_id, MESSAGE_WORKSPACE_UNAVAILABLE)
                    .await
            }
            Err(error) => Err(OrchestrationFailure::Plain(error.message)),
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
        // research R18: 후계 run도 이 작업대가 소유한 살아 있는 run이거나 흔적 없는 계획 id여야 한다(화면은 후계를 띄우기 전에 교대한다).
        let previous_run = self.get(bench_id).await?.and_then(|session| {
            let generation_id = session.active_coordinator_generation_id.as_ref()?;
            session
                .generations
                .iter()
                .find(|generation| &generation.id == generation_id)
                .map(|generation| generation.run_id.clone())
        });
        let newly_claimed = self
            .require_bindable_run(bench_id, &request.successor_run_id)
            .await?;
        let successor = request.successor_run_id.clone();
        let bench = bench_id.to_owned();
        let session = match self
            .blocking(move |service| service.handoff_coordinator(&bench, request))
            .await
        {
            Ok(session) => session,
            Err(error) => {
                self.release_failed_prebind(bench_id, &successor, newly_claimed);
                return Err(error);
            }
        };
        if let (Some(run_id), Some(decorator)) = (previous_run, &self.benches.launch_decorator) {
            decorator.revoke_run(&run_id);
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
        drop(dispatcher);
        self.spawn_notification_pass(bench_id, "notificationRecovery");
        self.snapshot_for(bench_id, MESSAGE_NOT_BOOTSTRAPPED).await
    }

    /// 작업대 닫기(research R3): 오늘 창 닫힘의 `release_window`와 같은 규칙으로 복구 가능 전환. 묶임 표 제거는
    /// 묶임 저장소 commit이 한다.
    /// 작업대 닫기 hook. 041 Codex 리뷰: 복구 가능 전환의 저장이 실패해도(디스크 부족 등) 메모리 묶임은 반드시
    /// 푼다 — 그러지 않으면 작업대가 사라진 뒤 작업 영역이 "이미 묶임"으로 남아 서버 재시작 전까지 재개할 수 없다.
    /// 저장된 표시 상태(주의 필요 등)는 다음 변경 때 따라잡는다. 실패는 로그로 남긴다.
    pub fn release(&self, bench_id: &str) {
        if let Err(error) = self.service().release_bench(bench_id) {
            eprintln!(
                "[workbench] orchestration release for bench {bench_id} failed to persist: {}; unbinding in memory",
                error.message
            );
            self.repository.unbind_bench(bench_id);
        }
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
                guard.attempt,
                &guard.run_id,
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

/// 작업 영역이 이 run을 노드·세대·과제 명령·보고(이전 시도 포함)로 기록하고 있는가.
fn session_records_run(session: &OrchestrationSession, run_id: &str) -> bool {
    session
        .nodes
        .iter()
        .any(|node| node.current_run_id.as_deref() == Some(run_id))
        || session
            .generations
            .iter()
            .any(|generation| generation.run_id == run_id)
        || session
            .commands
            .iter()
            .any(|command| command.run_id == run_id)
        || session
            .reports
            .iter()
            .any(|report| report.reporter_run_id == run_id)
}
