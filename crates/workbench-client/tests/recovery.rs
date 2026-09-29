use serde_json::json;
use workbench_client::{
    application::events::{EventRecovery, RecoveryAction, RecoveryPhase},
    domain::limits::{LimitConfig, Limits},
    ports::{ClientError, Snapshot, SnapshotPort},
};
use workbench_protocol::workbench::{EventEnvelope, GapNotice, GapReason, StreamCursor};
fn cursor(after: u64) -> StreamCursor {
    StreamCursor {
        stream_id: "run:r".into(),
        epoch: "e".into(),
        after_sequence: after,
    }
}
fn gap(after: u64) -> GapNotice {
    GapNotice {
        stream_id: "run:r".into(),
        epoch: "e".into(),
        reason: GapReason::RetentionExceeded,
        first_sequence: Some(after.saturating_sub(1)),
        last_sequence: Some(after),
    }
}
fn event(sequence: u64) -> EventEnvelope {
    EventEnvelope {
        event_id: format!("id-{sequence}"),
        stream_id: "run:r".into(),
        epoch: "e".into(),
        sequence,
        schema: "run.event.v1".into(),
        occurred_at: "2026-09-29T00:00:00Z".into(),
        correlation_id: None,
        body: json!({"private":"private-sentinel"}),
    }
}
fn snapshot(after: u64) -> Snapshot {
    Snapshot {
        cursor: cursor(after),
        value: json!({"lastSequence":after}),
    }
}
fn recovery() -> EventRecovery {
    EventRecovery::new(cursor(0), Limits::default()).unwrap()
}
#[test]
fn live_first_hello_snapshot_reset_filter_then_ack() {
    let mut r = recovery();
    let c = r.register(0).unwrap();
    assert!(matches!(r.start_gap(gap(6)).unwrap(),RecoveryAction::Connect(x) if x==cursor(6)));
    assert_eq!(r.cursor(), cursor(0));
    assert_eq!(r.phase(), RecoveryPhase::LiveBuffering);
    let load = r.hello(1, "e").unwrap().unwrap();
    assert_eq!(load.applied_cursor(), &cursor(0));
    assert_eq!(r.phase(), RecoveryPhase::SnapshotLoading);
    for seq in [7, 8] {
        r.receive(event(seq)).unwrap();
    }
    assert!(r.next(c).unwrap().is_none());
    let mut resets = r.snapshot_loaded(load, snapshot(7)).unwrap();
    assert_eq!(resets.len(), 1);
    let reset = resets.pop().unwrap();
    assert_eq!(reset.applied_cursor(), &cursor(0));
    assert_eq!(r.cursor(), cursor(0));
    r.reset_applied(reset).unwrap();
    assert_eq!(r.phase(), RecoveryPhase::Live);
    assert_eq!(r.cursor(), cursor(7));
    let delivery = r.next(c).unwrap().unwrap();
    assert_eq!(delivery.event.sequence, 8);
    r.ack(delivery).unwrap();
    assert_eq!(r.cursor(), cursor(8));
}
#[test]
fn hello_does_not_reset_budget_and_five_failed_rounds_exhaust() {
    let mut r = recovery();
    r.register(0).unwrap();
    r.start_gap(gap(1)).unwrap();
    for after in 1..=5 {
        let load = r.hello(1, "e").unwrap().unwrap();
        assert!(!r.is_live());
        let action = r.snapshot_failed(load).unwrap();
        if after == 5 {
            assert!(matches!(action, RecoveryAction::Exhausted));
        } else {
            assert!(matches!(action, RecoveryAction::Connect(_)));
        }
    }
    assert_eq!(r.phase(), RecoveryPhase::Exhausted);
    assert!(r.hello(1, "e").is_err());
}
#[test]
fn snapshot_failure_retry_reuses_boundary_and_bounded_backoff() {
    let mut r = recovery();
    r.register(0).unwrap();
    r.start_gap(gap(6)).unwrap();
    let load = r.hello(1, "e").unwrap().unwrap();
    assert!(matches!(r.snapshot_failed(load).unwrap(),RecoveryAction::Connect(c) if c==cursor(6)));
    assert_eq!(r.reconnect_cursor(), cursor(6));
    let delay = r.backoff(500).unwrap();
    assert!(
        delay >= std::time::Duration::from_millis(250)
            && delay <= std::time::Duration::from_secs(10)
    );
    assert!(r.backoff(1001).is_err());
}
#[test]
fn jitter_never_crosses_configured_backoff_bounds() {
    let mut r = recovery();
    r.register(0).unwrap();
    for attempt in 0..5 {
        if attempt > 0 {
            r.start_gap(gap(1)).unwrap();
        }
        for jitter in [0, 500, 1000] {
            let delay = r.backoff(jitter).unwrap();
            assert!(delay >= std::time::Duration::from_millis(250));
            assert!(delay <= std::time::Duration::from_secs(10));
        }
    }
}
#[test]
fn earlier_listener_reopens_ticket_and_live_before_replay_cannot_skip_sequence_one() {
    let mut r = recovery();
    let a = r.register(0).unwrap();
    r.hello(1, "e").unwrap();
    r.receive(event(1)).unwrap();
    let d = r.next(a).unwrap().unwrap();
    r.ack(d).unwrap();
    let b = r.register(0).unwrap();
    assert_eq!(r.take_replay_requirement(), Some(cursor(0)));
    assert_eq!(r.take_replay_requirement(), None);
    r.receive(event(2)).unwrap();
    assert!(r.next(b).unwrap().is_none());
    let d = r.next(a).unwrap().unwrap();
    r.ack(d).unwrap();
    assert_eq!(r.cursor(), cursor(0));
    r.receive(event(1)).unwrap();
    for expected in [1, 2] {
        let d = r.next(b).unwrap().unwrap();
        assert_eq!(d.event.sequence, expected);
        r.ack(d).unwrap();
        assert_eq!(r.cursor(), cursor(expected));
    }
    assert!(r.next(a).unwrap().is_none());
}
#[test]
fn listener_join_during_load_or_reset_uses_full_snapshot_and_own_applied_cursor() {
    for join_after_load in [false, true] {
        let mut r = recovery();
        let a = r.register(2).unwrap();
        r.start_gap(gap(5)).unwrap();
        let mut load = Some(r.hello(1, "e").unwrap().unwrap());
        assert_eq!(load.as_ref().unwrap().applied_cursor(), &cursor(2));
        let mut work = if join_after_load {
            r.snapshot_loaded(load.take().unwrap(), snapshot(5))
                .unwrap()
        } else {
            Vec::new()
        };
        let b = r.register(0).unwrap();
        if join_after_load {
            work.extend(r.pending_resets().unwrap());
        } else {
            assert!(r.hello(1, "e").unwrap().is_none());
            work = r
                .snapshot_loaded(load.take().unwrap(), snapshot(5))
                .unwrap();
        }
        r.receive(event(6)).unwrap();
        let joined = work.iter().find(|w| w.consumer() == b).unwrap();
        assert_eq!(joined.applied_cursor(), &cursor(0));
        for w in work {
            if w.consumer() == a {
                r.reset_applied(w).unwrap();
            } else {
                r.unregister(b).unwrap();
                assert!(matches!(
                    r.reset_applied(w),
                    Err(ClientError::StaleGeneration)
                ));
            }
        }
        assert!(r.is_live());
        let d = r.next(a).unwrap().unwrap();
        assert_eq!(d.event.sequence, 6);
        r.ack(d).unwrap();
        assert_eq!(r.cursor(), cursor(6));
    }
}
#[test]
fn new_gap_and_epoch_invalidate_old_load_and_old_consumer_reset() {
    let mut r = recovery();
    let c = r.register(0).unwrap();
    r.start_gap(gap(5)).unwrap();
    let old_load = r.hello(1, "e").unwrap().unwrap();
    r.start_gap(gap(6)).unwrap();
    assert!(matches!(
        r.snapshot_loaded(old_load, snapshot(5)),
        Err(ClientError::StaleGeneration)
    ));
    let load = r.hello(1, "e").unwrap().unwrap();
    let mut resets = r.snapshot_loaded(load, snapshot(6)).unwrap();
    let old_reset = resets.pop().unwrap();
    let mut newer = gap(0);
    newer.epoch = "e2".into();
    newer.reason = GapReason::EpochChanged;
    assert!(
        matches!(r.start_gap(newer).unwrap(),RecoveryAction::Connect(c) if c.epoch=="e2"&&c.after_sequence==0)
    );
    assert!(matches!(
        r.reset_applied(old_reset),
        Err(ClientError::StaleGeneration)
    ));
    assert_eq!(r.consumer_cursor(c).unwrap().after_sequence, 0);
    assert_eq!(r.reconnect_cursor().epoch, "e2");
}
#[test]
fn reset_failure_removal_and_stale_cross_instance_work_do_not_advance() {
    let mut a = recovery();
    let mut b = recovery();
    let ca = a.register(0).unwrap();
    let cb = b.register(0).unwrap();
    a.start_gap(gap(3)).unwrap();
    b.start_gap(gap(3)).unwrap();
    let la = a.hello(1, "e").unwrap().unwrap();
    let lb = b.hello(1, "e").unwrap().unwrap();
    let mut wa = a.snapshot_loaded(la, snapshot(3)).unwrap();
    let mut wb = b.snapshot_loaded(lb, snapshot(3)).unwrap();
    assert!(matches!(
        b.reset_applied(wa.pop().unwrap()),
        Err(ClientError::StaleGeneration)
    ));
    assert_eq!(b.cursor(), cursor(0));
    b.unregister(cb).unwrap();
    assert!(matches!(
        b.reset_applied(wb.pop().unwrap()),
        Err(ClientError::StaleGeneration)
    ));
    assert_eq!(b.cursor(), cursor(0));
    assert_eq!(a.consumer_cursor(ca).unwrap(), cursor(0));
}
#[test]
fn terminal_eviction_snapshots_once_and_never_reconnects_after_reset() {
    let mut r = recovery();
    r.register(0).unwrap();
    let mut notice = gap(3);
    notice.reason = GapReason::Evicted;
    let action = r.start_gap(notice).unwrap();
    let RecoveryAction::Load(load) = action else {
        panic!("eviction must load without live connect")
    };
    let mut resets = r.snapshot_loaded(load, snapshot(3)).unwrap();
    r.reset_applied(resets.pop().unwrap()).unwrap();
    assert_eq!(r.phase(), RecoveryPhase::Terminal);
    assert!(r.hello(1, "e").is_err());
}
#[tokio::test]
async fn pending_snapshot_new_gap_completion_is_stale_and_new_load_can_finish() {
    struct Pending {
        entered: std::sync::Arc<tokio::sync::Notify>,
        release: std::sync::Arc<tokio::sync::Notify>,
    }
    #[async_trait::async_trait]
    impl SnapshotPort for Pending {
        async fn snapshot(&mut self, _: &StreamCursor) -> Result<Snapshot, ClientError> {
            self.entered.notify_one();
            self.release.notified().await;
            Ok(snapshot(5))
        }
    }
    let mut r = recovery();
    let c = r.register(0).unwrap();
    r.start_gap(gap(5)).unwrap();
    let old = r.hello(1, "e").unwrap().unwrap();
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let mut port = Pending {
        entered: entered.clone(),
        release: release.clone(),
    };
    let requested = old.applied_cursor().clone();
    let job = tokio::spawn(async move { (old, port.snapshot(&requested).await) });
    entered.notified().await;
    r.start_gap(gap(6)).unwrap();
    let new = r.hello(1, "e").unwrap().unwrap();
    let mut work = r.snapshot_loaded(new, snapshot(6)).unwrap();
    r.reset_applied(work.pop().unwrap()).unwrap();
    release.notify_one();
    let (old, result) = tokio::time::timeout(std::time::Duration::from_secs(1), job)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        r.snapshot_loaded(old, result.unwrap()),
        Err(ClientError::StaleGeneration)
    ));
    assert_eq!(r.consumer_cursor(c).unwrap(), cursor(6));
}
#[test]
fn reconnect_during_snapshot_and_reset_keeps_boundary_and_replay_live_order() {
    let mut r = recovery();
    let c = r.register(0).unwrap();
    r.start_gap(gap(5)).unwrap();
    let load = r.hello(1, "e").unwrap().unwrap();
    r.receive(event(7)).unwrap();
    assert_eq!(r.reconnect_cursor(), cursor(5));
    assert!(r.hello(1, "e").unwrap().is_none());
    r.receive(event(6)).unwrap();
    r.receive(event(7)).unwrap();
    let mut resets = r.snapshot_loaded(load, snapshot(5)).unwrap();
    assert_eq!(r.reconnect_cursor(), cursor(5));
    r.reset_applied(resets.pop().unwrap()).unwrap();
    for expected in [6, 7] {
        let d = r.next(c).unwrap().unwrap();
        assert_eq!(d.event.sequence, expected);
        r.ack(d).unwrap();
    }
    assert_eq!(r.cursor(), cursor(7));
    assert!(r.next(c).unwrap().is_none());
}
#[test]
fn buffer_quota_and_wrong_hello_or_snapshot_are_typed_errors() {
    let limits = Limits::new(LimitConfig {
        queue_items: 1,
        ..Default::default()
    })
    .unwrap();
    let mut r = EventRecovery::new(cursor(0), limits).unwrap();
    r.register(0).unwrap();
    r.start_gap(gap(1)).unwrap();
    assert!(matches!(r.hello(99, "e"), Err(ClientError::Incompatible)));
    let load = r.hello(1, "e").unwrap().unwrap();
    r.receive(event(2)).unwrap();
    assert!(matches!(r.receive(event(3)), Err(ClientError::Limit(_))));
    let mut bad = snapshot(1);
    bad.cursor.stream_id = "run:wrong".into();
    assert!(matches!(
        r.snapshot_loaded(load, bad),
        Err(ClientError::Protocol)
    ));
}
#[test]
fn every_new_epoch_gap_reason_invalidates_old_load_failure_and_reset() {
    for reason in [
        GapReason::UnknownStream,
        GapReason::Evicted,
        GapReason::EpochChanged,
        GapReason::RetentionExceeded,
        GapReason::SubscriberLagged,
        GapReason::Shutdown,
    ] {
        for completion in 0..3 {
            let mut r = recovery();
            let c = r.register(0).unwrap();
            r.start_gap(gap(5)).unwrap();
            let old = r.hello(1, "e").unwrap().unwrap();
            let mut old_load = Some(old);
            let mut old_reset = None;
            if completion == 2 {
                old_reset = Some(
                    r.snapshot_loaded(old_load.take().unwrap(), snapshot(5))
                        .unwrap()
                        .pop()
                        .unwrap(),
                );
            }
            let mut notice = gap(9);
            notice.epoch = "e2".into();
            notice.reason = reason;
            let action = r.start_gap(notice).unwrap();
            assert_eq!(r.reconnect_cursor().epoch, "e2");
            assert_eq!(r.reconnect_cursor().after_sequence, 0);
            let phase = r.phase();
            let rejected = match completion {
                0 => r
                    .snapshot_loaded(old_load.take().unwrap(), snapshot(5))
                    .map(|_| ()),
                1 => r.snapshot_failed(old_load.take().unwrap()).map(|_| ()),
                _ => r.reset_applied(old_reset.take().unwrap()),
            };
            assert!(matches!(rejected, Err(ClientError::StaleGeneration)));
            assert_eq!(r.phase(), phase);
            assert_eq!(r.consumer_cursor(c).unwrap().epoch, "e2");
            assert_eq!(r.cursor().after_sequence, 0);
            let new = match action {
                RecoveryAction::Load(load) => load,
                RecoveryAction::Connect(c) => {
                    assert_eq!(c.epoch, "e2");
                    assert_eq!(c.after_sequence, 0);
                    r.hello(1, "e2").unwrap().unwrap()
                }
                _ => panic!("must recover"),
            };
            let mut fresh = snapshot(0);
            fresh.cursor.epoch = "e2".into();
            let mut resets = r.snapshot_loaded(new, fresh).unwrap();
            r.reset_applied(resets.pop().unwrap()).unwrap();
            assert_eq!(r.cursor().epoch, "e2");
        }
    }
}
#[tokio::test]
async fn failed_consumer_resyncs_independently_using_actual_applied_context() {
    use workbench_client::{
        application::events::{spawn_delivery, spawn_reset, spawn_snapshot, RecoveryOutcome},
        ports::EventConsumer,
    };
    struct Consumer {
        fail: bool,
        contexts: std::sync::Arc<std::sync::Mutex<Vec<u64>>>,
    }
    #[async_trait::async_trait]
    impl EventConsumer for Consumer {
        async fn consume(&mut self, _: &EventEnvelope) -> Result<(), ClientError> {
            if self.fail {
                Err(ClientError::Protocol)
            } else {
                Ok(())
            }
        }
        async fn reset(&mut self, _: &Snapshot) -> Result<(), ClientError> {
            panic!("reset_from context must be used")
        }
        async fn reset_from(
            &mut self,
            _: &Snapshot,
            applied: &StreamCursor,
        ) -> Result<(), ClientError> {
            self.contexts.lock().unwrap().push(applied.after_sequence);
            Ok(())
        }
    }
    struct Source;
    #[async_trait::async_trait]
    impl SnapshotPort for Source {
        async fn snapshot(&mut self, applied: &StreamCursor) -> Result<Snapshot, ClientError> {
            assert_eq!(applied, &cursor(0));
            Ok(snapshot(1))
        }
    }
    let mut r = recovery();
    let a = r.register(0).unwrap();
    let b = r.register(0).unwrap();
    r.hello(1, "e").unwrap();
    r.receive(event(1)).unwrap();
    let contexts = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let limits = Limits::default();
    let d = r.next(a).unwrap().unwrap();
    let (failed, completion) = spawn_delivery(
        d,
        Box::new(Consumer {
            fail: true,
            contexts: contexts.clone(),
        }),
        &limits,
    )
    .join()
    .await
    .unwrap();
    let action = r.complete_delivery(completion).unwrap().unwrap();
    let RecoveryAction::Load(load) = action else {
        panic!("must resync only failing consumer")
    };
    let d = r.next(b).unwrap().unwrap();
    let (_, completion) = spawn_delivery(
        d,
        Box::new(Consumer {
            fail: false,
            contexts: contexts.clone(),
        }),
        &limits,
    )
    .join()
    .await
    .unwrap();
    assert!(r.complete_delivery(completion).unwrap().is_none());
    assert_eq!(r.consumer_cursor(b).unwrap(), cursor(1));
    assert_eq!(r.consumer_cursor(a).unwrap(), cursor(0));
    let completion = spawn_snapshot(load, Box::new(Source), &limits)
        .join()
        .await
        .unwrap();
    let RecoveryOutcome::Reset(mut work) = r.complete_snapshot(completion).unwrap() else {
        panic!("must reset")
    };
    assert_eq!(work.len(), 1);
    assert_eq!(work[0].consumer(), a);
    let (_, completion) = spawn_reset(work.pop().unwrap(), failed, &limits)
        .join()
        .await
        .unwrap();
    assert!(r.complete_reset(completion).unwrap().is_none());
    assert_eq!(r.cursor(), cursor(1));
    assert_eq!(*contexts.lock().unwrap(), vec![0]);
}
#[tokio::test]
async fn owned_snapshot_cancel_drops_pending_source_and_new_round_finishes() {
    use workbench_client::application::events::spawn_snapshot;
    struct Never {
        entered: std::sync::Arc<tokio::sync::Notify>,
        dropped: std::sync::Arc<tokio::sync::Notify>,
    }
    impl Drop for Never {
        fn drop(&mut self) {
            self.dropped.notify_one();
        }
    }
    #[async_trait::async_trait]
    impl SnapshotPort for Never {
        async fn snapshot(&mut self, _: &StreamCursor) -> Result<Snapshot, ClientError> {
            self.entered.notify_one();
            std::future::pending().await
        }
    }
    let mut r = recovery();
    r.register(0).unwrap();
    r.start_gap(gap(5)).unwrap();
    let old = r.hello(1, "e").unwrap().unwrap();
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let dropped = std::sync::Arc::new(tokio::sync::Notify::new());
    let job = spawn_snapshot(
        old,
        Box::new(Never {
            entered: entered.clone(),
            dropped: dropped.clone(),
        }),
        &Limits::default(),
    );
    tokio::time::timeout(std::time::Duration::from_secs(1), entered.notified())
        .await
        .unwrap();
    r.start_gap(gap(6)).unwrap();
    job.cancel().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(1), dropped.notified())
        .await
        .unwrap();
    let new = r.hello(1, "e").unwrap().unwrap();
    let mut resets = r.snapshot_loaded(new, snapshot(6)).unwrap();
    r.reset_applied(resets.pop().unwrap()).unwrap();
    assert_eq!(r.cursor(), cursor(6));
}
#[tokio::test]
async fn owned_consumer_drop_or_explicit_cancel_settles_without_ack() {
    use workbench_client::{application::events::spawn_delivery, ports::EventConsumer};
    struct Never {
        entered: std::sync::Arc<tokio::sync::Notify>,
        dropped: std::sync::Arc<tokio::sync::Notify>,
    }
    impl Drop for Never {
        fn drop(&mut self) {
            self.dropped.notify_one();
        }
    }
    #[async_trait::async_trait]
    impl EventConsumer for Never {
        async fn consume(&mut self, _: &EventEnvelope) -> Result<(), ClientError> {
            self.entered.notify_one();
            std::future::pending().await
        }
        async fn reset(&mut self, _: &Snapshot) -> Result<(), ClientError> {
            Ok(())
        }
    }
    for explicit in [false, true] {
        let mut r = recovery();
        let c = r.register(0).unwrap();
        r.hello(1, "e").unwrap();
        r.receive(event(1)).unwrap();
        let entered = std::sync::Arc::new(tokio::sync::Notify::new());
        let dropped = std::sync::Arc::new(tokio::sync::Notify::new());
        let job = spawn_delivery(
            r.next(c).unwrap().unwrap(),
            Box::new(Never {
                entered: entered.clone(),
                dropped: dropped.clone(),
            }),
            &Limits::default(),
        );
        tokio::time::timeout(std::time::Duration::from_secs(1), entered.notified())
            .await
            .unwrap();
        if explicit {
            job.cancel().await.unwrap();
        } else {
            drop(job);
        }
        tokio::time::timeout(std::time::Duration::from_secs(1), dropped.notified())
            .await
            .unwrap();
        assert_eq!(r.cursor(), cursor(0));
        r.unregister(c).unwrap();
        assert_eq!(r.phase(), RecoveryPhase::Idle);
    }
}
#[tokio::test]
async fn slow_reset_does_not_block_other_consumer_and_old_result_is_stale() {
    use workbench_client::{application::events::spawn_reset, ports::EventConsumer};
    struct ResetConsumer {
        entered: std::sync::Arc<tokio::sync::Notify>,
        release: std::sync::Arc<tokio::sync::Notify>,
        slow: bool,
    }
    #[async_trait::async_trait]
    impl EventConsumer for ResetConsumer {
        async fn consume(&mut self, _: &EventEnvelope) -> Result<(), ClientError> {
            Ok(())
        }
        async fn reset(&mut self, _: &Snapshot) -> Result<(), ClientError> {
            self.entered.notify_one();
            if self.slow {
                self.release.notified().await;
            }
            Ok(())
        }
    }
    let mut r = recovery();
    let a = r.register(0).unwrap();
    let b = r.register(0).unwrap();
    r.start_gap(gap(5)).unwrap();
    let load = r.hello(1, "e").unwrap().unwrap();
    let work = r.snapshot_loaded(load, snapshot(5)).unwrap();
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let mut slow = None;
    for w in work {
        let id = w.consumer();
        let job = spawn_reset(
            w,
            Box::new(ResetConsumer {
                entered: entered.clone(),
                release: release.clone(),
                slow: id == a,
            }),
            &Limits::default(),
        );
        if id == a {
            slow = Some(job);
        } else {
            let (_, completion) = job.join().await.unwrap();
            r.complete_reset(completion).unwrap();
        }
    }
    tokio::time::timeout(std::time::Duration::from_secs(1), entered.notified())
        .await
        .unwrap();
    assert_eq!(r.consumer_cursor(a).unwrap(), cursor(0));
    assert_eq!(r.consumer_cursor(b).unwrap(), cursor(5));
    r.receive(event(6)).unwrap();
    let d = r.next(b).unwrap().unwrap();
    r.ack(d).unwrap();
    assert_eq!(r.consumer_cursor(b).unwrap(), cursor(6));
    r.start_gap(gap(6)).unwrap();
    release.notify_one();
    let (_, completion) = slow.unwrap().join().await.unwrap();
    assert!(matches!(
        r.complete_reset(completion),
        Err(ClientError::StaleGeneration)
    ));
    assert_eq!(r.consumer_cursor(a).unwrap(), cursor(0));
}
#[test]
fn stream_exhaustion_invalidates_listener_load_reset_and_delivery_without_advancing() {
    for completion in 0..4 {
        let limits = Limits::new(LimitConfig {
            recovery_attempts: 1,
            ..Default::default()
        })
        .unwrap();
        let mut r = EventRecovery::new(cursor(0), limits).unwrap();
        let a = r.register(0).unwrap();
        let b = r.register(0).unwrap();
        r.start_gap(gap(5)).unwrap();
        let load = r.hello(1, "e").unwrap().unwrap();
        let work = r.snapshot_loaded(load, snapshot(5)).unwrap();
        let mut listener_load = None;
        for w in work {
            if w.consumer() == a {
                let RecoveryAction::Load(load) = r.reset_failed(w).unwrap() else {
                    panic!("listener should retry")
                };
                listener_load = Some(load);
            } else {
                r.reset_applied(w).unwrap();
            }
        }
        let mut listener_reset = None;
        let mut delivery = None;
        if completion == 1 {
            listener_reset = Some(
                r.snapshot_loaded(listener_load.take().unwrap(), snapshot(5))
                    .unwrap()
                    .pop()
                    .unwrap(),
            );
        }
        if completion == 2 {
            r.receive(event(6)).unwrap();
            delivery = Some(r.next(b).unwrap().unwrap());
        }
        let before = r.cursor();
        let b_before = r.consumer_cursor(b).unwrap();
        assert!(matches!(
            r.start_gap(gap(6)).unwrap(),
            RecoveryAction::Exhausted
        ));
        assert_eq!(r.phase(), RecoveryPhase::Exhausted);
        let stale = match completion {
            0 => r
                .snapshot_loaded(listener_load.take().unwrap(), snapshot(5))
                .map(|_| ()),
            1 => r.reset_applied(listener_reset.take().unwrap()),
            2 => r.ack(delivery.take().unwrap()),
            _ => r.snapshot_failed(listener_load.take().unwrap()).map(|_| ()),
        };
        assert!(matches!(stale, Err(ClientError::StaleGeneration)));
        assert_eq!(r.phase(), RecoveryPhase::Exhausted);
        assert_eq!(r.cursor(), before);
        assert_eq!(r.consumer_cursor(b).unwrap(), b_before);
    }
}
#[test]
fn listener_only_exhaustion_keeps_other_consumers_live() {
    let limits = Limits::new(LimitConfig {
        recovery_attempts: 1,
        ..Default::default()
    })
    .unwrap();
    let mut r = EventRecovery::new(cursor(0), limits).unwrap();
    let a = r.register(0).unwrap();
    let b = r.register(0).unwrap();
    r.hello(1, "e").unwrap();
    r.receive(event(1)).unwrap();
    let failed = r.next(a).unwrap().unwrap();
    let RecoveryAction::Load(load) = r.consumer_failed(failed).unwrap() else {
        panic!("must load")
    };
    let work = r.snapshot_loaded(load, snapshot(1)).unwrap().pop().unwrap();
    assert!(matches!(r.reset_failed(work).unwrap(),RecoveryAction::ListenerExhausted(id) if id==a));
    assert_eq!(r.phase(), RecoveryPhase::Live);
    let d = r.next(b).unwrap().unwrap();
    r.ack(d).unwrap();
    assert_eq!(r.consumer_cursor(b).unwrap(), cursor(1));
    assert_eq!(r.cursor(), cursor(0));
    r.unregister(a).unwrap();
    r.receive(event(2)).unwrap();
    let d = r.next(b).unwrap().unwrap();
    r.ack(d).unwrap();
    assert_eq!(r.cursor(), cursor(2));
}
#[tokio::test]
async fn panicking_callback_retains_delivery_identity_for_listener_resync() {
    use workbench_client::{application::events::spawn_delivery, ports::EventConsumer};
    struct Panics;
    #[async_trait::async_trait]
    impl EventConsumer for Panics {
        async fn consume(&mut self, _: &EventEnvelope) -> Result<(), ClientError> {
            panic!("private-sentinel")
        }
        async fn reset(&mut self, _: &Snapshot) -> Result<(), ClientError> {
            Ok(())
        }
    }
    let mut r = recovery();
    let a = r.register(0).unwrap();
    r.hello(1, "e").unwrap();
    r.receive(event(1)).unwrap();
    let d = r.next(a).unwrap().unwrap();
    let (_, completion) = spawn_delivery(d, Box::new(Panics), &Limits::default())
        .join()
        .await
        .unwrap();
    assert!(matches!(
        r.complete_delivery(completion).unwrap(),
        Some(RecoveryAction::Load(_))
    ));
    assert_eq!(r.cursor(), cursor(0));
}
#[test]
fn notification_disconnect_requires_live_hello_then_snapshot_and_retained_reconnect_replays() {
    let mut signal_cursor = cursor(0);
    signal_cursor.stream_id = "worktree:/private/x".into();
    let mut r = EventRecovery::new(signal_cursor.clone(), Limits::default()).unwrap();
    let id = r.register(0).unwrap();
    r.hello(1, "e").unwrap();
    let mut notification = event(1);
    notification.stream_id = signal_cursor.stream_id.clone();
    notification.schema = "worktree.changed.v1".into();
    r.receive(notification).unwrap();
    let d = r.next(id).unwrap().unwrap();
    r.ack(d).unwrap();
    assert!(matches!(r.disconnected().unwrap(),RecoveryAction::Connect(c) if c.after_sequence==1));
    let load = r.hello(1, "e").unwrap().unwrap();
    assert_eq!(load.applied_cursor().after_sequence, 1);
    assert_eq!(load.live_cursor().after_sequence, 1);
    assert!(!r.is_live());
    let mut state = snapshot(1);
    state.cursor.stream_id = signal_cursor.stream_id;
    let w = r.snapshot_loaded(load, state).unwrap().pop().unwrap();
    r.reset_applied(w).unwrap();
    assert!(r.is_live());
    let mut retained = recovery();
    let c = retained.register(0).unwrap();
    retained.hello(1, "e").unwrap();
    retained.receive(event(1)).unwrap();
    let d = retained.next(c).unwrap().unwrap();
    retained.ack(d).unwrap();
    assert!(matches!(retained.disconnected().unwrap(),RecoveryAction::Connect(c) if c==cursor(1)));
    assert!(retained.hello(1, "e").unwrap().is_none());
}
#[tokio::test]
async fn owned_snapshot_deadline_is_failed_recovery_not_hello_success() {
    use workbench_client::application::events::{spawn_snapshot, RecoveryOutcome};
    struct Never;
    #[async_trait::async_trait]
    impl SnapshotPort for Never {
        async fn snapshot(&mut self, _: &StreamCursor) -> Result<Snapshot, ClientError> {
            std::future::pending().await
        }
    }
    let limits = Limits::new(LimitConfig {
        request_timeout: std::time::Duration::from_millis(10),
        ..Default::default()
    })
    .unwrap();
    let mut r = EventRecovery::new(cursor(0), limits.clone()).unwrap();
    r.register(0).unwrap();
    r.start_gap(gap(3)).unwrap();
    let load = r.hello(1, "e").unwrap().unwrap();
    let result = spawn_snapshot(load, Box::new(Never), &limits)
        .join()
        .await
        .unwrap();
    assert!(
        matches!(r.complete_snapshot(result).unwrap(),RecoveryOutcome::Action(RecoveryAction::Connect(c)) if c==cursor(3))
    );
    assert_eq!(r.cursor(), cursor(0));
    assert!(!r.is_live());
}

