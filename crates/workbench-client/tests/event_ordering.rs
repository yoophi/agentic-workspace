#[path = "support/harness.rs"]
mod harness;
mod support;
use async_trait::async_trait;
use futures_util::FutureExt;
use harness::{HarnessConnection, OwnedTask};
use serde_json::json;
use std::sync::{Arc, Mutex};
use support::{Action, Peer};
use tokio::sync::Notify;
use tokio_tungstenite::tungstenite::Message;
use workbench_client::{
    application::events::session::EventSession,
    domain::limits::Limits,
    infrastructure::{locator::LocatedEndpoint, websocket::WebSocketConnection},
    ports::{ClientError, EventConsumer, EventSocket, EventSource, Snapshot, SnapshotPort},
};
use workbench_protocol::{
    events::EventFrame,
    workbench::{EventEnvelope, StreamCursor},
    CallRequest, OperationId,
};
fn cursor(after: u64) -> StreamCursor {
    StreamCursor {
        stream_id: "orchestration:binding".into(),
        epoch: "e".into(),
        after_sequence: after,
    }
}
fn event(sequence: u64, revision: u64, reason: &str) -> EventEnvelope {
    EventEnvelope {
        event_id: format!("id-{sequence}"),
        stream_id: cursor(0).stream_id,
        epoch: "e".into(),
        sequence,
        schema: "orchestration.workspaceUpdated.v1".into(),
        occurred_at: "2026-09-29T00:00:00Z".into(),
        correlation_id: None,
        body: json!({"workspaceId":"workspace","revision":revision,"reason":reason}),
    }
}
fn frame(event: EventEnvelope) -> Message {
    Message::Text(serde_json::to_string(&EventFrame::Event { event }).unwrap())
}
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
struct Consumer {
    records: Arc<Mutex<Vec<EventEnvelope>>>,
    bootstrap: Arc<Notify>,
    targets: Arc<Notify>,
}
#[async_trait]
impl EventConsumer for Consumer {
    async fn consume(&mut self, event: &EventEnvelope) -> Result<(), ClientError> {
        self.records.lock().unwrap().push(event.clone());
        if event.sequence == 7 {
            self.bootstrap.notify_one();
        }
        if event.sequence == 9 {
            self.targets.notify_one();
        }
        Ok(())
    }
    async fn reset(&mut self, _: &Snapshot) -> Result<(), ClientError> {
        Err(ClientError::Protocol)
    }
}
async fn ordered(events_first: bool) {
    let mut job = OwnedTask::default();
    let mut call = OwnedTask::default();
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let events = Arc::new(Notify::new());
        let reply = Arc::new(Notify::new());
        let snapshot = json!({"workspaceId":"workspace","revision":41,"currentRunId":null});
        let mut peer = Peer::spawn_multi(vec![
            Action::Reply(
                200,
                json!({"ticket":"private-ticket","expiresAt":"2026-09-29T00:00:30Z"}),
            ),
            Action::WebSocketGateConcurrent {
                before: vec![
                    Message::Text(
                        json!({"type":"hello","protocolVersion":1,"epoch":"e"}).to_string(),
                    ),
                    frame(event(7, 40, "bootstrap")),
                ],
                gate: events.clone(),
                after: vec![
                    frame(event(8, 41, "runtimeReconciled")),
                    frame(event(9, 41, "notificationRecovery")),
                ],
            },
            Action::ReplyGate {
                status: 200,
                body: json!({"kind":"complete","output":snapshot,"revision":41}),
                gate: reply.clone(),
            },
            Action::Reply(
                200,
                json!({"kind":"complete","output":snapshot,"revision":41}),
            ),
        ])
        .await;
        let records = Arc::new(Mutex::new(Vec::new()));
        let bootstrap = Arc::new(Notify::new());
        let targets = Arc::new(Notify::new());
        let stop = Arc::new(Notify::new());
        let mut session = EventSession::new(
            cursor(6),
            Arc::new(Source(peer.endpoint.clone())),
            Limits::default(),
        )
        .unwrap();
        session
            .subscribe(
                6,
                Box::new(Consumer {
                    records: records.clone(),
                    bootstrap: bootstrap.clone(),
                    targets: targets.clone(),
                }),
            )
            .unwrap();
        let progress = session.progress();
        let stopped = stop.clone();
        job.start(async move { session.run(stopped.notified()).await });
        bootstrap.notified().await;
        // The bootstrap callback is acknowledged before the recover request starts.
        while progress.cursor().unwrap() != cursor(7) {
            tokio::task::yield_now().await;
        }
        let mut http = HarnessConnection::connect(peer.endpoint.clone()).await;
        // Test-only transport authority; production admission remains closed.
        call.start(async move {
            let result = http
                .post(
                    "/v1/calls",
                    serde_json::to_value(CallRequest::command(
                        OperationId::OrchestrationRecover,
                        json!({"benchId":"bench"}),
                    ))
                    .unwrap(),
                    true,
                )
                .await;
            (http, result)
        });
        // Both orders begin only after the peer physically receives this recover.
        loop {
            let notified = peer.received.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if peer.requests.lock().unwrap().iter().any(|(path, _, body)| {
                path == "/v1/calls" && body["operation"] == "orchestration.recover"
            }) {
                break;
            }
            notified.await;
        }
        assert_eq!(records.lock().unwrap().len(), 1);
        assert!(!call.is_finished()); // Request received; reply and event gates are both closed.
        if events_first {
            events.notify_one();
            targets.notified().await;
            while progress.cursor().unwrap() != cursor(9) {
                tokio::task::yield_now().await;
            }
            assert!(!call.is_finished()); // HTTP response cannot pass the unreleased gate.
            reply.notify_one();
        } else {
            reply.notify_one();
        }
        let (mut http, result) = call.wait().await;
        assert_eq!(result["revision"], 41);
        assert_eq!(result["output"], snapshot);
        if !events_first {
            assert_eq!(records.lock().unwrap().len(), 1); // Event gate is still closed.
            assert_eq!(progress.cursor().unwrap(), cursor(7));
            events.notify_one();
            targets.notified().await;
        }
        while progress.cursor().unwrap() != cursor(9) {
            tokio::task::yield_now().await;
        }
        let final_snapshot = http
            .post(
                "/v1/calls",
                serde_json::to_value(CallRequest::query(
                    OperationId::OrchestrationGet,
                    json!({"benchId":"bench"}),
                ))
                .unwrap(),
                true,
            )
            .await;
        assert_eq!(final_snapshot["output"], snapshot);
        assert_eq!(
            *records.lock().unwrap(),
            vec![
                event(7, 40, "bootstrap"),
                event(8, 41, "runtimeReconciled"),
                event(9, 41, "notificationRecovery")
            ]
        );
        http.close().await;
        stop.notify_one();
        let finished = job.wait().await;
        assert_eq!(finished.cursor, cursor(9));
        assert!(finished.cleanup_error.is_none());
        peer.settled().await;
        let requests = peer.requests.lock().unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|(path, _, _)| path == "/v1/calls")
                .count(),
            2
        );
        assert_eq!(peer.effects.lock().unwrap().len(), 1); // Exactly one recover identity, no retry.
    });
    let outcome = std::panic::AssertUnwindSafe(outcome).catch_unwind().await;
    let call_cleanup = call.cancel_join().await;
    let session_cleanup = job.cancel_join().await;
    assert!(call_cleanup.is_ok(), "{call_cleanup:?}");
    assert!(session_cleanup.is_ok(), "{session_cleanup:?}");
    match outcome {
        Ok(result) => result.expect("ordering fixture deadline after owned abort/join"),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}
#[tokio::test]
async fn reply_before_events_barrier_preserves_both_same_revision_sequences() {
    ordered(false).await;
}
#[tokio::test]
async fn events_before_reply_barrier_preserves_both_same_revision_sequences() {
    ordered(true).await;
}
