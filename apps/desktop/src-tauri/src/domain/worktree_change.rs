use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorktreeChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
}

/// worktree에서 HEAD 대비 변경된 파일 한 건.
/// - 수정/삭제/이름변경된 추적 파일은 `diff`(unified diff)를 채운다.
/// - 새 파일/untracked 파일은 `content`(미리보기용 전체 내용)를 채운다.
/// - binary이거나 diff/내용을 만들 수 없으면 둘 다 None이고 `binary`가 true다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeChange {
    pub path: String,
    pub old_path: Option<String>,
    pub change_type: WorktreeChangeType,
    pub binary: bool,
    pub diff: Option<String>,
    pub content: Option<String>,
    /// diff 또는 content가 표시 한도를 넘어 잘렸는지 여부.
    pub truncated: bool,
}