#[test]
fn same_epoch_notification_lag_and_shutdown_require_snapshot_and_all_resets_retained_replays() {
    for reason in [GapReason::SubscriberLagged, GapReason::Shutdown] {
        let initial = StreamCursor {
            stream_id: "bench:existing".into(),
            ..cursor(0)
        };
        let mut r = EventRecovery::new(initial.clone(), Limits::default()).unwrap();
        let a = r.register(0).unwrap();
        let b = r.register(0).unwrap();
        r.hello(1, "e").unwrap();
        let mut first = event(1);
        first.stream_id = initial.stream_id.clone();
        first.schema = "bench.titleRequested.v1".into();
        r.receive(first).unwrap();
        for id in [a, b] {
            let d = r.next(id).unwrap().unwrap();
            r.ack(d).unwrap();
        }
        let notice = GapNotice {
            stream_id: initial.stream_id.clone(),
            epoch: "e".into(),
            reason,
            first_sequence: Some(2),
            last_sequence: Some(2),
        };
        assert!(
            matches!(r.start_gap(notice.clone()).unwrap(), RecoveryAction::Connect(c) if c.after_sequence == 2)
        );
        let load = r
            .hello(1, "e")
            .unwrap()
            .expect("notification gap must snapshot after live hello");
        assert_eq!(load.applied_cursor().after_sequence, 1);
        assert_eq!(load.live_cursor().after_sequence, 2);
        // Replacement subscription is live-only: lost notification2 is never replayed.
        let mut live = event(3);
        live.stream_id = initial.stream_id.clone();
        live.schema = "bench.titleRequested.v1".into();
        r.receive(live).unwrap();
        for id in [a, b] {
            assert!(r.next(id).unwrap().is_none());
        }
        assert_eq!(r.cursor().after_sequence, 1);
        let mut state = snapshot(2);
        state.cursor.stream_id = initial.stream_id.clone();
        let mut work = r.snapshot_loaded(load, state).unwrap();
        assert_eq!(work.len(), 2);
        r.reset_applied(work.pop().unwrap()).unwrap();
        // One successful reset is insufficient for non-retaining stream delivery.
        for id in [a, b] {
            assert!(r.next(id).unwrap().is_none());
        }
        assert_eq!(r.cursor().after_sequence, 1);
        r.reset_applied(work.pop().unwrap()).unwrap();
        assert!(r.is_live());
        assert_eq!(r.cursor().after_sequence, 2);
        for id in [a, b] {
            let d = r.next(id).unwrap().unwrap();
            assert_eq!(d.event.sequence, 3);
            r.ack(d).unwrap();
        }
        assert_eq!(r.cursor().after_sequence, 3);
        let mut retained = recovery();
        let id = retained.register(0).unwrap();
        retained.hello(1, "e").unwrap();
        retained.receive(event(1)).unwrap();
        let d = retained.next(id).unwrap().unwrap();
        retained.ack(d).unwrap();
        let retained_notice = GapNotice {
            stream_id: cursor(0).stream_id,
            ..notice
        };
        assert!(
            matches!(retained.start_gap(retained_notice).unwrap(),RecoveryAction::Connect(c) if c==cursor(1))
        );
        assert!(retained.hello(1, "e").unwrap().is_none());
        retained.receive(event(2)).unwrap();
        let d = retained.next(id).unwrap().unwrap();
        retained.ack(d).unwrap();
        assert_eq!(retained.cursor(), cursor(2));
    }
}

