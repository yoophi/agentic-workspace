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
    /// status 계산을 건너뛴 경우(`include_status=false`). clean/dirty 판정이
    /// 필요한 화면은 기본 옵션으로 조회한다(specs/007 research R5).
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktreeCreateDraft {
    pub path: String,
    pub branch: Option<String>,
    pub reference: Option<String>,
}
