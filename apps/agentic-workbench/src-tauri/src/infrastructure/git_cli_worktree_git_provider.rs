use git_core::{GitCliHistoryReader, GitHistoryReader};

use crate::domain::{
    worktree_git::{GitCommitDetail, GitCommitGraph, GitCommitHistory, GitFileDiff},
    worktree_git_provider::WorktreeGitProvider,
};

/// AW worktree 화면용 어댑터. 공유 git-core의 `GitCliHistoryReader`를 AW의
/// `WorktreeGitProvider` 계약에 맞춘다. AW는 ref 필터를 노출하지 않으므로
/// included/excluded refs로 빈 슬라이스를 넘겨 기본 동작(history=HEAD, graph=--all)을 쓴다.
pub struct GitCliWorktreeGitProvider;

impl WorktreeGitProvider for GitCliWorktreeGitProvider {
    fn list_history(
        &self,
        working_directory: &str,
        limit: usize,
        offset: usize,
        cursor: Option<&str>,
    ) -> Result<GitCommitHistory, String> {
        GitCliHistoryReader.list_history(working_directory, limit, offset, cursor, &[], &[])
    }

    fn get_commit_graph(
        &self,
        working_directory: &str,
        limit: usize,
        offset: usize,
        cursor: Option<&str>,
    ) -> Result<GitCommitGraph, String> {
        GitCliHistoryReader.get_commit_graph(working_directory, limit, offset, cursor, &[], &[])
    }

    fn get_commit_detail(
        &self,
        working_directory: &str,
        commit_hash: &str,
    ) -> Result<GitCommitDetail, String> {
        GitCliHistoryReader.get_commit_detail(working_directory, commit_hash)
    }

    fn get_file_diff(
        &self,
        working_directory: &str,
        commit_hash: &str,
        file_path: &str,
    ) -> Result<GitFileDiff, String> {
        GitCliHistoryReader.get_file_diff(working_directory, commit_hash, file_path)
    }
}
