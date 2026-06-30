use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitSummary {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

impl GitCommitSummary {
    pub fn new(hash: String, message: String, author: String, date: String) -> Self {
        Self {
            hash,
            message,
            author,
            date,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitHistory {
    pub commits: Vec<GitCommitSummary>,
    pub page: GitCommitPage,
}

impl GitCommitHistory {
    pub fn new(commits: Vec<GitCommitSummary>, page: GitCommitPage) -> Self {
        Self { commits, page }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitPage {
    pub offset: usize,
    pub limit: usize,
    pub total_count: usize,
    pub has_more: bool,
}

impl GitCommitPage {
    pub fn new(offset: usize, limit: usize, total_count: usize, loaded_count: usize) -> Self {
        Self {
            offset,
            limit,
            total_count,
            has_more: offset.saturating_add(loaded_count) < total_count,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitGraph {
    pub commits: Vec<GitGraphCommit>,
    pub refs: Vec<GitGraphRef>,
    pub page: GitGraphPage,
    pub layout_hints: GitGraphLayoutHints,
}

impl GitCommitGraph {
    pub fn new(
        commits: Vec<GitGraphCommit>,
        refs: Vec<GitGraphRef>,
        page: GitGraphPage,
        layout_hints: GitGraphLayoutHints,
    ) -> Self {
        Self {
            commits,
            refs,
            page,
            layout_hints,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitGraphCommit {
    pub hash: String,
    pub short_hash: String,
    pub parents: Vec<String>,
    pub message: String,
    pub author: String,
    pub date: String,
    pub is_head: bool,
    pub is_merge: bool,
}

impl GitGraphCommit {
    pub fn new(
        hash: String,
        short_hash: String,
        parents: Vec<String>,
        message: String,
        author: String,
        date: String,
        is_head: bool,
    ) -> Self {
        let is_merge = parents.len() > 1;

        Self {
            hash,
            short_hash,
            parents,
            message,
            author,
            date,
            is_head,
            is_merge,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitGraphPage {
    pub offset: usize,
    pub limit: usize,
    pub total_count: usize,
    pub has_more: bool,
}

impl GitGraphPage {
    pub fn new(offset: usize, limit: usize, total_count: usize, loaded_count: usize) -> Self {
        Self {
            offset,
            limit,
            total_count,
            has_more: offset.saturating_add(loaded_count) < total_count,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitGraphLayoutHints {
    pub row_height: u16,
    pub max_initial_lanes: u16,
}

impl GitGraphLayoutHints {
    pub fn default_row_layout() -> Self {
        Self {
            row_height: 32,
            max_initial_lanes: 10,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitGraphRef {
    pub name: String,
    pub target: String,
    pub kind: GitGraphRefKind,
}

impl GitGraphRef {
    pub fn new(name: String, target: String, kind: GitGraphRefKind) -> Self {
        Self { name, target, kind }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GitGraphRefKind {
    LocalBranch,
    RemoteBranch,
    Tag,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitFileChange {
    pub path: String,
    pub status: String,
}

impl GitCommitFileChange {
    pub fn new(path: String, status: String) -> Self {
        Self { path, status }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileDiff {
    pub commit_hash: String,
    pub path: String,
    pub content: String,
    pub is_binary: bool,
    pub is_truncated: bool,
}

impl GitFileDiff {
    pub fn new(
        commit_hash: String,
        path: String,
        content: String,
        is_binary: bool,
        is_truncated: bool,
    ) -> Self {
        Self {
            commit_hash,
            path,
            content,
            is_binary,
            is_truncated,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitDetail {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
    pub files: Vec<GitCommitFileChange>,
}

impl GitCommitDetail {
    pub fn new(
        hash: String,
        message: String,
        author: String,
        date: String,
        files: Vec<GitCommitFileChange>,
    ) -> Self {
        Self {
            hash,
            message,
            author,
            date,
            files,
        }
    }
}
