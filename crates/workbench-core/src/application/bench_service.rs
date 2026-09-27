//! 작업대 서비스(040, research R1). 작업대 수명(열기·닫기)과 공통 검사, 그리고 run·교환 서비스가 함께 쓰는
//! 의존성(엔진·hub·데스크톱 포트·세대 멱등)을 한곳에 둔다.

use std::sync::{Arc, Mutex};

use workbench_protocol::{
    operations::bench::{BenchCloseOutput, BenchOpenOutput},
    AuthenticatedPrincipal, FaultCode, RequestId, WorkbenchFault,
};

use crate::{
    application::{
        agent_exchange_service::AgentExchangeService,
        epoch_idempotency::{open_scope, EpochIdempotency},
    },
    infrastructure::{
        bench::in_memory_bench_registry::{
            wait_closed, BenchAdmission, BenchError, BenchView, CloseStart, CloseTicket,
            InMemoryBenchRegistry,
        },
        event_hub::EventHub,
        exchange::{
            hub_event_sink::HubExchangeEventSink,
            in_memory_workspace_registry::InMemoryAgentWorkspaceRegistry, EngineRunOwners,
        },
        run::workbench_run_sink::WorkbenchRunSink,
    },
    ports::{
        desktop_bridge::{DesktopBridge, RunLaunchDecorator, RunTerminalHook},
        run_engine::RunEngine,
    },
};
use workbench_protocol::events::StreamKind;

pub const MESSAGE_BENCH_NOT_FOUND: &str = "bench not found.";
pub const MESSAGE_BENCH_FORBIDDEN: &str = "bench belongs to another principal.";
pub const MESSAGE_BENCH_LIMIT: &str = "too many open benches.";
pub const MESSAGE_NOT_DIRECTORY: &str = "Workspace path must be a directory.";

/// 작업대가 닫힐 때 정리할 것(orchestration 작업 영역 복구 가능 전환 등, 041). 소유 run 취소 뒤, 스트림 제거
/// 전에 닫기 정리 task 안에서 차례로 await된다. 입장권·저장소 경계를 기다리는 흐름과 교착하지 않도록 hook은
/// 입장권을 잡지 않는다(research R2·R3).
pub type BenchCloseHook = Arc<
    dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync,
>;

pub struct BenchServices {
    pub registry: Arc<InMemoryBenchRegistry>,
    pub engine: Arc<dyn RunEngine>,
    pub hub: Arc<EventHub>,
    pub desktop: Option<Arc<dyn DesktopBridge>>,
    pub terminal_hook: Option<Arc<dyn RunTerminalHook>>,
    pub launch_decorator: Option<Arc<dyn RunLaunchDecorator>>,
    pub idempotency: Arc<EpochIdempotency>,
    /// 작업대별 교환 작업 영역(040 US2).
    pub exchange_registry: InMemoryAgentWorkspaceRegistry,
    close_hooks: Mutex<Vec<BenchCloseHook>>,
}

pub fn bench_fault(request_id: &RequestId, error: BenchError) -> WorkbenchFault {
    match error {
        BenchError::NotFound => WorkbenchFault::new(
            FaultCode::NotFound,
            request_id.clone(),
            MESSAGE_BENCH_NOT_FOUND,
        ),
        BenchError::Forbidden => WorkbenchFault::new(
            FaultCode::Forbidden,
            request_id.clone(),
            MESSAGE_BENCH_FORBIDDEN,
        ),
        BenchError::Limit => WorkbenchFault::new(
            FaultCode::RateLimited,
            request_id.clone(),
            MESSAGE_BENCH_LIMIT,
        ),
    }
}

