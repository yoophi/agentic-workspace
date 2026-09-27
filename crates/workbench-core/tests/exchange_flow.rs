//! 040 US2: 교환은 작업대 단위로 격리되고(다른 작업대의 창에 전달 0), 같은 확인은 상태 이벤트를 한 번만 내며,
//! 교환 스트림은 요청·상태를 한 순번 체계로 싣고 작업대가 닫히면 gap으로 끝난다. 전송과 닫기는 경합해도 닫힌
//! 작업대에 작업 영역을 남기지 않는다.

#![allow(clippy::result_large_err)]

mod support;

use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use serde_json::{json, Value};
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_core::ports::{
    agent_workspace_registry::AgentWorkspaceRegistry, desktop_bridge::DesktopDelivery,
};
use workbench_protocol::{
    events::{EXCHANGE_REQUESTED_V1, EXCHANGE_STATUS_V1},
    AuthenticatedPrincipal, EventItem, GapReason, OperationId, StreamCursor, Subscription,
    Workbench,
};

async fn prepare(h: &BenchHarness) -> String {
    let bench = h.open().await;
    h.start(&bench, &format!("{bench}-r1")).await.unwrap();
    h.start(&bench, &format!("{bench}-r2")).await.unwrap();
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::ExchangeSyncWorkspace,
        json!({"benchId": bench, "request": {
            "worktreePath": h.dir, "revision": 1, "focusedPanelId": "main",
            "panels": [
                {"panelId": "main", "title": "Main", "runId": format!("{bench}-r1"), "status": "running"},
                {"panelId": "extra", "title": "Extra", "runId": format!("{bench}-r2"), "status": "running"}
            ]}}),
    )
    .await
    .unwrap();
    bench
}

async fn send(h: &BenchHarness, bench: &str) -> Result<Value, workbench_protocol::WorkbenchFault> {
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::ExchangeSend,
        json!({"benchId": bench, "request": {
            "requestId": "q1", "sourcePanelId": "main", "sourceRunId": format!("{bench}-r1"),
            "targetPanelId": "extra", "targetRunId": format!("{bench}-r2"),
            "message": "hello", "delivery": "send"}}),
    )
    .await
}

fn exchange_deliveries(h: &BenchHarness, bench: &str) -> Vec<DesktopDelivery> {
    h.desktop
        .deliveries_for(bench)
        .into_iter()
        .filter(|delivery| !matches!(delivery, DesktopDelivery::Run { .. }))
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exchanges_are_isolated_per_bench_and_acks_are_idempotent() {
    let h = BenchHarness::new(RunScript::default());
    let a = prepare(&h).await;
    let b = prepare(&h).await;

    let principal = AuthenticatedPrincipal::desktop();
    let mut stream =
        h.rt.runtime
            .events(
                principal.clone(),
                Subscription {
                    cursors: vec![StreamCursor {
                        stream_id: format!("exchange:{a}"),
                        epoch: h.rt.runtime.epoch().into(),
                        after_sequence: 0,
                    }],
                },
            )
            .unwrap();

    send(&h, &a).await.unwrap();
    let delivered = exchange_deliveries(&h, &a);
    assert!(
        matches!(&delivered[0], DesktopDelivery::ExchangeRequested { payload, .. }
        if payload["requestId"] == "q1" && payload["sequence"] == 1)
    );
    assert!(
        matches!(&delivered[1], DesktopDelivery::ExchangeStatus { payload, .. }
        if payload["status"] == "accepted" && payload["sequence"] == 2 && payload.get("windowLabel").is_none())
    );
    assert!(
        exchange_deliveries(&h, &b).is_empty(),
        "other bench receives nothing"
    );

    for _ in 0..2 {
        h.call(
            &principal,
            OperationId::ExchangeAcknowledge,
            json!({"benchId": a, "request": {"requestId": "q1", "targetPanelId": "extra", "outcome": "delivered", "reason": null}}),
        )
        .await
        .unwrap();
    }
    assert_eq!(
        exchange_deliveries(&h, &a).len(),
        3,
        "second identical ack emits nothing"
    );

    let mut schemas = Vec::new();
    for _ in 0..3 {
        match tokio::time::timeout(Duration::from_secs(2), stream.next()).await {
            Ok(Some(EventItem::Event { event })) => schemas.push((event.sequence, event.schema)),
            other => panic!("expected event, got {other:?}"),
        }
    }
    assert_eq!(
        schemas,
        vec![
            (1, EXCHANGE_REQUESTED_V1.to_owned()),
            (2, EXCHANGE_STATUS_V1.to_owned()),
            (3, EXCHANGE_STATUS_V1.to_owned()),
        ]
    );

    // 작업대 닫기 → 구독자 gap, 작업 영역 삭제, 닫힌 작업대 구독도 gap.
    h.close(&a).await.unwrap();
    match tokio::time::timeout(Duration::from_secs(2), stream.next()).await {
        Ok(Some(EventItem::Gap { gap })) => assert_eq!(gap.reason, GapReason::Evicted),
        other => panic!("expected gap, got {other:?}"),
    }
    assert!(h
        .rt
        .runtime
        .benches()
        .exchange_registry
        .snapshot(&a)
        .await
        .is_none());
    let mut late =
        h.rt.runtime
            .events(
                principal,
                Subscription {
                    cursors: vec![StreamCursor {
                        stream_id: format!("exchange:{a}"),
                        epoch: h.rt.runtime.epoch().into(),
                        after_sequence: 0,
                    }],
                },
            )
            .unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(2), late.next()).await,
        Ok(Some(EventItem::Gap { gap })) if gap.reason == GapReason::Evicted
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn send_racing_with_close_leaves_no_workspace_behind() {
    let h = Arc::new(BenchHarness::new(RunScript::default()));
    for round in 0..200 {
        let bench = prepare(&h).await;
        let sender = {
            let h = Arc::clone(&h);
            let bench = bench.clone();
            tokio::spawn(async move { send(&h, &bench).await })
        };
        h.close(&bench).await.unwrap();
        let _ = sender.await.unwrap();
        let registry = &h.rt.runtime.benches().exchange_registry;
        assert!(registry.snapshot(&bench).await.is_none(), "round {round}");
        assert!(
            registry.list_exchanges(&bench).await.is_empty(),
            "round {round}"
        );
    }
}
