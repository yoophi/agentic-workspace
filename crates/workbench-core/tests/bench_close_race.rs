//! 040 닫기 경합(research R1, Codex 리뷰): 입장 guard를 쥔 동작이 끝나기 전에 닫기가 끝나지 않고, 닫히는 중의
//! 입장은 `notFound`이며, 겹친 두 닫기 중 두 번째는 첫 번째를 기다린다.

mod support;

use std::time::Duration;

use support::TestRuntime;
use workbench_protocol::{AuthenticatedPrincipal, FaultCode, RequestId};

fn open(rt: &TestRuntime) -> String {
    let dir = rt.paths.app_data_dir().join("bench-work");
    std::fs::create_dir_all(&dir).unwrap();
    rt.runtime
        .benches()
        .open(
            &RequestId::random(),
            &AuthenticatedPrincipal::desktop(),
            dir.to_str().unwrap(),
        )
        .unwrap()
        .bench_id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn close_waits_for_admitted_work_and_rejects_new_admissions() {
    let rt = TestRuntime::new();
    let bench = open(&rt);
    let admission = rt
        .runtime
        .benches()
        .admit(&workbench_protocol::RequestId::random(), None, &bench)
        .expect("admit while open");

    let services = rt.runtime.benches().clone();
    let closing_bench = bench.clone();
    let first = tokio::spawn(async move {
        services
            .close(
                &RequestId::random(),
                &AuthenticatedPrincipal::desktop(),
                &closing_bench,
            )
            .await
            .unwrap()
    });
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !first.is_finished(),
        "close must wait for the admitted operation"
    );
    let fault = rt
        .runtime
        .benches()
        .admit(&workbench_protocol::RequestId::random(), None, &bench)
        .err()
        .expect("closing bench refuses admission");
    assert_eq!(fault.code, FaultCode::NotFound);

    let services = rt.runtime.benches().clone();
    let second_bench = bench.clone();
    let second = tokio::spawn(async move {
        services
            .close(
                &RequestId::random(),
                &AuthenticatedPrincipal::desktop(),
                &second_bench,
            )
            .await
            .unwrap()
    });
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(!second.is_finished(), "second close waits for the first");

    drop(admission);
    assert!(first.await.unwrap().closed);
    assert!(!second.await.unwrap().closed);
    assert!(rt.runtime.benches().registry.is_empty());
}

/// run.start(입장 구간을 늘린 가짜 엔진)와 bench.close를 동시에 반복한다. 닫기가 반환한 뒤 그 작업대 소유 run은
/// 0개이고, start는 성공(닫기 전 입장)하거나 `notFound`(닫기 후)다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn start_racing_with_close_never_leaves_runs_behind() {
    use support::{scripted_run_engine::RunScript, BenchHarness};
    let h = std::sync::Arc::new(BenchHarness::new(RunScript {
        start_delay_ms: 1,
        ..RunScript::default()
    }));
    for round in 0..1_000 {
        let bench = h.open().await;
        let starter = {
            let h = std::sync::Arc::clone(&h);
            let bench = bench.clone();
            tokio::spawn(async move { h.start(&bench, &format!("r{round}")).await })
        };
        if round % 2 == 0 {
            tokio::task::yield_now().await;
        }
        let closed = h.close(&bench).await.unwrap();
        assert!(closed["closed"].as_bool().unwrap());
        match starter.await.unwrap() {
            Ok(_) => {}
            Err(fault) => assert_eq!(fault.code, FaultCode::NotFound, "round {round}: {fault:?}"),
        }
        assert_eq!(
            h.engine.runs_owned_by(&bench),
            0,
            "round {round}: run left behind"
        );
    }
}

/// 오래 걸리는 `cancelAndSend`(입장하지 않는 제어)가 닫기를 막지 않는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn long_control_does_not_block_close() {
    use support::{scripted_run_engine::RunScript, BenchHarness};
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let closed = tokio::time::timeout(Duration::from_secs(2), h.close(&bench))
        .await
        .expect("close must not wait for controls");
    assert!(closed.unwrap()["closed"].as_bool().unwrap());
}

/// 코드 리뷰 반영: 닫기를 부른 쪽이 `Closing` 전이 뒤 사라져도(연결 끊김 → future 취소) 정리는 끝까지 간다.
/// 작업대가 `Closing`에 멈춰 작업대 자리(상한)·run·스트림을 붙잡지 않는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn close_completes_even_if_the_caller_is_cancelled() {
    let rt = TestRuntime::new();
    let bench = open(&rt);
    let held = rt
        .runtime
        .benches()
        .admit(&workbench_protocol::RequestId::random(), None, &bench)
        .expect("admit while open");
    let caller = {
        let services = std::sync::Arc::clone(rt.runtime.benches());
        let bench = bench.clone();
        tokio::spawn(async move {
            services
                .close(
                    &RequestId::random(),
                    &AuthenticatedPrincipal::desktop(),
                    &bench,
                )
                .await
        })
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while rt
        .runtime
        .benches()
        .admit(&workbench_protocol::RequestId::random(), None, &bench)
        .is_ok()
    {
        assert!(
            tokio::time::Instant::now() < deadline,
            "bench never entered Closing"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    caller.abort();
    let _ = caller.await;
    drop(held);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while rt.runtime.benches().registry.owner(&bench).is_some() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "close never finished after caller cancelled"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    // 다음 닫기는 이미 끝난 작업대를 모른다(대기 없이 `closed: false`).
    let again = rt
        .runtime
        .benches()
        .close(
            &RequestId::random(),
            &AuthenticatedPrincipal::desktop(),
            &bench,
        )
        .await
        .unwrap();
    assert!(!again.closed);
}
