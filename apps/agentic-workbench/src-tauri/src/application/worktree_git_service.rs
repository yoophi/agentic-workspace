use crate::domain::{
    worktree_git::{GitCommitDetail, GitCommitGraph, GitCommitHistory, GitFileDiff},
    worktree_git_provider::WorktreeGitProvider,
};

const DEFAULT_HISTORY_LIMIT: usize = 100;
const DEFAULT_GRAPH_LIMIT: usize = 300;
const MAX_LIMIT: usize = 500;

pub fn list_worktree_git_history(
    provider: &impl WorktreeGitProvider,
    working_directory: String,
    max_count: Option<usize>,
    offset: Option<usize>,
    cursor: Option<String>,
) -> Result<GitCommitHistory, String> {
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
) -> Result<GitCommitGraph, String> {
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
) -> Result<GitCommitDetail, String> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    let commit_hash = normalize_required(commit_hash, "Commit hash")?;
    provider.get_commit_detail(&working_directory, &commit_hash)
}

pub fn get_worktree_commit_file_diff(
    provider: &impl WorktreeGitProvider,
    working_directory: String,
    commit_hash: String,
    path: String,
) -> Result<GitFileDiff, String> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    let commit_hash = normalize_required(commit_hash, "Commit hash")?;
    let path = normalize_required(path, "File path")?;
    provider.get_file_diff(&working_directory, &commit_hash, &path)
}

fn normalize_required(value: String, field_name: &str) -> Result<String, String> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(format!("{field_name} is required."));
    }
    Ok(value)
}
