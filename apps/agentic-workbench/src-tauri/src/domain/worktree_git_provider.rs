use crate::domain::worktree_git::{GitCommitDetail, GitCommitGraph, GitCommitHistory, GitFileDiff};

pub trait WorktreeGitProvider {
    /// `cursor`는 마지막으로 받은 commit hash. 이력 재작성 감지와 count/refs
    /// 재계산 생략에 쓰인다(specs/007 research R8).
    fn list_history(
        &self,
        working_directory: &str,
        limit: usize,
        offset: usize,
        cursor: Option<&str>,
    ) -> Result<GitCommitHistory, String>;
    fn get_commit_graph(
        &self,
        working_directory: &str,
        limit: usize,
        offset: usize,
        cursor: Option<&str>,
    ) -> Result<GitCommitGraph, String>;
    fn get_commit_detail(
        &self,
        working_directory: &str,
        commit_hash: &str,
    ) -> Result<GitCommitDetail, String>;
    fn get_file_diff(
        &self,
        working_directory: &str,
        commit_hash: &str,
        file_path: &str,
    ) -> Result<GitFileDiff, String>;
}
