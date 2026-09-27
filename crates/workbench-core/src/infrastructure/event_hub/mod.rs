//! 이벤트 허브(039): 스트림별 journal, 구독 조정자, 세대, 보관 한도, 제거 표식.
//!
//! 계약: `specs/039-workbench-events/contracts/workbench-events.md`. 결정: research R2·R5·R6·R10.
//!
//! # lock 순서 (교착 방지)
//!
//! `streams` → 개별 스트림 → `retention` 순으로만 잡는다. 스트림 lock을 쥔 채 `streams`를 잡지 않고, 두 스트림
//! lock을 동시에 쥐지 않는다. run 정리(eviction)는 발행한 스트림의 lock을 푼 뒤 `streams` → `retention`을 잡아
//! 대상을 map에서 빼고, 두 lock을 푼 다음 대상 스트림 lock을 하나씩 잡아 구독자에게 gap을 보낸다.
//!
//! # 발행과 구독의 원자성
//!
//! 발행(순번 부여·journal·구독자 전달·데스크톱 `deliver`)과 구독(수신자 등록·기준점·replay 복사)은 **같은 스트림
//! lock** 안에서 일어난다. 그래서 기준점과 등록 사이에 발행이 끼어들 수 없고, 여러 발행자의 데스크톱 전달 순서는
//! 순번 순서와 같다.

mod stream;
mod subscription;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use workbench_protocol::{
    events::{parse_stream_id, EventClass, StreamKind},
    AuthenticatedPrincipal, EventEnvelope, EventItem, EventStream, FaultCode, GapNotice, GapReason,
    RequestId, Subscription, WorkbenchFault,
};

use stream::{
    decide_existing, decide_missing, CursorDecision, JournalEntry, StreamState, Subscriber,
};
use subscription::HubSubscription;

pub const MESSAGE_CURSOR_AHEAD: &str = "cursor is ahead of the stream.";
pub const MESSAGE_KIND_NOT_AVAILABLE: &str = "stream kind is not available yet.";
pub const MESSAGE_CURSORS_REQUIRED: &str = "at least one cursor is required.";

/// hub 한도(research R10). 테스트는 낮춘 값으로 overflow·정리를 재현한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventHubLimits {
    pub run_journal_capacity: usize,
    pub max_retained_runs: usize,
    pub max_tombstones: usize,
    pub subscriber_queue: usize,
    pub max_subscriptions: usize,
    pub max_cursors: usize,
}

impl Default for EventHubLimits {
    fn default() -> Self {
        Self {
            run_journal_capacity: 512,
            max_retained_runs: 256,
            max_tombstones: 4_096,
            subscriber_queue: 1_024,
            max_subscriptions: 256,
            max_cursors: 64,
        }
    }
}

/// 오늘 AW `RuntimeEventSnapshot`과 같은 JSON(호환 replay command용).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReplay {
    pub run_id: String,
    pub events: Vec<RunReplayEvent>,
    pub last_sequence: u64,
    pub terminal: bool,
    pub gap_detected: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReplayEvent {
    pub run_id: String,
    pub sequence: u64,
    pub event: serde_json::Value,
    pub terminal: bool,
}

struct StreamEntry {
    kind: StreamKind,
    state: Arc<Mutex<StreamState>>,
}

#[derive(Default)]
struct Retention {
    /// terminal이 된 순서의 run 스트림 id.
    terminal_order: VecDeque<String>,
    evicted_order: VecDeque<String>,
    evicted: HashSet<String>,
}

enum Lookup {
    Found(Arc<Mutex<StreamState>>),
    Missing,
    Evicted,
}

/// 알림용 스트림(worktree)의 감시 하나. 참조 수가 0이 되면 handle을 버려 감시를 멈춘다(US3).
struct WatchEntry {
    refcount: usize,
    _handle: Box<dyn Send>,
}

