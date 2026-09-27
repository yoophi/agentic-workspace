//! 서버 제어 상태(044): 작업 관문, 임대 표, 조립이 넣는 서버 host port, 그리고 생명주기 판정(research R7·R9·R10·R14,
//! contracts/server-lifecycle.md §5). `server.*`·`lease.*`·`desktop.*`·`bench.list` handler와 host의 감시 루프가 이것을 쓴다.
//!
//! - 활동 작업(`ActiveWork`) = 관문 예약 + 파생 값(진행 중·대기 task, 미소비 교환(데스크톱 임대가 있을 때만), 대상
//!   coordinator run이 살아 있는 미전달 알림, 이 프로세스의 ledger `pending`).
//! - ledger `unknown`은 활동 작업이 **아니다**. `unresolvedOperations`로만 보고한다(data-model ActiveWork, R7).
//! - 정지 판정은 G(관문 잠금) 아래에서 한다. 파생 값은 G 밖에서 읽으므로 읽기 전 활동 세대를 함께 넘긴다
//!   (`WorkGate::try_stop_at`) — 그 사이 끝난 작업이 있으면 다음 판정으로 미룬다.

use std::{
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use workbench_protocol::operations::{lease::LeaseClientKindDto, server::ActiveWorkDto};

use crate::{
    application::{
        bench_service::BenchServices,
        lease::LeaseTable,
        orchestration::runtime::OrchestrationRuntime,
        work_gate::{DrainMode, GateState, WorkGate},
    },
    domain::{
        agent_exchange::{AgentExchangeDelivery, AgentExchangeStatus},
        agent_orchestration::{CoordinatorNotificationStatus, TaskStatus, MAIN_AGENT_NODE_ID},
    },
    infrastructure::sqlite_ledger::SqliteOperationLedger,
    ports::{operation_ledger::LedgerState, server_host::ServerHost},
};

/// 파생 값(G 밖에서 읽음). `ActiveWorkDto`·`server.status`의 재료.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DerivedWork {
    pub orchestration_tasks: u64,
    pub queued_tasks: u64,
    /// 데스크톱 임대가 있을 때만 센 미소비 교환 수.
    pub pending_exchanges: u64,
    /// 데스크톱 임대가 없어 세지 않은 미소비 교환(보고만).
    pub undeliverable_exchanges: Vec<String>,
    pub pending_notifications: u64,
    pub pending_operations: u64,
    /// ledger `unknown`. 활동 작업이 아니다.
    pub unresolved_operations: u64,
}

impl DerivedWork {
    /// 정지를 막는 파생 합계. `unresolved_operations`·`undeliverable_exchanges`는 넣지 않는다.
    pub fn active_total(&self) -> u64 {
        self.orchestration_tasks
            + self.queued_tasks
            + self.pending_exchanges
            + self.pending_notifications
            + self.pending_operations
    }
}

/// 정지 요청 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopOutcome {
    /// `stopping`으로 전이했다(또는 이미 그렇다).
    Stopping,
    /// `wait`: 비우는 중. 활동 작업이 0이 되면 감시 루프가 전이한다.
    Draining,
    /// `default`: 활동 작업이 있어 거절(판정에 쓴 활동 작업).
    Blocked(ActiveWorkDto),
}

#[derive(Default)]
struct Idle {
    /// 유휴 조건(임대 0 + 활동 작업 0)이 시작된 시각.
    since: Option<(Instant, DateTime<Utc>)>,
}

pub struct ServerControl {
    work_gate: Arc<WorkGate>,
    leases: LeaseTable,
    host: OnceLock<Arc<dyn ServerHost>>,
    epoch: String,
    benches: Arc<BenchServices>,
    ledger: Arc<SqliteOperationLedger>,
    orchestration: Arc<OrchestrationRuntime>,
    idle: Mutex<Idle>,
    /// `stopping`에 들어가면 `true`. host의 감시 루프·serve가 기다린다.
    stopped: tokio::sync::watch::Sender<bool>,
}

impl ServerControl {
    pub fn new(
        work_gate: Arc<WorkGate>,
        epoch: String,
        benches: Arc<BenchServices>,
        ledger: Arc<SqliteOperationLedger>,
        orchestration: Arc<OrchestrationRuntime>,
    ) -> Self {
        Self {
            work_gate,
            leases: LeaseTable::default(),
            host: OnceLock::new(),
            epoch,
            benches,
            ledger,
            orchestration,
            idle: Mutex::new(Idle::default()),
            stopped: tokio::sync::watch::Sender::new(false),
        }
    }

    pub fn work_gate(&self) -> &Arc<WorkGate> {
        &self.work_gate
    }

    pub fn leases(&self) -> &LeaseTable {
        &self.leases
    }

    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    pub fn benches(&self) -> &Arc<BenchServices> {
        &self.benches
    }

