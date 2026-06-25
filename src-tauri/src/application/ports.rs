use crate::domain::{
    branch::GitBranch,
    commit::GitFileDiff,
    commit::{GitCommitDetail, GitCommitSummary},
    repository::Repository,
    repository_event::RepositoryChangeEvent,
    worktree::GitWorktree,
};

pub trait GitRepositoryValidator {
    fn validate_repository(&self, path: &str) -> Result<String, String>;
}

pub trait RepositoryStore {
    fn list(&self) -> Result<Vec<Repository>, String>;
    fn save_all(&self, repositories: &[Repository]) -> Result<(), String>;
}

pub trait GitWorktreeReader {
    fn list_worktrees(&self, repository_path: &str) -> Result<Vec<GitWorktree>, String>;
}

pub trait GitBranchReader {
    fn list_branches(&self, repository_path: &str) -> Result<Vec<GitBranch>, String>;
}

pub trait GitHistoryReader {
    fn list_history(&self, repository_path: &str) -> Result<Vec<GitCommitSummary>, String>;
    fn get_commit_detail(
        &self,
        repository_path: &str,
        commit_hash: &str,
    ) -> Result<GitCommitDetail, String>;
    fn get_file_diff(
        &self,
        repository_path: &str,
        commit_hash: &str,
        file_path: &str,
    ) -> Result<GitFileDiff, String>;
}

pub trait RepositoryWatcher {
    type WatchHandle;

    fn watch_repositories(
        &self,
        repositories: &[Repository],
        notify: Box<dyn Fn(RepositoryChangeEvent) + Send + Sync + 'static>,
    ) -> Result<Self::WatchHandle, String>;
}