/// worktree 감시를 시작하는 함수. hub는 fs를 모른다 — runtime이 주입한다(US3).
pub type StartWatch = Arc<
    dyn Fn(&Path, Box<dyn Fn(serde_json::Value) + Send + Sync>) -> Result<Box<dyn Send>, String>
        + Send
        + Sync,
>;

pub struct EventHub {
    epoch: String,
    limits: EventHubLimits,
    streams: Mutex<HashMap<String, StreamEntry>>,
    retention: Mutex<Retention>,
    subscriptions: AtomicUsize,
    next_subscriber: AtomicU64,
    watchers: Mutex<HashMap<PathBuf, WatchEntry>>,
    start_watch: Option<StartWatch>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn fault(code: FaultCode, message: impl Into<String>) -> WorkbenchFault {
    WorkbenchFault::new(code, RequestId::random(), message)
}

fn envelope(
    stream_id: &str,
    epoch: &str,
    sequence: u64,
    schema: &str,
    body: serde_json::Value,
) -> EventEnvelope {
    EventEnvelope {
        event_id: uuid::Uuid::new_v4().to_string(),
        stream_id: stream_id.to_owned(),
        epoch: epoch.to_owned(),
        sequence,
        schema: schema.to_owned(),
        occurred_at: Utc::now().to_rfc3339(),
        correlation_id: None,
        body,
    }
}

impl EventHub {
    pub fn new(epoch: impl Into<String>, limits: EventHubLimits) -> Arc<Self> {
        Self::with_watcher(epoch, limits, None)
    }

    pub fn with_watcher(
        epoch: impl Into<String>,
        limits: EventHubLimits,
        start_watch: Option<StartWatch>,
    ) -> Arc<Self> {
        Arc::new(Self {
            epoch: epoch.into(),
            limits,
            streams: Mutex::new(HashMap::new()),
            retention: Mutex::new(Retention::default()),
            subscriptions: AtomicUsize::new(0),
            next_subscriber: AtomicU64::new(1),
            watchers: Mutex::new(HashMap::new()),
            start_watch,
        })
    }

    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    pub fn limits(&self) -> EventHubLimits {
        self.limits
    }

    /// 현재 동시 구독 수(진단·테스트).
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.load(Ordering::SeqCst)
    }

    /// 돌고 있는 worktree 감시 수(진단·테스트).
    pub fn watcher_count(&self) -> usize {
        lock(&self.watchers).len()
    }

    /// `streams` → `retention` 순으로 잡아 제거 표식을 확인하고, 없으면(`create`면) 스트림을 만든다.
    fn lookup(&self, stream_id: &str, kind: StreamKind, create: bool) -> Lookup {
        let mut streams = lock(&self.streams);
        if let Some(entry) = streams.get(stream_id) {
            return Lookup::Found(Arc::clone(&entry.state));
        }
        if kind == StreamKind::Run && lock(&self.retention).evicted.contains(stream_id) {
            return Lookup::Evicted;
        }
        if !create {
            return Lookup::Missing;
        }
        let state = Arc::new(Mutex::new(StreamState::new(stream_id.to_owned())));
        streams.insert(
            stream_id.to_owned(),
            StreamEntry {
                kind,
                state: Arc::clone(&state),
            },
        );
        Lookup::Found(state)
    }

    /// 상태 복원용 스트림에 발행한다. `deliver`는 순번 부여와 같은 스트림 lock 안에서 불린다 — 막히지 않아야 하고
    /// hub를 다시 호출하면 안 된다. 제거된 run이면 버리고 `None`.
    pub fn publish_state(
        &self,
        kind: StreamKind,
        key: &str,
        schema: &str,
        body: serde_json::Value,
        terminal: bool,
        deliver: &mut dyn FnMut(&EventEnvelope),
    ) -> Option<EventEnvelope> {
        let stream_id = kind.stream_id(key);
        let state = match self.lookup(&stream_id, kind, true) {
            Lookup::Found(state) => state,
            Lookup::Evicted | Lookup::Missing => {
                eprintln!("[workbench] dropped event for evicted stream {stream_id}");
                return None;
            }
        };
        let (published, newly_terminal) = {
            let mut stream = lock(&state);
            if stream.removed {
                eprintln!("[workbench] dropped event for evicted stream {stream_id}");
                return None;
            }
            stream.sequence += 1;
            let published = envelope(&stream_id, &self.epoch, stream.sequence, schema, body);
            stream.push_journal(
                JournalEntry {
                    envelope: published.clone(),
                    terminal,
                },
                self.limits.run_journal_capacity,
            );
            stream.fan_out(&EventItem::Event {
                event: published.clone(),
            });
            deliver(&published);
            let newly_terminal = terminal && !stream.terminal;
            stream.terminal |= terminal;
            (published, newly_terminal)
        };
        if newly_terminal {
            self.retire(&stream_id);
        }
        Some(published)
    }

