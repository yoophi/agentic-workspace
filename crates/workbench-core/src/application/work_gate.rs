//! 작업 관문(044, research R14): 서버 상태, 활동 예약, 교환 전달 소비, task 기동 토큰, 정지 판정을 **한 잠금 G** 아래에 둔다.
//!
//! - G는 짧게만 잡는다(await를 걸치지 않는다). 비동기 작업은 G 아래에서 예약을 먼저 만들고, G 밖에서 수행하고, 결과를
//!   다시 G 아래에서 확정·해제한다.
//! - 예약은 **드롭하면 해제되는 guard**다. 성공·오류·취소·abort(future drop) 어느 쪽으로 끝나도 해제가 빠지지 않는다.
//! - 정지 판정은 G 아래에서만 한다. 판정 뒤(`stopping`)의 예약은 실패한다 — "0으로 보고 멈췄는데 방금 예약된 실행"이 없다.
//!
//! 활동 작업(유휴·wait 정지를 막는 것)은 예약 수 + 조립이 넘기는 파생 수(ledger `pending`, 미소비 교환, 미전달 알림 등)다.
//! 세션 수는 쓰지 않는다: 바쁜 run = A-turn 예약이 있는 run(Codex 재검토 E1).

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard},
};

/// 서버 상태(contracts/server-lifecycle.md §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateState {
    Serving,
    Draining(DrainMode),
    Stopping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainMode {
    /// 유휴 정지 판정 중. 임대가 잡히면 `Serving`으로 돌아간다.
    Idle,
    /// 정지 요청(`wait`). 돌아가지 않는다.
    Wait,
}

/// 예약 종류(R14 표).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ReservationKind {
    /// prompt 실행 future 전체(권한 대기 포함).
    Turn,
    /// 교환 전달 prompt의 엔진 대기열 등록까지(뒤는 Turn).
    Deliver,
    /// 대기 task 배정의 `Starting` 예약부터 실행 허용까지.
    TaskStart,
    /// coordinator 알림 전달 시도(결과 저장 commit까지).
    Notify,
    /// 받아들인 분리 호출(HTTP·MCP).
    Call,
}

/// 관찰용 사건(시험). 예약·해제 직후, G 아래에서 부른다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateEvent {
    pub kind: ReservationKind,
    pub run: Option<String>,
    /// 이 사건 뒤 그 run의 Turn 예약 수(run이 없으면 0).
    pub run_busy_after: usize,
    /// 이 사건 뒤 전체 예약 수.
    pub total_after: usize,
}

pub type GateObserver = Arc<dyn Fn(&GateEvent) + Send + Sync>;

/// task 기동 토큰 상태(R14 표 4·5·5').
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchState {
    /// 배정 중. 엔진 준비가 끝나지 않았다.
    Pending,
    /// G 아래 전이로 실행이 허용된 run.
    Registered(String),
    /// 전이 전에 취소됐다(실행 0).
    Cancelled,
    /// 전이 없이 끝났다(준비 실패·abort).
    Failed,
}

/// task 취소가 기동 토큰에 대해 얻은 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchCancel {
    /// `Pending`을 `Cancelled`로 바꿨다. 시작 장벽은 열리지 않는다.
    Prevented,
    /// 이미 실행이 허용됐다. 이 run을 registry에서 취소해야 한다.
    Registered(String),
    /// 모르는 토큰이거나 이미 `Cancelled`·`Failed`다.
    Unknown,
}

/// 기동 토큰을 `Registered`로 옮기려 했지만 이미 취소된 경우.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the task launch was cancelled before it was registered")]
pub struct LaunchCancelled;

/// 발급된 기동 토큰과 그 T-start 예약. `register_launch`로 확정하지 않고 drop하면 토큰은 `Failed`(아직 `Pending`일 때),
/// T-start는 해제된다.
#[must_use = "an unregistered launch ticket fails the launch when dropped"]
pub struct LaunchTicket {
    token: u64,
    task_start: Option<Reservation>,
}

impl LaunchTicket {
    pub fn token(&self) -> u64 {
        self.token
    }
}

impl std::fmt::Debug for LaunchTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LaunchTicket")
            .field("token", &self.token)
            .finish()
    }
}

impl Drop for LaunchTicket {
    fn drop(&mut self) {
        if let Some(reservation) = self.task_start.take() {
            let gate = Arc::clone(&reservation.gate);
            let mut inner = gate.lock();
            if inner.launches.get(&self.token) == Some(&LaunchState::Pending) {
                inner.launches.insert(self.token, LaunchState::Failed);
            }
            drop(inner);
            drop(reservation);
        }
    }
}

