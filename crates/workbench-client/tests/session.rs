#[allow(dead_code)]
#[path = "support/harness.rs"]
mod harness;
mod support;
use async_trait::async_trait;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use support::{Action, Peer};
use tokio::sync::Notify;
use tokio_tungstenite::tungstenite::Message;
use workbench_client::{
    application::events::session::EventSession,
    domain::limits::{LimitConfig, Limits},
    infrastructure::{locator::LocatedEndpoint, websocket::WebSocketConnection},
    ports::{ClientError, EventConsumer, EventSocket, EventSource, Snapshot, SnapshotPort},
};
use workbench_protocol::{
    events::EventFrame,
    workbench::{EventEnvelope, StreamCursor},
};
fn cursor(after: u64) -> StreamCursor {
    StreamCursor {
        stream_id: "orchestration:binding".into(),
        epoch: "e".into(),
        after_sequence: after,
    }
}
fn limits() -> Limits {
    Limits::new(LimitConfig {
        request_timeout: Duration::from_secs(2),
        connect_timeout: Duration::from_secs(1),
        backoff_min: Duration::from_millis(1),
        backoff_max: Duration::from_millis(2),
        ..Default::default()
    })
    .unwrap()
}
fn ticket() -> Action {
    Action::Reply(
        200,
        json!({"ticket":"private-ticket-sentinel","expiresAt":"2026-09-29T00:00:30Z"}),
    )
}
fn hello() -> Message {
    Message::Text(json!({"type":"hello","protocolVersion":1,"epoch":"e"}).to_string())
}
fn gap(after: u64) -> Message {
    Message::Text(json!({"type":"gap","streamId":"orchestration:binding","epoch":"e","reason":"retentionExceeded","firstSequence":after,"lastSequence":after}).to_string())
}
fn event(sequence: u64) -> EventEnvelope {
    EventEnvelope {
        event_id: format!("id-{sequence}"),
        stream_id: cursor(0).stream_id,
        epoch: "e".into(),
        sequence,
        schema: "orchestration.workspaceUpdated.v1".into(),
        occurred_at: "2026-09-29T00:00:00Z".into(),
        correlation_id: None,
        body: json!({"workspaceId":"different-from-binding","revision":2,"reason":"runtimeReconciled"}),
    }
}
fn message(sequence: u64) -> Message {
    Message::Text(
        serde_json::to_string(&EventFrame::Event {
            event: event(sequence),
        })
        .unwrap(),
    )
}
#[derive(Default)]
struct State {
    opened: u32,
    resets: Vec<u64>,
    events: Vec<u64>,
    open_drops: u32,
    callback_drops: u32,
}
struct Consumer {
    state: Arc<Mutex<State>>,
    open_entered: Arc<Notify>,
    open_release: Arc<Notify>,
    observed: Arc<Notify>,
    hold_open: bool,
}
struct OpenGuard(Arc<Mutex<State>>);
impl Drop for OpenGuard {
    fn drop(&mut self) {
        self.0.lock().unwrap().open_drops += 1;
    }
}
#[async_trait]
impl EventConsumer for Consumer {
    async fn opened(&mut self, _: &StreamCursor) -> Result<(), ClientError> {
        let _guard = OpenGuard(self.state.clone());
        self.open_entered.notify_one();
        if self.hold_open {
            self.open_release.notified().await;
        }
        self.state.lock().unwrap().opened += 1;
        self.observed.notify_one();
        Ok(())
    }
    async fn consume(&mut self, event: &EventEnvelope) -> Result<(), ClientError> {
        self.state.lock().unwrap().events.push(event.sequence);
        self.observed.notify_one();
        Ok(())
    }
    async fn reset(&mut self, snapshot: &Snapshot) -> Result<(), ClientError> {
        self.state
            .lock()
            .unwrap()
            .resets
            .push(snapshot.cursor.after_sequence);
        self.observed.notify_one();
        Ok(())
    }
}
struct Source {
    endpoint: Arc<LocatedEndpoint>,
    limits: Limits,
    snapshot_started: Arc<Notify>,
    snapshot_after: u64,
}
struct SourcePort {
    started: Arc<Notify>,
    after: u64,
}
#[async_trait]
impl SnapshotPort for SourcePort {
    async fn snapshot(&mut self, applied: &StreamCursor) -> Result<Snapshot, ClientError> {
        assert_eq!(applied.stream_id, cursor(0).stream_id);
        self.started.notify_one();
        Ok(Snapshot {
            cursor: cursor(self.after),
            value: json!({"lastSequence":self.after}),
        })
    }
}
#[async_trait]
impl EventSource for Source {
    async fn connect(&self, cursor: &StreamCursor) -> Result<Box<dyn EventSocket>, ClientError> {
        Ok(Box::new(
            WebSocketConnection::connect(
                self.endpoint.clone(),
                self.limits.clone(),
                vec![cursor.clone()],
            )
            .await?,
        ))
    }
    fn snapshot_port(&self) -> Box<dyn SnapshotPort> {
        Box::new(SourcePort {
            started: self.snapshot_started.clone(),
            after: self.snapshot_after,
        })
    }
}
async fn notified(n: &Notify) {
    tokio::time::timeout(Duration::from_secs(1), n.notified())
        .await
        .expect("explicit barrier not reached");
}
#[tokio::test]
async fn immediate_gap_during_pending_open_never_resets_or_consumes_before_successful_open() {
    let gap_release = Arc::new(Notify::new());
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocketGate {
            before: vec![hello()],
            gate: gap_release.clone(),
            after: vec![gap(5)],
        },
        ticket(),
        Action::WebSocket(vec![hello(), message(6)]),
    ])
    .await;
    let state = Arc::new(Mutex::new(State::default()));
    let open_entered = Arc::new(Notify::new());
    let open_release = Arc::new(Notify::new());
    let observed = Arc::new(Notify::new());
    let snapshot_started = Arc::new(Notify::new());
    let config = limits();
    let source = Arc::new(Source {
        endpoint: peer.endpoint.clone(),
        limits: config.clone(),
        snapshot_started: snapshot_started.clone(),
        snapshot_after: 5,
    });
    let mut session = EventSession::new(cursor(0), source, config).unwrap();
    session
        .subscribe(
            0,
            Box::new(Consumer {
                state: state.clone(),
                open_entered: open_entered.clone(),
                open_release: open_release.clone(),
                observed: observed.clone(),
                hold_open: true,
            }),
        )
        .unwrap();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let job = tokio::spawn(session.run(async {
        let _ = stop_rx.await;
    }));
    notified(&open_entered).await;
    gap_release.notify_one();
    notified(&snapshot_started).await;
    {
        let s = state.lock().unwrap();
        assert_eq!(s.opened, 0);
        assert_eq!(s.open_drops, 0);
        assert!(s.resets.is_empty());
        assert!(s.events.is_empty());
    }
    open_release.notify_one();
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let wait = observed.notified();
            if state.lock().unwrap().events == vec![6] {
                break;
            }
            wait.await;
        }
    })
    .await
    .unwrap();
    stop_tx.send(()).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), job)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(result.result, Err(ClientError::Cancelled)));
    assert!(result.cleanup_error.is_none());
    assert_eq!(result.cursor, cursor(6));
    {
        let s = state.lock().unwrap();
        assert_eq!(s.opened, 1);
        assert_eq!(s.open_drops, 1);
        assert_eq!(s.resets, vec![5]);
    }
    peer.settled().await;
}
struct CallbackGuard {
    dropped: Arc<Notify>,
    state: Arc<Mutex<State>>,
}
impl Drop for CallbackGuard {
    fn drop(&mut self) {
        self.state.lock().unwrap().callback_drops += 1;
        self.dropped.notify_one();
    }
}
struct SlowConsumer {
    entered: Arc<Notify>,
    release: Arc<Notify>,
    dropped: Arc<Notify>,
    state: Arc<Mutex<State>>,
}
#[async_trait]
impl EventConsumer for SlowConsumer {
    async fn consume(&mut self, event: &EventEnvelope) -> Result<(), ClientError> {
        let _guard = CallbackGuard {
            dropped: self.dropped.clone(),
            state: self.state.clone(),
        };
        self.entered.notify_one();
        self.release.notified().await;
        self.state.lock().unwrap().events.push(event.sequence);
        Ok(())
    }
    async fn reset(&mut self, _: &Snapshot) -> Result<(), ClientError> {
        Ok(())
    }
}
#[tokio::test]
async fn reader_channel_pending_send_and_slow_consumer_share_exact_item_and_byte_budget() {
    use workbench_client::domain::limits::{LimitError, Resource};
    let bytes = serde_json::to_vec(&event(1)).unwrap().len();
    for resource in [Resource::QueueItems, Resource::QueueBytes] {
        let burst = Arc::new(Notify::new());
        let mut peer = Peer::spawn_multi(vec![
            ticket(),
            Action::WebSocketGate {
                before: vec![hello(), message(1)],
                gate: burst.clone(),
                after: vec![message(2), message(3), message(4)],
            },
        ])
        .await;
        let config = Limits::new(LimitConfig {
            queue_items: if resource == Resource::QueueItems {
                3
            } else {
                256
            },
            queue_bytes: if resource == Resource::QueueBytes {
                bytes * 3
            } else {
                8 * 1024 * 1024
            },
            ..limits().config().clone()
        })
        .unwrap();
        let state = Arc::new(Mutex::new(State::default()));
        let entered = Arc::new(Notify::new());
        let dropped = Arc::new(Notify::new());
        let source = Arc::new(Source {
            endpoint: peer.endpoint.clone(),
            limits: config.clone(),
            snapshot_started: Arc::new(Notify::new()),
            snapshot_after: 0,
        });
        let mut session = EventSession::new(cursor(0), source, config.clone()).unwrap();
        let progress = session.progress();
        session
            .subscribe(
                0,
                Box::new(SlowConsumer {
                    entered: entered.clone(),
                    release: Arc::new(Notify::new()),
                    dropped: dropped.clone(),
                    state: state.clone(),
                }),
            )
            .unwrap();
        let job = tokio::spawn(session.run(std::future::pending::<()>()));
        notified(&entered).await;
        burst.notify_one();
        let result = tokio::time::timeout(Duration::from_secs(1), job)
            .await
            .unwrap()
            .unwrap();
        assert!(
            matches!(result.result,Err(ClientError::Limit(LimitError::Exceeded(r))) if r==resource)
        );
        assert_eq!(result.cursor, cursor(0));
        assert!(result.cleanup_error.is_none());
        notified(&dropped).await;
        assert_eq!(progress.queue_usage().unwrap(), (0, 0));
        let (peak_items, peak_bytes) = progress.queue_peaks().unwrap();
        assert_eq!(peak_items, 3);
        assert_eq!(peak_bytes, bytes * 3);
        assert!(peak_items <= config.maximum(Resource::QueueItems));
        assert!(peak_bytes <= config.maximum(Resource::QueueBytes));
        assert!(state.lock().unwrap().events.is_empty());
        assert_eq!(state.lock().unwrap().callback_drops, 1);
        peer.settled().await;
    }
}
#[tokio::test]
async fn cancelling_pending_open_with_immediate_gap_reaps_reader_and_never_resets_or_consumes() {
    let gap_release = Arc::new(Notify::new());
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocketGate {
            before: vec![hello()],
            gate: gap_release.clone(),
            after: vec![gap(5)],
        },
        ticket(),
        Action::WebSocket(vec![hello(), message(6)]),
    ])
    .await;
    let state = Arc::new(Mutex::new(State::default()));
    let entered = Arc::new(Notify::new());
    let snapshot_started = Arc::new(Notify::new());
    let config = limits();
    let source = Arc::new(Source {
        endpoint: peer.endpoint.clone(),
        limits: config.clone(),
        snapshot_started: snapshot_started.clone(),
        snapshot_after: 5,
    });
    let mut session = EventSession::new(cursor(0), source, config).unwrap();
    session
        .subscribe(
            0,
            Box::new(Consumer {
                state: state.clone(),
                open_entered: entered.clone(),
                open_release: Arc::new(Notify::new()),
                observed: Arc::new(Notify::new()),
                hold_open: true,
            }),
        )
        .unwrap();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let job = tokio::spawn(session.run(async {
        let _ = stop_rx.await;
    }));
    notified(&entered).await;
    gap_release.notify_one();
    notified(&snapshot_started).await;
    stop_tx.send(()).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), job)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(result.result, Err(ClientError::Cancelled)));
    assert!(result.cleanup_error.is_none());
    assert_eq!(result.cursor, cursor(0));
    {
        let s = state.lock().unwrap();
        assert_eq!(s.opened, 0);
        assert_eq!(s.open_drops, 1);
        assert!(s.resets.is_empty());
        assert!(s.events.is_empty());
    }
    peer.settled().await;
}
struct PlannedSource {
    base: Source,
    count: Arc<std::sync::atomic::AtomicU32>,
    pending_first: bool,
    fail_all: bool,
    load_dropped: Arc<Notify>,
}
struct PlannedPort {
    number: u32,
    pending_first: bool,
    fail_all: bool,
    started: Arc<Notify>,
    dropped: Arc<Notify>,
    after: u64,
}
struct PortGuard(Arc<Notify>);
impl Drop for PortGuard {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}
#[async_trait]
impl SnapshotPort for PlannedPort {
    async fn snapshot(&mut self, _: &StreamCursor) -> Result<Snapshot, ClientError> {
        let _guard = PortGuard(self.dropped.clone());
        self.started.notify_one();
        if self.pending_first && self.number == 0 {
            std::future::pending::<()>().await;
        }
        if self.fail_all {
            // This fixture exercises transient listener exhaustion, not terminal protocol errors.
            return Err(ClientError::Unavailable);
        }
        Ok(Snapshot {
            cursor: cursor(self.after),
            value: json!({"lastSequence":self.after}),
        })
    }
}
#[async_trait]
impl EventSource for PlannedSource {
    async fn connect(&self, cursor: &StreamCursor) -> Result<Box<dyn EventSocket>, ClientError> {
        self.base.connect(cursor).await
    }
    fn snapshot_port(&self) -> Box<dyn SnapshotPort> {
        Box::new(PlannedPort {
            number: self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            pending_first: self.pending_first,
            fail_all: self.fail_all,
            started: self.base.snapshot_started.clone(),
            dropped: self.load_dropped.clone(),
            after: self.base.snapshot_after,
        })
    }
}
fn ready_consumer(state: Arc<Mutex<State>>, observed: Arc<Notify>) -> Box<dyn EventConsumer> {
    Box::new(Consumer {
        state,
        open_entered: Arc::new(Notify::new()),
        open_release: Arc::new(Notify::new()),
        observed,
        hold_open: false,
    })
}
async fn events_observed(state: &Mutex<State>, observed: &Notify, expected: &[u64]) {
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let wait = observed.notified();
            if state.lock().unwrap().events == expected {
                break;
            }
            wait.await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn new_gap_cancels_pending_snapshot_port_before_fresh_round_applies() {
    let second_gap = Arc::new(Notify::new());
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocket(vec![hello(), gap(5)]),
        ticket(),
        Action::WebSocketGate {
            before: vec![hello()],
            gate: second_gap.clone(),
            after: vec![gap(6)],
        },
        ticket(),
        Action::WebSocket(vec![hello(), message(7)]),
    ])
    .await;
    let config = limits();
    let started = Arc::new(Notify::new());
    let dropped = Arc::new(Notify::new());
    let count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let source = Arc::new(PlannedSource {
        base: Source {
            endpoint: peer.endpoint.clone(),
            limits: config.clone(),
            snapshot_started: started.clone(),
            snapshot_after: 6,
        },
        count: count.clone(),
        pending_first: true,
        fail_all: false,
        load_dropped: dropped.clone(),
    });
    let state = Arc::new(Mutex::new(State::default()));
    let observed = Arc::new(Notify::new());
    let mut session = EventSession::new(cursor(0), source, config).unwrap();
    session
        .subscribe(0, ready_consumer(state.clone(), observed.clone()))
        .unwrap();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let job = tokio::spawn(session.run(async {
        let _ = stop_rx.await;
    }));
    notified(&started).await;
    second_gap.notify_one();
    notified(&dropped).await;
    events_observed(&state, &observed, &[7]).await;
    stop_tx.send(()).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), job)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.cursor, cursor(7));
    assert!(result.cleanup_error.is_none());
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(state.lock().unwrap().resets, vec![6]);
    peer.settled().await;
    let requests = peer.requests.lock().unwrap();
    let tickets = requests
        .iter()
        .filter(|r| r.0 == "/v1/event-tickets")
        .map(|r| r.2["cursors"][0]["afterSequence"].as_u64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(tickets, vec![0, 5, 6]);
}
struct FailConsumer;
#[async_trait]
impl EventConsumer for FailConsumer {
    async fn consume(&mut self, _: &EventEnvelope) -> Result<(), ClientError> {
        Err(ClientError::Protocol)
    }
    async fn reset(&mut self, _: &Snapshot) -> Result<(), ClientError> {
        Err(ClientError::Protocol)
    }
}
#[tokio::test]
async fn listener_only_exhaustion_does_not_stop_other_consumer_on_actual_socket() {
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocket(vec![hello(), message(1), message(2)]),
    ])
    .await;
    let count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let started = Arc::new(Notify::new());
    let config = limits();
    let source = Arc::new(PlannedSource {
        base: Source {
            endpoint: peer.endpoint.clone(),
            limits: config.clone(),
            snapshot_started: started.clone(),
            snapshot_after: 2,
        },
        count: count.clone(),
        pending_first: false,
        fail_all: true,
        load_dropped: Arc::new(Notify::new()),
    });
    let state = Arc::new(Mutex::new(State::default()));
    let observed = Arc::new(Notify::new());
    let mut session = EventSession::new(cursor(0), source, config).unwrap();
    let failed = session.subscribe(0, Box::new(FailConsumer)).unwrap();
    session
        .subscribe(0, ready_consumer(state.clone(), observed.clone()))
        .unwrap();
    let progress = session.progress();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let job = tokio::spawn(session.run(async {
        let _ = stop_rx.await;
    }));
    events_observed(&state, &observed, &[1, 2]).await;
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if progress.exhausted_consumers().unwrap() == vec![failed] {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 5);
    assert_eq!(progress.cursor().unwrap(), cursor(0));
    stop_tx.send(()).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), job)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.cursor, cursor(0));
    assert!(result.cleanup_error.is_none());
    peer.settled().await;
}
#[tokio::test]
async fn terminal_eviction_snapshots_without_reconnecting_and_returns_final_applied_cursor() {
    let mut notice = match gap(5) {
        Message::Text(text) => serde_json::from_str::<serde_json::Value>(&text).unwrap(),
        _ => unreachable!(),
    };
    notice["reason"] = json!("evicted");
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocket(vec![hello(), Message::Text(notice.to_string())]),
    ])
    .await;
    let config = limits();
    let source = Arc::new(Source {
        endpoint: peer.endpoint.clone(),
        limits: config.clone(),
        snapshot_started: Arc::new(Notify::new()),
        snapshot_after: 5,
    });
    let mut session = EventSession::new(cursor(0), source, config).unwrap();
    let state = Arc::new(Mutex::new(State::default()));
    session
        .subscribe(0, ready_consumer(state.clone(), Arc::new(Notify::new())))
        .unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        session.run(std::future::pending::<()>()),
    )
    .await
    .unwrap();
    assert!(result.result.is_ok());
    assert!(result.cleanup_error.is_none());
    assert_eq!(result.cursor, cursor(5));
    assert_eq!(state.lock().unwrap().resets, vec![5]);
    peer.settled().await;
    assert_eq!(
        peer.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.0 == "/v1/event-tickets")
            .count(),
        1
    );
}
struct RetrySource {
    base: Source,
    unavailable: Arc<LocatedEndpoint>,
    cursors: Arc<Mutex<Vec<StreamCursor>>>,
    failures: u32,
}
#[async_trait]
impl EventSource for RetrySource {
    async fn connect(&self, cursor: &StreamCursor) -> Result<Box<dyn EventSocket>, ClientError> {
        let attempt = {
            let mut seen = self.cursors.lock().unwrap();
            seen.push(cursor.clone());
            seen.len() as u32
        };
        if attempt > 1 && attempt <= self.failures + 1 {
            // An actual private descriptor points at a listener whose task has been stopped.
            return Ok(Box::new(
                WebSocketConnection::connect(
                    self.unavailable.clone(),
                    self.base.limits.clone(),
                    vec![cursor.clone()],
                )
                .await?,
            ));
        }
        self.base.connect(cursor).await
    }
    fn snapshot_port(&self) -> Box<dyn SnapshotPort> {
        self.base.snapshot_port()
    }
}
#[tokio::test]
async fn actual_disconnect_then_refused_connection_retries_same_applied_cursor_and_succeeds() {
    let close = Arc::new(Notify::new());
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocketGate {
            before: vec![hello(), message(1)],
            gate: close.clone(),
            after: vec![Message::Close(None)],
        },
        ticket(),
        Action::WebSocket(vec![hello(), message(2)]),
    ])
    .await;
    let mut unavailable = Peer::spawn(vec![], true).await;
    unavailable.stop().await;
    let config = limits();
    let cursors = Arc::new(Mutex::new(Vec::new()));
    let source = Arc::new(RetrySource {
        base: Source {
            endpoint: peer.endpoint.clone(),
            limits: config.clone(),
            snapshot_started: Arc::new(Notify::new()),
            snapshot_after: 0,
        },
        unavailable: unavailable.endpoint.clone(),
        cursors: cursors.clone(),
        failures: 1,
    });
    let state = Arc::new(Mutex::new(State::default()));
    let observed = Arc::new(Notify::new());
    let mut session = EventSession::new(cursor(0), source, config).unwrap();
    let progress = session.progress();
    session
        .subscribe(0, ready_consumer(state.clone(), observed.clone()))
        .unwrap();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let job = tokio::spawn(session.run(async {
        let _ = stop_rx.await;
    }));
    tokio::time::timeout(Duration::from_secs(1), async {
        while progress.cursor().unwrap() != cursor(1) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    close.notify_one();
    events_observed(&state, &observed, &[1, 2]).await;
    stop_tx.send(()).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), job)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.cursor, cursor(2));
    assert!(result.cleanup_error.is_none());
    assert_eq!(
        *cursors.lock().unwrap(),
        vec![cursor(0), cursor(1), cursor(1)]
    );
    assert!(unavailable.requests.lock().unwrap().is_empty());
    peer.settled().await;
    let sent = peer.requests.lock().unwrap();
    assert_eq!(
        sent.iter()
            .filter(|r| r.0 == "/v1/event-tickets")
            .map(|r| r.2["cursors"][0]["afterSequence"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
}
#[tokio::test]
async fn actual_transient_reconnect_exhaustion_uses_exact_bounded_attempts_and_keeps_cursor() {
    let close = Arc::new(Notify::new());
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocketGate {
            before: vec![hello(), message(1)],
            gate: close.clone(),
            after: vec![Message::Close(None)],
        },
    ])
    .await;
    let mut unavailable = Peer::spawn(vec![], true).await;
    unavailable.stop().await;
    let config = limits();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let source = Arc::new(RetrySource {
        base: Source {
            endpoint: peer.endpoint.clone(),
            limits: config.clone(),
            snapshot_started: Arc::new(Notify::new()),
            snapshot_after: 0,
        },
        unavailable: unavailable.endpoint.clone(),
        cursors: seen.clone(),
        failures: 20,
    });
    let state = Arc::new(Mutex::new(State::default()));
    let mut session = EventSession::new(cursor(0), source, config.clone()).unwrap();
    let progress = session.progress();
    session
        .subscribe(0, ready_consumer(state, Arc::new(Notify::new())))
        .unwrap();
    let job = tokio::spawn(session.run(std::future::pending::<()>()));
    tokio::time::timeout(Duration::from_secs(1), async {
        while progress.cursor().unwrap() != cursor(1) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    close.notify_one();
    let result = tokio::time::timeout(Duration::from_secs(1), job)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(result.result, Err(ClientError::Unavailable)));
    assert_eq!(result.cursor, cursor(1));
    assert!(result.cleanup_error.is_none());
    let seen = seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 1 + config.config().recovery_attempts as usize);
    assert_eq!(seen[0], cursor(0));
    assert!(seen[1..].iter().all(|c| c == &cursor(1)));
    assert!(unavailable.requests.lock().unwrap().is_empty());
    peer.settled().await;
}
#[tokio::test]
async fn transient_reconnect_keeps_gap_boundary_separate_from_unapplied_cursor() {
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocket(vec![hello(), gap(5)]),
        ticket(),
        Action::WebSocket(vec![hello(), message(6)]),
    ])
    .await;
    let mut unavailable = Peer::spawn(vec![], true).await;
    unavailable.stop().await;
    let config = limits();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let source = Arc::new(RetrySource {
        base: Source {
            endpoint: peer.endpoint.clone(),
            limits: config.clone(),
            snapshot_started: Arc::new(Notify::new()),
            snapshot_after: 5,
        },
        unavailable: unavailable.endpoint.clone(),
        cursors: seen.clone(),
        failures: 1,
    });
    let state = Arc::new(Mutex::new(State::default()));
    let observed = Arc::new(Notify::new());
    let mut session = EventSession::new(cursor(0), source, config).unwrap();
    session
        .subscribe(0, ready_consumer(state.clone(), observed.clone()))
        .unwrap();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let job = tokio::spawn(session.run(async {
        let _ = stop_rx.await;
    }));
    events_observed(&state, &observed, &[6]).await;
    stop_tx.send(()).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), job)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.cursor, cursor(6));
    assert!(result.cleanup_error.is_none());
    assert_eq!(*seen.lock().unwrap(), vec![cursor(0), cursor(5), cursor(5)]);
    assert_eq!(state.lock().unwrap().resets, vec![5]);
    peer.settled().await;
}
#[tokio::test]
async fn identity_protocol_and_auth_connect_failures_are_terminal_without_retry() {
    use workbench_protocol::{FaultCode, RequestId, WorkbenchFault};
    for family in 0..3 {
        let close = Arc::new(Notify::new());
        let mut peer = Peer::spawn_multi(vec![
            ticket(),
            Action::WebSocketGate {
                before: vec![hello(), message(1)],
                gate: close.clone(),
                after: vec![Message::Close(None)],
            },
        ])
        .await;
        let actions = match family {
            0 => vec![],
            1 => vec![Action::Reply(
                200,
                json!({"ticket":"malformed-ticket-sentinel"}),
            )],
            _ => {
                let mut body = serde_json::to_value(WorkbenchFault::new(
                    FaultCode::Unauthenticated,
                    RequestId::random(),
                    "private-sentinel",
                ))
                .unwrap();
                body["status"] = json!(401);
                vec![Action::Reply(401, body)]
            }
        };
        let mut rejected = Peer::spawn(actions, family != 0).await;
        let config = limits();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let source = Arc::new(RetrySource {
            base: Source {
                endpoint: peer.endpoint.clone(),
                limits: config.clone(),
                snapshot_started: Arc::new(Notify::new()),
                snapshot_after: 0,
            },
            unavailable: rejected.endpoint.clone(),
            cursors: seen.clone(),
            failures: 20,
        });
        let state = Arc::new(Mutex::new(State::default()));
        let mut session = EventSession::new(cursor(0), source, config).unwrap();
        let progress = session.progress();
        session
            .subscribe(0, ready_consumer(state, Arc::new(Notify::new())))
            .unwrap();
        let job = tokio::spawn(session.run(std::future::pending::<()>()));
        tokio::time::timeout(Duration::from_secs(1), async {
            while progress.cursor().unwrap() != cursor(1) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        close.notify_one();
        let result = tokio::time::timeout(Duration::from_secs(1), job)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            (&result.result, family),
            (Err(ClientError::Identity), 0)
                | (Err(ClientError::Protocol), 1)
                | (Err(ClientError::Fault(_)), 2)
        ));
        assert_eq!(result.cursor, cursor(1));
        assert!(result.cleanup_error.is_none());
        assert_eq!(*seen.lock().unwrap(), vec![cursor(0), cursor(1)]);
        rejected.settled().await;
        peer.settled().await;
        assert_eq!(
            rejected.requests.lock().unwrap().len(),
            if family == 0 { 1 } else { 3 }
        );
    }
}
struct AdvancingSource {
    base: Source,
    count: Arc<std::sync::atomic::AtomicU32>,
}
#[async_trait]
impl EventSource for AdvancingSource {
    async fn connect(&self, cursor: &StreamCursor) -> Result<Box<dyn EventSocket>, ClientError> {
        self.base.connect(cursor).await
    }
    fn snapshot_port(&self) -> Box<dyn SnapshotPort> {
        Box::new(SourcePort {
            started: self.base.snapshot_started.clone(),
            after: self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u64 + 1,
        })
    }
}
#[tokio::test]
async fn six_complete_snapshot_reset_recoveries_without_events_each_start_a_fresh_budget() {
    let gates = (0..5).map(|_| Arc::new(Notify::new())).collect::<Vec<_>>();
    let mut actions = vec![ticket(), Action::WebSocket(vec![hello(), gap(1)])];
    for round in 1..=6 {
        actions.push(ticket());
        actions.push(if round < 6 {
            Action::WebSocketGate {
                before: vec![hello()],
                gate: gates[round - 1].clone(),
                after: vec![gap(round as u64 + 1)],
            }
        } else {
            Action::WebSocket(vec![hello()])
        });
    }
    let mut peer = Peer::spawn_multi(actions).await;
    let config = limits();
    let count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let source = Arc::new(AdvancingSource {
        base: Source {
            endpoint: peer.endpoint.clone(),
            limits: config.clone(),
            snapshot_started: Arc::new(Notify::new()),
            snapshot_after: 0,
        },
        count: count.clone(),
    });
    let state = Arc::new(Mutex::new(State::default()));
    let mut session = EventSession::new(cursor(0), source, config).unwrap();
    session
        .subscribe(0, ready_consumer(state.clone(), Arc::new(Notify::new())))
        .unwrap();
    let progress = session.progress();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let job = tokio::spawn(session.run(async {
        let _ = stop_rx.await;
    }));
    for round in 1..=6 {
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if progress.cursor().unwrap() == cursor(round as u64)
                    && progress.phase().unwrap()
                        == workbench_client::application::events::RecoveryPhase::Live
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            state.lock().unwrap().resets,
            (1..=round as u64).collect::<Vec<_>>()
        );
        assert!(state.lock().unwrap().events.is_empty());
        if round < 6 {
            gates[round - 1].notify_one();
        }
    }
    assert!(!job.is_finished());
    stop_tx.send(()).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), job)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.cursor, cursor(6));
    assert!(result.cleanup_error.is_none());
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 6);
    peer.settled().await;
    assert_eq!(
        peer.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.0 == "/v1/event-tickets")
            .count(),
        7
    );
}
struct SlowReset {
    entered: Arc<Notify>,
    dropped: Arc<Notify>,
}
#[async_trait]
impl EventConsumer for SlowReset {
    async fn consume(&mut self, _: &EventEnvelope) -> Result<(), ClientError> {
        panic!("consumer may not receive before reset completion")
    }
    async fn reset(&mut self, _: &Snapshot) -> Result<(), ClientError> {
        let _guard = PortGuard(self.dropped.clone());
        self.entered.notify_one();
        std::future::pending().await
    }
}
#[tokio::test]
async fn partial_reset_and_fast_consumer_acks_do_not_reset_connection_recovery_budget() {
    let close = (0..3).map(|_| Arc::new(Notify::new())).collect::<Vec<_>>();
    let mut actions = vec![ticket(), Action::WebSocket(vec![hello(), gap(5)])];
    for (round, gate) in close.iter().enumerate() {
        actions.push(ticket());
        actions.push(Action::WebSocketGate {
            before: vec![hello(), message(round as u64 + 6)],
            gate: gate.clone(),
            after: vec![Message::Close(None)],
        });
    }
    let mut peer = Peer::spawn_multi(actions).await;
    let config = Limits::new(LimitConfig {
        recovery_attempts: 3,
        ..limits().config().clone()
    })
    .unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let source = Arc::new(RetrySource {
        base: Source {
            endpoint: peer.endpoint.clone(),
            limits: config.clone(),
            snapshot_started: Arc::new(Notify::new()),
            snapshot_after: 5,
        },
        unavailable: peer.endpoint.clone(),
        cursors: seen.clone(),
        failures: 0,
    });
    let slow_started = Arc::new(Notify::new());
    let slow_dropped = Arc::new(Notify::new());
    let state = Arc::new(Mutex::new(State::default()));
    let observed = Arc::new(Notify::new());
    let mut session = EventSession::new(cursor(0), source, config).unwrap();
    session
        .subscribe(
            0,
            Box::new(SlowReset {
                entered: slow_started.clone(),
                dropped: slow_dropped.clone(),
            }),
        )
        .unwrap();
    session
        .subscribe(0, ready_consumer(state.clone(), observed.clone()))
        .unwrap();
    let progress = session.progress();
    let job = tokio::spawn(session.run(std::future::pending::<()>()));
    notified(&slow_started).await;
    for (round, gate) in close.iter().enumerate() {
        events_observed(
            &state,
            &observed,
            &(6..=round as u64 + 6).collect::<Vec<_>>(),
        )
        .await;
        assert_eq!(
            progress.phase().unwrap(),
            workbench_client::application::events::RecoveryPhase::ConsumerReset
        );
        assert_eq!(progress.cursor().unwrap(), cursor(0));
        gate.notify_one();
    }
    let result = tokio::time::timeout(Duration::from_secs(1), job)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(result.result, Err(ClientError::Unavailable)));
    assert!(result.cleanup_error.is_none());
    assert_eq!(result.cursor, cursor(0));
    notified(&slow_dropped).await;
    peer.settled().await;
    assert_eq!(
        *seen.lock().unwrap(),
        vec![cursor(0), cursor(5), cursor(5), cursor(5)]
    );
    assert_eq!(state.lock().unwrap().resets, vec![5]);
}
#[tokio::test]
async fn dropping_session_future_cancels_pending_snapshot_and_owned_actual_reader() {
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocket(vec![hello(), gap(5)]),
        ticket(),
        Action::WebSocket(vec![hello()]),
    ])
    .await;
    let started = Arc::new(Notify::new());
    let dropped = Arc::new(Notify::new());
    let config = limits();
    let source = Arc::new(PlannedSource {
        base: Source {
            endpoint: peer.endpoint.clone(),
            limits: config.clone(),
            snapshot_started: started.clone(),
            snapshot_after: 5,
        },
        count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        pending_first: true,
        fail_all: false,
        load_dropped: dropped.clone(),
    });
    let mut session = EventSession::new(cursor(0), source, config).unwrap();
    session
        .subscribe(
            0,
            ready_consumer(
                Arc::new(Mutex::new(State::default())),
                Arc::new(Notify::new()),
            ),
        )
        .unwrap();
    let progress = session.progress();
    let job = tokio::spawn(session.run(std::future::pending::<()>()));
    notified(&started).await;
    job.abort();
    let done = tokio::time::timeout(Duration::from_secs(1), job)
        .await
        .unwrap();
    assert!(matches!(done,Err(error) if error.is_cancelled()));
    notified(&dropped).await;
    peer.settled().await;
    assert_eq!(
        progress.phase().unwrap(),
        workbench_client::application::events::RecoveryPhase::Terminal
    );
    assert_eq!(progress.cursor().unwrap(), cursor(0));
    assert_eq!(progress.queue_usage().unwrap(), (0, 0));
}
#[tokio::test]
async fn dropping_session_future_cancels_pending_consumer_without_late_ack_or_socket() {
    let mut peer =
        Peer::spawn_multi(vec![ticket(), Action::WebSocket(vec![hello(), message(1)])]).await;
    let entered = Arc::new(Notify::new());
    let dropped = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let state = Arc::new(Mutex::new(State::default()));
    let config = limits();
    let source = Arc::new(Source {
        endpoint: peer.endpoint.clone(),
        limits: config.clone(),
        snapshot_started: Arc::new(Notify::new()),
        snapshot_after: 0,
    });
    let mut session = EventSession::new(cursor(0), source, config).unwrap();
    session
        .subscribe(
            0,
            Box::new(SlowConsumer {
                entered: entered.clone(),
                release: release.clone(),
                dropped: dropped.clone(),
                state: state.clone(),
            }),
        )
        .unwrap();
    let progress = session.progress();
    let job = tokio::spawn(session.run(std::future::pending::<()>()));
    notified(&entered).await;
    job.abort();
    let done = tokio::time::timeout(Duration::from_secs(1), job)
        .await
        .unwrap();
    assert!(matches!(done,Err(error) if error.is_cancelled()));
    notified(&dropped).await;
    release.notify_one();
    peer.settled().await;
    assert_eq!(
        progress.phase().unwrap(),
        workbench_client::application::events::RecoveryPhase::Terminal
    );
    assert_eq!(progress.cursor().unwrap(), cursor(0));
    assert_eq!(progress.queue_usage().unwrap(), (0, 0));
    assert!(state.lock().unwrap().events.is_empty());
    assert_eq!(state.lock().unwrap().callback_drops, 1);
    assert_eq!(peer.requests.lock().unwrap().len(), 5);
}

