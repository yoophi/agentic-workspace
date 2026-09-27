//! 040 세대 범위 멱등성(research R7, Codex 리뷰): 결과 기록이 넘친 뒤의 재시도도 다시 실행되지 않고, 요약 한도에
//! 이르면 새 command를 거절하며, 작업대가 닫힌 뒤의 재시도는 `notFound`로 끝난다.

mod support;

use std::sync::{atomic::Ordering, Arc};

use serde_json::json;
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_core::application::epoch_idempotency::{
    EpochIdempotencyLimits, MESSAGE_CAPACITY_EXHAUSTED, MESSAGE_RESULT_EXPIRED,
};
use workbench_protocol::{FaultCode, OperationId, Outcome};

fn send(bench: &str) -> serde_json::Value {
    json!({"benchId": bench, "runId": "r1", "prompt": "hi"})
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn demoted_retries_never_execute_twice() {
    let h = BenchHarness::with(
        |adapters| {
            adapters.idempotency_limits = EpochIdempotencyLimits {
                max_results: 2,
                max_summaries: 2,
            }
        },
        RunScript::default(),
    );
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    for key in ["k1", "k2", "k3"] {
        h.keyed(OperationId::RunSendPrompt, key, send(&bench))
            .await
            .unwrap();
    }
    assert_eq!(h.engine.prompts.load(Ordering::SeqCst), 3);

    let retry = h
        .keyed(OperationId::RunSendPrompt, "k1", send(&bench))
        .await
        .unwrap_err();
    assert_eq!(
        (retry.code, retry.outcome),
        (FaultCode::Conflict, Outcome::Applied)
    );
    assert_eq!(retry.message, MESSAGE_RESULT_EXPIRED);
    assert_eq!(
        h.engine.prompts.load(Ordering::SeqCst),
        3,
        "no second execution"
    );

    // 요약 2개 한도: k4로 k2가 강등되어 요약이 2개가 되면 그 뒤 새 키는 거절된다.
    h.keyed(OperationId::RunSendPrompt, "k4", send(&bench))
        .await
        .unwrap();
    let full = h
        .keyed(OperationId::RunSendPrompt, "k5", send(&bench))
        .await
        .unwrap_err();
    assert_eq!(full.code, FaultCode::RateLimited);
    assert_eq!(full.message, MESSAGE_CAPACITY_EXHAUSTED);
    assert!(!full.retryable);

    h.close(&bench).await.unwrap();
    let after = h
        .keyed(OperationId::RunSendPrompt, "k1", send(&bench))
        .await
        .unwrap_err();
    assert_eq!(after.code, FaultCode::NotFound);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_same_key_executes_once() {
    let h = Arc::new(BenchHarness::new(RunScript::default()));
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let h = Arc::clone(&h);
        let bench = bench.clone();
        tasks.push(tokio::spawn(async move {
            h.keyed(OperationId::RunSendPrompt, "same", send(&bench))
                .await
        }));
    }
    for task in tasks {
        task.await.unwrap().unwrap();
    }
    assert_eq!(h.engine.prompts.load(Ordering::SeqCst), 1);
}