/// 관문 예약에서 파생하는 활동 작업.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GateActiveWork {
    pub busy_runs: usize,
    pub accepted_calls: usize,
    pub deliveries: usize,
    pub task_starts: usize,
    pub notifications: usize,
}

/// 교환 전달(K)을 시작하지 못한 까닭(R14 표 3·3').
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryRefused {
    /// 이미 전달 prompt가 소비된 교환(교환마다 1회).
    AlreadyConsumed,
    /// 서버가 정지 중이다.
    Stopping,
    /// 교환의 작업대가 이미 닫혔다(닫기보다 늦게 도착한 전달, OCR 4차 M1).
    BenchClosed,
}

/// 정지 뒤의 예약 시도.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("server is stopping")]
pub struct GateClosed;

pub const MESSAGE_STOPPING: &str = "server is stopping";

/// 호출 입구 판정([`WorkGate::admit`])의 거절.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmitRefused {
    /// 정지 중 — 어떤 호출도 받지 않는다.
    Stopping,
    /// 비우는 중 새 작업(N).
    Draining,
}

#[derive(Default)]
struct Inner {
    state: Option<GateState>,
    /// 서빙에서 비우기로 넘어간 시각. 대기 task 배정(K)은 이보다 먼저 만든 task만 이어 가기로 받는다. 서빙으로 돌아가면 지운다.
    drain_started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 예약이 풀릴 때마다 는다. 정지 판정은 G 밖에서 파생 값을 읽으므로, 그 사이 끝난 작업(예약 해제)이 파생 값을 바꿨을 수
    /// 있다 — 판정은 읽기 전 세대와 같을 때만 전이한다(`try_stop_at`).
    generation: u64,
    next_id: u64,
    reservations: HashMap<u64, (ReservationKind, Option<String>)>,
    busy: BTreeMap<String, usize>,
    /// 교환 전달 prompt 소비(교환마다 1회, K). 키는 (작업대, 요청 id)다 — 요청 id는 호출자가 정해 작업대마다 겹칠 수 있고,
    /// 교환 저장소도 같은 키로 구별한다(Codex r5).
    consumed_exchanges: HashSet<ExchangeKey>,
    /// 교환 기록을 거둔 닫힌 작업대(OCR 4차 M1): 닫기보다 늦게 도착한 전달이 기록을 되살리지 않게 한다. 작업대 id는 재사용되지
    /// 않는다.
    closed_exchange_benches: HashSet<String>,
    /// 소비했지만 엔진 대기열 등록에 실패한 교환(대상 run이 없음). `server.status`의 `failedExchangeDeliveries`.
    failed_deliveries: HashSet<ExchangeKey>,
    /// N-notify 예약 → 그 전달 시도 id(R14 표 6'·6'').
    notify_attempts: HashMap<u64, String>,
    /// task 기동 토큰 표.
    launches: HashMap<u64, LaunchState>,
    next_launch: u64,
    observer: Option<GateObserver>,
}

impl Inner {
    fn state(&self) -> GateState {
        self.state.unwrap_or(GateState::Serving)
    }

    /// 예약 한 건을 넣는다(G 아래).
    fn insert(&mut self, kind: ReservationKind, run: Option<String>) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.reservations.insert(id, (kind, run.clone()));
        let run_busy_after = if kind == ReservationKind::Turn {
            let count = self.busy.entry(run_id_key(run.as_deref())).or_default();
            *count += 1;
            *count
        } else {
            run.as_ref()
                .map_or(0, |run| self.busy.get(run).copied().unwrap_or(0))
        };
        notify(self, kind, run, run_busy_after);
        id
    }

    /// 예약 한 건을 뺀다(G 아래). 이미 빠졌으면 아무것도 하지 않는다(인계된 예약의 drop).
    fn remove(&mut self, id: u64) {
        let Some((kind, run)) = self.reservations.remove(&id) else {
            return;
        };
        self.generation += 1;
        self.notify_attempts.remove(&id);
        let run_busy_after = if kind == ReservationKind::Turn {
            let key = run_id_key(run.as_deref());
            let remaining = self.busy.get(&key).copied().unwrap_or(1).saturating_sub(1);
            if remaining == 0 {
                self.busy.remove(&key);
            } else {
                self.busy.insert(key, remaining);
            }
            remaining
        } else {
            run.as_ref()
                .map_or(0, |run| self.busy.get(run).copied().unwrap_or(0))
        };
        notify(self, kind, run, run_busy_after);
    }
}

