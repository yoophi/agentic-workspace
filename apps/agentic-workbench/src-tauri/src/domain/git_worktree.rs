use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktree {
    pub path: String,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub status: GitWorktreeStatus,
    pub prune_reason: Option<String>,
    pub can_delete: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GitWorktreeStatus {
    Clean,
    Prunable,
    Dirty,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktreeCreateDraft {
    pub path: String,
    pub branch: Option<String>,
    pub reference: Option<String>,
}
