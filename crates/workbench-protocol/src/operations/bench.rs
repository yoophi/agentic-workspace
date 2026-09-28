//! `bench.*` operation의 input/output wire 타입(040). 작업대(Bench)는 run과 교환 작업 영역의 주인이다
//! (ADR core 0004). 계약: `specs/040-workbench-owners/contracts/workbench-benches.md`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BenchOpenInput {
    /// 작업대의 대상 Worktree. 서버가 실제 경로로 정규화한다.
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BenchOpenOutput {
    pub bench_id: String,
    /// 정규화된 실제 경로.
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BenchCloseInput {
    pub bench_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BenchCloseOutput {
    /// 이 호출이 작업대를 닫았으면 true. 모르는 작업대·이미 닫힌 작업대는 false(멱등).
    pub closed: bool,
    /// 닫으면서 취소한 소유 run.
    pub cancelled_runs: Vec<String>,
}

/// agent 전용(주체 run == `runId`). 창 제목은 데스크톱 표현 상태라 서버는 요청만 알린다(ADR 0007).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BenchRequestTitleInput {
    pub run_id: String,
    pub title: String,
}

/// 오늘 MCP 제목 도구의 `TitleChangeResult`와 같은 모양.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TitleChangeResultDto {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// `bench.list`(044): 소유자는 모든 작업대, 그 밖의 주체는 자기가 연 작업대만 본다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BenchListInput {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BenchRunDto {
    pub run_id: String,
    /// `busy`(진행 중 turn·대기열·권한 대기) 또는 `idle`.
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BenchSummaryDto {
    pub bench_id: String,
    pub working_directory: String,
    /// 작업대를 연 주체.
    pub owner: String,
    pub runs: Vec<BenchRunDto>,
}

pub fn bench_list_output_schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
    super::common::array_schema(<BenchSummaryDto as utoipa::PartialSchema>::schema())
}
