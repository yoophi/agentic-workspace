//! 040 리뷰 반영: 작업대에 속한 스트림(`exchange:<id>`·`bench:<id>`)은 작업대를 연 주체만 구독한다(다른 주체·agent·
//! 모르는 id 거절, 닫힌 작업대는 gap). agent의 교환 전송은 닫는 중인 작업대에 입장하지 못하고, 닫기 뒤 교환 상태가
//! 남지 않는다.

#![allow(clippy::result_large_err)]

mod support;

use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use serde_json::json;
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_core::ports::agent_workspace_registry::AgentWorkspaceRegistry;
use workbench_protocol::{
    AuthenticatedPrincipal, EventItem, FaultCode, GapReason, OperationId, StreamCursor,
    Subscription, Workbench, WorkbenchFault,
};

fn subscribe(
    h: &BenchHarness,
    principal: AuthenticatedPrincipal,
    stream_id: String,
) -> Result<workbench_protocol::EventStream, WorkbenchFault> {
    h.rt.runtime.events(
        principal,
        Subscription {
            cursors: vec![StreamCursor {
                stream_id,
                epoch: h.rt.runtime.epoch().into(),
                after_sequence: 0,
            }],
        },
    )
}

fn rejected(
    result: Result<workbench_protocol::EventStream, WorkbenchFault>,
) -> (FaultCode, String) {
    match result {
        Ok(_) => panic!("subscription must be rejected"),
        Err(fault) => (fault.code, fault.message),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bench_streams_are_only_subscribable_by_the_opening_principal() {
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();

    for kind in ["exchange", "bench"] {
        let stream = format!("{kind}:{bench}");
        assert!(subscribe(&h, AuthenticatedPrincipal::desktop(), stream.clone()).is_ok());
        assert_eq!(
            rejected(subscribe(
                &h,
                AuthenticatedPrincipal::test_as("desktop2"),
                stream.clone()
            )),
            (
                FaultCode::Forbidden,
                "bench belongs to another principal.".to_owned()
            )
        );
        // agent는 자기 run의 작업대라도 구독하지 않는다(MCP 도구는 요청·응답만 쓴다).
        assert_eq!(
            rejected(subscribe(
                &h,
                AuthenticatedPrincipal::agent("r1"),
                stream.clone()
            ))
            .0,
            FaultCode::Forbidden
        );
        // 한 번도 없던 id는 나중에 열릴 스트림에 미리 붙지 않도록 거절한다.
        assert_eq!(
            rejected(subscribe(
                &h,
                AuthenticatedPrincipal::desktop(),
                format!("{kind}:never-opened")
            )),
            (FaultCode::NotFound, "bench not found.".to_owned())
        );
    }
    // 여러 cursor 중 하나라도 남의 작업대면 전체를 거절한다(일부만 등록되지 않는다).
    let other = h.open().await;
    let mixed = h.rt.runtime.events(
        AuthenticatedPrincipal::test_as("desktop2"),
        Subscription {
            cursors: vec![StreamCursor {
                stream_id: format!("exchange:{other}"),
                epoch: h.rt.runtime.epoch().into(),
                after_sequence: 0,
            }],
        },
    );
    assert_eq!(rejected(mixed).0, FaultCode::Forbidden);

    // 닫힌 작업대: 연 주체의 구독은 `Gap(evicted)`로 끝난다(계약 그대로).
    h.close(&bench).await.unwrap();
    let mut late = subscribe(
        &h,
        AuthenticatedPrincipal::desktop(),
        format!("exchange:{bench}"),
    )
    .unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(2), late.next()).await,
        Ok(Some(EventItem::Gap { gap })) if gap.reason == GapReason::Evicted
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn agent_send_is_rejected_while_the_bench_is_closing() {
    let h = Arc::new(BenchHarness::new(RunScript::default()));
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    h.start(&bench, "r2").await.unwrap();
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::ExchangeSyncWorkspace,
        json!({"benchId": bench, "request": {
        "worktreePath": h.dir, "revision": 1, "focusedPanelId": "main",
        "panels": [
            {"panelId": "main", "title": "Main", "runId": "r1", "status": "running"},
            {"panelId": "extra", "title": "Extra", "runId": "r2", "status": "running"}
        ]}}),
    )
    .await
    .unwrap();

    // 먼저 입장한 동작을 흉내 내 닫기를 `Closing`에 붙잡아 둔다.
    let held = h.rt.runtime.admit(&bench).unwrap();
    let closer = {
        let h = Arc::clone(&h);
        let bench = bench.clone();
        tokio::spawn(async move { h.close(&bench).await })
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while h.rt.runtime.admit(&bench).is_ok() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "bench never entered Closing"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    // run은 아직 살아 있지만(닫기가 입장한 동작을 기다리는 중) 전송은 입장하지 못한다.
    let fault = h
        .call(
            &AuthenticatedPrincipal::agent("r1"),
            OperationId::ExchangeSendFromRun,
            json!({"runId": "r1", "request": {
                "requestId": "q1", "sourcePanelId": "", "sourceRunId": "r1",
                "targetPanelId": "extra", "targetRunId": "r2",
                "message": "hello", "delivery": "send"}}),
        )
        .await
        .unwrap_err();
    assert_eq!(
        (fault.code, fault.message.as_str()),
        (FaultCode::NotFound, "bench not found.")
    );

    drop(held);
    closer.await.unwrap().unwrap();
    let registry = &h.rt.runtime.benches().exchange_registry;
    assert!(registry.snapshot(&bench).await.is_none());
    assert!(registry.list_exchanges(&bench).await.is_empty());
}
