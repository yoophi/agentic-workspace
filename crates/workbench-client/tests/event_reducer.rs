use serde_json::json;
use workbench_client::{
    domain::{
        events::EventReducer,
        limits::{LimitConfig, Limits},
    },
    ports::ClientError,
};
use workbench_protocol::workbench::{EventEnvelope, StreamCursor};
fn cursor(stream: &str, epoch: &str, after: u64) -> StreamCursor {
    StreamCursor {
        stream_id: stream.into(),
        epoch: epoch.into(),
        after_sequence: after,
    }
}
fn event(stream: &str, epoch: &str, sequence: u64, revision: u64) -> EventEnvelope {
    EventEnvelope {
        event_id: format!("e{sequence}"),
        stream_id: stream.into(),
        epoch: epoch.into(),
        sequence,
        schema: "orchestration.workspaceUpdated.v1".into(),
        occurred_at: "2026-09-29T00:00:00Z".into(),
        correlation_id: None,
        body: json!({"revision":revision,"private":"private-sentinel"}),
    }
}
fn reducer() -> EventReducer {
    EventReducer::new(
        cursor("orchestration:binding-a", "epoch-a", 0),
        Limits::default(),
    )
    .unwrap()
}
#[test]
fn received_never_advances_applied_and_all_consumers_contribute_minimum() {
    let mut r = reducer();
    let a = r.register(0).unwrap();
    let b = r.register(0).unwrap();
    r.receive(event("orchestration:binding-a", "epoch-a", 1, 4))
        .unwrap();
    assert_eq!(r.received(), 1);
    assert_eq!(r.cursor().after_sequence, 0);
    let first = r.next(a).unwrap().unwrap();
    assert!(r.next(a).unwrap().is_none());
    r.ack(first).unwrap();
    assert_eq!(r.cursor().after_sequence, 0);
    {
        let pending = r.next(b).unwrap().unwrap();
        r.ack(pending).unwrap();
    }
    assert_eq!(r.cursor().after_sequence, 1);
}
#[test]
fn same_revision_different_sequence_requires_two_distinct_acks() {
    let mut r = reducer();
    let a = r.register(0).unwrap();
    for s in [1, 2] {
        r.receive(event("orchestration:binding-a", "epoch-a", s, 5))
            .unwrap();
    }
    let first = r.next(a).unwrap().unwrap();
    assert_eq!(first.event.sequence, 1);
    r.ack(first).unwrap();
    assert_eq!(r.cursor().after_sequence, 1);
    let second = r.next(a).unwrap().unwrap();
    assert_eq!(second.event.sequence, 2);
    r.ack(second).unwrap();
    assert_eq!(r.cursor().after_sequence, 2);
}
#[test]
fn failure_resets_only_its_consumer_and_old_completion_cannot_mutate_new_queue() {
    let mut r = reducer();
    let a = r.register(0).unwrap();
    let b = r.register(0).unwrap();
    r.receive(event("orchestration:binding-a", "epoch-a", 1, 1))
        .unwrap();
    let old = r.next(a).unwrap().unwrap();
    let reset = r.begin_reset(a).unwrap();
    {
        let pending = r.next(b).unwrap().unwrap();
        r.ack(pending).unwrap();
    }
    assert_eq!(r.cursor().after_sequence, 0);
    r.receive(event("orchestration:binding-a", "epoch-a", 2, 2))
        .unwrap();
    r.finish_reset(reset, &cursor("orchestration:binding-a", "epoch-a", 1))
        .unwrap();
    assert!(matches!(r.ack(old), Err(ClientError::StaleGeneration)));
    let new = r.next(a).unwrap().unwrap();
    assert_eq!(new.event.sequence, 2);
    r.ack(new).unwrap();
    {
        let pending = r.next(b).unwrap().unwrap();
        r.ack(pending).unwrap();
    }
    assert_eq!(r.cursor().after_sequence, 2);
}
#[test]
fn unregister_during_pending_delivery_preserves_unapplied_backlog_and_invalidates_ack() {
    let mut r = reducer();
    let a = r.register(0).unwrap();
    r.receive(event("orchestration:binding-a", "epoch-a", 1, 1))
        .unwrap();
    let old = r.next(a).unwrap().unwrap();
    r.unregister(a).unwrap();
    assert_eq!(r.cursor().after_sequence, 0);
    let b = r.register(0).unwrap();
    assert!(matches!(r.ack(old), Err(ClientError::StaleGeneration)));
    {
        let pending = r.next(b).unwrap().unwrap();
        r.ack(pending).unwrap();
    }
    assert_eq!(r.cursor().after_sequence, 1);
}
#[test]
fn new_epoch_and_binding_replacement_isolate_old_delivery_and_reset_both_directions() {
    let mut r = reducer();
    let a = r.register(0).unwrap();
    r.receive(event("orchestration:binding-a", "epoch-a", 1, 1))
        .unwrap();
    let old = r.next(a).unwrap().unwrap();
    let reset = r.begin_reset(a).unwrap();
    r.rebind(cursor("orchestration:binding-b", "epoch-b", 0))
        .unwrap();
    assert!(matches!(r.ack(old), Err(ClientError::StaleGeneration)));
    assert!(matches!(
        r.finish_reset(reset, &cursor("orchestration:binding-a", "epoch-a", 5)),
        Err(ClientError::StaleGeneration)
    ));
    assert!(matches!(
        r.receive(event("orchestration:binding-a", "epoch-a", 2, 2)),
        Err(ClientError::Protocol)
    ));
    r.receive(event("orchestration:binding-b", "epoch-b", 1, 3))
        .unwrap();
    {
        let pending = r.next(a).unwrap().unwrap();
        r.ack(pending).unwrap();
    }
    assert_eq!(r.cursor(), cursor("orchestration:binding-b", "epoch-b", 1));
}
#[test]
fn newer_reset_wins_and_snapshot_covered_events_never_redeliver() {
    let mut r = reducer();
    let a = r.register(0).unwrap();
    let old = r.begin_reset(a).unwrap();
    let new = r.begin_reset(a).unwrap();
    r.finish_reset(new, &cursor("orchestration:binding-a", "epoch-a", 3))
        .unwrap();
    assert!(matches!(
        r.finish_reset(old, &cursor("orchestration:binding-a", "epoch-a", 1)),
        Err(ClientError::StaleGeneration)
    ));
    r.receive(event("orchestration:binding-a", "epoch-a", 3, 1))
        .unwrap();
    assert!(r.next(a).unwrap().is_none());
    r.receive(event("orchestration:binding-a", "epoch-a", 4, 1))
        .unwrap();
    {
        let pending = r.next(a).unwrap().unwrap();
        r.ack(pending).unwrap();
    }
    assert_eq!(r.cursor().after_sequence, 4);
}
#[test]
fn queue_pressure_is_error_before_any_partial_retention() {
    let limits = Limits::new(LimitConfig {
        queue_items: 1,
        ..Default::default()
    })
    .unwrap();
    let mut r = EventReducer::new(cursor("orchestration:b", "e", 0), limits).unwrap();
    let a = r.register(0).unwrap();
    r.receive(event("orchestration:b", "e", 1, 1)).unwrap();
    assert!(matches!(
        r.receive(event("orchestration:b", "e", 2, 2)),
        Err(ClientError::Limit(_))
    ));
    assert_eq!(r.received(), 1);
    {
        let pending = r.next(a).unwrap().unwrap();
        r.ack(pending).unwrap();
    }
    assert!(r.next(a).unwrap().is_none());
}
#[test]
fn earlier_join_requests_replay_and_other_consumers_do_not_redeliver() {
    let mut r = reducer();
    let a = r.register(0).unwrap();
    r.receive(event("orchestration:binding-a", "epoch-a", 1, 1))
        .unwrap();
    {
        let pending = r.next(a).unwrap().unwrap();
        r.ack(pending).unwrap();
    }
    let b = r.register(0).unwrap();
    assert!(r.requires_replay());
    r.receive(event("orchestration:binding-a", "epoch-a", 1, 1))
        .unwrap();
    assert!(r.next(a).unwrap().is_none());
    {
        let pending = r.next(b).unwrap().unwrap();
        r.ack(pending).unwrap();
    }
    assert_eq!(r.cursor().after_sequence, 1);
}