#[tokio::test]
async fn terminal_snapshot_failure_preserves_cause_and_closes_without_cursor_change() {
    use workbench_protocol::{FaultCode, Outcome, RequestId, WorkbenchFault};
    for error in [
        ClientError::Identity,
        ClientError::Incompatible,
        ClientError::Protocol,
        ClientError::Fault(Box::new(
            WorkbenchFault::new(
                FaultCode::Unauthenticated,
                RequestId::new("original").unwrap(),
                "private-sentinel",
            )
            .with_outcome(Outcome::NotApplied),
        )),
        ClientError::Fault(Box::new(
            WorkbenchFault::new(
                FaultCode::Unavailable,
                RequestId::new("original").unwrap(),
                "private-sentinel",
            )
            .with_retryable(false),
        )),
    ] {
        let expected = std::mem::discriminant(&error);
        let fault = if let ClientError::Fault(fault) = &error {
            Some(fault.clone())
        } else {
            None
        };
        let mut r = recovery();
        let id = r.register(0).unwrap();
        r.start_gap(gap(6)).unwrap();
        let load = r.hello(1, "e").unwrap().unwrap();
        let result = r.complete_snapshot(failed_snapshot_completion(load, error).await);
        let Err(returned) = result else {
            panic!("terminal snapshot error must not reconnect")
        };
        assert_eq!(std::mem::discriminant(&returned), expected);
        if let Some(fault) = fault {
            let ClientError::Fault(returned) = returned else {
                panic!("original fault lost")
            };
            assert_eq!(returned, fault);
        }
        assert_eq!(r.phase(), RecoveryPhase::Terminal);
        assert_eq!(r.cursor(), cursor(0));
        assert_eq!(r.consumer_cursor(id).unwrap(), cursor(0));
        assert!(r.next(id).unwrap().is_none());
        assert!(matches!(
            r.disconnected(),
            Err(ClientError::StaleGeneration)
        ));
    }
}
#[tokio::test]
async fn classified_transient_snapshot_failures_retry_the_same_boundary() {
    use workbench_client::application::events::RecoveryOutcome;
    use workbench_protocol::{FaultCode, RequestId, WorkbenchFault};
    for error in [
        ClientError::Unavailable,
        ClientError::Deadline,
        ClientError::TransportUnknown,
        ClientError::Fault(Box::new(WorkbenchFault::new(
            FaultCode::Unavailable,
            RequestId::new("original").unwrap(),
            "private-sentinel",
        ))),
        ClientError::Fault(Box::new(WorkbenchFault::new(
            FaultCode::RateLimited,
            RequestId::new("original").unwrap(),
            "private-sentinel",
        ))),
    ] {
        let mut r = recovery();
        r.register(0).unwrap();
        r.start_gap(gap(6)).unwrap();
        let load = r.hello(1, "e").unwrap().unwrap();
        assert!(
            matches!(r.complete_snapshot(failed_snapshot_completion(load, error).await).unwrap(), RecoveryOutcome::Action(RecoveryAction::Connect(c)) if c==cursor(6))
        );
        assert_eq!(r.cursor(), cursor(0));
    }
}

