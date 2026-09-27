//! `worktree.changed.v1` 본문: core `infrastructure::fs::worktree_watcher::WorktreeChangedEvent`의 미러(039).
//! 알림용 스트림이라 보관·replay가 없다(ADR core 0003). `workingDirectory`는 감시 중인 실제 경로다.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeChangedDto {
    pub working_directory: String,
    /// debounce 창 안의 대표 변경 경로.
    pub changed_path: String,
    pub kind: WorktreeChangeKindDto,
}

/// `git`: `.git` 메타데이터(브랜치·index 등) 변경. `file`: 작업 트리 파일 변경.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum WorktreeChangeKindDto {
    File,
    Git,
}