    /// 알림용 스트림에 발행한다(보관 없음). 스트림이 없으면(구독자 없음) 아무 일도 하지 않는다.
    pub fn publish_notification(
        &self,
        kind: StreamKind,
        key: &str,
        schema: &str,
        body: serde_json::Value,
    ) {
        let stream_id = kind.stream_id(key);
        let Lookup::Found(state) = self.lookup(&stream_id, kind, false) else {
            return;
        };
        let mut stream = lock(&state);
        if stream.removed {
            return;
        }
        stream.sequence += 1;
        let published = envelope(&stream_id, &self.epoch, stream.sequence, schema, body);
        stream.fan_out(&EventItem::Event { event: published });
    }

    /// terminal이 된 run을 정리 대상에 올리고, 보관 run 수가 상한을 넘으면 가장 먼저 끝난 run부터 제거한다.
    fn retire(&self, stream_id: &str) {
        let victims: Vec<Arc<Mutex<StreamState>>> = {
            let mut streams = lock(&self.streams);
            let mut retention = lock(&self.retention);
            retention.terminal_order.push_back(stream_id.to_owned());
            let mut victims = Vec::new();
            let mut run_count = streams
                .values()
                .filter(|entry| entry.kind == StreamKind::Run)
                .count();
            while run_count > self.limits.max_retained_runs {
                let Some(victim) = retention.terminal_order.pop_front() else {
                    break;
                };
                if let Some(entry) = streams.remove(&victim) {
                    victims.push(entry.state);
                    run_count -= 1;
                }
                retention.evicted.insert(victim.clone());
                retention.evicted_order.push_back(victim);
                while retention.evicted_order.len() > self.limits.max_tombstones {
                    if let Some(old) = retention.evicted_order.pop_front() {
                        retention.evicted.remove(&old);
                    }
                }
            }
            victims
        };
        for victim in victims {
            let mut stream = lock(&victim);
            stream.removed = true;
            let gap = EventItem::Gap {
                gap: GapNotice {
                    stream_id: stream.stream_id.clone(),
                    epoch: self.epoch.clone(),
                    reason: GapReason::Evicted,
                    first_sequence: None,
                    last_sequence: None,
                },
            };
            stream.fan_out(&gap);
            stream.subscribers.clear();
        }
    }

    /// 호환 replay command용 run replay(research R5 표).
    pub fn replay_run(&self, run_id: &str, after: u64) -> RunReplay {
        let stream_id = StreamKind::Run.stream_id(run_id);
        match self.lookup(&stream_id, StreamKind::Run, false) {
            Lookup::Evicted => RunReplay {
                run_id: run_id.to_owned(),
                events: Vec::new(),
                last_sequence: 0,
                terminal: true,
                gap_detected: true,
            },
            Lookup::Missing => RunReplay {
                run_id: run_id.to_owned(),
                events: Vec::new(),
                last_sequence: 0,
                terminal: false,
                gap_detected: after > 0,
            },
            Lookup::Found(state) => {
                let stream = lock(&state);
                if stream.removed {
                    return RunReplay {
                        run_id: run_id.to_owned(),
                        events: Vec::new(),
                        last_sequence: 0,
                        terminal: true,
                        gap_detected: true,
                    };
                }
                RunReplay {
                    run_id: run_id.to_owned(),
                    events: stream
                        .journal
                        .iter()
                        .filter(|entry| entry.envelope.sequence > after)
                        .map(|entry| RunReplayEvent {
                            run_id: run_id.to_owned(),
                            sequence: entry.envelope.sequence,
                            event: entry.envelope.body.clone(),
                            terminal: entry.terminal,
                        })
                        .collect(),
                    last_sequence: stream.sequence,
                    terminal: stream.terminal,
                    gap_detected: after.saturating_add(1) < stream.first_retained(),
                }
            }
        }
    }

