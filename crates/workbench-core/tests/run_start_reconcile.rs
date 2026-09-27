//! 040 US1: `run.start`는 변경 기록을 통과한다. apply 뒤 중단되면 재시작 판정은 `unknown`(ADR core 0005),
//! 완료된 `run.start`의 재시도는 저장된 결과를 돌려준다.

mod support;

use serde_json::json;
use support::{command_request, scripted_run_engine::RunScript, BenchHarness};
use workbench_core::{
    application::workbench_runtime::CrashPoint, ports::operation_ledger::LedgerState,
};
use workbench_protocol::{FaultCode, OperationId, Outcome};

fn start_input(bench: &str) -> serde_json::Value {
    json!({"benchId": bench, "request": {"goal": "g", "agentId": "codex", "runId": "r1"}})
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn completed_start_replays_the_stored_run() {
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    let first = h
        .keyed(OperationId::RunStart, "k-start", start_input(&bench))
        .await
        .unwrap();
    let again = h
        .keyed(OperationId::RunStart, "k-start", start_input(&bench))
        .await
        .unwrap();
    assert_eq!(first, again);
    assert_eq!(
        h.engine.starts.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "replay must not start the agent again"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interrupted_start_is_unknown_after_restart() {
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    h.rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AfterJsonSave));
    let crashed =
        h.rt.call(command_request(
            OperationId::RunStart,
            "k-crash",
            start_input(&bench),
        ))
        .await
        .unwrap_err();
    assert_eq!(crashed.code, FaultCode::Internal);

    let restarted = h.rt.restart();
    assert_eq!(
        restarted
            .runtime
            .ledger()
            .count_by_state(LedgerState::Unknown)
            .unwrap(),
        1
    );
    // 작업대는 메모리 전용이라 재시작 뒤에는 없다: 재시도는 ledger 판정 전에 `notFound`로 끝나 중복 기동이 없다.
    let retry = restarted
        .call(command_request(
            OperationId::RunStart,
            "k-crash",
            start_input(&bench),
        ))
        .await
        .unwrap_err();
    assert_eq!(
        (retry.code, retry.outcome),
        (FaultCode::NotFound, Outcome::NotApplied)
    );
    assert_eq!(retry.message, "bench not found.");
}
