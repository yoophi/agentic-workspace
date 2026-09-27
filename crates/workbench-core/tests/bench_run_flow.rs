//! 040 US1: run 수명(작업대 열기 → 시작 → 권한 → 프롬프트 → 닫기), 작업대 단위 데스크톱 전달, 구독 끊김 ≠ 취소.

mod support;

use serde_json::json;
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_core::ports::desktop_bridge::DesktopDelivery;
use workbench_protocol::{
    AuthenticatedPrincipal, OperationId, StreamCursor, Subscription, Workbench,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_lifecycle_delivers_to_its_bench_and_close_cancels() {
    let h = BenchHarness::new(RunScript {
        permission_id: Some("p1".into()),
        ..RunScript::default()
    });
    let desktop = AuthenticatedPrincipal::desktop();
    let bench = h.open().await;
    let other = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    h.call(
        &desktop,
        OperationId::RunRespondPermission,
        json!({"benchId": bench, "runId": "r1", "permissionId": "p1", "optionId": "allow"}),
    )
    .await
    .unwrap();
    h.call(
        &desktop,
        OperationId::RunSendPrompt,
        json!({"benchId": bench, "runId": "r1", "prompt": "hello"}),
    )
    .await
    .unwrap();

    // 전달은 run의 작업대로만, 순번 순서로.
    let deliveries = h.desktop.deliveries_for(&bench);
    let sequences: Vec<u64> = deliveries
        .iter()
        .map(|delivery| match delivery {
            DesktopDelivery::Run { payload, .. } => payload["sequence"].as_u64().unwrap(),
            other => panic!("unexpected delivery {other:?}"),
        })
        .collect();
    assert_eq!(sequences, vec![1, 2, 3], "started, permission, message");
    assert!(h.desktop.deliveries_for(&other).is_empty());
    assert_eq!(h.desktop.launches.lock().unwrap()[0].bench_id, bench);

    // 구독이 끊겨도 run은 계속된다(ADR 0005).
    let subscription =
        h.rt.runtime
            .events(
                desktop.clone(),
                Subscription {
                    cursors: vec![StreamCursor {
                        stream_id: "run:r1".into(),
                        epoch: h.rt.runtime.epoch().into(),
                        after_sequence: 0,
                    }],
                },
            )
            .unwrap();
    drop(subscription);
    assert_eq!(h.engine.runs_owned_by(&bench), 1);

    let closed = h.close(&bench).await.unwrap();
    assert_eq!(closed, json!({"closed": true, "cancelledRuns": ["r1"]}));
    assert_eq!(h.engine.runs_owned_by(&bench), 0);
    assert_eq!(h.close(&bench).await.unwrap()["closed"], false);
}
