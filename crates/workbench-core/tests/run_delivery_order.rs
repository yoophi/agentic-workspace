//! 039 research R6: 한 run에 여러 발행자(ACP 본 흐름·stderr 진단·steer/cancel)가 동시에 발행해도 데스크톱 `deliver`가
//! 불리는 순서는 순번 순서와 같고 빠진 번호가 없다. `deliver`가 순번 부여와 같은 스트림 lock 안에서 불리기 때문이다.

mod support;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use acp_agent_core::domain::events::RunEvent;
use support::TestRuntime;
use workbench_core::ports::event_publisher::RunEventPublisher;

fn diagnostic(message: &str) -> RunEvent {
    RunEvent::Diagnostic {
        message: message.into(),
    }
}

/// 순번 11의 전달이 50ms 멈춘 사이 다른 발행자가 발행해도 12가 11을 앞지르지 않는다.
#[test]
fn delayed_delivery_of_eleven_is_not_overtaken_by_twelve() {
    let rt = TestRuntime::new();
    let runtime = Arc::clone(&rt.runtime);
    let delivered = Arc::new(Mutex::new(Vec::<u64>::new()));
    for index in 0..10 {
        let delivered = Arc::clone(&delivered);
        runtime.publish_run(
            "r",
            &diagnostic(&index.to_string()),
            false,
            &mut |envelope| delivered.lock().unwrap().push(envelope.sequence),
        );
    }
    let slow = {
        let runtime = Arc::clone(&runtime);
        let delivered = Arc::clone(&delivered);
        std::thread::spawn(move || {
            runtime.publish_run("r", &diagnostic("slow"), false, &mut |envelope| {
                assert_eq!(envelope.sequence, 11);
                std::thread::sleep(Duration::from_millis(50));
                delivered.lock().unwrap().push(envelope.sequence);
            });
        })
    };
    std::thread::sleep(Duration::from_millis(10));
    let delivered_fast = Arc::clone(&delivered);
    runtime.publish_run("r", &diagnostic("fast"), false, &mut |envelope| {
        delivered_fast.lock().unwrap().push(envelope.sequence)
    });
    slow.join().unwrap();
    assert_eq!(*delivered.lock().unwrap(), (1..=12).collect::<Vec<_>>());
}

/// 네 발행자가 무작위 지연으로 같은 run에 1,000건 발행 → 전달 순서 = 1..=1000.
#[test]
fn concurrent_publishers_deliver_in_sequence_order() {
    let rt = TestRuntime::new();
    let runtime = Arc::clone(&rt.runtime);
    let delivered = Arc::new(Mutex::new(Vec::<u64>::new()));
    let workers: Vec<_> = (0..4u64)
        .map(|worker| {
            let runtime = Arc::clone(&runtime);
            let delivered = Arc::clone(&delivered);
            std::thread::spawn(move || {
                for index in 0..250u64 {
                    let pause = (worker * 31 + index * 17) % 5;
                    runtime.publish_run("r", &diagnostic("x"), false, &mut |envelope| {
                        if pause == 0 {
                            std::thread::sleep(Duration::from_micros(200));
                        }
                        delivered.lock().unwrap().push(envelope.sequence);
                    });
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(*delivered.lock().unwrap(), (1..=1_000).collect::<Vec<_>>());
}