#[tokio::test]
async fn old_terminal_snapshot_errors_cannot_end_a_new_live_round() {
    for origin in 0..3 {
        for error in [ClientError::Identity, ClientError::Protocol] {
            let mut r = recovery();
            let id = r.register(0).unwrap();
            let old = match origin {
                0 => {
                    r.start_gap(gap(3)).unwrap();
                    r.hello(1, "e").unwrap().unwrap()
                }
                1 => {
                    r.hello(1, "e").unwrap();
                    r.receive(event(1)).unwrap();
                    let delivery = r.next(id).unwrap().unwrap();
                    let RecoveryAction::Load(load) = r.consumer_failed(delivery).unwrap() else {
                        panic!("listener load required")
                    };
                    load
                }
                _ => {
                    let mut other = recovery();
                    other.register(0).unwrap();
                    other.start_gap(gap(3)).unwrap();
                    other.hello(1, "e").unwrap().unwrap()
                }
            };
            r.start_gap(gap(6)).unwrap();
            let current = r.hello(1, "e").unwrap().unwrap();
            let reset = r
                .snapshot_loaded(current, snapshot(6))
                .unwrap()
                .pop()
                .unwrap();
            r.reset_applied(reset).unwrap();
            assert!(r.is_live());
            assert!(matches!(
                r.complete_snapshot(failed_snapshot_completion(old, error).await),
                Err(ClientError::StaleGeneration)
            ));
            assert!(r.is_live());
            assert_eq!(r.cursor(), cursor(6));
            assert_eq!(r.consumer_cursor(id).unwrap(), cursor(6));
            r.receive(event(7)).unwrap();
            let delivery = r.next(id).unwrap().unwrap();
            r.ack(delivery).unwrap();
            assert_eq!(r.cursor(), cursor(7));
        }
    }
}

async fn failed_snapshot_completion(
    load: workbench_client::application::events::LoadRequest,
    error: ClientError,
) -> workbench_client::application::events::SnapshotCompletion {
    struct Fails(Option<ClientError>);
    #[async_trait::async_trait]
    impl SnapshotPort for Fails {
        async fn snapshot(&mut self, _: &StreamCursor) -> Result<Snapshot, ClientError> {
            Err(self.0.take().unwrap())
        }
    }
    workbench_client::application::events::spawn_snapshot(
        load,
        Box::new(Fails(Some(error))),
        &Limits::default(),
    )
    .join()
    .await
    .unwrap()
}
