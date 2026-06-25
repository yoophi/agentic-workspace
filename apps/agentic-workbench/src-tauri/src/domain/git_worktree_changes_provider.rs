use crate::domain::git_worktree_changes::{GitFileDiff, GitWorktreeChanges};

pub trait GitWorktreeChangesProvider {
    fn status(&self, working_directory: &str) -> Result<GitWorktreeChanges, String>;
    fn diff(&self, working_directory: &str, path: &str) -> Result<GitFileDiff, String>;
}
