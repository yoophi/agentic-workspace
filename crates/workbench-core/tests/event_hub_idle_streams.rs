//! 039 리뷰 반영: 없는 run을 cursor 0으로 기다리다 떠난 구독은 빈 스트림을 남기지 않는다. 그런 스트림은 보관 한도를
//! 차지하지 않아 실제 run의 replay가 지워지지 않고, 해지와 경합한 발행도 순번을 잃지 않는다.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use serde_json::json;
use workbench_core::infrastructure::event_hub::{EventHub, EventHubLimits};
use workbench_protocol::{
    events::{StreamKind, RUN_EVENT_V1},
    AuthenticatedPrincipal, StreamCursor, Subscription,
};

fn waiting(run: &str) -> Subscription {
    Subscription {
        cursors: vec![StreamCursor {
            stream_id: format!("run:{run}"),
            epoch: "epoch-1".into(),
            after_sequence: 0,
        }],
    }
}

fn limits() -> EventHubLimits {
    EventHubLimits {
        max_retained_runs: 2,
        ..EventHubLimits::default()
    }
}

#[test]
fn waiting_on_unknown_runs_leaves_no_streams_and_keeps_completed_runs() {
    let hub = EventHub::new("epoch-1", limits());
    let principal = AuthenticatedPrincipal::desktop();
    for index in 0..300 {
        let stream = hub
            .subscribe(&principal, waiting(&format!("ghost-{index}")))
            .unwrap();
        drop(stream);
    }
    assert_eq!(hub.stream_count(), 0, "idle streams must be removed");

    hub.publish_state(
        StreamKind::Run,
        "real",
        RUN_EVENT_V1,
        json!({}),
        true,
        &mut |_| {},
    );
    let held = hub.subscribe(&principal, waiting("ghost-held")).unwrap();
    hub.publish_state(
        StreamKind::Run,
        "real-2",
        RUN_EVENT_V1,
        json!({}),
        true,
        &mut |_| {},
    );

    // 보관 한도 2: 발행된 run 2개는 모두 남고, 기다리는 빈 스트림은 한도에 들지 않는다.
    let replay = hub.replay_run("real", 0);
    assert_eq!(replay.last_sequence, 1);
    assert!(
        !replay.gap_detected,
        "completed run must not be evicted by idle streams"
    );
    drop(held);
    assert_eq!(hub.stream_count(), 2);
}

#[test]
fn publishes_racing_with_idle_stream_removal_keep_contiguous_sequences() {
    const EVENTS: u64 = 2_000;
    let hub = EventHub::new("epoch-1", EventHubLimits::default());
    let stop = Arc::new(AtomicBool::new(false));
    let churn = {
        let hub = Arc::clone(&hub);
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            let principal = AuthenticatedPrincipal::desktop();
            while !stop.load(Ordering::Relaxed) {
                drop(hub.subscribe(&principal, waiting("race")).unwrap());
            }
        })
    };
    let mut delivered = Vec::with_capacity(EVENTS as usize);
    for _ in 0..EVENTS {
        hub.publish_state(
            StreamKind::Run,
            "race",
            RUN_EVENT_V1,
            json!({}),
            false,
            &mut |envelope| delivered.push(envelope.sequence),
        );
    }
    stop.store(true, Ordering::Relaxed);
    churn.join().unwrap();
    assert_eq!(delivered, (1..=EVENTS).collect::<Vec<_>>());
    assert_eq!(hub.replay_run("race", 0).last_sequence, EVENTS);
}
