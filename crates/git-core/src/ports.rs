use crate::domain::{
    GitCommitDetail, GitCommitGraph, GitCommitHistory, GitFileDiff, GitWorktreeChanges,
    GitWorktreeFileDiff,
};

/// working_directory(path) 기반 Git history/graph/detail/diff 조회 포트.
/// repositoryId 같은 앱별 식별자는 소비 앱(facade)에서 path로 변환해 전달한다.
pub trait GitHistoryReader {
    /// `cursor`는 소비자가 마지막으로 받은 commit hash다. 페이지 사이 이력
    /// 재작성(rebase 등)을 감지해 `cursor_invalidated`로 폴백을 알리고,
    /// cursor가 있거나 offset>0인 페이지는 count/refs 재계산을 생략한다
    /// (AW specs/007 research R8).
    fn list_history(
        &self,
        repository_path: &str,
        limit: usize,
        offset: usize,
        cursor: Option<&str>,
        included_refs: &[String],
        excluded_refs: &[String],
    ) -> Result<GitCommitHistory, String>;
    fn get_commit_graph(
        &self,
        repository_path: &str,
        limit: usize,
        offset: usize,
        cursor: Option<&str>,
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

/// working_directory(path) 기반 미커밋(working-tree) status/diff 조회 포트.
/// `git status --porcelain` 파싱과 `git diff`(+`--cached`)를 추상화한다.
pub trait GitWorktreeStatusReader {
    fn status(&self, repository_path: &str) -> Result<GitWorktreeChanges, String>;
    fn diff(&self, repository_path: &str, file_path: &str) -> Result<GitWorktreeFileDiff, String>;
}
