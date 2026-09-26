//! 038 US2: Git worktree 생성·삭제의 재시작 판정 — 종료 상태 규칙(ADR `crates/workbench-core/docs/adr/0001`,
//! FR-005·SC-003). 실제 git 저장소를 만들고 중단 지점을 주입한 뒤 재시작해서 `applied`/`unknown`을 확인한다.

mod support;

use std::{fs, path::PathBuf};

use serde_json::{json, Value};
use support::{command_request, git_repo, TestRuntime};
use workbench_core::{
    application::workbench_runtime::CrashPoint, ports::operation_ledger::LedgerState,
};
use workbench_protocol::{CallRequest, FaultCode, OperationId, Outcome};

struct Repo {
    root: PathBuf,
    parent: PathBuf,
}

fn build_repo(rt: &TestRuntime, worktrees: Value) -> Repo {
    let parent = rt.dir.path().join("repos");
    fs::create_dir_all(&parent).unwrap();
    let seed: git_repo::GitRepoSeed = serde_json::from_value(json!({
        "commits": [{ "message": "init", "files": { "README.md": "hello\n" } }],
        "worktrees": worktrees,
    }))
    .unwrap();
    let built = git_repo::build(&seed, &parent);
    Repo {
        root: fs::canonicalize(&built.root).unwrap(),
        parent: fs::canonicalize(&parent).unwrap(),
    }
}

fn path_str(path: &std::path::Path) -> String {
    path.to_str().unwrap().to_owned()
}

fn create(key: &str, repo: &Repo, name: &str) -> CallRequest {
    command_request(
        OperationId::GitCreateWorktree,
        key,
        json!({
            "workingDirectory": path_str(&repo.root),
            "path": path_str(&repo.parent.join(name)),
            "branch": name,
        }),
    )
}

fn delete(key: &str, repo: &Repo, name: &str) -> CallRequest {
    command_request(
        OperationId::GitDeleteWorktree,
        key,
        json!({ "workingDirectory": path_str(&repo.root), "path": path_str(&repo.parent.join(name)) }),
    )
}

fn count(rt: &TestRuntime, state: LedgerState) -> usize {
    rt.runtime.ledger().count_by_state(state).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_pending_with_registered_path_is_applied() {
    let rt = TestRuntime::new();
    let repo = build_repo(&rt, json!([]));
    rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AFTER_SIDE_EFFECT));
    let fault = rt.call(create("k1", &repo, "wt-new")).await.unwrap_err();
    assert!(fault.message.contains("crash injected"), "{fault}");
    assert_eq!(git_repo::worktree_count(&repo.root), 2, "git 명령은 끝났다");
    assert_eq!(count(&rt, LedgerState::Pending), 1);

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Applied), 1);
    assert_eq!(count(&rt, LedgerState::Unknown), 0);
    // 같은 키: 판정된 결과(null)를 재생하고 다시 만들지 않는다.
    let reply = rt.call(create("k1", &repo, "wt-new")).await.unwrap();
    assert_eq!(reply.output(), Some(&Value::Null));
    assert_eq!(reply.revision(), None, "Git 변경은 revision이 없다");
    assert_eq!(git_repo::worktree_count(&repo.root), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_pending_without_path_is_unknown() {
    let rt = TestRuntime::new();
    let repo = build_repo(&rt, json!([]));
    rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AfterPending));
    rt.call(create("k1", &repo, "wt-new")).await.unwrap_err();
    assert_eq!(git_repo::worktree_count(&repo.root), 1);

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Unknown), 1);
    let fault = rt.call(create("k1", &repo, "wt-new")).await.unwrap_err();
    assert_eq!(fault.code, FaultCode::Conflict);
    assert_eq!(fault.outcome, Outcome::Unknown);
    assert_eq!(git_repo::worktree_count(&repo.root), 1, "자동 재실행 없음");
    // 새 키로는 만들 수 있다(unknown은 예약을 붙잡지 않는다, R16).
    rt.call(create("k2", &repo, "wt-new")).await.unwrap();
    assert_eq!(git_repo::worktree_count(&repo.root), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_pending_with_absent_path_is_applied() {
    let rt = TestRuntime::new();
    let repo = build_repo(&rt, json!([{ "path": "wt-a", "branch": "wt-a" }]));
    rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AFTER_SIDE_EFFECT));
    rt.call(delete("d1", &repo, "wt-a")).await.unwrap_err();
    assert_eq!(git_repo::worktree_count(&repo.root), 1);

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Applied), 1);
    let reply = rt.call(delete("d1", &repo, "wt-a")).await.unwrap();
    assert_eq!(reply.output(), Some(&Value::Null));
    // 새 키로 다시 지우면 "없음"(spec edge case).
    let fault = rt.call(delete("d2", &repo, "wt-a")).await.unwrap_err();
    assert_eq!(fault.code, FaultCode::NotFound);
    assert_eq!(fault.message, "Git worktree not found.");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_pending_with_present_path_is_unknown() {
    let rt = TestRuntime::new();
    let repo = build_repo(&rt, json!([{ "path": "wt-a", "branch": "wt-a" }]));
    rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AfterPending));
    rt.call(delete("d1", &repo, "wt-a")).await.unwrap_err();

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Unknown), 1);
    assert_eq!(git_repo::worktree_count(&repo.root), 2, "지우지 않았다");
}

/// 디렉터리만 생기고 git 등록이 안 된 부분 상태는 applied가 아니다. 시스템은 그 디렉터리를 건드리지 않는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partial_directory_without_registration_is_unknown() {
    let rt = TestRuntime::new();
    let repo = build_repo(&rt, json!([]));
    rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AfterPending));
    rt.call(create("k1", &repo, "wt-new")).await.unwrap_err();
    let partial = repo.parent.join("wt-new");
    fs::create_dir_all(&partial).unwrap();

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Unknown), 1);
    assert_eq!(count(&rt, LedgerState::Applied), 0);
    assert!(partial.is_dir(), "부분 상태를 임의로 정리하지 않는다");
    assert_eq!(git_repo::worktree_count(&repo.root), 1);
}

/// 저장소를 읽을 수 없으면(삭제됨) 판정 근거가 없으므로 unknown이다 — 삭제 applied로 오판하지 않는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unreadable_repository_is_unknown_for_delete() {
    let rt = TestRuntime::new();
    let repo = build_repo(&rt, json!([{ "path": "wt-a", "branch": "wt-a" }]));
    rt.runtime
        .hooks()
        .set_crash_point(Some(CrashPoint::AfterPending));
    rt.call(delete("d1", &repo, "wt-a")).await.unwrap_err();
    fs::remove_dir_all(&repo.root).unwrap();

    let rt = rt.restart();
    assert_eq!(count(&rt, LedgerState::Unknown), 1);
    assert_eq!(count(&rt, LedgerState::Applied), 0);
}
