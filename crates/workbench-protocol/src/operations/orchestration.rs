//! `orchestration.*` operation의 input 타입과 output 스키마(041). 데스크톱 operation은 `benchId`를 받고 작업대에 묶인
//! 작업 영역을 다룬다. 본문 DTO는 `orchestration_dto`(core 도메인 미러). agent operation 입력은 US2에서 추가한다.

use serde::{Deserialize, Serialize};
use utoipa::{openapi::RefOr, openapi::Schema, PartialSchema, ToSchema};

use super::{
    common::{array_schema, nullable_schema},
    orchestration_dto::{
        BindMainRunRequestDto, CoordinatorHandoffRequestDto, DelegateGoalRequestDto,
        DispatchPromptRequestDto, OrchestrationSessionDto, OrchestrationTaskDto,
        PresentationStatusDto, PromptDeliveryDto, SetPresentationRequestDto, TaskActionRequestDto,
        TaskCommandKindDto, TaskReportDto,
    },
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestrationBootstrapInput {
    pub bench_id: String,
    pub worktree_path: String,
    #[serde(default)]
    pub resume_workspace_id: Option<String>,
}

/// `orchestration.get`·`collectReports`·`recover`: 작업대만 받는다.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestrationBenchInput {
    pub bench_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestrationListRecoverableInput {
    pub bench_id: String,
    pub worktree_path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestrationBindCoordinatorInput {
    pub bench_id: String,
    pub request: BindMainRunRequestDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestrationDelegateGoalInput {
    pub bench_id: String,
    pub request: DelegateGoalRequestDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestrationAdoptManualChildInput {
    pub bench_id: String,
    pub panel_id: String,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestrationListTasksInput {
    pub bench_id: String,
    pub generation_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestrationSetPresentationInput {
    pub bench_id: String,
    pub request: SetPresentationRequestDto,
}

/// 오늘 `send_orchestration_child_command`의 입력(`DeliverTaskCommandInput`). 명령 출처는 화면(`ui`)으로 고정된다.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeliverTaskCommandInputDto {
    pub request_id: String,
    pub task_id: String,
    pub kind: TaskCommandKindDto,
    pub message: Option<String>,
    pub input_report_id: Option<String>,
    pub delivery: PromptDeliveryDto,
    pub expected_task_revision: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestrationSendChildCommandInput {
    pub bench_id: String,
    pub input: DeliverTaskCommandInputDto,
}

/// `respondInput`·`cancelTask`·`retryTask`·`reassignTask`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestrationTaskActionInput {
    pub bench_id: String,
    pub request: TaskActionRequestDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestrationHandoffCoordinatorInput {
    pub bench_id: String,
    pub request: CoordinatorHandoffRequestDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrchestrationDispatchPromptInput {
    pub bench_id: String,
    pub request: DispatchPromptRequestDto,
}

pub fn session_or_null_schema() -> RefOr<Schema> {
    nullable_schema(OrchestrationSessionDto::schema())
}

pub fn sessions_schema() -> RefOr<Schema> {
    array_schema(OrchestrationSessionDto::schema())
}

pub fn tasks_schema() -> RefOr<Schema> {
    array_schema(OrchestrationTaskDto::schema())
}

pub fn reports_schema() -> RefOr<Schema> {
    array_schema(TaskReportDto::schema())
}

/// 화면 표시 상태 값(오늘 `set_orchestration_presentation`의 선택지)을 문서에 노출한다.
pub type OrchestrationPresentationStatus = PresentationStatusDto;

/// agent orchestration 도구 operation 16개의 입력(041 US2). `arguments`는 오늘 MCP 도구 인자 객체 그대로다(도구별
/// 스키마는 MCP `tools/list`가 싣는다, 검증 규칙·문구는 오늘과 같다). `runId`는 principal run과 같아야 한다.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentToolInput {
    pub run_id: String,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub arguments: serde_json::Value,
}

/// agent 도구 결과: 오늘 도구의 `structuredContent`와 같은 객체.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentToolOutput(pub serde_json::Value);

impl PartialSchema for AgentToolOutput {
    fn schema() -> RefOr<Schema> {
        utoipa::openapi::ObjectBuilder::new()
            .description(Some("오늘 MCP 도구의 structuredContent와 같은 객체."))
            .into()
    }
}

impl ToSchema for AgentToolOutput {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentRoleInput {
    pub run_id: String,
}

/// 부른 run의 서버 상태상 역할(research R7). MCP `tools/list`가 이것으로 도구 목록을 고른다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AgentRoleKindDto {
    Coordinator,
    Child,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentRoleDto {
    pub role: Option<AgentRoleKindDto>,
    pub workspace_id: Option<String>,
    pub task_id: Option<String>,
}
