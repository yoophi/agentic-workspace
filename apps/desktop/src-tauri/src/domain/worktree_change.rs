use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeChange {
    pub path: String,
    pub old_path: Option<String>,
    pub change_type: WorktreeChangeType,
    pub summary: String,
    pub diff: Option<String>,
    pub preview: Option<String>,
    pub binary: bool,
    pub truncated: bool,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorktreeChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Unmerged,
    Unknown,
}