    /// 구독(contracts §1·§2). 오류는 즉시, 이어 붙일 수 없는 cursor는 스트림 안의 gap으로.
    pub fn subscribe(
        self: &Arc<Self>,
        principal: &AuthenticatedPrincipal,
        request: Subscription,
    ) -> Result<EventStream, WorkbenchFault> {
        let cursors = request.cursors;
        if cursors.is_empty() {
            return Err(fault(FaultCode::InvalidArgument, MESSAGE_CURSORS_REQUIRED));
        }
        if cursors.len() > self.limits.max_cursors {
            return Err(fault(
                FaultCode::InvalidArgument,
                format!(
                    "at most {} cursors are allowed per subscription.",
                    self.limits.max_cursors
                ),
            ));
        }
        // 권한·형식은 등록 전에 전부 검사한다(일부만 등록된 채 실패하지 않게).
        let mut parsed = Vec::with_capacity(cursors.len());
        for cursor in &cursors {
            let Some((kind, key)) = parse_stream_id(&cursor.stream_id) else {
                return Err(fault(
                    FaultCode::InvalidArgument,
                    format!("invalid stream id: {}", cursor.stream_id),
                ));
            };
            if !kind.is_subscribable() {
                return Err(fault(
                    FaultCode::InvalidArgument,
                    MESSAGE_KIND_NOT_AVAILABLE,
                ));
            }
            if !principal.has_scope(kind.required_scope()) {
                return Err(WorkbenchFault::forbidden(
                    RequestId::random(),
                    &cursor.stream_id,
                ));
            }
            parsed.push((kind, key.to_owned(), cursor.clone()));
        }
        let previous = self.subscriptions.fetch_add(1, Ordering::SeqCst);
        if previous >= self.limits.max_subscriptions {
            self.subscriptions.fetch_sub(1, Ordering::SeqCst);
            return Err(fault(
                FaultCode::RateLimited,
                "too many concurrent event subscriptions.",
            ));
        }

        let (tx, rx) = mpsc::channel(self.limits.subscriber_queue.max(1));
        let lagged = Arc::new(AtomicBool::new(false));
        let lagged_stream = Arc::new(Mutex::new(None));
        // 이 뒤로는 실패해도 drop이 등록·참조 수·구독 수를 되돌린다.
        let mut subscription = HubSubscription {
            id: self.next_subscriber.fetch_add(1, Ordering::SeqCst),
            hub: Arc::clone(self),
            epoch: self.epoch.clone(),
            pending: VecDeque::new(),
            rx,
            high_water: HashMap::new(),
            registered: Vec::new(),
            watched: Vec::new(),
            lagged: Arc::clone(&lagged),
            lagged_stream: Arc::clone(&lagged_stream),
            finished: false,
        };
        let subscriber = |id: u64| Subscriber {
            id,
            tx: tx.clone(),
            lagged: Arc::clone(&lagged),
            lagged_stream: Arc::clone(&lagged_stream),
        };

        for (kind, key, cursor) in parsed {
            match kind.class() {
                EventClass::State => {
                    let stream_id = kind.stream_id(&key);
                    let gap = |reason, first, last| EventItem::Gap {
                        gap: GapNotice {
                            stream_id: stream_id.clone(),
                            epoch: self.epoch.clone(),
                            reason,
                            first_sequence: first,
                            last_sequence: last,
                        },
                    };
                    let state = match self.lookup(&stream_id, kind, cursor.after_sequence == 0) {
                        Lookup::Evicted => {
                            subscription
                                .pending
                                .push_back(gap(GapReason::Evicted, None, None));
                            continue;
                        }
                        Lookup::Missing => {
                            let decision =
                                decide_missing(cursor.after_sequence, &cursor.epoch, &self.epoch);
                            if let CursorDecision::Gap {
                                reason,
                                first,
                                last,
                            } = decision
                            {
                                subscription.pending.push_back(gap(reason, first, last));
                            }
                            continue;
                        }
                        Lookup::Found(state) => state,
                    };
                    let mut stream = lock(&state);
                    if stream.removed {
                        subscription
                            .pending
                            .push_back(gap(GapReason::Evicted, None, None));
                        continue;
                    }
                    let decision = decide_existing(
                        cursor.after_sequence,
                        &cursor.epoch,
                        &self.epoch,
                        stream.first_retained(),
                        stream.sequence,
                    );
                    match decision {
                        CursorDecision::Ahead => {
                            return Err(fault(FaultCode::InvalidArgument, MESSAGE_CURSOR_AHEAD));
                        }
                        CursorDecision::Gap {
                            reason,
                            first,
                            last,
                        } => {
                            subscription.pending.push_back(gap(reason, first, last));
                        }
                        CursorDecision::Replay { after } => {
                            // 수신자 먼저 → 기준점 → replay 복사. 모두 같은 lock 안.
                            stream.subscribers.push(subscriber(subscription.id));
                            let high_water = stream.sequence;
                            subscription
                                .high_water
                                .insert(stream_id.clone(), high_water);
                            for entry in stream.journal.iter().filter(|entry| {
                                entry.envelope.sequence > after
                                    && entry.envelope.sequence <= high_water
                            }) {
                                subscription.pending.push_back(EventItem::Event {
                                    event: entry.envelope.clone(),
                                });
                            }
                            drop(stream);
                            subscription.registered.push(state);
                        }
                        CursorDecision::Live => {
                            stream.subscribers.push(subscriber(subscription.id));
                            subscription
                                .high_water
                                .insert(stream_id.clone(), stream.sequence);
                            drop(stream);
                            subscription.registered.push(state);
                        }
                    }
                }
                EventClass::Notification => {
                    let canonical = self.acquire_watch(&key)?;
                    subscription.watched.push(canonical.clone());
                    let stream_id = kind.stream_id(&canonical.to_string_lossy());
                    let Lookup::Found(state) = self.lookup(&stream_id, kind, true) else {
                        continue;
                    };
                    let mut stream = lock(&state);
                    stream.subscribers.push(subscriber(subscription.id));
                    subscription
                        .high_water
                        .insert(stream_id.clone(), stream.sequence);
                    drop(stream);
                    subscription.registered.push(state);
                }
            }
        }
        Ok(EventStream::new(subscription))
    }

