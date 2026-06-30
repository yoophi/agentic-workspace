//! git-core: git-explorer(정본)와 agentic-workbench가 공유하는 Git history /
//! commit-graph / commit-detail / file-diff core.
//!
//! working_directory(path) 기반 `GitHistoryReader` 포트와 git CLI 구현
//! (`GitCliHistoryReader`)을 제공한다. repositoryId 등 앱별 식별자는 각 앱의
//! facade(service)에서 path로 변환해 전달한다.

pub mod domain;
pub mod git_cli;
pub mod ports;

pub use domain::{
    GitCommitDetail, GitCommitFileChange, GitCommitGraph, GitCommitHistory, GitCommitPage,
    GitCommitSummary, GitFileDiff, GitGraphCommit, GitGraphLayoutHints, GitGraphPage, GitGraphRef,
    GitGraphRefKind,
};
pub use git_cli::{git_error_message, GitCliHistoryReader};
pub use ports::GitHistoryReader;
