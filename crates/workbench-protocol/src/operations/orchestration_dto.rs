//! 041: orchestration DTO(미러). `crates/workbench-core`의 도메인·요청 타입과 wire 형태가 같다(core parity 테스트).
//! 이 파일은 scratchpad `gen_orch_dto.py`로 생성했다 — 도메인 타입을 바꾸면 다시 생성한다.
#![allow(clippy::large_enum_variant)]

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum OrchestrationErrorCodeDto {
    InvalidInput,
    InvalidTopology,
    InvalidTransition,
    ScopeMismatch,
    RevisionConflict,
    DuplicateConflict,
    NotFound,
    Unauthorized,
    CapacityExceeded,
    RuntimeLost,
    WorkerUnavailable,
    ReadOnlyViolation,
    /// No Main Coordinator run is bound, so orchestration cannot start at all.
    /// The user must start a Main run first (FR-022).
    CoordinatorInactive,
    /// A Main Coordinator run exists but cannot accept the request right now.
    /// The user should wait and retry (FR-022).
    CoordinatorBusy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AgentNodeKindDto {
    Main,
    Child,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AgentNodeCreatorDto {
    User,
    Coordinator,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatusDto {
    Pending,
    Ready,
    Running,
    InputRequired,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionStatusDto {
    Unassigned,
    Starting,
    Active,
    Idle,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum PresentationStatusDto {
    Background,
    AttentionRequired,
    Promoting,
    Panel,
    Detached,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum PromotionPolicyDto {
    Manual,
    OnAttention,
    Always,
    OnFailure,
    OnCompletion,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AccessPolicyDto {
    ReadOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentRoleProfileDto {
    pub id: String,
    pub name: String,
    pub responsibility: String,
    pub expected_output: String,
    pub system_instructions: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerRuntimeProfileDto {
    pub agent_profile_id: String,
    pub provider_id: String,
    pub model_id: Option<String>,
    pub access_policy: AccessPolicyDto,
    pub supports_read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentNodeDto {
    pub id: String,
    pub kind: AgentNodeKindDto,
    pub parent_node_id: Option<String>,
    pub role: AgentRoleProfileDto,
    pub current_run_id: Option<String>,
    pub assigned_task_id: Option<String>,
    pub execution_status: ExecutionStatusDto,
    pub presentation_status: PresentationStatusDto,
    pub promotion_policy: PromotionPolicyDto,
    pub runtime_profile: Option<WorkerRuntimeProfileDto>,
    pub last_activity_at: Option<String>,
    pub created_by: AgentNodeCreatorDto,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum CoordinatorGenerationStatusDto {
    Active,
    Ended,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorGenerationDto {
    pub id: String,
    pub ordinal: u32,
    pub main_node_id: String,
    pub run_id: String,
    pub previous_generation_id: Option<String>,
    pub status: CoordinatorGenerationStatusDto,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub handoff_summary: Option<String>,
    pub successor_generation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskFailureDto {
    pub code: OrchestrationErrorCodeDto,
    pub message: String,
    pub retryable: bool,
    pub partial_result_report_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationTaskDto {
    pub id: String,
    pub parent_task_id: Option<String>,
    pub coordinator_generation_id: String,
    pub assigned_node_id: Option<String>,
    pub title: String,
    pub objective: String,
    pub constraints: Vec<String>,
    pub expected_result: String,
    pub dependency_task_ids: Vec<String>,
    pub status: TaskStatusDto,
    pub awaiting_handoff: bool,
    pub access_policy: AccessPolicyDto,
    pub attempt: u32,
    pub latest_result_report_id: Option<String>,
    pub failure: Option<TaskFailureDto>,
    pub revision: u64,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum TaskReportTypeDto {
    Progress,
    Result,
    InputRequest,
    Blocked,
    Message,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum FindingSeverityDto {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskFindingDto {
    pub title: String,
    pub detail: String,
    pub evidence: Vec<String>,
    pub severity: FindingSeverityDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKindDto {
    File,
    Url,
    Text,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactReferenceDto {
    pub kind: ArtifactKindDto,
    pub uri: String,
    pub label: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskReportDto {
    pub id: String,
    pub request_id: String,
    pub task_id: String,
    pub reporter_node_id: String,
    pub reporter_run_id: String,
    #[serde(rename = "type")]
    pub report_type: TaskReportTypeDto,
    pub progress_percent: Option<u8>,
    pub summary: String,
    pub findings: Vec<TaskFindingDto>,
    pub artifact_refs: Vec<ArtifactReferenceDto>,
    pub unresolved: Vec<String>,
    pub confidence: Option<f64>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum TaskCommandKindDto {
    Message,
    InputResponse,
    Interrupt,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum TaskCommandSourceDto {
    User,
    Coordinator,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum TaskCommandStatusDto {
    Pending,
    Dispatching,
    Accepted,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandFailureDto {
    pub code: OrchestrationErrorCodeDto,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskCommandDto {
    pub id: String,
    pub request_id: String,
    pub payload_fingerprint: String,
    pub task_id: String,
    pub node_id: String,
    pub run_id: String,
    pub attempt: u32,
    pub kind: TaskCommandKindDto,
    pub message: Option<String>,
    pub input_report_id: Option<String>,
    pub delivery: PromptDeliveryDto,
    pub source: TaskCommandSourceDto,
    pub status: TaskCommandStatusDto,
    pub failure: Option<CommandFailureDto>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum CoordinatorNotificationStatusDto {
    Pending,
    Dispatching,
    /// Legacy status written before delivery and collection were tracked separately.
    Accepted,
    Delivered,
    Processed,
    Failed,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorNotificationDto {
    pub id: String,
    pub report_id: String,
    pub task_id: String,
    pub report_type: TaskReportTypeDto,
    pub generation_id: String,
    pub main_run_id: Option<String>,
    pub status: CoordinatorNotificationStatusDto,
    pub attempt_count: u32,
    /// 실제 전달 실패 수(전달 오류·중단된 시도). coordinator가 바빠 거절한 시도는 세지 않는다.
    #[serde(default)]
    pub delivery_failure_count: u32,
    pub failure: Option<CommandFailureDto>,
    #[serde(default)]
    pub collected_at: Option<String>,
    /// 마지막 전달 시도의 id(044). `dispatching`이면 그 시도가 소유한다.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum PromptDispatchIntentDto {
    Direct,
    Delegate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum PromptTargetModeDto {
    Focused,
    Selected,
    All,
    Coordinator,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum PromptDeliveryDto {
    Send,
    Queue,
    Draft,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum PromptDispatchTargetStatusDto {
    Pending,
    Accepted,
    Delivered,
    Rejected,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PromptDispatchTargetDto {
    pub panel_id: String,
    pub run_id: Option<String>,
    pub request_id: String,
    pub status: PromptDispatchTargetStatusDto,
    pub failure_code: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PromptDispatchDto {
    pub id: String,
    pub intent: PromptDispatchIntentDto,
    pub target_mode: PromptTargetModeDto,
    pub message: String,
    pub delivery: PromptDeliveryDto,
    pub targets: Vec<PromptDispatchTargetDto>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IdempotencyRecordDto {
    pub actor_key: String,
    pub operation: String,
    pub request_id: String,
    pub payload_fingerprint: String,
    pub result_ref: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationSessionDto {
    pub schema_version: u32,
    pub id: String,
    pub worktree_path: String,
    /// 현재 묶임의 orchestration 스트림(`orchestration:<bindingId>`). 묶이지 않았으면 null.
    pub event_stream_id: Option<String>,
    pub main_node_id: String,
    pub active_coordinator_generation_id: Option<String>,
    pub nodes: Vec<AgentNodeDto>,
    pub generations: Vec<CoordinatorGenerationDto>,
    pub tasks: Vec<OrchestrationTaskDto>,
    pub reports: Vec<TaskReportDto>,
    #[serde(default)]
    pub commands: Vec<TaskCommandDto>,
    #[serde(default)]
    pub coordinator_notifications: Vec<CoordinatorNotificationDto>,
    pub dispatches: Vec<PromptDispatchDto>,
    pub idempotency_records: Vec<IdempotencyRecordDto>,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum MainRunBindingStateDto {
    Active,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BindMainRunRequestDto {
    pub request_id: String,
    pub panel_id: String,
    pub run_id: String,
    pub state: MainRunBindingStateDto,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateChildTaskRequestDto {
    pub request_id: String,
    pub title: String,
    pub role: AgentRoleProfileDto,
    pub objective: String,
    pub constraints: Vec<String>,
    pub expected_result: String,
    pub dependency_task_ids: Vec<String>,
    pub preferred_node_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateChildTaskOutcomeDto {
    pub task_id: String,
    pub node_id: String,
    pub status: TaskStatusDto,
    pub execution_status: ExecutionStatusDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportTaskRequestDto {
    pub request_id: String,
    pub task_id: String,
    pub reporter_node_id: String,
    pub reporter_run_id: String,
    pub report_type: TaskReportTypeDto,
    pub progress_percent: Option<u8>,
    pub summary: String,
    pub findings: Vec<TaskFindingDto>,
    pub artifact_refs: Vec<ArtifactReferenceDto>,
    pub unresolved: Vec<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DelegateGoalRequestDto {
    pub request_id: String,
    pub goal: String,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DelegateGoalOutcomeDto {
    pub root_task_id: String,
    pub generation_id: String,
    pub dispatch_id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetPresentationRequestDto {
    pub request_id: String,
    pub node_id: String,
    pub presentation_status: PresentationStatusDto,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskActionRequestDto {
    pub request_id: String,
    pub task_id: String,
    pub expected_revision: u64,
    pub message: Option<String>,
    pub target_node_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorHandoffRequestDto {
    pub request_id: String,
    pub successor_run_id: String,
    pub summary: String,
    pub confirmed: bool,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DispatchPromptRequestDto {
    pub request_id: String,
    pub intent: PromptDispatchIntentDto,
    pub target_mode: PromptTargetModeDto,
    pub message: String,
    pub delivery: PromptDeliveryDto,
    pub panel_ids: Vec<String>,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeliverTaskCommandRequestDto {
    pub request_id: String,
    pub task_id: String,
    pub kind: TaskCommandKindDto,
    pub message: Option<String>,
    pub input_report_id: Option<String>,
    pub delivery: PromptDeliveryDto,
    pub source: TaskCommandSourceDto,
    pub expected_task_revision: Option<u64>,
}
