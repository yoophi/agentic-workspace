//! Git·worktree 조회/변경 포트(AW `domain/*_provider.rs`에서 이동). 오류는 `Result<_, String>` 대신 `GitError`다.

use git_core::domain::{
    GitCommitDetail, GitCommitGraph, GitCommitHistory, GitFileDiff, GitWorktreeChanges,
    GitWorktreeFileDiff,
};

use crate::domain::{
    errors::GitError,
    git_branch::GitBranch,
    git_remote::GitRemote,
    git_worktree::{GitWorktree, GitWorktreeCreateDraft},
    worktree_change::WorktreeChange,
};

pub trait GitRemoteProvider {
    fn list_remotes(&self, working_directory: &str) -> Result<Vec<GitRemote>, GitError>;
}

pub trait GitBranchProvider {
    fn list_branches(&self, working_directory: &str) -> Result<Vec<GitBranch>, GitError>;
}

pub trait GitWorktreeProvider {
    /// `include_status=false`면 worktree별 clean/dirty 계산(`git status`)을
    /// 건너뛰고 status를 `Unknown`으로 반환한다. prunable 판정은 유지된다.
    fn list_worktrees(
        &self,
        working_directory: &str,
        include_status: bool,
    ) -> Result<Vec<GitWorktree>, GitError>;
    fn create_worktree(
        &self,
        working_directory: &str,
        draft: GitWorktreeCreateDraft,
    ) -> Result<(), GitError>;
    fn delete_worktree(&self, working_directory: &str, path: &str) -> Result<(), GitError>;
}

pub trait WorktreeChangeProvider {
    /// working_directory에서 HEAD 대비 변경된 파일 목록을 반환한다.
    fn list_changes(&self, working_directory: &str) -> Result<Vec<WorktreeChange>, GitError>;
}

/// 미커밋 변경 요약·파일 diff. git-core `GitWorktreeStatusReader`를 `GitError`로 감싼다.
pub trait WorktreeStatusProvider {
    fn status(&self, working_directory: &str) -> Result<GitWorktreeChanges, GitError>;
    fn diff(&self, working_directory: &str, path: &str) -> Result<GitWorktreeFileDiff, GitError>;
}

pub trait WorktreeGitProvider {
    /// `cursor`는 마지막으로 받은 commit hash. 이력 재작성 감지와 count/refs
    /// 재계산 생략에 쓰인다(AW specs/007 research R8).
    fn list_history(
        &self,
        working_directory: &str,
        limit: usize,
        offset: usize,
        cursor: Option<&str>,
    ) -> Result<GitCommitHistory, GitError>;
    fn get_commit_graph(
        &self,
        working_directory: &str,
        limit: usize,
        offset: usize,
        cursor: Option<&str>,
    ) -> Result<GitCommitGraph, GitError>;
    fn get_commit_detail(
        &self,
        working_directory: &str,
        commit_hash: &str,
    ) -> Result<GitCommitDetail, GitError>;
    fn get_file_diff(
        &self,
        working_directory: &str,
        commit_hash: &str,
        file_path: &str,
    ) -> Result<GitFileDiff, GitError>;
}