#[derive(Default)]
pub struct WorkGate {
    inner: Mutex<Inner>,
}

/// 활동 예약. drop하면 해제된다.
#[must_use = "a reservation is released when dropped"]
pub struct Reservation {
    gate: Arc<WorkGate>,
    id: u64,
}

impl std::fmt::Debug for Reservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reservation").field("id", &self.id).finish()
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        self.gate.release(self.id);
    }
}

impl WorkGate {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set_observer(&self, observer: GateObserver) {
        self.lock().observer = Some(observer);
    }

    pub fn state(&self) -> GateState {
        self.lock().state()
    }

    pub fn is_stopping(&self) -> bool {
        self.state() == GateState::Stopping
    }

    /// 호출 입구(OCR 구현 리뷰): 정지·비우기 판정과 C-call 예약을 **G 한 번 아래에서** 한다. `new_work`는 비우기 분류
    /// N, `reserve`는 이 호출이 C-call 예약을 쥐는지다. 받은 호출은 판정 순간부터 예약을 쥐므로, 뒤이은 비우기의 정지
    /// 판정이 그 호출을 놓치지 않고, 서빙 중 판정을 통과한 새 작업이 비우기 안에서 예약 없이 도는 틈이 없다.
    pub fn admit(
        self: &Arc<Self>,
        new_work: bool,
        reserve: bool,
    ) -> Result<Option<Reservation>, AdmitRefused> {
        let mut inner = self.lock();
        match inner.state() {
            GateState::Stopping => return Err(AdmitRefused::Stopping),
            GateState::Draining(_) if new_work => return Err(AdmitRefused::Draining),
            _ => {}
        }
        if !reserve {
            return Ok(None);
        }
        let id = inner.insert(ReservationKind::Call, None);
        Ok(Some(Reservation {
            gate: Arc::clone(self),
            id,
        }))
    }

    /// 예약을 만든다. `stopping`이면 실패한다.
    pub fn reserve(
        self: &Arc<Self>,
        kind: ReservationKind,
        run: Option<&str>,
    ) -> Result<Reservation, GateClosed> {
        let mut inner = self.lock();
        if inner.state() == GateState::Stopping {
            return Err(GateClosed);
        }
        let id = inner.insert(kind, run.map(str::to_owned));
        Ok(Reservation {
            gate: Arc::clone(self),
            id,
        })
    }

    fn release(&self, id: u64) {
        self.lock().remove(id);
    }

    /// 대기 task 배정(R14 표 4): 기동 토큰 `Pending`과 T-start 예약을 G 아래에서 함께 만든다. `stopping`이면 실패한다.
    pub fn issue_launch(self: &Arc<Self>) -> Result<LaunchTicket, GateClosed> {
        let mut inner = self.lock();
        if inner.state() == GateState::Stopping {
            return Err(GateClosed);
        }
        inner.next_launch += 1;
        let token = inner.next_launch;
        inner.launches.insert(token, LaunchState::Pending);
        let id = inner.insert(ReservationKind::TaskStart, None);
        Ok(LaunchTicket {
            token,
            task_start: Some(Reservation {
                gate: Arc::clone(self),
                id,
            }),
        })
    }

    /// 엔진 준비 뒤의 선형화 지점(R14 표 4 성공·5'): 토큰이 `Pending`이면 `Registered{run}`로 바꾸고 T-start를 run의
    /// A-turn으로 **같은 G 아래에서** 인계한다. 이미 `Cancelled`면 전이하지 않는다(호출자는 준비한 run을 취소한다).
    pub fn register_launch(
        self: &Arc<Self>,
        mut ticket: LaunchTicket,
        run: &str,
    ) -> Result<Reservation, LaunchCancelled> {
        let task_start = ticket.task_start.take().expect("an unconsumed ticket");
        let mut inner = self.lock();
        if inner.launches.get(&ticket.token) != Some(&LaunchState::Pending) {
            drop(inner);
            drop(task_start);
            return Err(LaunchCancelled);
        }
        inner
            .launches
            .insert(ticket.token, LaunchState::Registered(run.to_owned()));
        let turn = inner.insert(ReservationKind::Turn, Some(run.to_owned()));
        inner.remove(task_start.id);
        drop(inner);
        drop(task_start);
        Ok(Reservation {
            gate: Arc::clone(self),
            id: turn,
        })
    }

