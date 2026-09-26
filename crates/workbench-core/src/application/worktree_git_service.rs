//! `worktree.listHistory`·`getGraph`·`getCommitDetail`·`getCommitFileDiff`(AW `worktree_git_service`에서 이동).
//! 페이지 기본값·상한은 그대로다.

use git_core::domain::{GitCommitDetail, GitCommitGraph, GitCommitHistory, GitFileDiff};

use crate::{
    application::git_service::normalize_required, domain::errors::GitError,
    ports::git_providers::WorktreeGitProvider,
};

pub const DEFAULT_HISTORY_LIMIT: usize = 100;
pub const DEFAULT_GRAPH_LIMIT: usize = 300;
pub const MAX_LIMIT: usize = 500;

pub fn list_worktree_git_history(
    provider: &impl WorktreeGitProvider,
    working_directory: String,
    max_count: Option<usize>,
    offset: Option<usize>,
    cursor: Option<String>,
) -> Result<GitCommitHistory, GitError> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    provider.list_history(
        &working_directory,
        max_count
            .unwrap_or(DEFAULT_HISTORY_LIMIT)
            .clamp(1, MAX_LIMIT),
        offset.unwrap_or(0),
        normalize_cursor(&cursor),
    )
}

pub fn get_worktree_git_graph(
    provider: &impl WorktreeGitProvider,
    working_directory: String,
    max_count: Option<usize>,
    offset: Option<usize>,
    cursor: Option<String>,
) -> Result<GitCommitGraph, GitError> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    provider.get_commit_graph(
        &working_directory,
        max_count.unwrap_or(DEFAULT_GRAPH_LIMIT).clamp(1, MAX_LIMIT),
        offset.unwrap_or(0),
        normalize_cursor(&cursor),
    )
}

fn normalize_cursor(cursor: &Option<String>) -> Option<&str> {
    cursor
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn get_worktree_commit_detail(
    provider: &impl WorktreeGitProvider,
    working_directory: String,
    commit_hash: String,
) -> Result<GitCommitDetail, GitError> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    let commit_hash = normalize_required(commit_hash, "Commit hash")?;
    provider.get_commit_detail(&working_directory, &commit_hash)
}

pub fn get_worktree_commit_file_diff(
    provider: &impl WorktreeGitProvider,
    working_directory: String,
    commit_hash: String,
    path: String,
) -> Result<GitFileDiff, GitError> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    let commit_hash = normalize_required(commit_hash, "Commit hash")?;
    let path = normalize_required(path, "File path")?;
    provider.get_file_diff(&working_directory, &commit_hash, &path)
}