struct NotificationSource {
    endpoint: Arc<LocatedEndpoint>,
    limits: Limits,
    loaded: Arc<Notify>,
    release: Arc<Notify>,
}
struct NotificationSnapshot {
    loaded: Arc<Notify>,
    release: Arc<Notify>,
}
#[async_trait]
impl SnapshotPort for NotificationSnapshot {
    async fn snapshot(&mut self, applied: &StreamCursor) -> Result<Snapshot, ClientError> {
        assert_eq!(applied.stream_id, "bench:existing");
        assert_eq!(applied.after_sequence, 1);
        self.loaded.notify_one();
        self.release.notified().await;
        Ok(Snapshot {
            cursor: StreamCursor {
                after_sequence: 2,
                ..applied.clone()
            },
            value: json!({"title":"current-state"}),
        })
    }
}
#[async_trait]
impl EventSource for NotificationSource {
    async fn connect(&self, cursor: &StreamCursor) -> Result<Box<dyn EventSocket>, ClientError> {
        Ok(Box::new(
            WebSocketConnection::connect(
                self.endpoint.clone(),
                self.limits.clone(),
                vec![cursor.clone()],
            )
            .await?,
        ))
    }
    fn snapshot_port(&self) -> Box<dyn SnapshotPort> {
        Box::new(NotificationSnapshot {
            loaded: self.loaded.clone(),
            release: self.release.clone(),
        })
    }
}
fn notification(sequence: u64) -> Message {
    let mut event = event(sequence);
    event.stream_id = "bench:existing".into();
    event.schema = "bench.titleRequested.v1".into();
    event.body = json!({"title":"current-state"});
    Message::Text(serde_json::to_string(&EventFrame::Event { event }).unwrap())
}
#[tokio::test]
async fn notification_disconnect_opens_live_socket_before_snapshot_and_resets_before_buffered_delivery(
) {
    notification_recovery_live_first(None).await;
}
#[tokio::test]
async fn notification_lag_and_shutdown_live_only_reconnect_snapshot_before_delivery() {
    for reason in ["subscriberLagged", "shutdown"] {
        notification_recovery_live_first(Some(reason)).await;
    }
}
struct GatedNotificationConsumer {
    inner: Box<dyn EventConsumer>,
    entered: Arc<Notify>,
    release: Arc<Notify>,
}
#[async_trait]
impl EventConsumer for GatedNotificationConsumer {
    async fn opened(&mut self, cursor: &StreamCursor) -> Result<(), ClientError> {
        self.inner.opened(cursor).await
    }
    async fn consume(&mut self, event: &EventEnvelope) -> Result<(), ClientError> {
        self.inner.consume(event).await
    }
    async fn reset(&mut self, snapshot: &Snapshot) -> Result<(), ClientError> {
        self.entered.notify_one();
        self.release.notified().await;
        self.inner.reset(snapshot).await
    }
}
async fn notification_recovery_live_first(reason: Option<&str>) {
    let close = Arc::new(Notify::new());
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocketGate {
            before: vec![hello(), notification(1)],
            gate: close.clone(),
            after: vec![match reason {
            Some(reason) => Message::Text(json!({"type":"gap","streamId":"bench:existing","epoch":"e","reason":reason,"firstSequence":2,"lastSequence":2}).to_string()),
            None => Message::Close(None),
        }],
        },
        ticket(),
        // Non-retaining replacement is live-only: lost notification2 is not replayed.
        Action::WebSocket(vec![hello(), notification(3)]),
    ])
    .await;
    let loaded = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let config = limits();
    let source = Arc::new(NotificationSource {
        endpoint: peer.endpoint.clone(),
        limits: config.clone(),
        loaded: loaded.clone(),
        release: release.clone(),
    });
    let initial = StreamCursor {
        stream_id: "bench:existing".into(),
        ..cursor(0)
    };
    let state = Arc::new(Mutex::new(State::default()));
    let observed = Arc::new(Notify::new());
    let mut session = EventSession::new(initial.clone(), source, config).unwrap();
    let reset_entered = Arc::new(Notify::new());
    let reset_release = Arc::new(Notify::new());
    session
        .subscribe(
            0,
            Box::new(GatedNotificationConsumer {
                inner: ready_consumer(state.clone(), observed.clone()),
                entered: reset_entered.clone(),
                release: reset_release.clone(),
            }),
        )
        .unwrap();
    let progress = session.progress();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let mut job = harness::OwnedTask::default();
    job.start(session.run(async {
        let _ = stop_rx.await;
    }));
    use futures_util::FutureExt;
    let outcome = tokio::time::timeout(
        Duration::from_secs(3),
        std::panic::AssertUnwindSafe(async {
            tokio::time::timeout(Duration::from_secs(1), async {
                while progress.cursor().unwrap().after_sequence != 1 {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            close.notify_one();
            notified(&loaded).await;
            // Snapshot is invoked only after the replacement connection's verified hello.
            assert_eq!(
                peer.requests
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|r| r.0.starts_with("/v1/events?"))
                    .count(),
                2
            );
            // The sole live-only event3 is actually held in the aggregate queue.
            tokio::time::timeout(Duration::from_secs(1), async {
                while progress.queue_usage().unwrap().0 == 0 {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            assert_eq!(state.lock().unwrap().events, vec![1]);
            assert!(state.lock().unwrap().resets.is_empty());
            assert_eq!(progress.cursor().unwrap().after_sequence, 1);
            release.notify_one();
            notified(&reset_entered).await;
            assert_eq!(state.lock().unwrap().events, vec![1]);
            assert!(state.lock().unwrap().resets.is_empty());
            assert_eq!(progress.cursor().unwrap().after_sequence, 1);
            reset_release.notify_one();
            tokio::time::timeout(Duration::from_secs(1), async {
                while progress.cursor().unwrap().after_sequence != 3 {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            assert_eq!(state.lock().unwrap().resets, vec![2]);
            assert_eq!(state.lock().unwrap().events, vec![1, 3]);
            stop_tx.send(()).unwrap();
            let result = job.wait().await;
            assert_eq!(
                result.cursor,
                StreamCursor {
                    after_sequence: 3,
                    ..initial
                }
            );
            assert!(matches!(result.result, Err(ClientError::Cancelled)));
            assert!(result.cleanup_error.is_none());
            assert_eq!(progress.queue_usage().unwrap(), (0, 0));
            assert_eq!(
                peer.requests
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|r| r.0 == "/v1/event-tickets")
                    .map(|r| r.2["cursors"][0]["afterSequence"].as_u64().unwrap())
                    .collect::<Vec<_>>(),
                vec![0, if reason.is_some() { 2 } else { 1 }]
            );
        })
        .catch_unwind(),
    )
    .await;
    job.cancel_join()
        .await
        .expect("notification fixture owned cleanup");
    peer.settled().await;
    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(_) => panic!("notification recovery deadline after owned abort/join and peer EOF"),
    }
}

struct StopAfterApplied {
    applied: Arc<Notify>,
}
#[async_trait]
impl EventConsumer for StopAfterApplied {
    async fn consume(&mut self, _: &EventEnvelope) -> Result<(), ClientError> {
        self.applied.notify_one();
        Ok(())
    }
    async fn reset(&mut self, _: &Snapshot) -> Result<(), ClientError> {
        Ok(())
    }
}
#[tokio::test]
async fn stop_after_successful_callback_preserves_its_ack_before_invalidating_pending_jobs() {
    let mut peer =
        Peer::spawn_multi(vec![ticket(), Action::WebSocket(vec![hello(), message(1)])]).await;
    let config = limits();
    let source = Arc::new(Source {
        endpoint: peer.endpoint.clone(),
        limits: config.clone(),
        snapshot_started: Arc::new(Notify::new()),
        snapshot_after: 0,
    });
    let applied = Arc::new(Notify::new());
    let mut session = EventSession::new(cursor(0), source, config).unwrap();
    session
        .subscribe(
            0,
            Box::new(StopAfterApplied {
                applied: applied.clone(),
            }),
        )
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), session.run(applied.notified()))
        .await
        .unwrap();
    assert!(matches!(result.result, Err(ClientError::Cancelled)));
    assert_eq!(result.cursor, cursor(1));
    assert!(result.cleanup_error.is_none());
    peer.settled().await;
}

struct FailingSnapshotSource {
    endpoint: Arc<LocatedEndpoint>,
    limits: Limits,
    error: Arc<Mutex<Option<ClientError>>>,
    loads: Arc<std::sync::atomic::AtomicUsize>,
    connections: Arc<Mutex<Vec<u64>>>,
    allow_success: bool,
}
struct FailingSnapshotPort {
    error: Arc<Mutex<Option<ClientError>>>,
    loads: Arc<std::sync::atomic::AtomicUsize>,
    allow_success: bool,
}
#[async_trait]
impl SnapshotPort for FailingSnapshotPort {
    async fn snapshot(&mut self, _: &StreamCursor) -> Result<Snapshot, ClientError> {
        self.loads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Some(error) = self.error.lock().unwrap().take() {
            return Err(error);
        }
        assert!(
            self.allow_success,
            "terminal failure must not issue another snapshot request"
        );
        Ok(Snapshot {
            cursor: cursor(5),
            value: json!({"revision":2}),
        })
    }
}
#[async_trait]
impl EventSource for FailingSnapshotSource {
    async fn connect(&self, cursor: &StreamCursor) -> Result<Box<dyn EventSocket>, ClientError> {
        self.connections.lock().unwrap().push(cursor.after_sequence);
        Ok(Box::new(
            WebSocketConnection::connect(
                self.endpoint.clone(),
                self.limits.clone(),
                vec![cursor.clone()],
            )
            .await?,
        ))
    }
    fn snapshot_port(&self) -> Box<dyn SnapshotPort> {
        Box::new(FailingSnapshotPort {
            error: self.error.clone(),
            loads: self.loads.clone(),
            allow_success: self.allow_success,
        })
    }
}
#[tokio::test]
async fn verified_live_snapshot_terminal_errors_stop_once_and_transient_error_recovers() {
    use futures_util::FutureExt;
    use workbench_protocol::{FaultCode, RequestId, WorkbenchFault};
    for error in [
        ClientError::Identity,
        ClientError::Incompatible,
        ClientError::Protocol,
        ClientError::Fault(Box::new(WorkbenchFault::new(
            FaultCode::Unauthenticated,
            RequestId::new("original").unwrap(),
            "private-sentinel",
        ))),
        ClientError::Unavailable,
    ] {
        let transient = matches!(error, ClientError::Unavailable);
        let expected = std::mem::discriminant(&error);
        let gate = Arc::new(Notify::new());
        let mut actions = vec![
            ticket(),
            Action::WebSocketGate {
                before: vec![hello()],
                gate: gate.clone(),
                after: vec![gap(5)],
            },
            ticket(),
            Action::WebSocket(vec![hello(), message(6)]),
        ];
        if transient {
            actions.extend([ticket(), Action::WebSocket(vec![hello(), message(6)])]);
        }
        let mut peer = Peer::spawn_multi(actions).await;
        let state = Arc::new(Mutex::new(State::default()));
        let observed = Arc::new(Notify::new());
        let loads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let connections = Arc::new(Mutex::new(Vec::new()));
        let config = limits();
        let source = Arc::new(FailingSnapshotSource {
            endpoint: peer.endpoint.clone(),
            limits: config.clone(),
            error: Arc::new(Mutex::new(Some(error))),
            loads: loads.clone(),
            connections: connections.clone(),
            allow_success: transient,
        });
        let mut session = EventSession::new(cursor(0), source, config).unwrap();
        session
            .subscribe(0, ready_consumer(state.clone(), observed.clone()))
            .unwrap();
        let progress = session.progress();
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        let mut job = harness::OwnedTask::default();
        job.start(session.run(async {
            let _ = stop_rx.await;
        }));
        let outcome = tokio::time::timeout(
            Duration::from_secs(3),
            std::panic::AssertUnwindSafe(async {
                tokio::time::timeout(Duration::from_secs(1), async {
                    while state.lock().unwrap().opened != 1 {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .unwrap();
                gate.notify_one();
                if transient {
                    tokio::time::timeout(Duration::from_secs(1), async {
                        while progress.cursor().unwrap().after_sequence != 6 {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await
                    .unwrap();
                    stop_tx.send(()).unwrap();
                }
                let result = job.wait().await;
                assert!(result.cleanup_error.is_none());
                assert_eq!(progress.queue_usage().unwrap(), (0, 0));
                if transient {
                    assert!(matches!(result.result, Err(ClientError::Cancelled)));
                    assert_eq!(result.cursor, cursor(6));
                    assert_eq!(loads.load(std::sync::atomic::Ordering::SeqCst), 2);
                    assert_eq!(*connections.lock().unwrap(), vec![0, 5, 5]);
                    assert_eq!(state.lock().unwrap().resets, vec![5]);
                    assert_eq!(state.lock().unwrap().events, vec![6]);
                } else {
                    assert_eq!(
                        std::mem::discriminant(&result.result.unwrap_err()),
                        expected
                    );
                    assert_eq!(result.cursor, cursor(0));
                    assert_eq!(loads.load(std::sync::atomic::Ordering::SeqCst), 1);
                    assert_eq!(*connections.lock().unwrap(), vec![0, 5]);
                    assert!(state.lock().unwrap().resets.is_empty());
                    assert!(state.lock().unwrap().events.is_empty());
                }
                assert_eq!(
                    peer.requests
                        .lock()
                        .unwrap()
                        .iter()
                        .filter(|r| r.0.starts_with("/v1/events?"))
                        .count(),
                    if transient { 3 } else { 2 }
                );
            })
            .catch_unwind(),
        )
        .await;
        job.cancel_join()
            .await
            .expect("owned snapshot session cleanup");
        peer.settled().await;
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(panic)) => std::panic::resume_unwind(panic),
            Err(_) => panic!("snapshot session deadline after abort/join and actual EOF"),
        }
    }
}
