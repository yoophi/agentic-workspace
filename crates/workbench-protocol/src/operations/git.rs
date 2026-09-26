//! `git.*`(저장소 단위) operation의 input/output wire 타입. core `domain::{git_remote,git_branch,git_worktree}`의
//! 미러(research R1). 필드·표기는 AW가 도메인 타입을 그대로 직렬화하던 형태와 같다.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitRemoteDto {
    pub name: String,
    pub fetch_url: Option<String>,
    pub push_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchDto {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum GitWorktreeStatus {
    Clean,
    Prunable,
    Dirty,
    /// status 계산을 건너뜀(`includeStatus: false`).
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktreeDto {
    pub path: String,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub status: GitWorktreeStatus,
    pub prune_reason: Option<String>,
    pub can_delete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitListRemotesInput {
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitListBranchesInput {
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitListWorktreesInput {
    pub working_directory: String,
    /// 생략하면 `true`(worktree별 clean/dirty 계산). 오늘의 데스크톱 기본값과 같다.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_status: Option<bool>,
}

/// `path`가 빈 문자열이면 서버가 `<parent>/worktrees/<repoName>/<branch>`를, `branch`가 없으면
/// `worktree-{nanos:x}`를 채운다. 채운 뒤의 경로가 변경 기록에 남는다(FR-005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitCreateWorktreeInput {
    pub working_directory: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitDeleteWorktreeInput {
    pub working_directory: String,
    pub path: String,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn inputs_reject_unknown_fields_and_keep_optional_fields_optional() {
        assert!(serde_json::from_value::<GitListWorktreesInput>(
            json!({"workingDirectory": "/r", "extra": 1})
        )
        .is_err());
        let input: GitListWorktreesInput =
            serde_json::from_value(json!({"workingDirectory": "/r"})).unwrap();
        assert_eq!(input.include_status, None);
        assert_eq!(
            serde_json::to_value(&input).unwrap(),
            json!({"workingDirectory": "/r"})
        );
        let create: GitCreateWorktreeInput =
            serde_json::from_value(json!({"workingDirectory": "/r", "path": ""})).unwrap();
        assert_eq!(create.branch, None);
    }
}
