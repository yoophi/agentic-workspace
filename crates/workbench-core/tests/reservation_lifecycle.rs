//! 038 US2: 예약은 `pending` 동안만 배타이고 종료 상태에서 풀린다(research R16, ledger schema v2).
//! 037의 v1 index였다면 같은 worktree 경로를 만들고-지우고-다시 만들거나 실패 뒤 재시도할 수 없었다.

mod support;

use std::{fs, path::PathBuf};

use serde_json::{json, Value};
use support::{command_request, git_repo, TestRuntime};
use workbench_core::{
    infrastructure::storage_coordinator::git_worktrees_aggregate,
    ports::operation_ledger::{LedgerKey, LedgerState, NewLedgerEntry, OperationLedger},
};
use workbench_protocol::{
    FaultCode, IdempotencyKey, OperationId, Outcome, PrincipalKind, RequestId, CONTRACT_REVISION,
};

struct Repo {
    root: PathBuf,
    target: PathBuf,
}

fn build_repo(rt: &TestRuntime) -> Repo {
    let parent = rt.dir.path().join("repos");
    fs::create_dir_all(&parent).unwrap();
    let seed: git_repo::GitRepoSeed = serde_json::from_value(json!({
        "commits": [{ "message": "init", "files": { "README.md": "hello\n" } }],
    }))
    .unwrap();
    let built = git_repo::build(&seed, &parent);
    let parent = fs::canonicalize(&parent).unwrap();
    Repo {
        root: fs::canonicalize(&built.root).unwrap(),
        target: parent.join("wt-p"),
    }
}

fn create_input(repo: &Repo, branch: &str, reference: Option<&str>) -> Value {
    let mut input = json!({
        "workingDirectory": repo.root.to_str().unwrap(),
        "path": repo.target.to_str().unwrap(),
        "branch": branch,
    });
    if let Some(reference) = reference {
        input["reference"] = json!(reference);
    }
    input
}

fn delete_input(repo: &Repo) -> Value {
    json!({ "workingDirectory": repo.root.to_str().unwrap(), "path": repo.target.to_str().unwrap() })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_delete_create_same_path_with_fresh_keys_succeeds() {
    let rt = TestRuntime::new();
    let repo = build_repo(&rt);
    rt.call(command_request(
        OperationId::GitCreateWorktree,
        "c1",
        create_input(&repo, "b1", None),
    ))
    .await
    .expect("first create");
    rt.call(command_request(
        OperationId::GitDeleteWorktree,
        "d1",
        delete_input(&repo),
    ))
    .await
    .expect("delete");
    rt.call(command_request(
        OperationId::GitCreateWorktree,
        "c2",
        create_input(&repo, "b2", None),
    ))
    .await
    .expect("same path again with a fresh key");
    assert_eq!(git_repo::worktree_count(&repo.root), 2);
    assert_eq!(
        rt.runtime
            .ledger()
            .count_by_state(LedgerState::Applied)
            .unwrap(),
        3
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retry_after_failed_create_same_path_succeeds() {
    let rt = TestRuntime::new();
    let repo = build_repo(&rt);
    let fault = rt
        .call(command_request(
            OperationId::GitCreateWorktree,
            "c1",
            create_input(&repo, "b1", Some("no-such-ref")),
        ))
        .await
        .unwrap_err();
    assert_eq!(fault.code, FaultCode::Internal, "{fault}");
    assert_eq!(fault.outcome, Outcome::NotApplied);
    assert!(!fault.retryable);
    assert!(
        fault.message.starts_with("Failed to create git worktree"),
        "git 문구 그대로: {}",
        fault.message
    );
    assert_eq!(
        rt.runtime
            .ledger()
            .count_by_state(LedgerState::Failed)
            .unwrap(),
        1
    );

    rt.call(command_request(
        OperationId::GitCreateWorktree,
        "c2",
        create_input(&repo, "b1", None),
    ))
    .await
    .expect("retry on the same path after a failed attempt");
    assert_eq!(git_repo::worktree_count(&repo.root), 2);
}

/// 다른 실행이 같은 경로를 `pending`으로 잡고 있으면 재시도 없이 `conflict`(outcome unknown, retryable).
/// 동시 요청의 타이밍에 기대지 않도록, 진행 중인 실행을 ledger에 직접 만든다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn same_path_create_while_another_is_pending_is_conflict_unknown() {
    let rt = TestRuntime::new();
    let repo = build_repo(&rt);
    rt.runtime
        .ledger()
        .begin(NewLedgerEntry {
            key: LedgerKey {
                principal_kind: PrincipalKind::Desktop,
                operation: OperationId::GitCreateWorktree,
                contract_revision: CONTRACT_REVISION,
                idempotency_key: IdempotencyKey::new("other").unwrap(),
            },
            input_fingerprint: "other".into(),
            aggregate: git_worktrees_aggregate(&repo.root),
            reserved_resource_id: Some(repo.target.to_str().unwrap().to_owned()),
            request_id: RequestId::new("r-other").unwrap(),
        })
        .unwrap();

    let fault = rt
        .call(command_request(
            OperationId::GitCreateWorktree,
            "mine",
            create_input(&repo, "b1", None),
        ))
        .await
        .unwrap_err();
    assert_eq!(fault.code, FaultCode::Conflict);
    assert_eq!(fault.outcome, Outcome::Unknown);
    assert!(fault.retryable);
    assert_eq!(
        fault.message,
        "Another change to this worktree path is still in progress."
    );
    assert_eq!(
        git_repo::worktree_count(&repo.root),
        1,
        "git 명령을 실행하지 않았다"
    );

    // 같은 경로의 삭제도 진행 중인 생성과 배타다.
    let fault = rt
        .call(command_request(
            OperationId::GitDeleteWorktree,
            "mine-delete",
            delete_input(&repo),
        ))
        .await
        .unwrap_err();
    assert_eq!(fault.code, FaultCode::Conflict);
}

/// 서로 다른 경로의 동시 생성은 저장소 단위로 직렬화될 뿐 모두 성공한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_creates_on_different_paths_all_apply() {
    let rt = std::sync::Arc::new(TestRuntime::new());
    let repo = build_repo(&rt);
    let parent = repo.target.parent().unwrap().to_path_buf();
    let root = repo.root.clone();
    let tasks: Vec<_> = (0..5)
        .map(|index| {
            let rt = std::sync::Arc::clone(&rt);
            let root = root.clone();
            let path = parent.join(format!("wt-{index}"));
            tokio::spawn(async move {
                rt.call(command_request(
                    OperationId::GitCreateWorktree,
                    &format!("k{index}"),
                    json!({
                        "workingDirectory": root.to_str().unwrap(),
                        "path": path.to_str().unwrap(),
                        "branch": format!("b{index}"),
                    }),
                ))
                .await
            })
        })
        .collect();
    for task in tasks {
        task.await.unwrap().expect("each create succeeds");
    }
    assert_eq!(git_repo::worktree_count(&repo.root), 6);
}
