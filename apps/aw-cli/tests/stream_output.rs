use aw_cli::infrastructure::output::stream::JsonlOutput;
use serde_json::json;
use std::future::Future;
use std::{
    io,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tokio::io::AsyncWrite;
use workbench_client::{
    domain::limits::Limits,
    ports::{EventConsumer, Snapshot},
};
use workbench_protocol::workbench::{EventEnvelope, StreamCursor};
fn cursor(after: u64) -> StreamCursor {
    StreamCursor {
        stream_id: "orchestration:binding".into(),
        epoch: "e".into(),
        after_sequence: after,
    }
}
fn event() -> EventEnvelope {
    EventEnvelope {
        event_id: "event1".into(),
        stream_id: cursor(0).stream_id,
        epoch: "e".into(),
        sequence: 1,
        schema: "orchestration.workspaceUpdated.v1".into(),
        occurred_at: "2026-09-29T00:00:00Z".into(),
        correlation_id: None,
        body: json!({"workspaceId":"workspace","revision":1,"reason":"notificationRecovery"}),
    }
}
#[derive(Default)]
struct State {
    bytes: Vec<u8>,
    flushes: usize,
    fail_at: Option<usize>,
    hold_at: Option<usize>,
    hold_flush: bool,
    flush_pending: Option<Arc<tokio::sync::Notify>>,
    flush_waker: Option<std::task::Waker>,
    flushed: Option<(usize, Arc<tokio::sync::Notify>)>,
    writing_pending: Option<Arc<tokio::sync::Notify>>,
}
struct Writer(Arc<Mutex<State>>);
impl AsyncWrite for Writer {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut state = self.0.lock().unwrap();
        if state.fail_at.is_some_and(|n| state.bytes.len() >= n) {
            return Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()));
        }
        if state.hold_at.is_some_and(|n| state.bytes.len() >= n) {
            if let Some(notify) = &state.writing_pending {
                notify.notify_one();
            }
            return Poll::Pending;
        }
        state.bytes.push(bytes[0]);
        Poll::Ready(Ok(1))
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut state = self.0.lock().unwrap();
        if state.hold_flush {
            state.flush_waker = Some(cx.waker().clone());
            if let Some(notify) = &state.flush_pending {
                notify.notify_one();
            }
            return Poll::Pending;
        }
        state.flushes += 1;
        if let Some((count, notify)) = &state.flushed {
            if *count == state.flushes {
                notify.notify_one();
            }
        }
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
fn sink(state: Arc<Mutex<State>>) -> JsonlOutput<Writer> {
    JsonlOutput::new(Writer(state), Limits::default())
}
#[tokio::test]
async fn one_byte_partial_writes_produce_complete_open_event_reset_end_records() {
    let state = Arc::new(Mutex::new(State::default()));
    let out = sink(state.clone());
    let mut consumer = out.consumer();
    consumer.opened(&cursor(0)).await.unwrap();
    consumer.consume(&event()).await.unwrap();
    consumer
        .reset_from(
            &Snapshot {
                cursor: cursor(2),
                value: json!({"revision":2}),
            },
            &cursor(1),
        )
        .await
        .unwrap();
    out.finish(&cursor(2), None).await.unwrap();
    assert!(out.opened());
    let state = state.lock().unwrap();
    assert_eq!(state.flushes, 4);
    let records = state
        .bytes
        .split(|b| *b == b'\n')
        .filter(|b| !b.is_empty())
        .map(|b| serde_json::from_slice::<serde_json::Value>(b).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 4);
    assert_eq!(records[0]["type"], "stream.open");
    assert_eq!(records[1]["event"]["sequence"], 1);
    assert_eq!(records[2]["applied"]["afterSequence"], 1);
    assert_eq!(records[2]["cursor"]["afterSequence"], 2);
    assert_eq!(records[3]["type"], "stream.end");
}
#[tokio::test]
async fn broken_pipe_during_event_keeps_cursor_unapplied_and_prevents_appending_end() {
    let state = Arc::new(Mutex::new(State::default()));
    let out = sink(state.clone());
    let mut consumer = out.consumer();
    consumer.opened(&cursor(0)).await.unwrap();
    {
        let mut guard = state.lock().unwrap();
        guard.fail_at = Some(guard.bytes.len() + 7);
    }
    assert!(consumer.consume(&event()).await.is_err());
    let length = state.lock().unwrap().bytes.len();
    assert!(out.failed());
    assert!(out.finish(&cursor(0), None).await.is_err());
    assert_eq!(state.lock().unwrap().bytes.len(), length);
    assert_eq!(
        state
            .lock()
            .unwrap()
            .bytes
            .iter()
            .filter(|b| **b == b'\n')
            .count(),
        1
    );
}
#[tokio::test]
async fn cancellation_after_partial_event_poison_sink_without_newline_or_ack() {
    let state = Arc::new(Mutex::new(State::default()));
    let out = sink(state.clone());
    let mut consumer = out.consumer();
    consumer.opened(&cursor(0)).await.unwrap();
    let length = state.lock().unwrap().bytes.len();
    state.lock().unwrap().hold_at = Some(length + 3);
    let envelope = event();
    let mut delivery = Box::pin(consumer.consume(&envelope));
    assert!(matches!(
        std::future::poll_fn(|cx| Poll::Ready(delivery.as_mut().poll(cx))).await,
        Poll::Pending
    ));
    drop(delivery);
    assert!(out.failed());
    assert!(out.finish(&cursor(0), None).await.is_err());
    assert_eq!(state.lock().unwrap().bytes.len(), length + 3);
}
#[tokio::test]
async fn complete_newline_without_flush_is_not_success_and_open_is_not_published() {
    let state = Arc::new(Mutex::new(State {
        hold_flush: true,
        ..State::default()
    }));
    let out = sink(state.clone());
    let mut consumer = out.consumer();
    let initial = cursor(0);
    let mut opening = Box::pin(consumer.opened(&initial));
    assert!(matches!(
        std::future::poll_fn(|cx| Poll::Ready(opening.as_mut().poll(cx))).await,
        Poll::Pending
    ));
    assert_eq!(state.lock().unwrap().bytes.last(), Some(&b'\n'));
    assert!(!out.opened());
    drop(opening);
    assert!(out.failed());
}