#[test]
fn consumer_failure_only_blocks_itself_until_snapshot_ack() {
    let mut r = reducer();
    let a = r.register(0).unwrap();
    let b = r.register(0).unwrap();
    r.receive(event("orchestration:binding-a", "epoch-a", 1, 1))
        .unwrap();
    let pending = r.next(a).unwrap().unwrap();
    let reset = r.fail(pending).unwrap();
    assert!(r.next(a).unwrap().is_none());
    let pending = r.next(b).unwrap().unwrap();
    r.ack(pending).unwrap();
    assert_eq!(r.cursor().after_sequence, 0);
    r.finish_reset(reset, &cursor("orchestration:binding-a", "epoch-a", 1))
        .unwrap();
    assert_eq!(r.cursor().after_sequence, 1);
}
#[test]
fn tokens_from_another_reducer_cannot_complete_even_with_equal_cursor_and_generation() {
    let mut a = reducer();
    let mut b = reducer();
    let ca = a.register(0).unwrap();
    let cb = b.register(0).unwrap();
    for r in [&mut a, &mut b] {
        r.receive(event("orchestration:binding-a", "epoch-a", 1, 1))
            .unwrap();
    }
    let wrong = a.next(ca).unwrap().unwrap();
    let right = b.next(cb).unwrap().unwrap();
    assert!(matches!(b.ack(wrong), Err(ClientError::StaleGeneration)));
    assert_eq!(b.cursor().after_sequence, 0);
    b.ack(right).unwrap();
    let reset = a.begin_reset(ca).unwrap();
    b.begin_reset(cb).unwrap();
    assert!(matches!(
        b.finish_reset(reset, &cursor("orchestration:binding-a", "epoch-a", 9)),
        Err(ClientError::StaleGeneration)
    ));
}
#[test]
fn notification_streams_require_reconnect_snapshot_and_retained_streams_replay() {
    let signal =
        EventReducer::new(cursor("worktree:/private/x", "e", 0), Limits::default()).unwrap();
    assert!(signal.snapshot_on_reconnect());
    let retained = reducer();
    assert!(!retained.snapshot_on_reconnect());
}
#[test]
fn consumer_ids_are_bound_to_their_reducer_and_wrong_handles_mutate_nothing() {
    let mut a = reducer();
    let mut b = reducer();
    let wrong = a.register(0).unwrap();
    let own = b.register(0).unwrap();
    b.receive(event("orchestration:binding-a", "epoch-a", 1, 1))
        .unwrap();
    assert!(matches!(b.next(wrong), Err(ClientError::StaleGeneration)));
    assert!(matches!(
        b.begin_reset(wrong),
        Err(ClientError::StaleGeneration)
    ));
    assert!(matches!(
        b.unregister(wrong),
        Err(ClientError::StaleGeneration)
    ));
    assert_eq!(b.cursor().after_sequence, 0);
    let pending = b.next(own).unwrap().unwrap();
    b.ack(pending).unwrap();
    assert_eq!(b.cursor().after_sequence, 1);
}
#[test]
fn earlier_join_live_two_before_replay_one_must_ack_one_then_two() {
    let mut r = reducer();
    let a = r.register(0).unwrap();
    r.receive(event("orchestration:binding-a", "epoch-a", 1, 1))
        .unwrap();
    let pending = r.next(a).unwrap().unwrap();
    r.ack(pending).unwrap();
    let b = r.register(0).unwrap();
    r.receive(event("orchestration:binding-a", "epoch-a", 2, 2))
        .unwrap();
    assert!(r.next(b).unwrap().is_none());
    assert_eq!(r.cursor().after_sequence, 0);
    r.receive(event("orchestration:binding-a", "epoch-a", 1, 1))
        .unwrap();
    let one = r.next(b).unwrap().unwrap();
    assert_eq!(one.event.sequence, 1);
    r.ack(one).unwrap();
    assert_eq!(r.cursor().after_sequence, 1);
    let two = r.next(b).unwrap().unwrap();
    assert_eq!(two.event.sequence, 2);
    r.ack(two).unwrap();
    let other = r.next(a).unwrap().unwrap();
    r.ack(other).unwrap();
    assert_eq!(r.cursor().after_sequence, 2);
}
