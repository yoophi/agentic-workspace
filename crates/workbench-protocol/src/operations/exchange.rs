//! `exchange.*` operation의 input/output wire 타입(040). core `domain::agent_exchange`의 미러(serde 속성 동일,
//! core `application/exchange_dto.rs` parity 테스트가 고정). `windowLabel`은 계약에 없다 — 작업 영역은 작업대에 묶인다.
//! `*FromRun`·`listPeers`·`getForRun`은 agent 전용(principal 주체의 run == `runId`).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AgentPanelStatusDto {
    Idle,
    Running,
    Closing,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AgentExchangeDeliveryDto {
    Send,
    Queue,
    Draft,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AgentExchangeStatusDto {
    Pending,
    Accepted,
    Delivered,
    Rejected,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentPanelEndpointDto {
    pub panel_id: String,
    pub title: String,
    pub run_id: Option<String>,
    pub status: AgentPanelStatusDto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkspaceSyncRequestDto {
    pub worktree_path: String,
    pub revision: u64,
    pub focused_panel_id: String,
    pub panels: Vec<AgentPanelEndpointDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkspaceSyncResponseDto {
    pub revision: u64,
    pub accepted_panels: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendAgentExchangeRequestDto {
    pub request_id: String,
    pub source_panel_id: String,
    pub source_run_id: Option<String>,
    pub target_panel_id: String,
    pub target_run_id: Option<String>,
    pub message: String,
    pub delivery: AgentExchangeDeliveryDto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentExchangeAckRequestDto {
    pub request_id: String,
    pub target_panel_id: String,
    pub outcome: AgentExchangeStatusDto,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentExchangeEndpointRefDto {
    pub panel_id: String,
    pub title: String,
    pub run_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentExchangeDto {
    pub request_id: String,
    pub worktree_path: String,
    pub source: AgentExchangeEndpointRefDto,
    pub target: AgentExchangeEndpointRefDto,
    pub message: String,
    pub delivery: AgentExchangeDeliveryDto,
    pub status: AgentExchangeStatusDto,
    pub failure_code: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// `exchange.listPeers` 출력.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentPeersDto {
    pub peers: Vec<AgentPanelEndpointDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExchangeSyncWorkspaceInput {
    pub bench_id: String,
    pub request: AgentWorkspaceSyncRequestDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExchangeSendInput {
    pub bench_id: String,
    pub request: SendAgentExchangeRequestDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExchangeAcknowledgeInput {
    pub bench_id: String,
    pub request: AgentExchangeAckRequestDto,
}

/// 화면 대기열에서 지운 교환 prompt의 전달 포기(044 Codex r7). 이미 확인(`delivered`)했지만 run에 보내지 않은 교환을 소비된
/// 것으로 표시해 활동 작업에서 빼고, 이후 같은 교환의 전달은 거절된다. 이미 전달·포기된 교환이면 효과 없이 성공한다.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExchangeDiscardDeliveryInput {
    pub bench_id: String,
    pub request_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExchangeListInput {
    pub bench_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExchangeListPeersInput {
    pub run_id: String,
}

/// agent가 보내는 교환. 출발 패널은 서버가 run으로 찾으므로 요청의 `sourcePanelId`·`sourceRunId`는 덮어쓴다.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExchangeSendFromRunInput {
    pub run_id: String,
    pub request: SendAgentExchangeRequestDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExchangeGetForRunInput {
    pub run_id: String,
    pub request_id: String,
}

/// `exchange.list` 출력 스키마.
pub fn exchange_list_output_schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
    super::common::array_schema(<AgentExchangeDto as utoipa::PartialSchema>::schema())
}
