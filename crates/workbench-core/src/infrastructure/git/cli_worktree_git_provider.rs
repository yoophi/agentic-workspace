//! worktree 화면용 Git 조회 어댑터. 공유 git-core의 reader를 core 포트에 맞춘다(git-core는 변경하지 않는다, ADR 0002).
//! git-core는 `Result<_, String>`을 돌려주므로 실패는 전부 `GitError::CommandFailed`(문구 그대로)다.

use git_core::{
    domain::{
        GitCommitDetail, GitCommitGraph, GitCommitHistory, GitFileDiff, GitWorktreeChanges,
        GitWorktreeFileDiff,
    },
    GitCliHistoryReader, GitCliWorktreeStatusReader, GitHistoryReader, GitWorktreeStatusReader,
};

use crate::{
    domain::errors::GitError,
    ports::git_providers::{WorktreeGitProvider, WorktreeStatusProvider},
};

/// AW는 ref 필터를 노출하지 않으므로 included/excluded refs로 빈 슬라이스를 넘겨 기본 동작
/// (history=HEAD, graph=--all)을 쓴다.
pub struct GitCliWorktreeGitProvider;

impl WorktreeGitProvider for GitCliWorktreeGitProvider {
    fn list_history(
        &self,
        working_directory: &str,
        limit: usize,
        offset: usize,
        cursor: Option<&str>,
    ) -> Result<GitCommitHistory, GitError> {
        GitCliHistoryReader
            .list_history(working_directory, limit, offset, cursor, &[], &[])
            .map_err(GitError::CommandFailed)
    }

    fn get_commit_graph(
        &self,
        working_directory: &str,
        limit: usize,
        offset: usize,
        cursor: Option<&str>,
    ) -> Result<GitCommitGraph, GitError> {
        GitCliHistoryReader
            .get_commit_graph(working_directory, limit, offset, cursor, &[], &[])
            .map_err(GitError::CommandFailed)
    }

    fn get_commit_detail(
        &self,
        working_directory: &str,
        commit_hash: &str,
    ) -> Result<GitCommitDetail, GitError> {
        GitCliHistoryReader
            .get_commit_detail(working_directory, commit_hash)
            .map_err(GitError::CommandFailed)
    }

    fn get_file_diff(
        &self,
        working_directory: &str,
        commit_hash: &str,
        file_path: &str,
    ) -> Result<GitFileDiff, GitError> {
        GitCliHistoryReader
            .get_file_diff(working_directory, commit_hash, file_path)
            .map_err(GitError::CommandFailed)
    }
}

pub struct GitCliWorktreeStatusProvider;

impl WorktreeStatusProvider for GitCliWorktreeStatusProvider {
    fn status(&self, working_directory: &str) -> Result<GitWorktreeChanges, GitError> {
        GitCliWorktreeStatusReader
            .status(working_directory)
            .map_err(GitError::CommandFailed)
    }

    fn diff(&self, working_directory: &str, path: &str) -> Result<GitWorktreeFileDiff, GitError> {
        GitCliWorktreeStatusReader
            .diff(working_directory, path)
            .map_err(GitError::CommandFailed)
    }
}
