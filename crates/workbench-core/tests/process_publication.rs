use serde_json::json;
use std::sync::{Arc, Barrier};
use workbench_core::{
    application::process_publication::{
        ProcessPublicationService, PublicationDeliveryError, PublicationProjection,
    },
    infrastructure::{data_paths::DataPaths, sqlite_ledger::SqliteOperationLedger},
    ports::{
        operation_ledger::OperationLedger,
        process_publication_store::{
            ProcessPublicationStore, PublicationState, PublicationStoreError, PublishOutcome,
            PublishRequest, WithdrawOutcome,
        },
    },
};

fn store() -> (tempfile::TempDir, SqliteOperationLedger) {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(dir.path());
    paths.ensure_dirs().unwrap();
    let store = SqliteOperationLedger::open(&paths).unwrap();
    store.migrate().unwrap();
    (dir, store)
}

fn request(attempt: &str) -> PublishRequest {
    PublishRequest {
        attempt_id: attempt.into(),
        event_id: format!("{attempt}:started"),
        event_kind: "started".into(),
        result: json!({"runId": "run-1"}),
        payload: json!({"runId": "run-1", "status": "started"}),
    }
}

#[test]
fn publish_transaction_replays_exactly_and_rejects_changed_payload() {
    let (_dir, store) = store();
    store.reserve("attempt-1").unwrap();
    let first = store.publish(&request("attempt-1")).unwrap();
    assert!(matches!(first, PublishOutcome::Published(_)));
    assert!(matches!(
        store.publish(&request("attempt-1")).unwrap(),
        PublishOutcome::Replayed(_)
    ));
    assert_eq!(store.pending_events(10).unwrap().len(), 1);
    assert_eq!(
        ProcessPublicationStore::find_publication(&store, "attempt-1")
            .unwrap()
            .unwrap()
            .result,
        Some(json!({"runId": "run-1"}))
    );

    let mut changed = request("attempt-1");
    changed.payload = json!({"runId": "different"});
    assert_eq!(
        store.publish(&changed).unwrap_err(),
        PublicationStoreError::ReplayMismatch("attempt-1".into())
    );
}

#[test]
fn withdraw_and_publish_have_one_stable_winner_in_both_orders() {
    let (_dir, store) = store();
    store.reserve("withdraw-first").unwrap();
    assert_eq!(
        store.withdraw("withdraw-first").unwrap(),
        WithdrawOutcome::Withdrawn
    );
    assert_eq!(
        store.publish(&request("withdraw-first")).unwrap(),
        PublishOutcome::Lost(PublicationState::Withdrawn)
    );

    store.reserve("publish-first").unwrap();
    store.publish(&request("publish-first")).unwrap();
    assert_eq!(
        store.withdraw("publish-first").unwrap(),
        WithdrawOutcome::Lost(PublicationState::Published)
    );
    assert_eq!(store.pending_events(10).unwrap().len(), 1);
}

#[test]
fn send_and_ack_loss_replays_to_one_live_projection_instance() {
    let (_dir, store) = store();
    let service = ProcessPublicationService::new(&store);
    service.reserve("attempt-replay").unwrap();
    service.publish(&request("attempt-replay")).unwrap();

    let mut projection = PublicationProjection::default();
    let first = service.deliver_pending(10, |event| {
        assert!(projection.apply_once(event));
        Err("simulated acknowledgement loss".into())
    });
    assert!(matches!(first, Err(PublicationDeliveryError::Send(_))));
    assert_eq!(store.pending_events(10).unwrap().len(), 1);

    assert_eq!(
        service
            .deliver_pending(10, |event| {
                assert!(!projection.apply_once(event));
                Ok(())
            })
            .unwrap(),
        1
    );
    assert!(store.pending_events(10).unwrap().is_empty());
    store.acknowledge_event("attempt-replay:started").unwrap();
    assert_eq!(service.deliver_pending(10, |_| Ok(())).unwrap(), 0);
}

#[test]
fn publish_and_withdraw_race_at_a_barrier_has_one_cas_winner() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(dir.path());
    paths.ensure_dirs().unwrap();
    let publisher_store = SqliteOperationLedger::open(&paths).unwrap();
    publisher_store.migrate().unwrap();
    publisher_store.reserve("attempt-race").unwrap();
    let withdrawer_store = SqliteOperationLedger::open(&paths).unwrap();
    withdrawer_store.migrate().unwrap();
    let barrier = Arc::new(Barrier::new(3));

    let publisher_barrier = Arc::clone(&barrier);
    let publisher = std::thread::spawn(move || {
        publisher_barrier.wait();
        publisher_store.publish(&request("attempt-race")).unwrap()
    });
    let withdrawer_barrier = Arc::clone(&barrier);
    let withdrawer = std::thread::spawn(move || {
        withdrawer_barrier.wait();
        withdrawer_store.withdraw("attempt-race").unwrap()
    });
    barrier.wait();

    let publish = publisher.join().unwrap();
    let withdraw = withdrawer.join().unwrap();
    let observer = SqliteOperationLedger::open(&paths).unwrap();
    observer.migrate().unwrap();
    match (publish, withdraw) {
        (PublishOutcome::Published(_), WithdrawOutcome::Lost(PublicationState::Published)) => {
            assert_eq!(observer.pending_events(10).unwrap().len(), 1);
        }
        (PublishOutcome::Lost(PublicationState::Withdrawn), WithdrawOutcome::Withdrawn) => {
            assert!(observer.pending_events(10).unwrap().is_empty());
        }
        other => panic!("invalid CAS outcomes: {other:?}"),
    }
}

#[test]
fn committed_outbox_survives_store_close_and_reopen_before_send() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(dir.path());
    paths.ensure_dirs().unwrap();
    {
        let store = SqliteOperationLedger::open(&paths).unwrap();
        store.migrate().unwrap();
        store.reserve("attempt-reopen").unwrap();
        store.publish(&request("attempt-reopen")).unwrap();
        assert_eq!(store.pending_events(10).unwrap().len(), 1);
    }

    let reopened = SqliteOperationLedger::open(&paths).unwrap();
    reopened.migrate().unwrap();
    assert!(matches!(
        reopened.publish(&request("attempt-reopen")).unwrap(),
        PublishOutcome::Replayed(_)
    ));
    let events = reopened.pending_events(10).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, "attempt-reopen:started");
}

#[test]
fn outbox_reopens_pending_before_ack_and_delivered_after_ack() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(dir.path());
    paths.ensure_dirs().unwrap();
    {
        let store = SqliteOperationLedger::open(&paths).unwrap();
        store.migrate().unwrap();
        store.reserve("attempt-ack-crash").unwrap();
        store.publish(&request("attempt-ack-crash")).unwrap();
        let sent = store.pending_events(10).unwrap();
        assert_eq!(sent.len(), 1);
        // Simulate a process exit after the transport accepted the event but before durable ack.
    }

    {
        let reopened = SqliteOperationLedger::open(&paths).unwrap();
        reopened.migrate().unwrap();
        let replay = reopened.pending_events(10).unwrap();
        assert_eq!(replay.len(), 1);
        reopened.acknowledge_event(&replay[0].event_id).unwrap();
        // Simulate a second process exit immediately after the ack commit.
    }

    let reopened = SqliteOperationLedger::open(&paths).unwrap();
    reopened.migrate().unwrap();
    assert!(reopened.pending_events(10).unwrap().is_empty());
    reopened
        .acknowledge_event("attempt-ack-crash:started")
        .unwrap();
}