    /// 알림 전달 시도(R14 표 6'): 시도 id를 가진 N-notify 예약을 만든다. `stopping`이면 실패한다. 예약은 그 시도의
    /// 결과 저장 commit까지 쥔다(A-turn과 별개).
    pub fn reserve_notify(self: &Arc<Self>, attempt_id: &str) -> Result<Reservation, GateClosed> {
        let mut inner = self.lock();
        if inner.state() == GateState::Stopping {
            return Err(GateClosed);
        }
        let id = inner.insert(ReservationKind::Notify, None);
        inner.notify_attempts.insert(id, attempt_id.to_owned());
        Ok(Reservation {
            gate: Arc::clone(self),
            id,
        })
    }

    /// 이 전달 시도의 N-notify 예약이 살아 있는가(회수 판정, R14 표 6'').
    pub fn notify_attempt_live(&self, attempt_id: &str) -> bool {
        self.lock()
            .notify_attempts
            .values()
            .any(|held| held == attempt_id)
    }

    /// task 취소(R14 표 5).
    pub fn cancel_launch(&self, token: u64) -> LaunchCancel {
        let mut inner = self.lock();
        match inner.launches.get(&token).cloned() {
            Some(LaunchState::Pending) => {
                inner.launches.insert(token, LaunchState::Cancelled);
                LaunchCancel::Prevented
            }
            Some(LaunchState::Registered(run)) => LaunchCancel::Registered(run),
            _ => LaunchCancel::Unknown,
        }
    }

    pub fn launch_state(&self, token: u64) -> Option<LaunchState> {
        self.lock().launches.get(&token).cloned()
    }

    /// run의 Turn 예약 수(0이면 쉬는 세션이거나 run이 없음).
    pub fn busy_run_count(&self, run: &str) -> usize {
        self.lock().busy.get(run).copied().unwrap_or(0)
    }

    /// 바쁜 run id 목록.
    pub fn busy_runs(&self) -> Vec<String> {
        self.lock().busy.keys().cloned().collect()
    }

    /// 전체 예약 수(종류별).
    pub fn reservation_counts(&self) -> BTreeMap<ReservationKind, usize> {
        let mut counts = BTreeMap::new();
        for (kind, _) in self.lock().reservations.values() {
            *counts.entry(*kind).or_insert(0) += 1;
        }
        counts
    }

    pub fn reservation_total(&self) -> usize {
        self.lock().reservations.len()
    }

    /// 예약에서 파생하는 활동 작업. 바쁜 run은 Turn 예약이 있는 run 수(예약 수가 아님).
    pub fn active_work(&self) -> GateActiveWork {
        let inner = self.lock();
        let count = |kind| {
            inner
                .reservations
                .values()
                .filter(|(k, _)| *k == kind)
                .count()
        };
        GateActiveWork {
            busy_runs: inner.busy.len(),
            accepted_calls: count(ReservationKind::Call),
            deliveries: count(ReservationKind::Deliver),
            task_starts: count(ReservationKind::TaskStart),
            notifications: count(ReservationKind::Notify),
        }
    }

    /// 비우기에 들어간다. `stopping`이면 그대로 둔다. `Wait`는 `Idle`을 덮는다.
    pub fn begin_drain(&self, mode: DrainMode) {
        let mut inner = self.lock();
        match inner.state() {
            GateState::Stopping => {}
            GateState::Draining(DrainMode::Wait) => {}
            GateState::Draining(DrainMode::Idle) => {
                if mode != DrainMode::Idle {
                    inner.state = Some(GateState::Draining(mode));
                    inner.generation += 1;
                }
            }
            GateState::Serving => {
                inner.drain_started_at = Some(chrono::Utc::now());
                inner.state = Some(GateState::Draining(mode));
                // 상태 전이도 세대를 바꾼다: 전이 전에 시작한 정지 판정은 쓸 수 없다.
                inner.generation += 1;
            }
        }
    }

