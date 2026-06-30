use crate::domain::{GitCommitDetail, GitCommitGraph, GitCommitHistory, GitFileDiff};

/// working_directory(path) 기반 Git history/graph/detail/diff 조회 포트.
/// repositoryId 같은 앱별 식별자는 소비 앱(facade)에서 path로 변환해 전달한다.
pub trait GitHistoryReader {
    fn list_history(
        &self,
        repository_path: &str,
        limit: usize,
        offset: usize,
        included_refs: &[String],
        excluded_refs: &[String],
    ) -> Result<GitCommitHistory, String>;
    fn get_commit_graph(
        &self,
        repository_path: &str,
        limit: usize,
        offset: usize,
        included_refs: &[String],
        excluded_refs: &[String],
    ) -> Result<GitCommitGraph, String>;
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
