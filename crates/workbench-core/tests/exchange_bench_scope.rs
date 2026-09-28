//! Codex r5(crates high): 교환 요청 id는 호출자가 정하므로 작업대마다 겹칠 수 있다. 교환 저장소는 (작업대, 요청 id)로
//! 구별하므로, 관문의 전달 소비·실패 기록도 같은 키여야 한다. 작업대 A의 소비가 작업대 B의 같은 id 교환을 막거나(이미
//! 소비됨), B의 미소비 교환을 활동 작업에서 빼서 정지를 허용하면 안 된다.

#![allow(clippy::result_large_err)]

mod support;

use serde_json::{json, Value};
use support::{command_request, query_request, scripted_run_engine::RunScript, BenchHarness};
use workbench_core::application::work_gate::DrainMode;
use workbench_protocol::{AuthenticatedPrincipal, OperationId, Workbench, WorkbenchFault};

const EXCHANGE: &str = "same-id";
const KEY: &str = "exchange-delivery:same-id";

/// 작업대 하나(작업 디렉터리 `dir`)에 run 둘(`from` → `to`)과 교환 작업 영역을 만든다.
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
    bench
}

/// 교환 요청(`queue`) + 화면의 확인(`delivered`): 전달 prompt는 아직 소비되지 않았다.
async fn request_exchange(h: &BenchHarness, bench: &str, from: &str, to: &str) {
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

/// 두 작업대에 같은 id의 교환이 있다(A: r1 → r2, B: r3 → r4).
async fn two_benches(h: &BenchHarness) -> (String, String) {
    let second = format!("{}/second", h.dir);
    std::fs::create_dir_all(&second).unwrap();
    let a = prepare(h, &h.dir, "r1", "r2").await;
    let b = prepare(h, &second, "r3", "r4").await;
    assert_ne!(a, b);
    request_exchange(h, &a, "r1", "r2").await;
    request_exchange(h, &b, "r3", "r4").await;
    (a, b)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_same_exchange_id_is_delivered_once_in_each_bench() {
    let h = BenchHarness::new(RunScript::default());
    let (a, b) = two_benches(&h).await;
    deliver(&h, &a, "r2")
        .await
        .expect("bench A delivers its exchange");
    deliver(&h, &b, "r4")
        .await
        .expect("bench B's own exchange with the same id is not consumed by bench A");
    // 같은 키·같은 입력 재시도는 멱등 결과(효과 없음)다. 각 작업대의 대상 run은 전달 prompt를 정확히 한 번 받는다.
    deliver(&h, &b, "r4").await.expect("an idempotent retry");
    let applied = h.engine.applied();
    for run in ["r2", "r4"] {
        let label = format!("prompt:{run}:hello peer");
        assert_eq!(
            applied.iter().filter(|seen| **seen == label).count(),
            1,
            "{run} got the delivery exactly once: {applied:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn consuming_in_one_bench_leaves_the_other_benchs_exchange_pending_and_blocks_the_stop() {
    let h = BenchHarness::new(RunScript::default());
    let (a, b) = two_benches(&h).await;
    owner(
        &h,
        OperationId::LeaseAcquire,
        json!({"clientKind": "desktop", "clientId": "app"}),
    )
    .await;
    let before = owner(&h, OperationId::ServerStatus, json!({})).await;
    assert_eq!(before["activeWork"]["pendingExchanges"], 2, "{before}");

    deliver(&h, &a, "r2").await.expect("bench A delivers");
    let after = owner(&h, OperationId::ServerStatus, json!({})).await;
    assert_eq!(
        after["activeWork"]["pendingExchanges"], 1,
        "bench B's exchange is still pending: {after}"
    );

    let control = h.rt.runtime.server_control();
    control.work_gate().begin_drain(DrainMode::Wait);
    assert!(
        !control.try_stop().await,
        "the stop waits for bench B's pending exchange"
    );
    deliver(&h, &b, "r4")
        .await
        .expect("bench B delivers (continuation)");
    let done = owner(&h, OperationId::ServerStatus, json!({})).await;
    assert_eq!(done["activeWork"]["pendingExchanges"], 0, "{done}");
}

/// 작업대를 닫으면 그 작업대의 교환 소비 기록도 지운다(교환 자체가 지워지고 작업대 id는 다시 쓰이지 않는다). 다른
/// 작업대의 기록은 남는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closing_a_bench_drops_only_its_exchange_records() {
    let h = BenchHarness::new(RunScript::default());
    let (a, b) = two_benches(&h).await;
    deliver(&h, &a, "r2").await.expect("bench A delivers");
    deliver(&h, &b, "r4").await.expect("bench B delivers");
    let gate = h.rt.runtime.work_gate();
    assert!(gate.exchange_consumed(&a, EXCHANGE) && gate.exchange_consumed(&b, EXCHANGE));
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::BenchClose,
        json!({ "benchId": a }),
    )
    .await
    .expect("close bench A");
    assert!(
        !gate.exchange_consumed(&a, EXCHANGE),
        "bench A's record is gone"
    );
    assert!(
        gate.exchange_consumed(&b, EXCHANGE),
        "bench B's record stays"
    );
}