    fn subscription_closed(&self) {
        self.subscriptions.fetch_sub(1, Ordering::SeqCst);
    }

    /// worktree 경로의 감시 참조 수를 올린다. 첫 참조면 감시를 시작한다. 실제 경로를 돌려준다(US3).
    fn acquire_watch(self: &Arc<Self>, key: &str) -> Result<PathBuf, WorkbenchFault> {
        let canonical = std::fs::canonicalize(key).map_err(|_| {
            fault(
                FaultCode::NotFound,
                format!("Cannot watch missing worktree path: {key}"),
            )
        })?;
        let mut watchers = lock(&self.watchers);
        if let Some(entry) = watchers.get_mut(&canonical) {
            entry.refcount += 1;
            return Ok(canonical);
        }
        let Some(start_watch) = &self.start_watch else {
            return Err(fault(
                FaultCode::Unavailable,
                "worktree watching is not configured.",
            ));
        };
        let hub = Arc::downgrade(self);
        let key = canonical.to_string_lossy().into_owned();
        let notify: Box<dyn Fn(serde_json::Value) + Send + Sync> = Box::new(move |body| {
            if let Some(hub) = hub.upgrade() {
                hub.publish_notification(
                    StreamKind::Worktree,
                    &key,
                    workbench_protocol::events::WORKTREE_CHANGED_V1,
                    body,
                );
            }
        });
        let handle = start_watch(&canonical, notify)
            .map_err(|message| fault(FaultCode::NotFound, message))?;
        watchers.insert(
            canonical.clone(),
            WatchEntry {
                refcount: 1,
                _handle: handle,
            },
        );
        Ok(canonical)
    }