mod support;
use async_trait::async_trait;
use support::peer::{Action, Peer};
use tokio::sync::Notify;
use tokio_tungstenite::tungstenite::Message;
use workbench_client::{
    application::events::session::EventSession,
    infrastructure::{locator::LocatedEndpoint, websocket::WebSocketConnection},
    ports::{ClientError, EventSocket, EventSource, SnapshotPort},
};
struct Source(Arc<LocatedEndpoint>);
struct NoSnapshot;
#[async_trait]
impl SnapshotPort for NoSnapshot {
    async fn snapshot(&mut self, _: &StreamCursor) -> Result<Snapshot, ClientError> {
        Err(ClientError::Protocol)
    }
}
#[async_trait]
impl EventSource for Source {
    async fn connect(&self, cursor: &StreamCursor) -> Result<Box<dyn EventSocket>, ClientError> {
        Ok(Box::new(
            WebSocketConnection::connect(self.0.clone(), Limits::default(), vec![cursor.clone()])
                .await?,
        ))
    }
    fn snapshot_port(&self) -> Box<dyn SnapshotPort> {
        Box::new(NoSnapshot)
    }
}
async fn peer() -> Peer {
    Peer::spawn_multi(vec![
        Action::Reply(
            200,
            json!({"ticket":"private-sentinel","expiresAt":"2026-09-29T00:00:30Z"}),
        ),
        Action::WebSocket(vec![
            Message::Text(json!({"type":"hello","protocolVersion":1,"epoch":"e"}).to_string()),
            Message::Text(
                serde_json::to_string(&workbench_protocol::events::EventFrame::Event {
                    event: event(),
                })
                .unwrap(),
            ),
        ]),
    ])
    .await
}
#[tokio::test]
async fn successful_jsonl_flush_then_immediate_stop_preserves_session_ack_and_end_cursor() {
    let mut peer = peer().await;
    let stop = Arc::new(Notify::new());
    let state = Arc::new(Mutex::new(State {
        flushed: Some((2, stop.clone())),
        ..State::default()
    }));
    let out = sink(state.clone());
    let mut session = EventSession::new(
        cursor(0),
        Arc::new(Source(peer.endpoint.clone())),
        Limits::default(),
    )
    .unwrap();
    session.subscribe(0, Box::new(out.consumer())).unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        session.run(stop.notified()),
    )
    .await
    .unwrap();
    assert_eq!(result.cursor, cursor(1));
    assert!(matches!(result.result, Err(ClientError::Cancelled)));
    assert!(result.cleanup_error.is_none());
    out.finish(
        &result.cursor,
        Some(&aw_cli::infrastructure::output::CliError::from_client(
            ClientError::Cancelled,
        )),
    )
    .await
    .unwrap();
    peer.settled().await;
    let state = state.lock().unwrap();
    let lines = state
        .bytes
        .split(|b| *b == b'\n')
        .filter(|b| !b.is_empty())
        .map(|b| serde_json::from_slice::<serde_json::Value>(b).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1]["event"]["sequence"], 1);
    assert_eq!(lines[2]["cursor"]["afterSequence"], 1);
    assert_eq!(lines[2]["error"]["code"], "cancelled");
}
#[tokio::test]
async fn stop_during_actual_session_partial_jsonl_write_aborts_callback_and_keeps_ack_zero() {
    let mut peer = peer().await;
    let stop = Arc::new(Notify::new());
    let expected_open = serde_json::to_vec(&json!({"type":"stream.open","cursor":cursor(0)}))
        .unwrap()
        .len()
        + 1;
    let state = Arc::new(Mutex::new(State {
        hold_at: Some(expected_open + 7),
        writing_pending: Some(stop.clone()),
        ..State::default()
    }));
    let out = sink(state.clone());
    let mut session = EventSession::new(
        cursor(0),
        Arc::new(Source(peer.endpoint.clone())),
        Limits::default(),
    )
    .unwrap();
    session.subscribe(0, Box::new(out.consumer())).unwrap();
    let progress = session.progress();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        session.run(stop.notified()),
    )
    .await
    .unwrap();
    assert_eq!(result.cursor, cursor(0));
    assert!(result.cleanup_error.is_none());
    assert!(out.opened());
    assert!(out.failed());
    assert!(out.finish(&result.cursor, None).await.is_err());
    peer.settled().await;
    assert_eq!(progress.queue_usage().unwrap(), (0, 0));
    let state = state.lock().unwrap();
    assert_eq!(state.bytes.len(), expected_open + 7);
    assert_eq!(state.bytes.iter().filter(|b| **b == b'\n').count(), 1);
}

