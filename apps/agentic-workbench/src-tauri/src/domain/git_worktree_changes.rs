use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktreeChanges {
    pub working_directory: String,
    pub files: Vec<GitChangedFile>,
    pub staged_count: usize,
    pub unstaged_count: usize,
    pub untracked_count: usize,
    pub conflicted_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitChangedFile {
    pub path: String,
    pub old_path: Option<String>,
    pub staged_status: Option<String>,
    pub unstaged_status: Option<String>,
    pub group: GitChangedFileGroup,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GitChangedFileGroup {
    Staged,
    Unstaged,
    Untracked,
    Conflicted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileDiff {
    pub path: String,
    pub diff: String,
    pub truncated: bool,
    pub binary: bool,
}