    /// 비우기 시작 시각(서빙 중이면 없음).
    pub fn drain_started_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.lock().drain_started_at
    }

    /// 유휴 비우기를 취소하고 서빙으로 돌아간다(임대 획득). `Wait`·`stopping`은 돌아가지 않는다.
    pub fn resume_serving(&self) -> bool {
        let mut inner = self.lock();
        match inner.state() {
            GateState::Draining(DrainMode::Idle) => {
                inner.state = Some(GateState::Serving);
                inner.drain_started_at = None;
                // Codex 구현 리뷰(high): 비우기 동안 시작한 정지 판정(세대를 읽고 파생 값을 기다리는 중)이 서빙을
                // `stopping`으로 바꾸지 못하게 세대를 바꾼다.
                inner.generation += 1;
                true
            }
            GateState::Serving => true,
            _ => false,
        }
    }

    /// 임대 획득(G 아래, Codex r6 high): `stopping`이 아니면 어느 상태에서든 세대를 바꾼다 — 임대가 없다고 보고 파생한
    /// 정지 판정(데스크톱 임대가 있어야 미소비 교환이 활동이다)이 임대를 넣은 뒤 멈추지 못하게. 유휴 비우기는 서빙으로
    /// 되돌린다([`WorkGate::resume_serving`]). `wait` 비우기는 그대로 둔다. `stopping`이면 false(임대를 거절한다).
    pub fn note_lease_acquired(&self) -> bool {
        let mut inner = self.lock();
        match inner.state() {
            GateState::Stopping => false,
            state => {
                if state == GateState::Draining(DrainMode::Idle) {
                    inner.state = Some(GateState::Serving);
                    inner.drain_started_at = None;
                }
                inner.generation += 1;
                true
            }
        }
    }

    /// 정지 판정(G 아래): 예약 0이고 `derived_active()`(파생 활동 수)가 0이면 `stopping`으로 전이하고 true.
    /// `derived_active`는 G를 쥔 채 불린다 — 그 안에서 WorkGate를 다시 부르면 안 된다.
    pub fn try_stop(&self, derived_active: impl FnOnce() -> u64) -> bool {
        let mut inner = self.lock();
        if inner.state() == GateState::Stopping {
            return true;
        }
        if !inner.reservations.is_empty() || derived_active() != 0 {
            return false;
        }
        inner.state = Some(GateState::Stopping);
        true
    }

    /// 활동 세대(예약 해제와 상태 전이 — 비우기 시작·유휴 비우기 취소 — 마다 증가). [`WorkGate::try_stop_at`]과 짝.
    pub fn activity_generation(&self) -> u64 {
        self.lock().generation
    }

    /// 파생 값을 읽기 **전에** 얻은 세대로 하는 정지 판정: 그 뒤 예약이 하나라도 풀렸으면(파생 값이 낡았을 수 있음) 전이하지
    /// 않는다. 나머지는 [`WorkGate::try_stop`]과 같다.
    pub fn try_stop_at(&self, generation: u64, derived_active: impl FnOnce() -> u64) -> bool {
        let mut inner = self.lock();
        if inner.state() == GateState::Stopping {
            return true;
        }
        if inner.generation != generation || !inner.reservations.is_empty() || derived_active() != 0
        {
            return false;
        }
        inner.state = Some(GateState::Stopping);
        true
    }

    /// 강제 정지: 예약과 상관없이 `stopping`으로 전이한다(작업대 닫기가 예약을 drop으로 푼다).
    pub fn force_stop(&self) {
        self.lock().state = Some(GateState::Stopping);
    }

    /// 교환 전달 prompt 소비(작업대의 교환마다 1회). 처음이면 true.
    pub fn consume_exchange(&self, bench_id: &str, request_id: &str) -> bool {
        self.lock()
            .consumed_exchanges
            .insert(exchange_key(bench_id, request_id))
    }

    /// 교환 전달 시작(R14 표 3): **같은 G 아래에서** 소비 표시와 X-deliver 예약을 함께 만든다. 이미 소비됐거나 정지 중이면
    /// 둘 다 만들지 않는다. 호출자는 엔진 대기열 등록(그 안에서 A-turn이 동기 예약된다) 뒤에 이 예약을 drop한다 — A-turn이
    /// 먼저 잡히므로 활동이 0이 되는 틈이 없다.
    pub fn begin_exchange_delivery(
        self: &Arc<Self>,
        bench_id: &str,
        request_id: &str,
        run: &str,
    ) -> Result<Reservation, DeliveryRefused> {
        let mut inner = self.lock();
        if inner.state() == GateState::Stopping {
            return Err(DeliveryRefused::Stopping);
        }
        if inner.closed_exchange_benches.contains(bench_id) {
            return Err(DeliveryRefused::BenchClosed);
        }
        if !inner
            .consumed_exchanges
            .insert(exchange_key(bench_id, request_id))
        {
            return Err(DeliveryRefused::AlreadyConsumed);
        }
        let id = inner.insert(ReservationKind::Deliver, Some(run.to_owned()));
        Ok(Reservation {
            gate: Arc::clone(self),
            id,
        })
    }

    /// 소비한 교환의 대기열 등록이 실패했다(대상 run이 없음). 소비 표시는 남긴다(R14 표 3 오류).
    pub fn record_failed_delivery(&self, bench_id: &str, request_id: &str) {
        let mut inner = self.lock();
        // 닫힌 작업대의 늦은 실패는 적지 않는다(닫기 뒤 보고에 남지 않게).
        if inner.closed_exchange_benches.contains(bench_id) {
            return;
        }
        inner
            .failed_deliveries
            .insert(exchange_key(bench_id, request_id));
    }

    /// 전달이 실패한 교환(`server.status.failedExchangeDeliveries`). 요청 id는 작업대마다 겹칠 수 있으므로
    /// `<benchId>/<requestId>`로 적는다.
    pub fn failed_deliveries(&self) -> Vec<String> {
        let mut ids: Vec<_> = self
            .lock()
            .failed_deliveries
            .iter()
            .map(|(bench, request)| format!("{bench}/{request}"))
            .collect();
        ids.sort();
        ids
    }

    pub fn exchange_consumed(&self, bench_id: &str, request_id: &str) -> bool {
        self.lock()
            .consumed_exchanges
            .contains(&exchange_key(bench_id, request_id))
    }

    /// 작업대 닫기: 그 작업대의 교환 소비·실패 기록을 지운다. 닫기는 교환 자체도 지우고, 닫힌 작업대로의 늦은 전달은
    /// 작업대 확인(`resolve`)에서 먼저 거절되며, 작업대 id는 다시 쓰이지 않는다 — 기록이 남을 이유가 없다(서버 수명 동안
    /// 쌓이지 않게).
    pub fn forget_bench_exchanges(&self, bench_id: &str) {
        let mut inner = self.lock();
        inner.closed_exchange_benches.insert(bench_id.to_owned());
        inner
            .consumed_exchanges
            .retain(|(bench, _)| bench != bench_id);
        inner
            .failed_deliveries
            .retain(|(bench, _)| bench != bench_id);
    }
}