fn recovery_gap(after: u64) -> workbench_protocol::workbench::GapNotice {
    workbench_protocol::workbench::GapNotice {
        stream_id: cursor(0).stream_id,
        epoch: "e".into(),
        reason: workbench_protocol::workbench::GapReason::RetentionExceeded,
        first_sequence: Some(after),
        last_sequence: Some(after),
    }
}
fn release_flush(state: &Arc<Mutex<State>>) {
    let waker = {
        let mut state = state.lock().unwrap();
        state.hold_flush = false;
        state.flush_waker.take().unwrap()
    };
    waker.wake();
}
#[tokio::test]
async fn successful_old_generation_jsonl_delivery_completion_cannot_ack_after_new_gap() {
    use workbench_client::application::events::{spawn_delivery, EventRecovery};
    let pending = Arc::new(Notify::new());
    let state = Arc::new(Mutex::new(State::default()));
    let out = sink(state.clone());
    let mut consumer = out.consumer();
    consumer.opened(&cursor(0)).await.unwrap();
    {
        let mut state = state.lock().unwrap();
        state.hold_flush = true;
        state.flush_pending = Some(pending.clone());
    }
    let mut model = EventRecovery::new(cursor(0), Limits::default()).unwrap();
    let id = model.register(0).unwrap();
    model.hello(1, "e").unwrap();
    model.receive(event()).unwrap();
    let work = model.next(id).unwrap().unwrap();
    let job = spawn_delivery(work, Box::new(consumer), &Limits::default());
    tokio::time::timeout(std::time::Duration::from_secs(1), pending.notified())
        .await
        .unwrap();
    assert_eq!(model.cursor(), cursor(0));
    assert_eq!(state.lock().unwrap().bytes.last(), Some(&b'\n'));
    model.start_gap(recovery_gap(5)).unwrap();
    release_flush(&state);
    let (_, completion) = job.join().await.unwrap();
    assert!(!out.failed());
    assert!(matches!(
        model.complete_delivery(completion),
        Err(ClientError::StaleGeneration)
    ));
    assert_eq!(model.cursor(), cursor(0));
    assert_eq!(model.reconnect_cursor(), cursor(5));
}
#[tokio::test]
async fn successful_old_generation_jsonl_reset_completion_cannot_advance_after_new_gap() {
    use workbench_client::application::events::{spawn_reset, EventRecovery};
    let pending = Arc::new(Notify::new());
    let state = Arc::new(Mutex::new(State::default()));
    let out = sink(state.clone());
    let mut consumer = out.consumer();
    consumer.opened(&cursor(0)).await.unwrap();
    {
        let mut state = state.lock().unwrap();
        state.hold_flush = true;
        state.flush_pending = Some(pending.clone());
    }
    let mut model = EventRecovery::new(cursor(0), Limits::default()).unwrap();
    model.register(0).unwrap();
    model.start_gap(recovery_gap(5)).unwrap();
    let load = model.hello(1, "e").unwrap().unwrap();
    let work = model
        .snapshot_loaded(
            load,
            Snapshot {
                cursor: cursor(5),
                value: json!({"revision":7}),
            },
        )
        .unwrap()
        .pop()
        .unwrap();
    let job = spawn_reset(work, Box::new(consumer), &Limits::default());
    tokio::time::timeout(std::time::Duration::from_secs(1), pending.notified())
        .await
        .unwrap();
    assert_eq!(model.cursor(), cursor(0));
    model.start_gap(recovery_gap(8)).unwrap();
    release_flush(&state);
    let (_, completion) = job.join().await.unwrap();
    assert!(!out.failed());
    assert!(matches!(
        model.complete_reset(completion),
        Err(ClientError::StaleGeneration)
    ));
    assert_eq!(model.cursor(), cursor(0));
    assert_eq!(model.reconnect_cursor(), cursor(8));
}