    /// 참조 수를 내리고 0이면 감시를 멈춘다(handle drop). 알림용 스트림도 map에서 뺀다.
    fn release_watch(&self, canonical: &Path) {
        let stopped = {
            let mut watchers = lock(&self.watchers);
            match watchers.get_mut(canonical) {
                Some(entry) if entry.refcount > 1 => {
                    entry.refcount -= 1;
                    None
                }
                Some(_) => watchers.remove(canonical),
                None => None,
            }
        };
        if stopped.is_some() {
            let stream_id = StreamKind::Worktree.stream_id(&canonical.to_string_lossy());
            lock(&self.streams).remove(&stream_id);
        }
        drop(stopped);
    }
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use serde_json::json;
    use workbench_protocol::{events::RUN_EVENT_V1, StreamCursor};

    use super::*;

    fn hub(limits: EventHubLimits) -> Arc<EventHub> {
        EventHub::new("epoch-1", limits)
    }

    fn publish(hub: &EventHub, run: &str, n: usize) {
        for index in 0..n {
            hub.publish_state(
                StreamKind::Run,
                run,
                RUN_EVENT_V1,
                json!({"i": index}),
                false,
                &mut |_| {},
            );
        }
    }

    fn cursor(stream: &str, after: u64) -> Subscription {
        Subscription {
            cursors: vec![StreamCursor {
                stream_id: stream.into(),
                epoch: "epoch-1".into(),
                after_sequence: after,
            }],
        }
    }

    async fn take(stream: &mut EventStream, n: usize) -> Vec<EventItem> {
        let mut items = Vec::new();
        for _ in 0..n {
            let item = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
                .await
                .expect("item in time")
                .expect("stream open");
            items.push(item);
        }
        items
    }

    fn seqs(items: &[EventItem]) -> Vec<u64> {
        items
            .iter()
            .filter_map(|item| match item {
                EventItem::Event { event } => Some(event.sequence),
                EventItem::Gap { .. } => None,
            })
            .collect()
    }

    #[test]
    fn per_run_sequences_and_journal_capacity() {
        let hub = hub(EventHubLimits {
            run_journal_capacity: 4,
            ..Default::default()
        });
        publish(&hub, "a", 1);
        publish(&hub, "b", 1);
        publish(&hub, "a", 5);
        let replay = hub.replay_run("a", 0);
        assert_eq!(replay.last_sequence, 6);
        assert_eq!(
            replay.events.iter().map(|e| e.sequence).collect::<Vec<_>>(),
            vec![3, 4, 5, 6]
        );
        assert!(replay.gap_detected, "1–2 were dropped");
        assert!(!hub.replay_run("a", 2).gap_detected);
        assert_eq!(hub.replay_run("b", 0).last_sequence, 1);
        let unknown = hub.replay_run("zzz", 3);
        assert!(unknown.gap_detected && !unknown.terminal);
    }

    /// 호환 replay의 JSON은 화면의 `RuntimeEventSnapshot` 타입과 같다(키 이름·중첩).
    #[test]
    fn run_replay_json_matches_the_desktop_snapshot_shape() {
        let hub = hub(EventHubLimits::default());
        hub.publish_state(
            StreamKind::Run,
            "r1",
            RUN_EVENT_V1,
            json!({"type": "lifecycle", "status": "completed", "message": ""}),
            true,
            &mut |_| {},
        );
        let value = serde_json::to_value(hub.replay_run("r1", 0)).unwrap();
        assert_eq!(
            value,
            json!({
                "runId": "r1",
                "events": [{"runId": "r1", "sequence": 1, "event": {"type": "lifecycle", "status": "completed", "message": ""}, "terminal": true}],
                "lastSequence": 1,
                "terminal": true,
                "gapDetected": false
            })
        );
    }

