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
    let admission = rt.runtime.admit(&bench).expect("admit while open");

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
        .admit(&bench)
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
