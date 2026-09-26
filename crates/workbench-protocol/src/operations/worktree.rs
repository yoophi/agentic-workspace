//! `worktree.*`(체크아웃 디렉터리 단위) operation의 input/output wire 타입. core `domain::{worktree_change,
//! worktree_file}`과 git-core `domain`의 미러(research R1). serde 속성은 원본과 같다 — 특히
//! `GitCommitPage.cursorInvalidated`는 없으면 생략된다.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum WorktreeChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeChangeDto {
    pub path: String,
    pub old_path: Option<String>,
    pub change_type: WorktreeChangeType,
    pub binary: bool,
    pub diff: Option<String>,
    pub content: Option<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum GitChangedFileGroup {
    Staged,
    Unstaged,
    Untracked,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitChangedFileDto {
    pub path: String,
    pub old_path: Option<String>,
    pub staged_status: Option<String>,
    pub unstaged_status: Option<String>,
    pub group: GitChangedFileGroup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktreeChangesDto {
    pub working_directory: String,
    pub files: Vec<GitChangedFileDto>,
    pub staged_count: u64,
    pub unstaged_count: u64,
    pub untracked_count: u64,
    pub conflicted_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktreeFileDiffDto {
    pub path: String,
    pub content: String,
    pub is_binary: bool,
    pub is_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeFileEntryDto {
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeTextFileDto {
    pub path: String,
    pub relative_path: String,
    pub content: String,
    pub size: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum WorktreeFileListKind {
    #[default]
    All,
    /// markdown 파일과 그 조상 디렉터리만.
    Markdown,
}

/// 조회 범위. 기본값(전체 트리)은 범위를 생략한 호출과 같다. 중첩 객체라 unknown 필드를 거절하지 않는다(R2).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeFileListScopeDto {
    #[serde(default)]
    pub kind: WorktreeFileListKind,
    /// 조회 시작 상대 경로. worktree 밖이면 `forbidden`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    /// 1이면 해당 디렉터리 직계만. 없으면 무제한.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitSummaryDto {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitPageDto {
    pub offset: u64,
    pub limit: u64,
    /// 첫 페이지에서만 계산된다.
    pub total_count: Option<u64>,
    pub has_more: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_invalidated: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitHistoryDto {
    pub commits: Vec<GitCommitSummaryDto>,
    pub page: GitCommitPageDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitGraphCommitDto {
    pub hash: String,
    pub short_hash: String,
    pub parents: Vec<String>,
    pub message: String,
    pub author: String,
    pub date: String,
    pub is_head: bool,
    pub is_merge: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum GitGraphRefKind {
    LocalBranch,
    RemoteBranch,
    Tag,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitGraphRefDto {
    pub name: String,
    pub target: String,
    pub kind: GitGraphRefKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitGraphLayoutHintsDto {
    pub row_height: u16,
    pub max_initial_lanes: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitGraphDto {
    pub commits: Vec<GitGraphCommitDto>,
    pub refs: Vec<GitGraphRefDto>,
    pub page: GitCommitPageDto,
    pub layout_hints: GitGraphLayoutHintsDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitFileChangeDto {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitDetailDto {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
    pub files: Vec<GitCommitFileChangeDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitFileDiffDto {
    pub commit_hash: String,
    pub path: String,
    pub content: String,
    pub is_binary: bool,
    pub is_truncated: bool,
}

// ---- inputs ----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorktreeListChangesInput {
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorktreeGetChangesInput {
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorktreeGetFileDiffInput {
    pub working_directory: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorktreeListFilesInput {
    pub working_directory: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<WorktreeFileListScopeDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorktreeReadTextFileInput {
    pub working_directory: String,
    pub path: String,
}

/// `maxCount` 기본 100(이력)·300(그래프), 상한 500(초과 시 clamp).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorktreeListHistoryInput {
    pub working_directory: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// 마지막으로 받은 commit hash. 이력이 재작성됐으면 `page.cursorInvalidated: true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorktreeGetGraphInput {
    pub working_directory: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorktreeGetCommitDetailInput {
    pub working_directory: String,
    pub commit_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorktreeGetCommitFileDiffInput {
    pub working_directory: String,
    pub commit_hash: String,
    pub path: String,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn scope_defaults_to_all_and_nested_unknown_fields_are_tolerated() {
        let input: WorktreeListFilesInput = serde_json::from_value(
            json!({"workingDirectory": "/r", "scope": {"dir": "src", "extra": true}}),
        )
        .unwrap();
        let scope = input.scope.unwrap();
        assert_eq!(scope.kind, WorktreeFileListKind::All);
        assert_eq!(scope.dir.as_deref(), Some("src"));
        assert!(serde_json::from_value::<WorktreeListFilesInput>(
            json!({"workingDirectory": "/r", "extra": 1})
        )
        .is_err());
    }

    #[test]
    fn cursor_invalidated_is_omitted_when_absent() {
        let page = GitCommitPageDto {
            offset: 0,
            limit: 10,
            total_count: Some(3),
            has_more: false,
            cursor_invalidated: None,
        };
        assert_eq!(
            serde_json::to_value(&page).unwrap(),
            json!({"offset": 0, "limit": 10, "totalCount": 3, "hasMore": false})
        );
    }
}