struct ChunkWriter(Arc<Mutex<State>>);
impl AsyncWrite for ChunkWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.0.lock().unwrap().bytes.extend_from_slice(bytes);
        Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.0.lock().unwrap().flushes += 1;
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
#[tokio::test]
async fn event_reset_end_records_include_normalization_large_cursors_wrappers_and_newlines() {
    use workbench_client::{
        application::events::EventRecovery,
        domain::limits::{LimitConfig, Resource},
    };
    use workbench_protocol::workbench::{GapNotice, GapReason};
    let config = LimitConfig {
        input_bytes: 1024,
        body_bytes: 1024,
        snapshot_bytes: 24 * 1024,
        queue_bytes: 24 * 1024,
        jsonl_record_bytes: 154 * 1024,
        ..Default::default()
    };
    let limits = Limits::new(config).unwrap();
    let mut start = cursor(u64::MAX);
    let overhead = serde_json::to_vec(&start).unwrap().len();
    start
        .stream_id
        .push_str(&"x".repeat(limits.maximum(Resource::Input) - overhead));
    start.after_sequence = 0;
    limits.check_cursor(&start).unwrap();
    let state = Arc::new(Mutex::new(State::default()));
    let output = JsonlOutput::new(ChunkWriter(state.clone()), limits.clone());
    let mut consumer = output.consumer();
    let mut model = EventRecovery::new(start.clone(), limits.clone()).unwrap();
    let id = model.register(0).unwrap();
    model.hello(1, "e").unwrap();
    consumer.opened(&start).await.unwrap();
    let mut envelope = event();
    envelope.stream_id = start.stream_id.clone();
    envelope.body = json!({"padding":""});
    let overhead = serde_json::to_vec(&envelope).unwrap().len();
    envelope.body["padding"] = json!("x".repeat(limits.maximum(Resource::QueueBytes) - overhead));
    assert_eq!(
        serde_json::to_vec(&envelope).unwrap().len(),
        limits.maximum(Resource::QueueBytes)
    );
    model.receive(envelope).unwrap();
    let delivery = model.next(id).unwrap().unwrap();
    consumer.consume(&delivery.event).await.unwrap();
    model.ack(delivery).unwrap();
    assert_eq!(model.cursor().after_sequence, 1);
    let raw = format!(
        "[{}]",
        std::iter::repeat_n("1e10", 150)
            .collect::<Vec<_>>()
            .join(",")
    );
    assert!(raw.len() < limits.maximum(Resource::Body));
    let numbers: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(serde_json::to_vec(&numbers).unwrap().len() > limits.maximum(Resource::Body));
    let mut value = json!({"numbers":numbers,"padding":""});
    let overhead = serde_json::to_vec(&value).unwrap().len();
    value["padding"] = json!("x".repeat(limits.maximum(Resource::Snapshot) - overhead));
    assert_eq!(
        serde_json::to_vec(&value).unwrap().len(),
        limits.maximum(Resource::Snapshot)
    );
    model
        .start_gap(GapNotice {
            stream_id: start.stream_id.clone(),
            epoch: "e".into(),
            reason: GapReason::RetentionExceeded,
            first_sequence: Some(6),
            last_sequence: Some(6),
        })
        .unwrap();
    let load = model.hello(1, "e").unwrap().unwrap();
    let applied = model.cursor();
    let snapshot = Snapshot {
        cursor: StreamCursor {
            after_sequence: 6,
            ..start.clone()
        },
        value,
    };
    let reset = model
        .snapshot_loaded(
            load,
            Snapshot {
                cursor: snapshot.cursor.clone(),
                value: snapshot.value.clone(),
            },
        )
        .unwrap()
        .pop()
        .unwrap();
    consumer.reset_from(&snapshot, &applied).await.unwrap();
    model.reset_applied(reset).unwrap();
    assert_eq!(model.cursor().after_sequence, 6);
    let error = aw_cli::infrastructure::output::CliError::new(
        "internal",
        1,
        workbench_protocol::Outcome::Unknown,
        false,
    );
    output.finish(&model.cursor(), Some(&error)).await.unwrap();
    let bytes = &state.lock().unwrap().bytes;
    let records = bytes
        .split_inclusive(|byte| *byte == b'\n')
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 4);
    for record in &records {
        assert_eq!(record.last(), Some(&b'\n'));
        assert!(record.len() <= limits.maximum(Resource::JsonlRecord));
    }
    assert!(records[1].len() > limits.maximum(Resource::QueueBytes));
    assert!(records[2].len() > limits.maximum(Resource::Snapshot));
    let parsed = records
        .iter()
        .map(|record| serde_json::from_slice::<serde_json::Value>(record).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        parsed
            .iter()
            .map(|v| v["type"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["stream.open", "event", "stream.reset", "stream.end"]
    );
    assert_eq!(parsed[3]["cursor"]["afterSequence"], 6);
    assert_eq!(parsed[3]["error"]["code"], "internal");
    assert_eq!(parsed[3]["ok"], false);
}
