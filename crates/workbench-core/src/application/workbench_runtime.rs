//! `Workbench` 구현체. protocol 검사 → authorization → handler dispatch.
//! 기동 순서(`bootstrap`): 디렉터리 → ledger open·migrate → GC → coordinator → reconcile → revision 캐시.

use std::sync::Arc;

use acp_agent_core::domain::events::{LifecycleStatus, RunEvent};
use async_trait::async_trait;
use chrono::Utc;
use workbench_protocol::{
    events::{parse_stream_id, StreamKind},
    operations::spec_for,
    AuthenticatedPrincipal, CallReply, CallRequest, EventEnvelope, EventStream, FaultCode,
    OperationKind, RequestId, Subscription, Workbench, WorkbenchFault, PROTOCOL_VERSION,
};

pub const MESSAGE_KEY_ON_QUERY: &str = "idempotencyKey is only accepted for command operations.";

use crate::{
    application::{
        authorization,
        bench_service::{BenchServices, MESSAGE_BENCH_FORBIDDEN, MESSAGE_BENCH_NOT_FOUND},
        epoch_idempotency::{EpochIdempotency, EpochIdempotencyLimits},
        reconcilers::ReconcilerRegistry,
        registry::{CallContext, Registry},
    },
    domain::project_error::ProjectError,
    infrastructure::event_hub::{EventHub, EventHubLimits},
    infrastructure::{
        bench::in_memory_bench_registry::{BenchAdmission, BenchLimits, InMemoryBenchRegistry},
        fs::acp_session_store::JsonAcpSessionStore,
        run::{acp_run_engine::AcpRunEngine, workbench_run_sink::WorkbenchRunSink},
    },
    infrastructure::{
        data_paths::DataPaths,
        sqlite_ledger::SqliteOperationLedger,
        storage_coordinator::{Repositories, StorageCoordinator, STORE_AGGREGATES},
    },
    ports::{
        agent_catalog_reader::AgentCatalogReader,
        desktop_bridge::{DesktopBridge, RunLaunchDecorator, RunTerminalHook},
        event_publisher::RunEventPublisher,
        operation_ledger::{LedgerError, OperationLedger},
        provider_session_repository::ProviderSessionRepository,
        run_engine::RunEngine,
    },
};

/// 실행 환경을 읽는 어댑터(038 US3). 운영은 `production()`, 테스트는 stub을 넣는다.
#[derive(Clone)]
pub struct RuntimeAdapters {
    pub agent_catalog: Arc<dyn AgentCatalogReader>,
    pub provider_sessions: Arc<dyn ProviderSessionRepository>,
    /// 이벤트 hub 한도(039). 테스트는 낮춘 값으로 overflow·정리를 재현한다.
    pub event_limits: EventHubLimits,
    /// run 기계(040). `None`이면 bootstrap이 acp-agent-core 기반 `AcpRunEngine`을 만든다. 테스트는 가짜 엔진.
    pub run_engine: Option<Arc<dyn RunEngine>>,
    /// 데스크톱 전달·run 종료 후처리·`run.start` 보강(040). 없으면 no-op.
    pub desktop: Option<Arc<dyn DesktopBridge>>,
    pub terminal_hook: Option<Arc<dyn RunTerminalHook>>,
    pub launch_decorator: Option<Arc<dyn RunLaunchDecorator>>,
    pub bench_limits: BenchLimits,
    pub idempotency_limits: EpochIdempotencyLimits,
}