/// 교환 기록의 키: (작업대 id, 교환 요청 id).
type ExchangeKey = (String, String);

fn exchange_key(bench_id: &str, request_id: &str) -> ExchangeKey {
    (bench_id.to_owned(), request_id.to_owned())
}

fn run_id_key(run: Option<&str>) -> String {
    run.unwrap_or_default().to_owned()
}

fn notify(inner: &Inner, kind: ReservationKind, run: Option<String>, run_busy_after: usize) {
    if let Some(observer) = &inner.observer {
        observer(&GateEvent {
            kind,
            run,
            run_busy_after,
            total_after: inner.reservations.len(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dropped_reservation_is_released_and_counted_per_run() {
        let gate = WorkGate::new();
        let a = gate.reserve(ReservationKind::Turn, Some("r1")).unwrap();
        let b = gate.reserve(ReservationKind::Turn, Some("r1")).unwrap();
        assert_eq!(gate.busy_run_count("r1"), 2);
        drop(a);
        assert_eq!(gate.busy_run_count("r1"), 1);
        drop(b);
        assert_eq!(gate.busy_run_count("r1"), 0);
        assert_eq!(gate.reservation_total(), 0);
    }

    #[test]
    fn stopping_refuses_new_reservations_and_waits_for_live_ones() {
        let gate = WorkGate::new();
        let turn = gate.reserve(ReservationKind::Turn, Some("r1")).unwrap();
        gate.begin_drain(DrainMode::Wait);
        assert!(!gate.try_stop(|| 0), "a live reservation blocks stopping");
        drop(turn);
        assert!(!gate.try_stop(|| 1), "derived activity blocks stopping");
        assert!(gate.try_stop(|| 0));
        assert!(gate.reserve(ReservationKind::Turn, Some("r1")).is_err());
    }

    #[test]
    fn idle_drain_resumes_on_lease_but_wait_drain_does_not() {
        let gate = WorkGate::new();
        gate.begin_drain(DrainMode::Idle);
        assert!(gate.resume_serving());
        assert_eq!(gate.state(), GateState::Serving);
        gate.begin_drain(DrainMode::Wait);
        gate.begin_drain(DrainMode::Idle);
        assert_eq!(gate.state(), GateState::Draining(DrainMode::Wait));
        assert!(!gate.resume_serving());
    }
}
