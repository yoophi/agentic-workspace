//! 038 US1 재시작 판정(research R6): 저장 단위 4개의 변경에 대해 중단 지점별 applied/unknown 판정과
//! "자동 재실행 없음"을 확인한다. 037의 `ledger_crash_points.rs`는 손대지 않고 새 파일에 둔다.

mod support;

use serde_json::json;
use support::{command_request, query_request, TestRuntime};
use workbench_core::{
    application::workbench_runtime::{CrashPoint, FailPoint},
    ports::operation_ledger::LedgerState,
};
use workbench_protocol::{CallRequest, FaultCode, OperationId, Outcome};

fn count(rt: &TestRuntime, state: LedgerState) -> usize {
    rt.runtime.ledger().count_by_state(state).unwrap()
}

fn saved_prompt_create(key: &str, label: &str) -> CallRequest {
    command_request(
        OperationId::SavedPromptCreate,
        key,
        json!({ "label": label, "prompt": "keep going" }),
    )
}

fn saved_prompt_delete(key: &str, id: &str) -> CallRequest {
    command_request(OperationId::SavedPromptDelete, key, json!({ "id": id }))
}

fn goal_create(key: &str, objective: &str) -> CallRequest {
    command_request(
        OperationId::GoalCreate,
        key,
        json!({ "workingDirectory": "/repo/wt", "objective": objective, "tokenBudget": 1000 }),
    )
}

fn goal_progress(key: &str, tokens: u64) -> CallRequest {
    command_request(
        OperationId::GoalRecordProgress,
        key,
        json!({ "workingDirectory": "/repo/wt", "tokensUsed": tokens, "timeUsedSeconds": 1 }),
    )
}

fn settings_save(key: &str) -> CallRequest {
    command_request(
        OperationId::AgentRunSettingsSave,
        key,
        json!({ "settings": { "workingDirectory": "/repo/wt", "agentId": "codex" } }),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saved_prompt_create_after_pending_is_unknown_and_blocks_same_key() {
    let rt = TestRuntime::new();
    rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AfterPending));
    let fault = rt.call(saved_prompt_create("k1", "A")).await.unwrap_err();
    assert!(fault.message.contains("crash injected"), "{fault}");
    assert_eq!(count(&rt, LedgerState::Pending), 1);
    assert!(rt.saved_prompts().is_empty());

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Unknown), 1);
    assert_eq!(count(&rt, LedgerState::Applied), 0);
    assert!(rt.saved_prompts().is_empty());

    let fault = rt.call(saved_prompt_create("k1", "A")).await.unwrap_err();
    assert_eq!(fault.code, FaultCode::Conflict);
    assert_eq!(fault.outcome, Outcome::Unknown);
    assert!(!fault.retryable);

    let reply = rt.call(saved_prompt_create("k2", "B")).await.unwrap();
    assert_eq!(reply.revision(), Some(1));
    assert_eq!(rt.saved_prompts().len(), 1);
}

async fn saved_prompt_create_after_side_effect(point: CrashPoint) {
    let rt = TestRuntime::new();
    rt.runtime.hooks().set_crash_point(Some(point));
    let fault = rt.call(saved_prompt_create("k1", "A")).await.unwrap_err();
    assert!(
        fault.message.contains("crash injected"),
        "{point:?}: {fault}"
    );
    assert_eq!(rt.saved_prompts().len(), 1, "{point:?}: JSON 저장은 끝났다");
    assert_eq!(count(&rt, LedgerState::Pending), 1);

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Applied), 1, "{point:?}");
    assert_eq!(count(&rt, LedgerState::Unknown), 0, "{point:?}");
    assert_eq!(rt.runtime.coordinator().revision_of("saved-prompts"), 1);
    let id = rt.saved_prompts()[0]["id"].as_str().unwrap().to_owned();

    let reply = rt.call(saved_prompt_create("k1", "A")).await.unwrap();
    assert_eq!(
        reply.output().unwrap()["id"],
        id,
        "{point:?}: 저장된 결과 재생"
    );
    assert_eq!(rt.saved_prompts().len(), 1, "{point:?}: 중복 없음");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saved_prompt_create_after_json_save_is_applied_after_restart() {
    saved_prompt_create_after_side_effect(CrashPoint::AfterJsonSave).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saved_prompt_create_before_applied_is_applied_after_restart() {
    saved_prompt_create_after_side_effect(CrashPoint::BeforeApplied).await;
}

/// 수정(upsert)은 관찰로 적용 여부를 알 수 없으므로 unknown이다. 파일에는 반영돼 있고 재실행되지 않는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn goal_record_progress_after_json_save_is_unknown() {
    let rt = TestRuntime::new();
    rt.call(goal_create("g1", "Ship")).await.unwrap();
    rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AfterJsonSave));
    let fault = rt.call(goal_progress("p1", 40)).await.unwrap_err();
    assert!(fault.message.contains("crash injected"), "{fault}");
    assert_eq!(rt.goals()[0]["tokensUsed"], 40, "파일에는 반영됐다");

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Unknown), 1);
    assert_eq!(count(&rt, LedgerState::Applied), 1, "goal.create만 applied");
    assert_eq!(rt.goals()[0]["tokensUsed"], 40);

    let fault = rt.call(goal_progress("p1", 40)).await.unwrap_err();
    assert_eq!(fault.code, FaultCode::Conflict);
    assert_eq!(fault.outcome, Outcome::Unknown);
    assert!(!fault.retryable);
    // 재조회로 실제 상태 확인 후 새 키로 진행
    let reply = rt
        .call(query_request(
            OperationId::GoalGet,
            json!({ "workingDirectory": "/repo/wt" }),
        ))
        .await
        .unwrap();
    assert_eq!(reply.output().unwrap()["tokensUsed"], 40);
    let reply = rt.call(goal_progress("p2", 70)).await.unwrap();
    assert_eq!(reply.output().unwrap()["tokensUsed"], 70);
}

