use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeFileEntry {
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeTextFile {
    pub path: String,
    pub relative_path: String,
    pub content: String,
    pub size: u64,
    pub truncated: bool,
}

/// `list_worktree_files` 조회 범위(specs/007 research R10).
/// 기본값(전체 트리)은 기존 호출과 동일하게 동작한다.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeFileListScope {
    #[serde(default)]
    pub kind: WorktreeFileListKind,
    /// 조회 시작 상대 경로. 경로 탈출 방지 검증이 적용된다.
    pub dir: Option<String>,
    /// 1이면 해당 디렉터리 직계만(폴더 펼침용). None이면 무제한.
    pub depth: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorktreeFileListKind {
    #[default]
    All,
    /// markdown 파일과 그 조상 디렉터리만 반환한다.
    Markdown,
}
