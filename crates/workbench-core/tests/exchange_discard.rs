//! Codex r7(apps medium): 화면은 교환 prompt를 패널 대기열에 넣고 전달 확인(`delivered`)을 먼저 보낸다. 대기열에서 그 항목을
//! **지우면** 서버에는 확인됐지만 소비되지 않은 교환이 남아, 대상 run이 살아 있고 데스크톱 임대가 있는 동안
//! `pendingExchanges`로 계속 세어져 wait-stop이 끝나지 않는다. `exchange.discardDelivery`(C)는 그 교환의 전달을 포기한다:
//! 소비된 것으로 표시해 활동 작업에서 빼고, 이후 같은 교환의 전달(이어 가기)은 이미 소비됨으로 거절된다. 작업대 범위이고
//! 멱등이다.

#![allow(clippy::result_large_err)]

mod support;

use serde_json::{json, Value};
use support::{command_request, query_request, scripted_run_engine::RunScript, BenchHarness};
use workbench_core::application::work_gate::DrainMode;
use workbench_protocol::{
    AuthenticatedPrincipal, FaultCode, OperationId, Workbench, WorkbenchFault,
};

const EXCHANGE: &str = "x-discard";
const KEY: &str = "exchange-delivery:x-discard";

async fn prepare(h: &BenchHarness, dir: &str, from: &str, to: &str) -> String {
    let bench = h
        .call(
            &AuthenticatedPrincipal::desktop(),
            OperationId::BenchOpen,
            json!({ "workingDirectory": dir }),
        )
        .await
        .expect("bench.open")["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    h.start(&bench, from).await.unwrap();
    h.start(&bench, to).await.unwrap();
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::ExchangeSyncWorkspace,
        json!({"benchId": bench, "request": {
        "worktreePath": dir, "revision": 1, "focusedPanelId": "main",
        "panels": [
            {"panelId": "main", "title": "Main", "runId": from, "status": "running"},
            {"panelId": "extra", "title": "Extra", "runId": to, "status": "running"}
        ]}}),
    )
    .await
    .unwrap();
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::ExchangeSend,
        json!({"benchId": bench, "request": {
            "requestId": EXCHANGE, "sourcePanelId": "main", "sourceRunId": from,
            "targetPanelId": "extra", "targetRunId": to,
            "message": "hello peer", "delivery": "queue"}}),
    )
    .await
    .unwrap();
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::ExchangeAcknowledge,
        json!({"benchId": bench, "request": {
            "requestId": EXCHANGE, "targetPanelId": "extra", "outcome": "delivered", "reason": null}}),
    )
    .await
    .unwrap();
    bench
}

async fn discard(h: &BenchHarness, bench: &str, request_id: &str) -> Result<Value, WorkbenchFault> {
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::ExchangeDiscardDelivery,
        json!({"benchId": bench, "requestId": request_id}),
    )
    .await
}

async fn deliver(h: &BenchHarness, bench: &str, run: &str) -> Result<Value, WorkbenchFault> {
    h.keyed(
        OperationId::RunSendPrompt,
        KEY,
        json!({"benchId": bench, "runId": run, "prompt": "hello peer",
            "continuation": {"exchangeRequestId": EXCHANGE}}),
    )
    .await
}

async fn owner(h: &BenchHarness, operation: OperationId, input: Value) -> Value {
    let request = if operation == OperationId::ServerStatus {
        query_request(operation, input)
    } else {
        command_request(operation, &support::uuid_key(), input)
    };
    h.rt.runtime
        .call(AuthenticatedPrincipal::owner(), request)
        .await
        .map(|reply| reply.output().cloned().unwrap_or(Value::Null))
        .unwrap()
}

async fn pending(h: &BenchHarness) -> Value {
    owner(h, OperationId::ServerStatus, json!({})).await["activeWork"]["pendingExchanges"].clone()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discarding_a_queued_exchange_ends_it_so_the_wait_stop_completes() {
    let h = BenchHarness::new(RunScript::default());
    let bench = prepare(&h, &h.dir, "r1", "r2").await;
    owner(
        &h,
        OperationId::LeaseAcquire,
        json!({"clientKind": "desktop", "clientId": "app"}),
    )
    .await;
    assert_eq!(pending(&h).await, 1);
    let control = h.rt.runtime.server_control();
    control.work_gate().begin_drain(DrainMode::Wait);
    assert!(
        !control.try_stop().await,
        "the acknowledged but unconsumed exchange blocks the stop"
    );

    // 비우는 중에도 받는다(C: 이미 있는 교환을 끝낸다).
    discard(&h, &bench, EXCHANGE)
        .await
        .expect("discard is accepted while draining");
    assert_eq!(pending(&h).await, 0, "the discarded exchange is no longer active");
    assert!(control.try_stop().await, "the wait-stop completes");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_discarded_exchange_is_not_delivered_later_and_discard_is_idempotent() {
    let h = BenchHarness::new(RunScript::default());
    let bench = prepare(&h, &h.dir, "r1", "r2").await;
    discard(&h, &bench, EXCHANGE).await.expect("discard");
    discard(&h, &bench, EXCHANGE)
        .await
        .expect("a second discard is a no-op");
    let refused = deliver(&h, &bench, "r2")
        .await
        .expect_err("a discarded exchange is not delivered to the agent");
    assert_eq!(refused.code, FaultCode::Conflict, "{refused:?}");
    assert!(
        !h.engine
            .applied()
            .iter()
            .any(|seen| seen == "prompt:r2:hello peer"),
        "the agent never received the discarded prompt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discarding_after_the_delivery_changes_nothing() {
    let h = BenchHarness::new(RunScript::default());
    let bench = prepare(&h, &h.dir, "r1", "r2").await;
    deliver(&h, &bench, "r2").await.expect("delivered once");
    discard(&h, &bench, EXCHANGE)
        .await
        .expect("discard after delivery is a no-op");
    let applied = h.engine.applied();
    assert_eq!(
        applied
            .iter()
            .filter(|seen| *seen == "prompt:r2:hello peer")
            .count(),
        1,
        "{applied:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discard_is_scoped_to_the_bench_and_its_exchanges() {
    let h = BenchHarness::new(RunScript::default());
    let second = format!("{}/second", h.dir);
    std::fs::create_dir_all(&second).unwrap();
    let a = prepare(&h, &h.dir, "r1", "r2").await;
    let b = prepare(&h, &second, "r3", "r4").await;
    owner(
        &h,
        OperationId::LeaseAcquire,
        json!({"clientKind": "desktop", "clientId": "app"}),
    )
    .await;
    assert_eq!(pending(&h).await, 2);
    discard(&h, &a, EXCHANGE).await.expect("discard in bench A");
    assert_eq!(
        pending(&h).await,
        1,
        "bench B's exchange with the same id is still pending"
    );
    deliver(&h, &b, "r4")
        .await
        .expect("bench B still delivers its own exchange");

    let unknown = discard(&h, &a, "no-such-exchange")
        .await
        .expect_err("an unknown exchange");
    assert_eq!(unknown.code, FaultCode::NotFound, "{unknown:?}");
}
