//! 작업대 서비스(040, research R1). 작업대 수명(열기·닫기)과 공통 검사, 그리고 run·교환 서비스가 함께 쓰는
//! 의존성(엔진·hub·데스크톱 포트·세대 멱등)을 한곳에 둔다.

use std::sync::{Arc, Mutex, OnceLock};

use workbench_protocol::{
    operations::bench::{BenchCloseOutput, BenchOpenOutput},
    AuthenticatedPrincipal, FaultCode, RequestId, WorkbenchFault,
};

use crate::{
    application::{
        agent_exchange_service::AgentExchangeService,
        epoch_idempotency::{open_scope, EpochIdempotency},
        work_gate::WorkGate,
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
    /// 작업 관문(044 R14). 조립이 한 번 넣는다. 없으면(단위 시험 조립) 관문 판정 없이 동작한다.
    work_gate: OnceLock<Arc<WorkGate>>,
}

/// 소유자 주체(044)는 작업대 소유 판정을 우회한다.
pub fn is_owner(principal: &AuthenticatedPrincipal) -> bool {
    principal.kind == workbench_protocol::PrincipalKind::Owner
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
            work_gate: OnceLock::new(),
        }
    }

    /// 작업 관문을 넣는다(한 번만).
    pub fn attach_work_gate(&self, gate: Arc<WorkGate>) {
        let _ = self.work_gate.set(gate);
    }

    pub fn work_gate(&self) -> Option<&Arc<WorkGate>> {
        self.work_gate.get()
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
        // 044: 소유자 주체는 작업대 소유 판정을 우회한다(contracts/server-lifecycle.md §4 우회 지점 1).
        let resolved = if is_owner(principal) {
            self.registry.resolve_any(bench_id)
        } else {
            self.registry.resolve(bench_id, &principal.subject)
        };
        resolved.map_err(|error| bench_fault(request_id, error))
    }

    pub fn admit(
        &self,
        request_id: &RequestId,
        principal: Option<&AuthenticatedPrincipal>,
        bench_id: &str,
    ) -> Result<BenchAdmission, WorkbenchFault> {
        let subject = principal
            .filter(|principal| !is_owner(principal))
            .map(|principal| &principal.subject);
        self.registry
            .admit(bench_id, subject)
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
        // 044: 소유자는 연 주체로서 닫는다(없는 작업대는 오늘처럼 `closed: false`).
        let subject = if is_owner(principal) {
            self.registry
                .owner(bench_id)
                .unwrap_or_else(|| principal.subject.clone())
        } else {
            principal.subject.clone()
        };
        self.close_as(request_id, &subject, bench_id).await
    }

    /// 044 창 폐기(`desktop.retireWindow{closeBench}`): `subject`가 연 작업대를 모두 닫고, 닫은 작업대 id를 돌려준다.
    pub async fn close_opened_by(
        self: &Arc<Self>,
        request_id: &RequestId,
        subject: &workbench_protocol::PrincipalSubject,
    ) -> Vec<String> {
        let mut closed = Vec::new();
        for (bench_id, owner) in self.registry.open_benches() {
            if &owner != subject {
                continue;
            }
            if let Ok(output) = self.close_as(request_id, subject, &bench_id).await {
                if output.closed {
                    closed.push(bench_id);
                }
            }
        }
        closed.sort();
        closed
    }

    /// 열린 작업대 수.
    pub fn open_count(&self) -> usize {
        self.registry.len()
    }

    /// 서버 종료(042): 연 주체와 무관하게 열린 작업대를 모두 닫는다. 작업대 닫기는 소유 run을 취소하고 그 run의 권한
    /// 대기를 지운다 — 수락이 닫혀 응답·취소 요청이 더 들어올 수 없는 종료 중에, 사용자 응답을 기다리던 받아들인
    /// 호출이 끝나 drain이 완료된다. 닫은 작업대 수를 돌려준다.
    pub async fn close_all(self: &Arc<Self>) -> usize {
        let open = self.registry.open_benches();
        let request_id = RequestId::random();
        let mut closed = 0;
        for (bench_id, owner) in open {
            if let Ok(output) = self.close_as(&request_id, &owner, &bench_id).await {
                closed += usize::from(output.closed);
            }
        }
        closed
    }

    async fn close_as(
        self: &Arc<Self>,
        request_id: &RequestId,
        subject: &workbench_protocol::PrincipalSubject,
        bench_id: &str,
    ) -> Result<BenchCloseOutput, WorkbenchFault> {
        let start = self
            .registry
            .begin_close(bench_id, subject)
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
        // 엔진은 소유 표 순회 순서로 돌려준다 — 결과는 run id 순으로 정렬해 결정적으로 둔다(041 계약 fixture).
        let mut cancelled_runs = self.engine.cancel_runs_owned_by(bench_id).await;
        cancelled_runs.sort();
        // 닫힌 작업대의 run은 끝났다 — MCP 토큰도 폐기한다(041 Codex 리뷰: 다른 작업대가 작업 영역을 재개해도 이전
        // 토큰이 다시 쓰이지 않게. 역할 판정도 살아 있는 소유를 요구해 이중으로 막는다).
        if let Some(decorator) = &self.launch_decorator {
            for run_id in &cancelled_runs {
                decorator.revoke_run(run_id);
            }
        }
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
