//! FR-009 / SC-003: 처리 중 세 지점에서 중단된 뒤 재시작하면 reconciler가 `applied`/`unknown`을 정확히 판정하고,
//! 자동 재실행 없이 같은 키 재요청이 계약대로 응답한다(research R6).

mod support;

use support::{create_request, TestRuntime};
use workbench_core::{
    application::workbench_runtime::CrashPoint, ports::operation_ledger::LedgerState,
};
use workbench_protocol::{FaultCode, Outcome};

fn count(rt: &TestRuntime, state: LedgerState) -> usize {
    rt.runtime.ledger().count_by_state(state).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crash_after_pending_becomes_unknown_and_blocks_same_key() {
    let rt = TestRuntime::new();
    rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AfterPending));

    let fault = rt
        .call(create_request("k1", "AW", "/tmp/aw"))
        .await
        .unwrap_err();
    assert!(fault.message.contains("crash injected"), "{fault}");
    assert_eq!(count(&rt, LedgerState::Pending), 1);
    assert!(
        rt.projects().is_empty(),
        "부작용 전 중단이므로 프로젝트가 없어야 한다"
    );

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Pending), 0);
    assert_eq!(count(&rt, LedgerState::Unknown), 1);
    assert_eq!(count(&rt, LedgerState::Applied), 0);
    assert_eq!(rt.runtime.coordinator().revision(), 0);
    assert!(rt.projects().is_empty());

    // 같은 키 재요청: 자동 재실행하지 않고 unknown을 알린다.
    let fault = rt
        .call(create_request("k1", "AW", "/tmp/aw"))
        .await
        .unwrap_err();
    assert_eq!(fault.code, FaultCode::Conflict);
    assert_eq!(fault.outcome, Outcome::Unknown);
    assert!(!fault.retryable);
    assert!(rt.projects().is_empty());

    // 다른 키는 정상 동작한다.
    let reply = rt
        .call(create_request("k2", "Other", "/tmp/other"))
        .await
        .unwrap();
    assert_eq!(reply.revision(), Some(1));
    assert_eq!(rt.projects().len(), 1);
}

async fn crash_after_side_effect_is_applied_after_restart(point: CrashPoint) {
    let rt = TestRuntime::new();
    rt.runtime.hooks().set_crash_point(Some(point));

    let fault = rt
        .call(create_request("k1", "AW", "/tmp/aw"))
        .await
        .unwrap_err();
    assert!(fault.message.contains("crash injected"), "{fault}");
    assert_eq!(count(&rt, LedgerState::Pending), 1, "{point:?}");
    assert_eq!(rt.projects().len(), 1, "{point:?}: JSON 저장은 이미 끝났다");
    assert_eq!(
        rt.runtime.coordinator().revision(),
        0,
        "{point:?}: applied 전이라 revision은 아직 0"
    );

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Pending), 0, "{point:?}");
    assert_eq!(count(&rt, LedgerState::Applied), 1, "{point:?}");
    assert_eq!(count(&rt, LedgerState::Unknown), 0, "{point:?}");
    assert_eq!(rt.runtime.coordinator().revision(), 1, "{point:?}");
    let projects = rt.projects();
    assert_eq!(projects.len(), 1);
    let id = projects[0]["id"].as_str().unwrap().to_owned();

    // 같은 키 재요청은 재실행이 아니라 저장된 결과를 돌려준다.
    let reply = rt
        .call(create_request("k1", "AW", "/tmp/aw"))
        .await
        .unwrap();
    assert_eq!(reply.output().unwrap()["id"], id);
    assert_eq!(reply.output().unwrap()["name"], "AW");
    assert_eq!(reply.revision(), Some(1));
    assert_eq!(rt.projects().len(), 1, "{point:?}: 중복 생성 없음");
    assert_eq!(count(&rt, LedgerState::Applied), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crash_after_json_save_becomes_applied_after_restart() {
    crash_after_side_effect_is_applied_after_restart(CrashPoint::AfterJsonSave).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crash_before_applied_becomes_applied_after_restart() {
    crash_after_side_effect_is_applied_after_restart(CrashPoint::BeforeApplied).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crash_hook_does_not_leak_into_restarted_runtime() {
    let rt = TestRuntime::new();
    rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AfterPending));
    let _ = rt.call(create_request("k1", "AW", "/tmp/aw")).await;
    let rt = rt.restart();
    let reply = rt
        .call(create_request("k9", "Nine", "/tmp/nine"))
        .await
        .unwrap();
    assert!(reply.revision().is_some());
}

/// Codex 리뷰(2026-09-26) 반영: JSON 저장 뒤 ledger `complete`가 실패해도 `notApplied`로 보고하지 않는다.
/// row는 `pending`으로 남고, 응답은 `unknown`, 재시작 시 `applied`로 확정되며 중복 생성이 없다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ledger_complete_failure_after_save_is_reported_unknown_and_reconciled() {
    use workbench_core::application::workbench_runtime::FailPoint;

    let rt = TestRuntime::new();
    rt.runtime
        .hooks()
        .set_fail_point(Some(FailPoint::LedgerComplete));

    let fault = rt
        .call(create_request("k1", "AW", "/tmp/aw"))
        .await
        .unwrap_err();
    assert_eq!(fault.code, FaultCode::Unavailable, "{fault}");
    assert_eq!(
        fault.outcome,
        Outcome::Unknown,
        "저장이 끝났으므로 notApplied가 아니다"
    );
    assert!(fault.retryable);
    assert_eq!(rt.projects().len(), 1, "프로젝트는 저장돼 있다");
    assert_eq!(
        count(&rt, LedgerState::Pending),
        1,
        "failed로 닫지 않고 pending으로 남긴다"
    );
    assert_eq!(count(&rt, LedgerState::Failed), 0);
    assert_eq!(rt.runtime.coordinator().revision(), 0);

    // 확정 전 같은 키 재요청: 진행 중 충돌(unknown), 재실행 없음
    let fault = rt
        .call(create_request("k1", "AW", "/tmp/aw"))
        .await
        .unwrap_err();
    assert_eq!(fault.code, FaultCode::Conflict);
    assert_eq!(fault.outcome, Outcome::Unknown);
    assert_eq!(rt.projects().len(), 1);

    // 재시작 → reconciler가 JSON에서 예약 id를 찾아 applied로 확정
    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Pending), 0);
    assert_eq!(count(&rt, LedgerState::Applied), 1);
    assert_eq!(rt.runtime.coordinator().revision(), 1);
    let id = rt.projects()[0]["id"].as_str().unwrap().to_owned();

    // 같은 키 재요청은 저장된 결과를 돌려주고, 프로젝트는 여전히 1개
    let reply = rt
        .call(create_request("k1", "AW", "/tmp/aw"))
        .await
        .unwrap();
    assert_eq!(reply.output().unwrap()["id"], id);
    assert_eq!(reply.revision(), Some(1));
    assert_eq!(rt.projects().len(), 1);
}
