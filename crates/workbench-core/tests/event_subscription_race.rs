//! 039 SC-001: 구독 시작 전·중·후에 발행이 계속되는 동안 1,000회 구독해, 매번 받은 순번이 cursor 다음부터 빈틈없이
//! 연속이고 중복이 없는지 확인한다(발행과 구독이 같은 스트림 lock에서 원자적임을 검증).

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use futures_util::StreamExt;
use serde_json::json;
use workbench_core::infrastructure::event_hub::{EventHub, EventHubLimits};
use workbench_protocol::{
    events::{StreamKind, RUN_EVENT_V1},
    AuthenticatedPrincipal, EventItem, StreamCursor, Subscription,
};

const ROUNDS: usize = 1_000;
const TAKE: usize = 20;

fn spawn_publisher(
    hub: Arc<EventHub>,
    run: &'static str,
    stop: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            hub.publish_state(
                StreamKind::Run,
                run,
                RUN_EVENT_V1,
                json!({}),
                false,
                &mut |_| {},
            );
            std::thread::yield_now();
        }
    })
}

async fn contiguous_from(
    stream: &mut workbench_protocol::EventStream,
    stream_id: &str,
    after: u64,
) {
    let mut expected = after + 1;
    let mut got = 0;
    while got < TAKE {
        let item = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .expect("item in time")
            .expect("stream open");
        match item {
            EventItem::Event { event } if event.stream_id == stream_id => {
                assert_eq!(
                    event.sequence, expected,
                    "gap or duplicate after cursor {after}"
                );
                expected += 1;
                got += 1;
            }
            EventItem::Event { .. } => {}
            EventItem::Gap { gap } => panic!("unexpected gap {gap:?}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn subscribing_while_publishing_never_loses_or_duplicates() {
    let hub = EventHub::new(
        "e",
        EventHubLimits {
            // 발행이 빠르므로 cursor가 보관 범위 밖으로 밀려나지 않게 넉넉히.
            run_journal_capacity: 1 << 20,
            subscriber_queue: 1 << 16,
            ..Default::default()
        },
    );
    let stop = Arc::new(AtomicBool::new(false));
    let publisher = spawn_publisher(Arc::clone(&hub), "r1", Arc::clone(&stop));
    let principal = AuthenticatedPrincipal::desktop();
    for round in 0..ROUNDS {
        let last = hub.replay_run("r1", u64::MAX).last_sequence;
        // 이미 지난 구간, 막 지난 지점, 끝 — 세 가지 cursor를 돌아가며.
        let after = match round % 3 {
            0 => last / 2,
            1 => last.saturating_sub(1),
            _ => last,
        };
        let mut stream = hub
            .subscribe(
                &principal,
                Subscription {
                    cursors: vec![StreamCursor {
                        stream_id: "run:r1".into(),
                        epoch: "e".into(),
                        after_sequence: after,
                    }],
                },
            )
            .expect("subscribe");
        contiguous_from(&mut stream, "run:r1", after).await;
    }
    stop.store(true, Ordering::Relaxed);
    publisher.join().unwrap();
}

/// 두 스트림을 한 구독으로 받아도 각 스트림 안에서 연속이다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multi_stream_subscription_is_contiguous_per_stream() {
    let hub = EventHub::new(
        "e",
        EventHubLimits {
            run_journal_capacity: 1 << 20,
            subscriber_queue: 1 << 16,
            ..Default::default()
        },
    );
    let stop = Arc::new(AtomicBool::new(false));
    let a = spawn_publisher(Arc::clone(&hub), "a", Arc::clone(&stop));
    let b = spawn_publisher(Arc::clone(&hub), "b", Arc::clone(&stop));
    for _ in 0..100 {
        let after_a = hub.replay_run("a", u64::MAX).last_sequence;
        let after_b = hub.replay_run("b", u64::MAX).last_sequence;
        let mut stream = hub
            .subscribe(
                &AuthenticatedPrincipal::desktop(),
                Subscription {
                    cursors: vec![
                        StreamCursor {
                            stream_id: "run:a".into(),
                            epoch: "e".into(),
                            after_sequence: after_a,
                        },
                        StreamCursor {
                            stream_id: "run:b".into(),
                            epoch: "e".into(),
                            after_sequence: after_b,
                        },
                    ],
                },
            )
            .unwrap();
        let (mut next_a, mut next_b) = (after_a + 1, after_b + 1);
        let mut seen = 0;
        while seen < 40 {
            let item = tokio::time::timeout(Duration::from_secs(5), stream.next())
                .await
                .unwrap()
                .unwrap();
            if let EventItem::Event { event } = item {
                let next = if event.stream_id == "run:a" {
                    &mut next_a
                } else {
                    &mut next_b
                };
                assert_eq!(event.sequence, *next, "{}", event.stream_id);
                *next += 1;
                seen += 1;
            }
        }
    }
    stop.store(true, Ordering::Relaxed);
    a.join().unwrap();
    b.join().unwrap();
}