    /// 조립이 한 번 넣는다. 두 번째는 무시하고 false.
    pub fn attach_host(&self, host: Arc<dyn ServerHost>) -> bool {
        self.host.set(host).is_ok()
    }

    pub fn host(&self) -> Option<&Arc<dyn ServerHost>> {
        self.host.get()
    }

    /// `stopping` 전이 알림(값이 `true`가 되면 멈춘 것).
    pub fn stopped(&self) -> tokio::sync::watch::Receiver<bool> {
        self.stopped.subscribe()
    }

    fn lock_idle(&self) -> std::sync::MutexGuard<'_, Idle> {
        self.idle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 유휴 조건이 시작된 시각(RFC 3339). 유휴가 아니면 없음.
    pub fn idle_since(&self) -> Option<String> {
        self.lock_idle().since.map(|(_, at)| at.to_rfc3339())
    }

    fn mark_stopped(&self) {
        self.stopped.send_replace(true);
    }

    /// 임대 획득: 유휴 비우기면 서빙으로 돌아간다(wait 비우기·정지는 돌아가지 않는다). 유휴 시계를 되돌린다.
    pub fn lease_acquired(&self) {
        self.work_gate.resume_serving();
        self.lock_idle().since = None;
    }

    /// 파생 값을 읽는다(G 밖). 열린 작업대만 본다 — 닫힌 작업대의 저장된 task·알림은 이 프로세스가 진행할 수 없다.
    pub async fn derive(&self) -> DerivedWork {
        let gate = &self.work_gate;
        let drain_started_at = gate.drain_started_at();
        let desktop_leased = self.leases.count_kind(LeaseClientKindDto::Desktop) > 0;
        let mut derived = DerivedWork {
            pending_operations: self
                .ledger
                .count_by_state(LedgerState::Pending)
                .unwrap_or(0) as u64,
            unresolved_operations: self
                .ledger
                .count_by_state(LedgerState::Unknown)
                .unwrap_or(0) as u64,
            ..DerivedWork::default()
        };
        let engine = &self.benches.engine;
        for (bench_id, _) in self.benches.registry.open_benches() {
            let alive = |run_id: String| {
                let bench_id = bench_id.clone();
                async move { engine.active_owner_of(&run_id).await.as_deref() == Some(&bench_id) }
            };
            if let Ok(Some(session)) = self.orchestration.get(&bench_id).await {
                for task in &session.tasks {
                    match task.status {
                        TaskStatus::Running => derived.orchestration_tasks += 1,
                        TaskStatus::Pending | TaskStatus::Ready => {
                            let before_drain = match drain_started_at {
                                None => true,
                                Some(started) => DateTime::parse_from_rfc3339(&task.created_at)
                                    .is_ok_and(|created| created < started),
                            };
                            if before_drain {
                                derived.queued_tasks += 1;
                            }
                        }
                        _ => {}
                    }
                }
                let coordinator_run = session
                    .nodes
                    .iter()
                    .find(|node| node.id == MAIN_AGENT_NODE_ID)
                    .and_then(|node| node.current_run_id.clone());
                if let (Some(generation), Some(run)) = (
                    session.active_coordinator_generation_id.as_deref(),
                    coordinator_run,
                ) {
                    let undelivered = session
                        .coordinator_notifications
                        .iter()
                        .filter(|notification| {
                            notification.generation_id == generation
                                && match notification.status {
                                    CoordinatorNotificationStatus::Pending
                                    | CoordinatorNotificationStatus::Dispatching => true,
                                    CoordinatorNotificationStatus::Failed => notification
                                        .failure
                                        .as_ref()
                                        .is_some_and(|failure| failure.retryable),
                                    _ => false,
                                }
                        })
                        .count() as u64;
                    if undelivered > 0 && alive(run).await {
                        derived.pending_notifications += undelivered;
                    }
                }
            }
            for exchange in self
                .benches
                .exchange_service()
                .list_exchanges(&bench_id)
                .await
            {
                if exchange.delivery == AgentExchangeDelivery::Draft
                    || !matches!(
                        exchange.status,
                        AgentExchangeStatus::Accepted | AgentExchangeStatus::Delivered
                    )
                    || gate.exchange_consumed(&exchange.request_id)
                {
                    continue;
                }
                let Some(target) = exchange.target.run_id.clone() else {
                    continue;
                };
                if !alive(target).await {
                    continue;
                }
                if desktop_leased {
                    derived.pending_exchanges += 1;
                } else {
                    derived.undeliverable_exchanges.push(exchange.request_id);
                }
            }
        }
        derived.undeliverable_exchanges.sort();
        derived
    }