impl BenchServices {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registry: Arc<InMemoryBenchRegistry>,
        engine: Arc<dyn RunEngine>,
        hub: Arc<EventHub>,
        desktop: Option<Arc<dyn DesktopBridge>>,
        terminal_hook: Option<Arc<dyn RunTerminalHook>>,
        launch_decorator: Option<Arc<dyn RunLaunchDecorator>>,
        idempotency: Arc<EpochIdempotency>,
    ) -> Self {
        Self {
            registry,
            engine,
            hub,
            desktop,
            terminal_hook,
            launch_decorator,
            idempotency,
            exchange_registry: InMemoryAgentWorkspaceRegistry::default(),
            close_hooks: Mutex::default(),
        }
    }

    /// 교환 서비스. 소유 조회는 run 엔진, 발행은 교환 스트림 + 데스크톱 전달.
    pub fn exchange_service(
        &self,
    ) -> AgentExchangeService<InMemoryAgentWorkspaceRegistry, EngineRunOwners, HubExchangeEventSink>
    {
        AgentExchangeService::new(
            self.exchange_registry.clone(),
            EngineRunOwners(Arc::clone(&self.engine)),
            HubExchangeEventSink::new(Arc::clone(&self.hub), self.desktop.clone()),
        )
    }

    pub fn add_close_hook(&self, hook: BenchCloseHook) {
        self.close_hooks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(hook);
    }

    /// 작업대 단위 run 이벤트 sink.
    pub fn run_sink(&self, bench_id: &str) -> WorkbenchRunSink {
        WorkbenchRunSink::new(
            bench_id,
            Arc::clone(&self.hub),
            self.desktop.clone(),
            self.terminal_hook.clone(),
        )
    }

    pub fn open(
        &self,
        request_id: &RequestId,
        principal: &AuthenticatedPrincipal,
        working_directory: &str,
    ) -> Result<BenchOpenOutput, WorkbenchFault> {
        let canonical = std::fs::canonicalize(working_directory).map_err(|error| {
            WorkbenchFault::invalid_argument(
                request_id.clone(),
                format!("Failed to resolve workspace path: {error}"),
                Some("/workingDirectory"),
            )
        })?;
        if !canonical.is_dir() {
            return Err(WorkbenchFault::invalid_argument(
                request_id.clone(),
                MESSAGE_NOT_DIRECTORY,
                Some("/workingDirectory"),
            ));
        }
        let view = self
            .registry
            .open(
                canonical.to_string_lossy().into_owned(),
                principal.subject.clone(),
            )
            .map_err(|error| bench_fault(request_id, error))?;
        Ok(BenchOpenOutput {
            bench_id: view.id,
            working_directory: view.working_directory,
        })
    }

    pub fn resolve(
        &self,
        request_id: &RequestId,
        principal: &AuthenticatedPrincipal,
        bench_id: &str,
    ) -> Result<BenchView, WorkbenchFault> {
        self.registry
            .resolve(bench_id, &principal.subject)
            .map_err(|error| bench_fault(request_id, error))
    }

    pub fn admit(
        &self,
        request_id: &RequestId,
        principal: Option<&AuthenticatedPrincipal>,
        bench_id: &str,
    ) -> Result<BenchAdmission, WorkbenchFault> {
        self.registry
            .admit(bench_id, principal.map(|principal| &principal.subject))
            .map_err(|error| bench_fault(request_id, error))
    }

    /// 닫기(research R1): `Closing` 전이 → 입장한 동작 대기 → 소유 run 취소 → 정리 hook → 스트림 제거 → 삭제.
    /// `Closing` 전이 뒤 정리는 별도 task에서 끝까지 간다: 호출자 future가 취소돼도 작업대가 `Closing`에 멈춰
    /// run·스트림·작업대 자리(상한)를 붙잡지 않는다.
    pub async fn close(
        self: &Arc<Self>,
        request_id: &RequestId,
        principal: &AuthenticatedPrincipal,
        bench_id: &str,
    ) -> Result<BenchCloseOutput, WorkbenchFault> {
        let start = self
            .registry
            .begin_close(bench_id, &principal.subject)
            .map_err(|error| bench_fault(request_id, error))?;
        let ticket = match start {
            CloseStart::Unknown => {
                return Ok(BenchCloseOutput {
                    closed: false,
                    cancelled_runs: Vec::new(),
                })
            }
            CloseStart::InProgress(done) => {
                wait_closed(done).await;
                return Ok(BenchCloseOutput {
                    closed: false,
                    cancelled_runs: Vec::new(),
                });
            }
            CloseStart::Started(ticket) => ticket,
        };
        let services = Arc::clone(self);
        let bench_id = bench_id.to_owned();
        tokio::spawn(async move { services.finish_close(ticket, &bench_id).await })
            .await
            .map_err(|error| WorkbenchFault::internal(request_id.clone(), error.to_string()))
    }

    async fn finish_close(&self, ticket: CloseTicket, bench_id: &str) -> BenchCloseOutput {
        let drained = ticket.wait_admissions().await;
        let cancelled_runs = self.engine.cancel_runs_owned_by(bench_id).await;
        let hooks = self
            .close_hooks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        self.exchange_registry.remove_bench_now(bench_id);
        for hook in hooks {
            hook(bench_id.to_owned()).await;
        }
        self.hub.remove_stream(StreamKind::Exchange, bench_id);
        self.hub.remove_stream(StreamKind::Bench, bench_id);
        self.idempotency.drop_bench(bench_id);
        self.idempotency
            .forget_open_of(&open_scope(&ticket.bench.opened_by), bench_id);
        drop(drained);
        self.registry.finish_close(ticket);
        BenchCloseOutput {
            closed: true,
            cancelled_runs,
        }
    }
}
