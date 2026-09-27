//! 039 SC-006: run 이벤트 발행 비용과 구독자 수신 지연(`cargo test --test event_latency -- --ignored --nocapture`).
//! 기준선은 AW가 쓰던 방식(Mutex + VecDeque에 같은 `Value` append)이다. 결과는 tasks.md Notes에 기록한다.

use std::{
    collections::VecDeque,
    sync::Mutex,
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use serde_json::json;
use workbench_core::infrastructure::event_hub::{EventHub, EventHubLimits};
use workbench_protocol::{
    events::{StreamKind, RUN_EVENT_V1},
    AuthenticatedPrincipal, StreamCursor, Subscription,
};

const ROUNDS: usize = 1_000;

fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[samples.len() * 95 / 100]
}

fn body(index: usize) -> serde_json::Value {
    json!({"type": "agentMessage", "text": format!("chunk {index} of a streamed agent message")})
}

#[tokio::test]
#[ignore = "measurement; run with --ignored --nocapture"]
async fn publish_and_delivery_latency_stays_within_budget() {
    let baseline_journal = Mutex::new(VecDeque::new());
    let baseline: Vec<Duration> = (0..ROUNDS)
        .map(|index| {
            let value = body(index);
            let started = Instant::now();
            let mut journal = baseline_journal.lock().unwrap();
            journal.push_back((index as u64, value));
            if journal.len() > 512 {
                journal.pop_front();
            }
            started.elapsed()
        })
        .collect();

    let hub = EventHub::new("epoch-1", EventHubLimits::default());
    let publish: Vec<Duration> = (0..ROUNDS)
        .map(|index| {
            let value = body(index);
            let started = Instant::now();
            hub.publish_state(
                StreamKind::Run,
                "r1",
                RUN_EVENT_V1,
                value,
                false,
                &mut |_| {},
            );
            started.elapsed()
        })
        .collect();

    let mut stream = hub
        .subscribe(
            &AuthenticatedPrincipal::desktop(),
            Subscription {
                cursors: vec![StreamCursor {
                    stream_id: "run:r2".into(),
                    epoch: "epoch-1".into(),
                    after_sequence: 0,
                }],
            },
        )
        .unwrap();
    let mut delivery = Vec::with_capacity(ROUNDS);
    for index in 0..ROUNDS {
        let started = Instant::now();
        hub.publish_state(
            StreamKind::Run,
            "r2",
            RUN_EVENT_V1,
            body(index),
            false,
            &mut |_| {},
        );
        stream.next().await.expect("event");
        delivery.push(started.elapsed());
    }

    let (baseline, publish, delivery) = (p95(baseline), p95(publish), p95(delivery));
    println!("p95 baseline append {baseline:?}, hub publish {publish:?}, publish→subscriber {delivery:?}");
    assert!(
        delivery.saturating_sub(baseline) < Duration::from_millis(10),
        "SC-006: added latency must stay under 10ms"
    );
}
