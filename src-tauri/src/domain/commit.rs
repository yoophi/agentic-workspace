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