    #[tokio::test]
    async fn replay_then_live_without_gap_or_duplicate() {
        let hub = hub(EventHubLimits::default());
        publish(&hub, "r1", 10);
        let mut stream = hub
            .subscribe(&AuthenticatedPrincipal::desktop(), cursor("run:r1", 4))
            .unwrap();
        publish(&hub, "r1", 2);
        let items = take(&mut stream, 8).await;
        assert_eq!(seqs(&items), (5..=12).collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn evicted_runs_are_tombstoned_and_late_publishes_dropped() {
        let hub = hub(EventHubLimits {
            max_retained_runs: 1,
            ..Default::default()
        });
        let mut delivered = 0;
        hub.publish_state(
            StreamKind::Run,
            "old",
            RUN_EVENT_V1,
            json!({}),
            true,
            &mut |_| delivered += 1,
        );
        hub.publish_state(
            StreamKind::Run,
            "new",
            RUN_EVENT_V1,
            json!({}),
            true,
            &mut |_| delivered += 1,
        );
        assert_eq!(delivered, 2);
        let replay = hub.replay_run("old", 0);
        assert!(replay.gap_detected && replay.terminal);
        let mut stream = hub
            .subscribe(&AuthenticatedPrincipal::desktop(), cursor("run:old", 0))
            .unwrap();
        let items = take(&mut stream, 1).await;
        assert!(matches!(
            &items[0],
            EventItem::Gap { gap } if gap.reason == GapReason::Evicted
        ));
        assert!(hub
            .publish_state(
                StreamKind::Run,
                "old",
                RUN_EVENT_V1,
                json!({}),
                false,
                &mut |_| delivered += 1
            )
            .is_none());
        assert_eq!(delivered, 2, "late publish is not delivered");
    }

    #[tokio::test]
    async fn lagged_subscriber_gets_gap_and_publisher_never_blocks() {
        let hub = hub(EventHubLimits {
            subscriber_queue: 2,
            ..Default::default()
        });
        let mut stream = hub
            .subscribe(&AuthenticatedPrincipal::desktop(), cursor("run:r", 0))
            .unwrap();
        publish(&hub, "r", 10);
        let item = take(&mut stream, 1).await;
        assert!(matches!(
            &item[0],
            EventItem::Gap { gap } if gap.reason == GapReason::SubscriberLagged
        ));
        assert!(stream.next().await.is_none());
        drop(stream);
        assert_eq!(hub.subscription_count(), 0);
    }

    #[test]
    fn subscribe_validates_before_registering() {
        let hub = hub(EventHubLimits::default());
        let desktop = AuthenticatedPrincipal::desktop();
        let error = hub
            .subscribe(&desktop, Subscription::default())
            .unwrap_err();
        assert_eq!(error.code, FaultCode::InvalidArgument);
        let error = hub
            .subscribe(&desktop, cursor("orchestration:w1", 0))
            .unwrap_err();
        assert_eq!(error.message, MESSAGE_KIND_NOT_AVAILABLE);
        publish(&hub, "r", 3);
        let error = hub.subscribe(&desktop, cursor("run:r", 9)).unwrap_err();
        assert_eq!(error.message, MESSAGE_CURSOR_AHEAD);
        let no_scope = AuthenticatedPrincipal::new(workbench_protocol::PrincipalKind::Desktop, []);
        let error = hub.subscribe(&no_scope, cursor("run:r", 0)).unwrap_err();
        assert_eq!(error.code, FaultCode::Forbidden);
        assert_eq!(hub.subscription_count(), 0);
    }
}
