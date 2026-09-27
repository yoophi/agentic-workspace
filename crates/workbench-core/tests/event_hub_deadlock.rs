//! 039 lock 순서 회귀: 발행(terminal → 정리 유발)·구독·drop·replay를 8 thread에서 동시에 반복해도 진행이 멈추지 않는다.
//! 스트림 lock을 쥔 채 `streams`를 잡는 코드가 생기면 이 테스트가 watchdog 시간 안에 끝나지 않는다.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use serde_json::json;
use workbench_core::infrastructure::event_hub::{EventHub, EventHubLimits};
use workbench_protocol::{
    events::{StreamKind, RUN_EVENT_V1},
    AuthenticatedPrincipal, StreamCursor, Subscription,
};

#[test]
fn concurrent_publish_subscribe_evict_and_drop_make_progress() {
    let hub = EventHub::new(
        "e",
        EventHubLimits {
            max_retained_runs: 4,
            max_tombstones: 16,
            subscriber_queue: 8,
            max_subscriptions: 10_000,
            ..Default::default()
        },
    );
    let stop = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(AtomicU64::new(0));
    let mut workers = Vec::new();
    for worker in 0..8u64 {
        let hub = Arc::clone(&hub);
        let stop = Arc::clone(&stop);
        let progress = Arc::clone(&progress);
        workers.push(std::thread::spawn(move || {
            let mut n = 0u64;
            while !stop.load(Ordering::Relaxed) {
                n += 1;
                let run = format!("r{}", (worker * 7 + n) % 12);
                match (worker + n) % 4 {
                    0 | 1 => {
                        let terminal = n.is_multiple_of(5);
                        hub.publish_state(
                            StreamKind::Run,
                            &run,
                            RUN_EVENT_V1,
                            json!({}),
                            terminal,
                            &mut |_| {},
                        );
                    }
                    2 => {
                        let stream = hub.subscribe(
                            &AuthenticatedPrincipal::desktop(),
                            Subscription {
                                cursors: vec![
                                    StreamCursor {
                                        stream_id: format!("run:{run}"),
                                        epoch: "e".into(),
                                        after_sequence: 0,
                                    },
                                    StreamCursor {
                                        stream_id: format!("run:r{}", n % 12),
                                        epoch: "e".into(),
                                        after_sequence: 0,
                                    },
                                ],
                            },
                        );
                        drop(stream);
                    }
                    _ => {
                        let _ = hub.replay_run(&run, 0);
                    }
                }
                progress.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }
    let started = Instant::now();
    let mut last = 0;
    while started.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(500));
        let now = progress.load(Ordering::Relaxed);
        assert!(now > last, "no progress in 500ms — possible deadlock");
        last = now;
    }
    stop.store(true, Ordering::Relaxed);
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(
        hub.subscription_count(),
        0,
        "every subscription was released"
    );
}
