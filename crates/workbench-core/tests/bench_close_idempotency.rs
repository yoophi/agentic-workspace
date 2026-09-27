//! 044 #207: 작업대가 닫힌 **뒤에** 끝난 그 작업대의 변경 호출은 epoch 멱등 기록을 되살리지 않는다. 닫은 뒤 같은 키
//! 재시도는 `notFound`이고 효과를 다시 적용하지 않는다(research R12). 시간 지연 없이 엔진의 완료 문(gate)으로 순서를
//! 뒤집는다: 효과 → 작업대 닫기 → 호출 완료.

mod support;

use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use serde_json::json;
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_protocol::{FaultCode, OperationId};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_call_that_finishes_after_its_bench_closed_leaves_no_idempotency_record() {
    let h = Arc::new(BenchHarness::new(RunScript::default()));
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    *h.engine.prompt_gate.lock().unwrap() = Some(gate.clone());
    let input = json!({"benchId": bench, "runId": "r1", "prompt": "late"});

    let pending = {
        let h = h.clone();
        let input = input.clone();
        tokio::spawn(async move { h.keyed(OperationId::RunSendPrompt, "k-late", input).await })
    };
    h.engine
        .wait_applied(|label| label == "prompt:r1:late", Duration::from_secs(10))
        .await;
    assert_eq!(h.rt.runtime.close_all_benches().await, 1, "the bench closes while the call is in flight");

    gate.add_permits(1);
    pending
        .await
        .unwrap()
        .expect("the in-flight call itself still succeeds");
    *h.engine.prompt_gate.lock().unwrap() = None;

    let retry = h
        .keyed(OperationId::RunSendPrompt, "k-late", input)
        .await
        .expect_err("the closed bench's record must not come back");
    assert_eq!(retry.code, FaultCode::NotFound, "{retry:?}");
    assert_eq!(h.engine.prompts.load(Ordering::SeqCst), 1, "no second execution");
}
