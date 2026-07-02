use crate::domain::git_worktree::{GitWorktree, GitWorktreeCreateDraft};

pub trait GitWorktreeProvider {
    /// `include_status=false`면 worktree별 clean/dirty 계산(`git status`)을
    /// 건너뛰고 status를 `Unknown`으로 반환한다. prunable 판정은 유지된다.
    fn list_worktrees(
        &self,
        working_directory: &str,
        include_status: bool,
    ) -> Result<Vec<GitWorktree>, String>;
    fn create_worktree(
        &self,
        working_directory: &str,
        draft: GitWorktreeCreateDraft,
    ) -> Result<(), String>;
    fn delete_worktree(&self, working_directory: &str, path: &str) -> Result<(), String>;
}
