//! 042 SC-003: 운영 router의 표 구독(`POST /v1/event-tickets` → `GET /v1/events`)이 발행이 계속되는 동안 1,000회
//! 이어져도, 매번 받은 순번이 cursor 다음부터 빈틈없이 연속이고 중복이 없다(039 in-process 시험의 HTTP판).

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use serde_json::json;
use support::{
    http_harness::{Harness, TOKEN_DESKTOP},
    scripted_run_engine::RunScript,
    BenchHarness,
};
use workbench_core::infrastructure::event_hub::EventHubLimits;
use workbench_protocol::{
    events::{EventFrame, StreamKind, RUN_EVENT_V1},
    AuthenticatedPrincipal, OperationId, StreamCursor, Workbench,
};

mod support;

const ROUNDS: usize = 1_000;
const TAKE: usize = 10;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ticketed_subscriptions_while_publishing_never_lose_or_duplicate() {
    let h = BenchHarness::with(
        |adapters| {
            adapters.event_limits = EventHubLimits {
                run_journal_capacity: 1 << 20,
                subscriber_queue: 1 << 16,
                ..Default::default()
            };
        },
        RunScript::default(),
    );
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let epoch = h
        .call(
            &AuthenticatedPrincipal::desktop(),
            OperationId::SystemDescribe,
            json!({}),
        )
        .await
        .unwrap()["epoch"]
        .as_str()
        .unwrap()
        .to_owned();
    let hub = Arc::clone(h.rt.runtime.events_hub());
    let stop = Arc::new(AtomicBool::new(false));
    let publisher = {
        let (hub, stop) = (Arc::clone(&hub), Arc::clone(&stop));
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                hub.publish_state(
                    StreamKind::Run,
                    "r1",
                    RUN_EVENT_V1,
                    json!({}),
                    false,
                    &mut |_| {},
                );
                std::thread::sleep(Duration::from_micros(50));
            }
        })
    };
    let harness = Harness::spawn(h.rt.runtime.clone() as Arc<dyn Workbench>).await;
    for round in 0..ROUNDS {
        let last = hub.replay_run("r1", u64::MAX).last_sequence;
        let after = match round % 3 {
            0 => last / 2,
            1 => last.saturating_sub(1),
            _ => last,
        };
        let cursors = vec![StreamCursor {
            stream_id: "run:r1".into(),
            epoch: epoch.clone(),
            after_sequence: after,
        }];
        let mut ws = harness.subscribe(TOKEN_DESKTOP, cursors).await;
        let mut expected = after + 1;
        let mut got = 0;
        while got < TAKE {
            match ws.next_frame(Duration::from_secs(5)).await {
                Some(EventFrame::Event { event }) if event.stream_id == "run:r1" => {
                    assert_eq!(
                        event.sequence, expected,
                        "round {round}: gap or duplicate after {after}"
                    );
                    expected += 1;
                    got += 1;
                }
                Some(EventFrame::Event { .. }) => {}
                other => panic!("round {round}: unexpected {other:?}"),
            }
        }
    }
    stop.store(true, Ordering::Relaxed);
    publisher.join().unwrap();
}
