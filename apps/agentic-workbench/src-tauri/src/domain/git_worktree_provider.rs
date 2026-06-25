use crate::domain::git_worktree::{GitWorktree, GitWorktreeCreateDraft};

pub trait GitWorktreeProvider {
    fn list_worktrees(&self, working_directory: &str) -> Result<Vec<GitWorktree>, String>;
    fn create_worktree(
        &self,
        working_directory: &str,
        draft: GitWorktreeCreateDraft,
    ) -> Result<(), String>;
    fn delete_worktree(&self, working_directory: &str, path: &str) -> Result<(), String>;
}