    /// 정지 판정용 활동 작업. `accepted_calls`는 관문의 C-call 예약(런타임 안 호출)이다 — 전송 계층이 받은 호출 수는
    /// 판정하는 호출 자신을 포함하므로 쓰지 않는다(정지 뒤 host가 받은 호출을 drain한다).
    pub fn active_work(&self, derived: &DerivedWork) -> ActiveWorkDto {
        let gate = self.work_gate.active_work();
        ActiveWorkDto {
            busy_runs: gate.busy_runs as u64,
            orchestration_tasks: Some(derived.orchestration_tasks),
            queued_tasks: Some(derived.queued_tasks),
            pending_exchanges: Some(derived.pending_exchanges),
            pending_notifications: Some(derived.pending_notifications),
            pending_operations: Some(derived.pending_operations),
            accepted_calls: gate.accepted_calls as u64,
            reservations: self.work_gate.reservation_total() as u64,
        }
    }

    /// 활동 작업이 0이면 `stopping`으로 전이한다(G 아래 판정). 전이했거나 이미 `stopping`이면 true.
    pub async fn try_stop(&self) -> bool {
        if self.work_gate.is_stopping() {
            self.mark_stopped();
            return true;
        }
        let generation = self.work_gate.activity_generation();
        let derived = self.derive().await;
        let stopped = self
            .work_gate
            .try_stop_at(generation, || derived.active_total());
        if stopped {
            self.mark_stopped();
        }
        stopped
    }

    /// 비우기에 들어갈 때 서버가 스스로 알림 전달 한 바퀴를 돈다(R14 F2: 저장된 미전달 알림이 외부 계기 없이 wait를
    /// 막지 않게).
    fn notification_pass_for_open_benches(&self) {
        for (bench_id, _) in self.benches.registry.open_benches() {
            self.orchestration
                .spawn_notification_pass(&bench_id, "server-draining");
        }
    }

    /// `server.stop`(R10). `force`는 새 작업을 막고(비우기) 작업대를 모두 닫은 뒤 `stopping`으로 간다.
    pub async fn request_stop(
        &self,
        mode: workbench_protocol::operations::server::StopModeDto,
    ) -> StopOutcome {
        use workbench_protocol::operations::server::StopModeDto;
        match mode {
            StopModeDto::Default => {
                if self.work_gate.is_stopping() {
                    return StopOutcome::Stopping;
                }
                let generation = self.work_gate.activity_generation();
                let derived = self.derive().await;
                let active = self.active_work(&derived);
                if active.blocks_stop()
                    || !self
                        .work_gate
                        .try_stop_at(generation, || derived.active_total())
                {
                    return StopOutcome::Blocked(self.active_work(&derived));
                }
                self.mark_stopped();
                StopOutcome::Stopping
            }
            StopModeDto::Wait => {
                self.work_gate.begin_drain(DrainMode::Wait);
                self.notification_pass_for_open_benches();
                if self.try_stop().await {
                    StopOutcome::Stopping
                } else {
                    StopOutcome::Draining
                }
            }
            StopModeDto::Force => {
                self.force_stop().await;
                StopOutcome::Stopping
            }
        }
    }

    /// 강제 정지(SIGTERM·SIGINT와 같음): 비우기로 새 작업을 막고 → 작업대를 모두 닫고(run 취소·권한 대기 해제) → `stopping`.
    pub async fn force_stop(&self) {
        if !self.work_gate.is_stopping() {
            self.work_gate.begin_drain(DrainMode::Wait);
            self.benches.close_all().await;
            self.work_gate.force_stop();
        }
        self.mark_stopped();
    }

    /// 감시 한 바퀴(host가 주기적으로 부른다): 유휴 시계·유휴 비우기·wait 비우기의 정지 판정. 멈췄으면 true.
    pub async fn tick(&self, idle_timeout: Duration) -> bool {
        match self.work_gate.state() {
            GateState::Stopping => {
                self.mark_stopped();
                true
            }
            GateState::Draining(DrainMode::Wait) => self.try_stop().await,
            GateState::Draining(DrainMode::Idle) => {
                if self.leases.count() > 0 {
                    self.lease_acquired();
                    return false;
                }
                self.try_stop().await
            }
            GateState::Serving => {
                let quiet =
                    self.leases.count() == 0 && self.work_gate.reservation_total() == 0 && {
                        let derived = self.derive().await;
                        !self.active_work(&derived).blocks_stop()
                    };
                let mut idle = self.lock_idle();
                if !quiet {
                    idle.since = None;
                    return false;
                }
                let (started, _) = *idle
                    .since
                    .get_or_insert_with(|| (Instant::now(), Utc::now()));
                if started.elapsed() < idle_timeout {
                    return false;
                }
                drop(idle);
                self.work_gate.begin_drain(DrainMode::Idle);
                self.try_stop().await
            }
        }
    }
}
