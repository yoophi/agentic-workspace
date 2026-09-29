mod support;
use aw_cli::infrastructure::event_source::HttpEventSource;
use serde_json::{json, Value};
use support::peer::{Action, Peer};
use workbench_client::{
    domain::limits::Limits,
    ports::{ClientError, EventSource},
};
use workbench_protocol::workbench::StreamCursor;
fn cursor(after: u64) -> StreamCursor {
    StreamCursor {
        stream_id: "orchestration:binding".into(),
        epoch: "e".into(),
        after_sequence: after,
    }
}
fn bench(id: &str) -> Value {
    json!({"benchId":id,"workingDirectory":"/private/fixture","owner":"owner","runs":[]})
}
fn session(stream: &str) -> Value {
    json!({"schemaVersion":1,"id":"workspace-not-binding","worktreePath":"/private/fixture","eventStreamId":stream,"mainNodeId":"main","activeCoordinatorGenerationId":null,"nodes":[],"generations":[],"tasks":[],"reports":[],"commands":[],"coordinatorNotifications":[],"dispatches":[],"idempotencyRecords":[],"revision":99,"createdAt":"2026-09-29T00:00:00Z","updatedAt":"2026-09-29T00:00:00Z"})
}
fn complete(output: Value) -> Action {
    Action::Reply(
        200,
        json!({"kind":"complete","output":output,"replayed":false}),
    )
}
#[tokio::test]
async fn resolves_binding_by_server_dto_and_keeps_boundary_separate_from_revision() {
    let mut peer = Peer::spawn_multi(vec![
        complete(json!([bench("different-bench"), bench("actual-bench")])),
        complete(session("orchestration:other-binding")),
        complete(session("orchestration:binding")),
    ])
    .await;
    let source = HttpEventSource::new(peer.endpoint.clone(), Limits::default());
    let snapshot = source
        .snapshot_port()
        .snapshot_after(&cursor(0), &cursor(5))
        .await
        .unwrap();
    assert_eq!(snapshot.cursor, cursor(5));
    assert_eq!(snapshot.value["revision"], 99);
    assert_eq!(snapshot.value["id"], "workspace-not-binding");
    peer.stop().await;
    let calls = peer
        .requests
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r.0 == "/v1/calls")
        .map(|r| r.2.clone())
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0]["operation"], "bench.list");
    assert_eq!(calls[1]["input"]["benchId"], "different-bench");
    assert_eq!(calls[2]["input"]["benchId"], "actual-bench");
    assert!(calls.iter().all(|r| r["idempotencyKey"].is_null()));
}
#[tokio::test]
async fn mismatched_cursor_and_gated_stream_snapshots_transmit_nothing() {
    let mut peer = Peer::spawn_multi(vec![]).await;
    let source = HttpEventSource::new(peer.endpoint.clone(), Limits::default());
    let bad = StreamCursor {
        epoch: "other-epoch".into(),
        ..cursor(0)
    };
    assert!(matches!(
        source.snapshot_port().snapshot_after(&bad, &bad).await,
        Err(ClientError::Incompatible)
    ));
    let bad = StreamCursor {
        stream_id: "orchestration:old-binding".into(),
        ..cursor(0)
    };
    assert!(matches!(
        source
            .snapshot_port()
            .snapshot_after(&bad, &cursor(5))
            .await,
        Err(ClientError::Incompatible)
    ));
    let gated = StreamCursor {
        stream_id: "run:existing".into(),
        ..cursor(0)
    };
    assert!(matches!(
        source.snapshot_port().snapshot(&gated).await,
        Err(ClientError::PrerequisiteUnavailable)
    ));
    assert!(peer.requests.lock().unwrap().is_empty());
    peer.stop().await;
}
#[tokio::test]
async fn missing_duplicate_or_malformed_binding_is_not_a_successful_snapshot() {
    for (index, actions) in [
        vec![
            complete(json!([bench("bench")])),
            complete(session("orchestration:other-binding")),
        ],
        vec![
            complete(json!([bench("first"), bench("second")])),
            complete(session("orchestration:binding")),
            complete(session("orchestration:binding")),
        ],
        vec![
            complete(json!([bench("bench")])),
            complete(json!({"eventStreamId":"orchestration:binding"})),
        ],
    ]
    .into_iter()
    .enumerate()
    {
        let mut peer = Peer::spawn_multi(actions).await;
        let source = HttpEventSource::new(peer.endpoint.clone(), Limits::default());
        let result = source
            .snapshot_port()
            .snapshot_after(&cursor(0), &cursor(5))
            .await;
        if index == 0 {
            assert!(matches!(result, Err(ClientError::Unavailable)));
        } else {
            assert!(matches!(result, Err(ClientError::Protocol)));
        }
        peer.stop().await;
        assert!(peer
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.0 == "/v1/calls")
            .all(|r| r.2["operation"] == "bench.list" || r.2["operation"] == "orchestration.get"));
    }
}
#[tokio::test]
async fn notification_reset_explicitly_marks_unretained_signals_without_inventing_title_state() {
    let mut peer = Peer::spawn_multi(vec![complete(json!([bench("existing")]))]).await;
    let source = HttpEventSource::new(peer.endpoint.clone(), Limits::default());
    let applied = StreamCursor {
        stream_id: "bench:existing".into(),
        ..cursor(1)
    };
    let live = StreamCursor {
        after_sequence: 3,
        ..applied.clone()
    };
    let snapshot = source
        .snapshot_port()
        .snapshot_after(&applied, &live)
        .await
        .unwrap();
    assert_eq!(snapshot.cursor, live);
    assert_eq!(snapshot.value["notificationsRetained"], false);
    assert!(snapshot.value.get("title").is_none());
    assert_eq!(snapshot.value["bench"]["benchId"], "existing");
    peer.stop().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 3);
}
