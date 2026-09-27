//! `run.*` operation의 input/output wire 타입(040). run 요청·결과·도구 후보는 acp-agent-core `domain::run`·
//! `domain::agent_tool_candidate`의 미러다(serde 속성 동일, core `application/run_dto.rs`의 wire parity 테스트가 고정).
//! 모든 입력은 작업대 id를 받는다(ADR core 0004).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::agent_run_settings::{ContextSizePreset, PermissionMode};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ResumePolicyDto {
    #[default]
    Fresh,
    ResumeIfAvailable,
    ResumeRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RalphLoopRequestDto {
    pub enabled: bool,
    pub max_iterations: usize,
    pub prompt_template: String,
    pub stop_on_error: bool,
    pub stop_on_permission: bool,
    pub delay_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentMcpHttpHeaderDto {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMcpServerConfigDto {
    Http {
        name: String,
        url: String,
        #[serde(default)]
        headers: Vec<AgentMcpHttpHeaderDto>,
    },
}

/// 새 run 요청. 프론트 `AgentRunRequest`와 필드·표기가 같다.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunRequestDto {
    pub goal: String,
    pub agent_id: String,
    pub workspace_id: Option<String>,
    pub checkout_id: Option<String>,
    pub cwd: Option<String>,
    pub agent_command: Option<String>,
    #[serde(default)]
    pub agent_env: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub mcp_servers: Vec<AgentMcpServerConfigDto>,
    pub stdio_buffer_limit_mb: Option<usize>,
    pub auto_allow: Option<bool>,
    pub permission_mode: Option<PermissionMode>,
    pub model_id: Option<String>,
    pub effort_id: Option<String>,
    pub context_size: Option<ContextSizePreset>,
    pub run_id: Option<String>,
    pub resume_session_id: Option<String>,
    pub resume_policy: Option<ResumePolicyDto>,
    pub ralph_loop: Option<RalphLoopRequestDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunDto {
    pub id: String,
    pub goal: String,
    pub agent_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCandidateQueryDto {
    pub run_id: Option<String>,
    pub agent_id: String,
    pub working_directory: String,
    pub session_mode: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCandidateScopeDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AgentToolCandidateSourceDto {
    SessionTool,
    AppCommand,
    Extension,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCandidateDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub insert_text: String,
    pub source: AgentToolCandidateSourceDto,
    pub scope: AgentToolCandidateScopeDto,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AgentToolCandidateStatusDto {
    Loading,
    Ready,
    Empty,
    Error,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCandidateResponseDto {
    pub status: AgentToolCandidateStatusDto,
    pub candidates: Vec<AgentToolCandidateDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunListToolCandidatesInput {
    pub bench_id: String,
    pub query: AgentToolCandidateQueryDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunStartInput {
    pub bench_id: String,
    pub request: AgentRunRequestDto,
    /// 패널 식별자. Main Coordinator 패널이면 데스크톱이 orchestration principal로 MCP 토큰을 만든다.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel_id: Option<String>,
}

/// 프롬프트를 받는 run 제어(`sendPrompt`·`steer`·`cancelAndSend`) 공통 입력.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunPromptInput {
    pub bench_id: String,
    pub run_id: String,
    pub prompt: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunSetPermissionModeInput {
    pub bench_id: String,
    pub run_id: String,
    pub mode: PermissionMode,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunCancelInput {
    pub bench_id: String,
    pub run_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunRespondPermissionInput {
    pub bench_id: String,
    pub run_id: String,
    pub permission_id: String,
    pub option_id: String,
}

/// `run.replay`(041): 작업대가 소유했던 run(또는 작업대에 묶인 orchestration 작업 영역의 노드 run)의 journal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunReplayInput {
    pub bench_id: String,
    pub run_id: String,
    pub after_sequence: u64,
}

/// core `RunReplay`의 미러(오늘 `RuntimeEventSnapshot`과 같은 JSON).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunReplayDto {
    pub run_id: String,
    pub events: Vec<RunReplayEventDto>,
    pub last_sequence: u64,
    pub terminal: bool,
    pub gap_detected: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunReplayEventDto {
    pub run_id: String,
    pub sequence: u64,
    #[schema(value_type = Object)]
    pub event: serde_json::Value,
    pub terminal: bool,
}
