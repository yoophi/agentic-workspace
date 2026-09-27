//! 읽기 전용 자식 run의 worktree 감시(041: AW `acp_agent_worker_adapter.rs`에서 이동, 창 label 제거). 자식을
//! 기동할 때 지문을 남기고, run이 끝나면 다시 계산해 바뀌었으면 과제를 실패 처리한다(오늘 사유·문구 그대로).

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    process::Command,
    sync::Mutex,
};

use crate::domain::agent_orchestration::{OrchestrationError, OrchestrationErrorCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeGuard {
    pub bench_id: String,
    pub workspace_id: String,
    pub node_id: String,
    pub task_id: String,
    pub worktree_path: String,
    pub baseline: String,
}

/// run id → 감시. 런타임 하나가 소유한다(오늘 프로세스 전역 표와 같은 의미).
#[derive(Debug, Default)]
pub struct WorktreeGuards {
    guards: Mutex<HashMap<String, WorktreeGuard>>,
}

impl WorktreeGuards {
    pub fn insert(&self, run_id: &str, guard: WorktreeGuard) {
        self.lock().insert(run_id.to_owned(), guard);
    }

    pub fn take(&self, run_id: &str) -> Option<WorktreeGuard> {
        self.lock().remove(run_id)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, WorktreeGuard>> {
        self.guards
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub fn fingerprint_worktree(path: &str) -> Result<String, OrchestrationError> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for arguments in [
        vec!["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        vec!["diff", "--binary", "HEAD"],
        vec!["diff", "--cached", "--binary", "HEAD"],
    ] {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(arguments)
            .output()
            .map_err(|error| {
                OrchestrationError::new(
                    OrchestrationErrorCode::WorkerUnavailable,
                    format!("Failed to fingerprint worktree: {error}"),
                )
            })?;
        if !output.status.success() {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidInput,
                "Background workers require a valid Git worktree.",
            ));
        }
        output.stdout.hash(&mut hasher);
    }
    Ok(format!("{:016x}", hasher.finish()))
}

pub fn verify_worktree_unchanged(path: &str, baseline: &str) -> Result<(), OrchestrationError> {
    let current = fingerprint_worktree(path)?;
    if current == baseline {
        Ok(())
    } else {
        Err(OrchestrationError::new(
            OrchestrationErrorCode::ReadOnlyViolation,
            "The read-only child changed the worktree; changes were preserved for review.",
        ))
    }
}