/// 삭제는 종료 상태 규칙: 대상이 없으면 applied.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saved_prompt_delete_after_json_save_is_applied() {
    let rt = TestRuntime::new();
    let reply = rt.call(saved_prompt_create("c1", "A")).await.unwrap();
    let id = reply.output().unwrap()["id"].as_str().unwrap().to_owned();
    rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AfterJsonSave));
    let fault = rt.call(saved_prompt_delete("d1", &id)).await.unwrap_err();
    assert!(fault.message.contains("crash injected"), "{fault}");
    assert!(rt.saved_prompts().is_empty());

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Applied), 2, "create + delete");
    assert_eq!(count(&rt, LedgerState::Unknown), 0);
    let reply = rt.call(saved_prompt_delete("d1", &id)).await.unwrap();
    assert_eq!(reply.output(), Some(&serde_json::Value::Null));
    assert_eq!(reply.revision(), Some(2));
}

/// Codex 리뷰 반영(research R6): 기존 목표가 있어도 교체 요청의 적용 증거가 아니다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn goal_create_over_existing_after_pending_is_unknown_and_keeps_old_goal() {
    let rt = TestRuntime::new();
    rt.call(goal_create("g1", "Old")).await.unwrap();
    rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AfterPending));
    let fault = rt.call(goal_create("g2", "New")).await.unwrap_err();
    assert!(fault.message.contains("crash injected"), "{fault}");
    assert_eq!(rt.goals()[0]["objective"], "Old");

    let rt = rt.restart();
    assert_eq!(
        count(&rt, LedgerState::Unknown),
        1,
        "목표가 존재하지만 이전 목표이므로 applied가 아니다"
    );
    assert_eq!(rt.goals()[0]["objective"], "Old", "교체는 일어나지 않았다");

    let fault = rt.call(goal_create("g2", "New")).await.unwrap_err();
    assert_eq!(fault.code, FaultCode::Conflict);
    assert_eq!(fault.outcome, Outcome::Unknown);

    let reply = rt.call(goal_create("g3", "New")).await.unwrap();
    assert_eq!(reply.output().unwrap()["objective"], "New");
    assert_eq!(rt.goals().len(), 1);
}

/// 저장 뒤 ledger 확정 실패: pending 유지·unknown 응답. upsert라 재시작 뒤에도 unknown이고 파일은 반영돼 있다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_run_settings_save_ledger_complete_failure_is_unknown_and_stays_pending() {
    let rt = TestRuntime::new();
    rt.runtime
        .hooks()
        .set_fail_point(Some(FailPoint::LedgerComplete));
    let fault = rt.call(settings_save("s1")).await.unwrap_err();
    assert_eq!(fault.code, FaultCode::Unavailable, "{fault}");
    assert_eq!(fault.outcome, Outcome::Unknown);
    assert!(fault.retryable);
    assert_eq!(rt.agent_run_settings().len(), 1, "설정은 저장돼 있다");
    assert_eq!(count(&rt, LedgerState::Pending), 1);
    assert_eq!(count(&rt, LedgerState::Failed), 0);

    let fault = rt.call(settings_save("s1")).await.unwrap_err();
    assert_eq!(fault.code, FaultCode::Conflict);
    assert_eq!(fault.outcome, Outcome::Unknown);

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Pending), 0);
    assert_eq!(
        count(&rt, LedgerState::Unknown),
        1,
        "upsert는 unknown으로 닫힌다"
    );
    assert_eq!(rt.agent_run_settings().len(), 1);
    assert_eq!(
        rt.runtime.coordinator().revision_of("agent-run-settings"),
        0
    );
    let reply = rt.call(settings_save("s2")).await.unwrap();
    assert_eq!(reply.revision(), Some(1));
    assert_eq!(rt.agent_run_settings().len(), 1, "같은 worktree는 교체");
}
