//! Git worktree 생성·삭제의 재시작 판정 — **종료 상태 규칙**(ADR `crates/workbench-core/docs/adr/0001`, FR-005).
//!
//! `pending` 기록의 예약 경로(`reserved_resource_id`)가 저장소의 `git worktree list`에
//! - 생성: **있으면** applied, 없으면 unknown(디렉터리만 생긴 부분 상태도 unknown)
//! - 삭제: **없으면** applied, 있으면 unknown
//!
//! 누가 만들었는지는 구별하지 않는다. 저장소를 읽을 수 없거나 목록이 비면(main worktree조차 없음 = 읽기 실패)
//! unknown이다. 시스템은 부분 상태를 정리하거나 재실행하지 않는다.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::Reconciler;
use crate::{
    infrastructure::git::cli_worktree_provider::GitCliWorktreeProvider,
    ports::{git_providers::GitWorktreeProvider, operation_ledger::LedgerRecord},
};

const AGGREGATE_PREFIX: &str = "git-worktrees:";

pub struct GitWorktreeReconciler {
    /// 생성이면 true(경로가 있어야 applied), 삭제면 false.
    expect_present: bool,
}

impl GitWorktreeReconciler {
    pub fn create() -> Self {
        Self {
            expect_present: true,
        }
    }

    pub fn delete() -> Self {
        Self {
            expect_present: false,
        }
    }
}

/// 비교용 경로. 존재하면 실제 경로(`git worktree list`도 실제 경로를 낸다), 없으면 입력 그대로.
fn comparable(path: &str) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| Path::new(path).to_path_buf())
}

impl Reconciler for GitWorktreeReconciler {
    fn resolve(&self, record: &LedgerRecord) -> Option<Value> {
        let repo = record.aggregate.strip_prefix(AGGREGATE_PREFIX)?;
        let target = comparable(record.reserved_resource_id.as_deref()?);
        let worktrees = GitCliWorktreeProvider.list_worktrees(repo, false).ok()?;
        if worktrees.is_empty() {
            return None;
        }
        let present = worktrees
            .iter()
            .any(|worktree| comparable(&worktree.path) == target);
        (present == self.expect_present).then_some(Value::Null)
    }
}