impl RuntimeAdapters {
    /// 환경 변수 기반 agent catalog와 provider 로컬 세션 파일(오늘 command가 쓰던 구현).
    pub fn production() -> Self {
        Self {
            agent_catalog: Arc::new(
                acp_agent_core::infrastructure::agent_catalog::ConfigurableAgentCatalog::from_env(),
            ),
            provider_sessions: Arc::new(
                crate::infrastructure::fs::provider_session_repository::FsProviderSessionRepository::new(),
            ),
            event_limits: EventHubLimits::default(),
            run_engine: None,
            desktop: None,
            terminal_hook: None,
            launch_decorator: None,
            bench_limits: BenchLimits::default(),
            idempotency_limits: EpochIdempotencyLimits::default(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error("Failed to create app data directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Ledger(#[from] LedgerError),
    #[error("{0}")]
    Storage(#[from] ProjectError),
}

/// 테스트 전용 주입점. `test-hooks` feature가 꺼진 빌드에서는 항상 no-op이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashPoint {
    AfterPending,
    AfterJsonSave,
    BeforeApplied,
}

impl CrashPoint {
    /// 부작용(JSON 저장 또는 Git 명령) 직후. Git 변경 테스트에서 의미가 드러나도록 붙인 이름이다.
    pub const AFTER_SIDE_EFFECT: CrashPoint = CrashPoint::AfterJsonSave;
}

/// 프로세스는 살아 있지만 특정 저장 단계가 실패하는 상황을 주입한다(crash와 달리 handler가 계속 실행된다).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailPoint {
    /// JSON 저장은 성공했지만 ledger `complete` 트랜잭션이 실패한다.
    LedgerComplete,
}

#[derive(Default)]
pub struct TestHooks {
    #[cfg(feature = "test-hooks")]
    crash_point: std::sync::Mutex<Option<CrashPoint>>,
    #[cfg(feature = "test-hooks")]
    fail_point: std::sync::Mutex<Option<FailPoint>>,
}

impl TestHooks {
    #[cfg(feature = "test-hooks")]
    pub fn set_crash_point(&self, point: Option<CrashPoint>) {
        *self.crash_point.lock().unwrap() = point;
    }

    #[cfg(feature = "test-hooks")]
    pub fn set_fail_point(&self, point: Option<FailPoint>) {
        *self.fail_point.lock().unwrap() = point;
    }

    /// 설정된 지점이면 true. handler는 해당 저장 단계를 실패한 것으로 처리한다.
    pub fn should_fail(&self, point: FailPoint) -> bool {
        #[cfg(feature = "test-hooks")]
        {
            return *self.fail_point.lock().unwrap() == Some(point);
        }
        #[allow(unreachable_code)]
        {
            let _ = point;
            false
        }
    }

    /// 설정된 지점이면 `Err`를 돌려주어 handler를 그 자리에서 중단시킨다(프로세스 종료 흉내).
    pub fn crash_if(&self, point: CrashPoint) -> Result<(), CrashInjected> {
        #[cfg(feature = "test-hooks")]
        {
            if *self.crash_point.lock().unwrap() == Some(point) {
                return Err(CrashInjected(point));
            }
        }
        let _ = point;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrashInjected(pub CrashPoint);

pub struct WorkbenchRuntime {
    paths: DataPaths,
    events: Arc<EventHub>,
    ledger: Arc<SqliteOperationLedger>,
    coordinator: Arc<StorageCoordinator>,
    registry: Registry,
    reconcilers: ReconcilerRegistry,
    hooks: Arc<TestHooks>,
    benches: Arc<BenchServices>,
}

impl WorkbenchRuntime {
    pub fn bootstrap(paths: DataPaths) -> Result<Arc<Self>, BootstrapError> {
        Self::bootstrap_with(paths, RuntimeAdapters::production())
    }

    pub fn bootstrap_with(
        paths: DataPaths,
        adapters: RuntimeAdapters,
    ) -> Result<Arc<Self>, BootstrapError> {
        paths.ensure_dirs()?;

        let ledger = Arc::new(SqliteOperationLedger::open(&paths)?);
        ledger.migrate()?;
        ledger.gc_expired(Utc::now())?;

        let coordinator = Arc::new(StorageCoordinator::new(Repositories::json(&paths)));
        for aggregate in STORE_AGGREGATES {
            coordinator.set_revision_of(aggregate, ledger.current_revision(aggregate)?);
        }

        // 세대: 기동마다 새로 만든다. 모든 이벤트·gap·describe가 같은 값을 싣는다(ADR core 0002).
        let epoch = uuid::Uuid::new_v4().to_string();
        let events = EventHub::with_watcher(
            epoch.clone(),
            adapters.event_limits,
            Some(crate::infrastructure::fs::worktree_watcher::start_watch()),
        );

        let engine: Arc<dyn RunEngine> = match adapters.run_engine.clone() {
            Some(engine) => engine,
            None => Arc::new(AcpRunEngine::new(
                acp_agent_core::infrastructure::agent_session_registry::AppState::default(),
                Arc::new(JsonAcpSessionStore::from_paths(&paths)),
            )),
        };
        let benches = Arc::new(BenchServices::new(
            Arc::new(InMemoryBenchRegistry::new(adapters.bench_limits)),
            engine,
            Arc::clone(&events),
            adapters.desktop.clone(),
            adapters.terminal_hook.clone(),
            adapters.launch_decorator.clone(),
            Arc::new(EpochIdempotency::new(adapters.idempotency_limits)),
        ));

        let hooks = Arc::new(TestHooks::default());
        let (registry, reconcilers) = crate::application::handlers::build_registry(
            Arc::clone(&ledger),
            Arc::clone(&coordinator),
            Arc::clone(&hooks),
            &adapters,
            &epoch,
            &benches,
        );

        // 중단된 변경의 적용 여부를 operation별 reconciler로 판정한다. 자동 재실행은 하지 않는다(FR-009).
        // 등록되지 않은 operation(upsert·수정)은 unknown이다(research R6).
        ledger.reconcile_pending(&mut |record| reconcilers.resolve(record))?;
        for aggregate in STORE_AGGREGATES {
            coordinator.set_revision_of(aggregate, ledger.current_revision(aggregate)?);
        }

        Ok(Arc::new(Self {
            paths,
            events,
            ledger,
            coordinator,
            registry,
            reconcilers,
            hooks,
            benches,
        }))
    }

    /// 작업대·run·교환 서비스(040).
    pub fn benches(&self) -> &Arc<BenchServices> {
        &self.benches
    }

    /// run 기계(040). AW 과도기 orchestration은 `acp_registry()`·`acp_session_store()`로 같은 기계를 빌린다.
    pub fn run_engine(&self) -> &Arc<dyn RunEngine> {
        &self.benches.engine
    }

    /// 041 전 과도기: AW orchestration이 자식 run을 작업대 단위 sink로 발행하게 한다. 041에서 제거한다.
    pub fn run_sink(&self, bench_id: &str) -> WorkbenchRunSink {
        self.benches.run_sink(bench_id)
    }

    /// 041 전 과도기: AW orchestration이 run을 띄우는 동안 작업대 입장권을 잡는다(research R1·R12). 041에서 제거한다.
    pub fn admit(&self, bench_id: &str) -> Result<BenchAdmission, WorkbenchFault> {
        self.benches
            .admit(&workbench_protocol::RequestId::random(), None, bench_id)
    }

    /// 작업대에 속한 스트림(`exchange:<id>`·`bench:<id>`)은 작업대를 연 주체만 구독한다(040). hub의 scope 검사는
    /// 스트림 종류만 보므로, 등록 전에 cursor 전부를 여기서 검사한다. agent principal은 구독하지 않는다 — MCP 도구는
    /// 요청·응답만 쓰고 주체가 작업대를 연 주체와 다르다. 닫힌 작업대(제거 표식)는 hub가 `Gap(evicted)`로 답하게
    /// 두고, 한 번도 없던 id는 나중에 열릴 스트림에 미리 붙지 않도록 거절한다.
    fn authorize_bench_streams(
        &self,
        principal: &AuthenticatedPrincipal,
        request: &Subscription,
    ) -> Result<(), WorkbenchFault> {
        for cursor in &request.cursors {
            let Some((kind, bench_id)) = parse_stream_id(&cursor.stream_id) else {
                continue;
            };
            if !matches!(kind, StreamKind::Exchange | StreamKind::Bench) {
                continue;
            }
            match self.benches.registry.owner(bench_id) {
                Some(owner) if owner == principal.subject => {}
                Some(_) => {
                    return Err(WorkbenchFault::new(
                        FaultCode::Forbidden,
                        RequestId::random(),
                        MESSAGE_BENCH_FORBIDDEN,
                    ))
                }
                None if self.events.is_evicted(&cursor.stream_id) => {}
                None => {
                    return Err(WorkbenchFault::new(
                        FaultCode::NotFound,
                        RequestId::random(),
                        MESSAGE_BENCH_NOT_FOUND,
                    ))
                }
            }
        }
        Ok(())
    }

    /// 이벤트 hub(039). 발행은 `publish_run`을, 구독은 `Workbench::events`를 쓴다.
    pub fn events_hub(&self) -> &Arc<EventHub> {
        &self.events
    }

    pub fn epoch(&self) -> &str {
        self.events.epoch()
    }

    pub fn reconcilers(&self) -> &ReconcilerRegistry {
        &self.reconcilers
    }

    pub fn paths(&self) -> &DataPaths {
        &self.paths
    }

    pub fn ledger(&self) -> &Arc<SqliteOperationLedger> {
        &self.ledger
    }

    pub fn coordinator(&self) -> &Arc<StorageCoordinator> {
        &self.coordinator
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn hooks(&self) -> &Arc<TestHooks> {
        &self.hooks
    }
}

#[async_trait]
impl Workbench for WorkbenchRuntime {
    async fn call(
        &self,
        principal: AuthenticatedPrincipal,
        request: CallRequest,
    ) -> Result<CallReply, WorkbenchFault> {
        if request.protocol_version != PROTOCOL_VERSION {
            return Err(WorkbenchFault::unsupported_protocol(
                request.request_id,
                request.protocol_version,
                PROTOCOL_VERSION,
            ));
        }
        if let Some(timeout) = request.timeout_ms {
            if !(1..=600_000).contains(&timeout) {
                return Err(WorkbenchFault::invalid_argument(
                    request.request_id,
                    "timeoutMs는 1..=600000 범위여야 합니다.",
                    Some("/timeoutMs"),
                ));
            }
        }

        let operation =
            authorization::resolve_operation(&request.request_id, &principal, &request.operation)?;
        // 조회에 멱등성 키를 실어 보내는 것은 계약 위반이다(contracts §1 규칙 1). 조용히 무시하면 호출자가
        // 재시도 중복 제거가 되는 줄 오해한다.
        if matches!(spec_for(operation).kind, OperationKind::Query)
            && request.idempotency_key.is_some()
        {
            return Err(WorkbenchFault::invalid_argument(
                request.request_id,
                MESSAGE_KEY_ON_QUERY,
                Some("/idempotencyKey"),
            ));
        }
        let handler = self.registry.handler_for(operation).ok_or_else(|| {
            WorkbenchFault::internal(
                request.request_id.clone(),
                format!("operation {operation}의 handler가 등록되지 않았습니다."),
            )
        })?;

        let ctx = CallContext {
            principal,
            request_id: request.request_id,
            idempotency_key: request.idempotency_key,
            expected_revision: request.expected_revision,
        };
        handler.handle(&ctx, request.input).await
    }

    fn events(
        &self,
        principal: AuthenticatedPrincipal,
        request: Subscription,
    ) -> Result<EventStream, WorkbenchFault> {
        self.authorize_bench_streams(&principal, &request)?;
        self.events.subscribe(&principal, request)
    }
}

impl RunEventPublisher for WorkbenchRuntime {
    fn publish_run(
        &self,
        run_id: &str,
        event: &RunEvent,
        terminal: bool,
        deliver: &mut dyn FnMut(&EventEnvelope),
    ) -> Option<EventEnvelope> {
        let body = serde_json::to_value(event).expect("run event serializes");
        self.events.publish_state(
            StreamKind::Run,
            run_id,
            workbench_protocol::events::RUN_EVENT_V1,
            body,
            terminal,
            deliver,
        )
    }
}

/// 오늘과 같은 terminal 판정: `Lifecycle Completed | Cancelled`.
pub fn is_terminal_run_event(event: &RunEvent) -> bool {
    matches!(
        event,
        RunEvent::Lifecycle {
            status: LifecycleStatus::Completed | LifecycleStatus::Cancelled,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use workbench_protocol::{FaultCode, OperationId, RequestId};

    use super::*;

    fn runtime() -> (tempfile::TempDir, Arc<WorkbenchRuntime>) {
        let dir = tempfile::tempdir().unwrap();
        let runtime = WorkbenchRuntime::bootstrap(DataPaths::new(dir.path())).unwrap();
        (dir, runtime)
    }

    fn request(operation: &str, protocol_version: u16) -> CallRequest {
        CallRequest {
            protocol_version,
            operation: operation.into(),
            request_id: RequestId::new("r1").unwrap(),
            input: json!({}),
            idempotency_key: None,
            expected_revision: None,
            timeout_ms: None,
        }
    }

    #[tokio::test]
    async fn rejects_unsupported_protocol_before_anything_else() {
        let (_dir, runtime) = runtime();
        let fault = runtime
            .call(
                AuthenticatedPrincipal::desktop(),
                request("project.list", 2),
            )
            .await
            .unwrap_err();
        assert_eq!(fault.code, FaultCode::UnsupportedProtocol);
        assert_eq!(
            fault.details.unwrap()["supportedProtocolRevisions"],
            json!([1])
        );
    }

    #[tokio::test]
    async fn unknown_operation_is_not_found_and_forbidden_needs_scope() {
        let (_dir, runtime) = runtime();
        let fault = runtime
            .call(
                AuthenticatedPrincipal::desktop(),
                request("project.rename", 1),
            )
            .await
            .unwrap_err();
        assert_eq!(fault.code, FaultCode::NotFound);
        let fault = runtime
            .call(
                AuthenticatedPrincipal::test_readonly(),
                request("project.create", 1),
            )
            .await
            .unwrap_err();
        assert_eq!(fault.code, FaultCode::Forbidden);
    }

    /// 039: `events`가 구독을 연다. 037의 `events_are_unsupported_in_037`를 대체한다(계약이 뒤집혔다).
    #[tokio::test]
    async fn events_reject_unknown_stream_kind_and_describe_carries_epoch() {
        let (_dir, runtime) = runtime();
        let fault = runtime
            .events(
                AuthenticatedPrincipal::desktop(),
                Subscription {
                    cursors: vec![workbench_protocol::StreamCursor {
                        stream_id: "orchestration:w1".into(),
                        epoch: runtime.epoch().into(),
                        after_sequence: 0,
                    }],
                },
            )
            .unwrap_err();
        assert_eq!(fault.code, FaultCode::InvalidArgument);
        assert!(!runtime.epoch().is_empty());
    }

    #[tokio::test]
    async fn bootstrap_creates_ledger_and_zero_revision() {
        let (_dir, runtime) = runtime();
        assert!(runtime.paths().ledger_file().exists());
        assert_eq!(runtime.coordinator().revision(), 0);
        assert!(runtime
            .registry()
            .handler_for(OperationId::ProjectList)
            .is_some());
    }
}
