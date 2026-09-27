//! OpenAPI 3.1 문서 조립(research R1).
//!
//! 개별 타입 스키마는 `utoipa::ToSchema` derive로 얻고, `CallRequest`·`CallReplyByOperation`의 `oneOf`는
//! operation registry(`operations::OPERATIONS`)를 순회해 **프로그램적으로** 만든다. variant마다 `operation`
//! 속성을 단일값 `enum`으로 두어 `openapi-typescript`가 판별 union으로 읽게 하고, `discriminator` object는
//! 쓰지 않는다. 생성물은 `openapi/workbench.openapi.json`에 커밋되며 CI가 drift를 검사한다.

use utoipa::{
    openapi::{
        self,
        path::{HttpMethod, OperationBuilder, PathItem},
        request_body::RequestBodyBuilder,
        schema::{ObjectBuilder, OneOfBuilder, Schema, Type},
        Content, Ref, RefOr, Required, ResponseBuilder, ResponsesBuilder,
    },
    OpenApi,
};

use crate::{
    call::OperationId,
    descriptor::OperationKind,
    events::{EventSchemaSpec, EVENT_SCHEMAS},
    operations::{OperationSpec, OPERATIONS},
};

/// 스키마 id ↔ typed 본문 봉투의 판별 union(039). 본문 DTO가 있는 `EVENT_SCHEMAS`만 싣는다.
pub const EVENT_BY_SCHEMA: &str = "EventBySchema";

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Agentic Workbench — Workbench API",
        version = "1",
        description = "Agentic Workbench 서버-클라이언트 Seam의 v1 계약. `POST /v1/calls` 하나로 모든 operation을 호출한다. 이 문서는 `crates/workbench-protocol`의 registry에서 생성된다(수정 금지)."
    ),
    components(schemas(
        crate::call::RequestId,
        crate::call::IdempotencyKey,
        crate::call::CallReply,
        crate::fault::FaultCode,
        crate::fault::Outcome,
        crate::fault::WorkbenchFault,
        crate::principal::Scope,
        crate::descriptor::OperationKind,
        crate::descriptor::Effect,
        crate::descriptor::OperationDescriptor,
        crate::descriptor::DescribeOutput,
        crate::operations::project::ProjectDto,
        crate::operations::project::ProjectListInput,
        crate::operations::project::ProjectCreateInput,
        crate::operations::project::ProjectUpdateInput,
        crate::operations::project::ProjectDeleteInput,
        crate::operations::saved_prompt::SavedPromptDto,
        crate::operations::saved_prompt::SavedPromptListInput,
        crate::operations::saved_prompt::SavedPromptCreateInput,
        crate::operations::saved_prompt::SavedPromptUpdateInput,
        crate::operations::saved_prompt::SavedPromptDeleteInput,
        crate::operations::goal::GoalStatus,
        crate::operations::goal::GoalDto,
        crate::operations::goal::GoalGetInput,
        crate::operations::goal::GoalCreateInput,
        crate::operations::goal::GoalUpdateInput,
        crate::operations::goal::GoalClearInput,
        crate::operations::goal::GoalRecordProgressInput,
        crate::operations::agent_run_settings::PermissionMode,
        crate::operations::agent_run_settings::ContextSizePreset,
        crate::operations::agent_run_settings::AgentRunSessionMode,
        crate::operations::agent_run_settings::AgentRunSettingsDto,
        crate::operations::agent_run_settings::AgentCommandOverridesDto,
        crate::operations::agent_run_settings::AgentProfileDto,
        crate::operations::agent_run_settings::AgentRunSettingsRalphLoopDto,
        crate::operations::agent_run_settings::AgentRunSettingsGetInput,
        crate::operations::agent_run_settings::AgentRunSettingsSaveInput,
        crate::operations::git::GitRemoteDto,
        crate::operations::git::GitBranchDto,
        crate::operations::git::GitWorktreeStatus,
        crate::operations::git::GitWorktreeDto,
        crate::operations::git::GitListRemotesInput,
        crate::operations::git::GitListBranchesInput,
        crate::operations::git::GitListWorktreesInput,
        crate::operations::git::GitCreateWorktreeInput,
        crate::operations::git::GitDeleteWorktreeInput,
        crate::operations::worktree::WorktreeChangeType,
        crate::operations::worktree::WorktreeChangeDto,
        crate::operations::worktree::GitChangedFileGroup,
        crate::operations::worktree::GitChangedFileDto,
        crate::operations::worktree::GitWorktreeChangesDto,
        crate::operations::worktree::GitWorktreeFileDiffDto,
        crate::operations::worktree::WorktreeFileEntryDto,
        crate::operations::worktree::WorktreeTextFileDto,
        crate::operations::worktree::WorktreeFileListKind,
        crate::operations::worktree::WorktreeFileListScopeDto,
        crate::operations::worktree::GitCommitSummaryDto,
        crate::operations::worktree::GitCommitPageDto,
        crate::operations::worktree::GitCommitHistoryDto,
        crate::operations::worktree::GitGraphCommitDto,
        crate::operations::worktree::GitGraphRefKind,
        crate::operations::worktree::GitGraphRefDto,
        crate::operations::worktree::GitGraphLayoutHintsDto,
        crate::operations::worktree::GitCommitGraphDto,
        crate::operations::worktree::GitCommitFileChangeDto,
        crate::operations::worktree::GitCommitDetailDto,
        crate::operations::worktree::GitFileDiffDto,
        crate::operations::worktree::WorktreeListChangesInput,
        crate::operations::worktree::WorktreeGetChangesInput,
        crate::operations::worktree::WorktreeGetFileDiffInput,
        crate::operations::worktree::WorktreeListFilesInput,
        crate::operations::worktree::WorktreeReadTextFileInput,
        crate::operations::worktree::WorktreeListHistoryInput,
        crate::operations::worktree::WorktreeGetGraphInput,
        crate::operations::worktree::WorktreeGetCommitDetailInput,
        crate::operations::worktree::WorktreeGetCommitFileDiffInput,
        crate::operations::agent::AgentOptionDescriptorDto,
        crate::operations::agent::AgentDescriptorDto,
        crate::operations::agent::ProviderSessionDto,
        crate::operations::agent::AgentListInput,
        crate::operations::agent::AgentListProviderSessionsInput,
        crate::operations::bench::BenchOpenInput,
        crate::operations::bench::BenchOpenOutput,
        crate::operations::bench::BenchCloseInput,
        crate::operations::bench::BenchCloseOutput,
        crate::operations::bench::BenchRequestTitleInput,
        crate::operations::bench::TitleChangeResultDto,
        crate::operations::run::ResumePolicyDto,
        crate::operations::run::RalphLoopRequestDto,
        crate::operations::run::AgentMcpHttpHeaderDto,
        crate::operations::run::AgentMcpServerConfigDto,
        crate::operations::run::AgentRunRequestDto,
        crate::operations::run::AgentRunDto,
        crate::operations::run::AgentToolCandidateQueryDto,
        crate::operations::run::AgentToolCandidateScopeDto,
        crate::operations::run::AgentToolCandidateSourceDto,
        crate::operations::run::AgentToolCandidateDto,
        crate::operations::run::AgentToolCandidateStatusDto,
        crate::operations::run::AgentToolCandidateResponseDto,
        crate::operations::run::RunListToolCandidatesInput,
        crate::operations::run::RunStartInput,
        crate::operations::run::RunPromptInput,
        crate::operations::run::RunSetPermissionModeInput,
        crate::operations::run::RunCancelInput,
        crate::operations::run::RunRespondPermissionInput,
        crate::operations::exchange::AgentPanelStatusDto,
        crate::operations::exchange::AgentExchangeDeliveryDto,
        crate::operations::exchange::AgentExchangeStatusDto,
        crate::operations::exchange::AgentPanelEndpointDto,
        crate::operations::exchange::AgentWorkspaceSyncRequestDto,
        crate::operations::exchange::AgentWorkspaceSyncResponseDto,
        crate::operations::exchange::SendAgentExchangeRequestDto,
        crate::operations::exchange::AgentExchangeAckRequestDto,
        crate::operations::exchange::AgentExchangeEndpointRefDto,
        crate::operations::exchange::AgentExchangeDto,
        crate::operations::exchange::AgentPeersDto,
        crate::operations::exchange::ExchangeSyncWorkspaceInput,
        crate::operations::exchange::ExchangeSendInput,
        crate::operations::exchange::ExchangeAcknowledgeInput,
        crate::operations::exchange::ExchangeListInput,
        crate::operations::exchange::ExchangeListPeersInput,
        crate::operations::exchange::ExchangeSendFromRunInput,
        crate::operations::exchange::ExchangeGetForRunInput,
        crate::operations::orchestration_dto::OrchestrationErrorCodeDto,
        crate::operations::orchestration_dto::AgentNodeKindDto,
        crate::operations::orchestration_dto::AgentNodeCreatorDto,
        crate::operations::orchestration_dto::TaskStatusDto,
        crate::operations::orchestration_dto::ExecutionStatusDto,
        crate::operations::orchestration_dto::PresentationStatusDto,
        crate::operations::orchestration_dto::PromotionPolicyDto,
        crate::operations::orchestration_dto::AccessPolicyDto,
        crate::operations::orchestration_dto::AgentRoleProfileDto,
        crate::operations::orchestration_dto::WorkerRuntimeProfileDto,
        crate::operations::orchestration_dto::AgentNodeDto,
        crate::operations::orchestration_dto::CoordinatorGenerationStatusDto,
        crate::operations::orchestration_dto::CoordinatorGenerationDto,
        crate::operations::orchestration_dto::TaskFailureDto,
        crate::operations::orchestration_dto::OrchestrationTaskDto,
        crate::operations::orchestration_dto::TaskReportTypeDto,
        crate::operations::orchestration_dto::FindingSeverityDto,
        crate::operations::orchestration_dto::TaskFindingDto,
        crate::operations::orchestration_dto::ArtifactKindDto,
        crate::operations::orchestration_dto::ArtifactReferenceDto,
        crate::operations::orchestration_dto::TaskReportDto,
        crate::operations::orchestration_dto::TaskCommandKindDto,
        crate::operations::orchestration_dto::TaskCommandSourceDto,
        crate::operations::orchestration_dto::TaskCommandStatusDto,
        crate::operations::orchestration_dto::CommandFailureDto,
        crate::operations::orchestration_dto::TaskCommandDto,
        crate::operations::orchestration_dto::CoordinatorNotificationStatusDto,
        crate::operations::orchestration_dto::CoordinatorNotificationDto,
        crate::operations::orchestration_dto::PromptDispatchIntentDto,
        crate::operations::orchestration_dto::PromptTargetModeDto,
        crate::operations::orchestration_dto::PromptDeliveryDto,
        crate::operations::orchestration_dto::PromptDispatchTargetStatusDto,
        crate::operations::orchestration_dto::PromptDispatchTargetDto,
        crate::operations::orchestration_dto::PromptDispatchDto,
        crate::operations::orchestration_dto::IdempotencyRecordDto,
        crate::operations::orchestration_dto::OrchestrationSessionDto,
        crate::operations::orchestration_dto::MainRunBindingStateDto,
        crate::operations::orchestration_dto::BindMainRunRequestDto,
        crate::operations::orchestration_dto::CreateChildTaskRequestDto,
        crate::operations::orchestration_dto::CreateChildTaskOutcomeDto,
        crate::operations::orchestration_dto::ReportTaskRequestDto,
        crate::operations::orchestration_dto::DelegateGoalRequestDto,
        crate::operations::orchestration_dto::DelegateGoalOutcomeDto,
        crate::operations::orchestration_dto::SetPresentationRequestDto,
        crate::operations::orchestration_dto::TaskActionRequestDto,
        crate::operations::orchestration_dto::CoordinatorHandoffRequestDto,
        crate::operations::orchestration_dto::DispatchPromptRequestDto,
        crate::operations::orchestration_dto::DeliverTaskCommandRequestDto,
        crate::operations::orchestration::OrchestrationBootstrapInput,
        crate::operations::orchestration::OrchestrationBenchInput,
        crate::operations::orchestration::OrchestrationListRecoverableInput,
        crate::operations::orchestration::OrchestrationBindCoordinatorInput,
        crate::operations::orchestration::OrchestrationDelegateGoalInput,
        crate::operations::orchestration::OrchestrationAdoptManualChildInput,
        crate::operations::orchestration::OrchestrationListTasksInput,
        crate::operations::orchestration::OrchestrationSetPresentationInput,
        crate::operations::orchestration::DeliverTaskCommandInputDto,
        crate::operations::orchestration::OrchestrationSendChildCommandInput,
        crate::operations::orchestration::OrchestrationTaskActionInput,
        crate::operations::orchestration::OrchestrationHandoffCoordinatorInput,
        crate::operations::orchestration::OrchestrationDispatchPromptInput,
        crate::operations::run::RunReplayInput,
        crate::operations::run::RunReplayDto,
        crate::operations::run::RunReplayEventDto,
        crate::operations::orchestration::AgentToolInput,
        crate::operations::orchestration::AgentToolOutput,
        crate::operations::orchestration::AgentRoleInput,
        crate::operations::orchestration::AgentRoleKindDto,
        crate::operations::orchestration::AgentRoleDto,
        crate::operations::system::SystemDescribeInput,
        crate::workbench::StreamCursor,
        crate::workbench::Subscription,
        crate::workbench::EventEnvelope,
        crate::workbench::GapReason,
        crate::workbench::GapNotice,
        crate::workbench::EventItem,
        crate::events::EventClass,
        crate::events::StreamKind,
        crate::events::EventSchemaDescriptor,
        crate::events::EventFrame,
        crate::events::run::RunEventDto,
        crate::events::run::LifecycleStatusDto,
        crate::events::run::RalphLoopStatusDto,
        crate::events::run::PlanEntryDto,
        crate::events::run::ToolFileChangeDto,
        crate::events::run::ToolFileChangeKindDto,
        crate::events::run::ToolFileChangeStatusDto,
        crate::events::run::PermissionOptionDto,
        crate::events::worktree::WorktreeChangedDto,
        crate::events::worktree::WorktreeChangeKindDto,
        crate::events::orchestration::OrchestrationEventDto,
        crate::events::exchange::ExchangeRequestedDto,
        crate::events::bench::TitleRequestedDto,
    ))
)]
struct ApiDoc;

pub const CALL_REQUEST_SCHEMA: &str = "CallRequest";
pub const CALL_REPLY_BY_OPERATION_SCHEMA: &str = "CallReplyByOperation";
pub const CALLS_PATH: &str = "/v1/calls";

fn input_schema_name(id: OperationId) -> &'static str {
    match id {
        OperationId::ProjectList => "ProjectListInput",
        OperationId::ProjectCreate => "ProjectCreateInput",
        OperationId::ProjectUpdate => "ProjectUpdateInput",
        OperationId::ProjectDelete => "ProjectDeleteInput",
        OperationId::SavedPromptList => "SavedPromptListInput",
        OperationId::SavedPromptCreate => "SavedPromptCreateInput",
        OperationId::SavedPromptUpdate => "SavedPromptUpdateInput",
        OperationId::SavedPromptDelete => "SavedPromptDeleteInput",
        OperationId::GoalGet => "GoalGetInput",
        OperationId::GoalCreate => "GoalCreateInput",
        OperationId::GoalUpdate => "GoalUpdateInput",
        OperationId::GoalClear => "GoalClearInput",
        OperationId::GoalRecordProgress => "GoalRecordProgressInput",
        OperationId::AgentRunSettingsGet => "AgentRunSettingsGetInput",
        OperationId::AgentRunSettingsSave => "AgentRunSettingsSaveInput",
        OperationId::GitListRemotes => "GitListRemotesInput",
        OperationId::GitListBranches => "GitListBranchesInput",
        OperationId::GitListWorktrees => "GitListWorktreesInput",
        OperationId::GitCreateWorktree => "GitCreateWorktreeInput",
        OperationId::GitDeleteWorktree => "GitDeleteWorktreeInput",
        OperationId::WorktreeListChanges => "WorktreeListChangesInput",
        OperationId::WorktreeGetChanges => "WorktreeGetChangesInput",
        OperationId::WorktreeGetFileDiff => "WorktreeGetFileDiffInput",
        OperationId::WorktreeListFiles => "WorktreeListFilesInput",
        OperationId::WorktreeReadTextFile => "WorktreeReadTextFileInput",
        OperationId::WorktreeListHistory => "WorktreeListHistoryInput",
        OperationId::WorktreeGetGraph => "WorktreeGetGraphInput",
        OperationId::WorktreeGetCommitDetail => "WorktreeGetCommitDetailInput",
        OperationId::WorktreeGetCommitFileDiff => "WorktreeGetCommitFileDiffInput",
        OperationId::AgentList => "AgentListInput",
        OperationId::AgentListProviderSessions => "AgentListProviderSessionsInput",
        OperationId::BenchOpen => "BenchOpenInput",
        OperationId::BenchClose => "BenchCloseInput",
        OperationId::BenchRequestTitle => "BenchRequestTitleInput",
        OperationId::RunListToolCandidates => "RunListToolCandidatesInput",
        OperationId::RunStart => "RunStartInput",
        OperationId::RunSendPrompt => "RunPromptInput",
        OperationId::RunSteer => "RunPromptInput",
        OperationId::RunCancelAndSend => "RunPromptInput",
        OperationId::RunSetPermissionMode => "RunSetPermissionModeInput",
        OperationId::RunCancel => "RunCancelInput",
        OperationId::RunRespondPermission => "RunRespondPermissionInput",
        OperationId::ExchangeSyncWorkspace => "ExchangeSyncWorkspaceInput",
        OperationId::ExchangeSend => "ExchangeSendInput",
        OperationId::ExchangeAcknowledge => "ExchangeAcknowledgeInput",
        OperationId::ExchangeList => "ExchangeListInput",
        OperationId::ExchangeListPeers => "ExchangeListPeersInput",
        OperationId::ExchangeSendFromRun => "ExchangeSendFromRunInput",
        OperationId::ExchangeGetForRun => "ExchangeGetForRunInput",
        OperationId::OrchestrationBootstrap => "OrchestrationBootstrapInput",
        OperationId::OrchestrationGet => "OrchestrationBenchInput",
        OperationId::OrchestrationListRecoverable => "OrchestrationListRecoverableInput",
        OperationId::OrchestrationBindCoordinator => "OrchestrationBindCoordinatorInput",
        OperationId::OrchestrationDelegateGoal => "OrchestrationDelegateGoalInput",
        OperationId::OrchestrationAdoptManualChild => "OrchestrationAdoptManualChildInput",
        OperationId::OrchestrationListTasks => "OrchestrationListTasksInput",
        OperationId::OrchestrationCollectReports => "OrchestrationBenchInput",
        OperationId::OrchestrationSetPresentation => "OrchestrationSetPresentationInput",
        OperationId::OrchestrationSendChildCommand => "OrchestrationSendChildCommandInput",
        OperationId::OrchestrationRespondInput => "OrchestrationTaskActionInput",
        OperationId::OrchestrationCancelTask => "OrchestrationTaskActionInput",
        OperationId::OrchestrationRetryTask => "OrchestrationTaskActionInput",
        OperationId::OrchestrationReassignTask => "OrchestrationTaskActionInput",
        OperationId::OrchestrationHandoffCoordinator => "OrchestrationHandoffCoordinatorInput",
        OperationId::OrchestrationDispatchPrompt => "OrchestrationDispatchPromptInput",
        OperationId::OrchestrationRecover => "OrchestrationBenchInput",
        OperationId::RunReplay => "RunReplayInput",
        OperationId::OrchestrationCreateChildTask => "AgentToolInput",
        OperationId::OrchestrationAssignChildTask => "AgentToolInput",
        OperationId::OrchestrationListChildTasks => "AgentToolInput",
        OperationId::OrchestrationSendChildMessage => "AgentToolInput",
        OperationId::OrchestrationWaitChildTasks => "AgentToolInput",
        OperationId::OrchestrationCollectChildResults => "AgentToolInput",
        OperationId::OrchestrationInterruptChildTask => "AgentToolInput",
        OperationId::OrchestrationCancelChildTask => "AgentToolInput",
        OperationId::OrchestrationRetryChildTask => "AgentToolInput",
        OperationId::OrchestrationReassignChildTask => "AgentToolInput",
        OperationId::OrchestrationGetOwnTask => "AgentToolInput",
        OperationId::OrchestrationReportProgress => "AgentToolInput",
        OperationId::OrchestrationReportResult => "AgentToolInput",
        OperationId::OrchestrationRequestParentInput => "AgentToolInput",
        OperationId::OrchestrationReportBlocked => "AgentToolInput",
        OperationId::OrchestrationSendParentMessage => "AgentToolInput",
        OperationId::OrchestrationGetAgentRole => "AgentRoleInput",
        OperationId::SystemDescribe => "SystemDescribeInput",
    }
}

fn dto(name: &str) -> RefOr<Schema> {
    Ref::from_schema_name(name).into()
}

fn output_schema(id: OperationId) -> RefOr<Schema> {
    use crate::operations::common::{array_schema, null_schema, nullable_schema};

    match id {
        OperationId::ProjectList => array_schema(dto("ProjectDto")),
        OperationId::ProjectCreate | OperationId::ProjectUpdate => dto("ProjectDto"),
        OperationId::ProjectDelete | OperationId::SavedPromptDelete | OperationId::GoalClear => {
            null_schema()
        }
        OperationId::SavedPromptList => array_schema(dto("SavedPromptDto")),
        OperationId::SavedPromptCreate | OperationId::SavedPromptUpdate => dto("SavedPromptDto"),
        OperationId::GoalGet => nullable_schema(dto("GoalDto")),
        OperationId::GoalCreate | OperationId::GoalUpdate | OperationId::GoalRecordProgress => {
            dto("GoalDto")
        }
        OperationId::AgentRunSettingsGet => nullable_schema(dto("AgentRunSettingsDto")),
        OperationId::AgentRunSettingsSave => dto("AgentRunSettingsDto"),
        OperationId::GitListRemotes => array_schema(dto("GitRemoteDto")),
        OperationId::GitListBranches => array_schema(dto("GitBranchDto")),
        OperationId::GitListWorktrees => array_schema(dto("GitWorktreeDto")),
        OperationId::GitCreateWorktree => null_schema(),
        OperationId::GitDeleteWorktree => null_schema(),
        OperationId::WorktreeListChanges => array_schema(dto("WorktreeChangeDto")),
        OperationId::WorktreeGetChanges => dto("GitWorktreeChangesDto"),
        OperationId::WorktreeGetFileDiff => dto("GitWorktreeFileDiffDto"),
        OperationId::WorktreeListFiles => array_schema(dto("WorktreeFileEntryDto")),
        OperationId::WorktreeReadTextFile => dto("WorktreeTextFileDto"),
        OperationId::WorktreeListHistory => dto("GitCommitHistoryDto"),
        OperationId::WorktreeGetGraph => dto("GitCommitGraphDto"),
        OperationId::WorktreeGetCommitDetail => dto("GitCommitDetailDto"),
        OperationId::WorktreeGetCommitFileDiff => dto("GitFileDiffDto"),
        OperationId::AgentList => array_schema(dto("AgentDescriptorDto")),
        OperationId::AgentListProviderSessions => array_schema(dto("ProviderSessionDto")),
        OperationId::BenchOpen => dto("BenchOpenOutput"),
        OperationId::BenchClose => dto("BenchCloseOutput"),
        OperationId::BenchRequestTitle => dto("TitleChangeResultDto"),
        OperationId::RunListToolCandidates => dto("AgentToolCandidateResponseDto"),
        OperationId::RunStart => dto("AgentRunDto"),
        OperationId::RunSendPrompt => null_schema(),
        OperationId::RunSteer => null_schema(),
        OperationId::RunCancelAndSend => null_schema(),
        OperationId::RunSetPermissionMode => null_schema(),
        OperationId::RunCancel => null_schema(),
        OperationId::RunRespondPermission => null_schema(),
        OperationId::ExchangeSyncWorkspace => dto("AgentWorkspaceSyncResponseDto"),
        OperationId::ExchangeSend => dto("AgentExchangeDto"),
        OperationId::ExchangeAcknowledge => dto("AgentExchangeDto"),
        OperationId::ExchangeList => array_schema(dto("AgentExchangeDto")),
        OperationId::ExchangeListPeers => dto("AgentPeersDto"),
        OperationId::ExchangeSendFromRun => dto("AgentExchangeDto"),
        OperationId::ExchangeGetForRun => dto("AgentExchangeDto"),
        OperationId::OrchestrationBootstrap => dto("OrchestrationSessionDto"),
        OperationId::OrchestrationGet => nullable_schema(dto("OrchestrationSessionDto")),
        OperationId::OrchestrationListRecoverable => array_schema(dto("OrchestrationSessionDto")),
        OperationId::OrchestrationBindCoordinator => dto("OrchestrationSessionDto"),
        OperationId::OrchestrationDelegateGoal => dto("DelegateGoalOutcomeDto"),
        OperationId::OrchestrationAdoptManualChild => dto("OrchestrationSessionDto"),
        OperationId::OrchestrationListTasks => array_schema(dto("OrchestrationTaskDto")),
        OperationId::OrchestrationCollectReports => array_schema(dto("TaskReportDto")),
        OperationId::OrchestrationSetPresentation => dto("OrchestrationSessionDto"),
        OperationId::OrchestrationSendChildCommand => dto("TaskCommandDto"),
        OperationId::OrchestrationRespondInput => dto("TaskCommandDto"),
        OperationId::OrchestrationCancelTask => dto("OrchestrationSessionDto"),
        OperationId::OrchestrationRetryTask => dto("OrchestrationSessionDto"),
        OperationId::OrchestrationReassignTask => dto("OrchestrationSessionDto"),
        OperationId::OrchestrationHandoffCoordinator => dto("OrchestrationSessionDto"),
        OperationId::OrchestrationDispatchPrompt => dto("PromptDispatchDto"),
        OperationId::OrchestrationRecover => dto("OrchestrationSessionDto"),
        OperationId::RunReplay => dto("RunReplayDto"),
        OperationId::OrchestrationCreateChildTask => dto("AgentToolOutput"),
        OperationId::OrchestrationAssignChildTask => dto("AgentToolOutput"),
        OperationId::OrchestrationListChildTasks => dto("AgentToolOutput"),
        OperationId::OrchestrationSendChildMessage => dto("AgentToolOutput"),
        OperationId::OrchestrationWaitChildTasks => dto("AgentToolOutput"),
        OperationId::OrchestrationCollectChildResults => dto("AgentToolOutput"),
        OperationId::OrchestrationInterruptChildTask => dto("AgentToolOutput"),
        OperationId::OrchestrationCancelChildTask => dto("AgentToolOutput"),
        OperationId::OrchestrationRetryChildTask => dto("AgentToolOutput"),
        OperationId::OrchestrationReassignChildTask => dto("AgentToolOutput"),
        OperationId::OrchestrationGetOwnTask => dto("AgentToolOutput"),
        OperationId::OrchestrationReportProgress => dto("AgentToolOutput"),
        OperationId::OrchestrationReportResult => dto("AgentToolOutput"),
        OperationId::OrchestrationRequestParentInput => dto("AgentToolOutput"),
        OperationId::OrchestrationReportBlocked => dto("AgentToolOutput"),
        OperationId::OrchestrationSendParentMessage => dto("AgentToolOutput"),
        OperationId::OrchestrationGetAgentRole => dto("AgentRoleDto"),
        OperationId::SystemDescribe => dto("DescribeOutput"),
    }
}

fn variant_title(prefix: &str, id: OperationId) -> String {
    format!("{prefix}_{}", id.as_str().replace('.', "_"))
}

fn operation_literal(id: OperationId) -> ObjectBuilder {
    ObjectBuilder::new()
        .schema_type(Type::String)
        .enum_values(Some([id.as_str()]))
}

fn integer(description: &str) -> ObjectBuilder {
    ObjectBuilder::new()
        .schema_type(Type::Integer)
        .description(Some(description))
}

/// `CallRequest` variant: 공통 필드 + `operation` 리터럴 + typed `input`.
fn request_variant(spec: &OperationSpec) -> RefOr<Schema> {
    let mut object = ObjectBuilder::new()
        .title(Some(variant_title("CallRequest", spec.id)))
        .property(
            "protocolVersion",
            integer("generic call/event wire 호환성 축. 037은 1만 지원한다."),
        )
        .property("operation", operation_literal(spec.id))
        .property("requestId", Ref::from_schema_name("RequestId"))
        .property("input", Ref::from_schema_name(input_schema_name(spec.id)))
        .property("idempotencyKey", Ref::from_schema_name("IdempotencyKey"))
        .property(
            "expectedRevision",
            integer("command에서만 의미. aggregate revision과 다르면 preconditionFailed."),
        )
        .property(
            "timeoutMs",
            integer("서버가 상한을 적용하는 상대 시간(ms). 037은 검증만 한다."),
        )
        .required("protocolVersion")
        .required("operation")
        .required("requestId")
        .required("input");
    if spec.kind == OperationKind::Command {
        object = object.required("idempotencyKey");
    }
    RefOr::T(Schema::Object(object.build()))
}

/// typed result: `operation` 리터럴 + `output`(정본 "typed result schema").
fn reply_variant(spec: &OperationSpec) -> RefOr<Schema> {
    let object = ObjectBuilder::new()
        .title(Some(variant_title("CallReply", spec.id)))
        .property(
            "kind",
            ObjectBuilder::new()
                .schema_type(Type::String)
                .enum_values(Some(["complete"])),
        )
        .property("operation", operation_literal(spec.id))
        .property("output", output_schema(spec.id))
        .property(
            "revision",
            integer("command 성공 시 새 aggregate revision. query는 없다."),
        )
        .required("kind")
        .required("operation")
        .required("output");
    RefOr::T(Schema::Object(object.build()))
}

/// `EventBySchema` variant: `EventEnvelope`의 필드 + `schema` 리터럴 + typed `body`.
fn event_variant(spec: &EventSchemaSpec, body_schema: &str) -> RefOr<Schema> {
    let string = || ObjectBuilder::new().schema_type(Type::String);
    let object = ObjectBuilder::new()
        .title(Some(format!("Event_{}", spec.schema.replace('.', "_"))))
        .property("eventId", string())
        .property("streamId", string())
        .property("epoch", string())
        .property(
            "sequence",
            integer("스트림 안에서 1부터 1씩 증가한다. 알림용 스트림은 구독 단위로만 의미가 있다."),
        )
        .property("schema", string().enum_values(Some([spec.schema])))
        .property("occurredAt", string())
        .property("correlationId", Ref::from_schema_name("RequestId"))
        .property("body", Ref::from_schema_name(body_schema))
        .required("eventId")
        .required("streamId")
        .required("epoch")
        .required("sequence")
        .required("schema")
        .required("occurredAt")
        .required("body");
    RefOr::T(Schema::Object(object.build()))
}

fn one_of(variants: impl IntoIterator<Item = RefOr<Schema>>) -> RefOr<Schema> {
    let mut builder = OneOfBuilder::new();
    for variant in variants {
        builder = builder.item(variant);
    }
    RefOr::T(Schema::OneOf(builder.build()))
}

fn calls_path_item() -> PathItem {
    let request_body = RequestBodyBuilder::new()
        .description(Some("CallRequest — operation별 variant 중 하나"))
        .required(Some(Required::True))
        .content(
            "application/json",
            Content::new(Some(Ref::from_schema_name(CALL_REQUEST_SCHEMA))),
        )
        .build();
    let responses = ResponsesBuilder::new()
        .response(
            "200",
            ResponseBuilder::new()
                .description("CallReply")
                .content(
                    "application/json",
                    Content::new(Some(Ref::from_schema_name("CallReply"))),
                )
                .build(),
        )
        .response(
            "default",
            ResponseBuilder::new()
                .description(
                    "WorkbenchFault (RFC 9457 problem+json). HTTP status는 FaultCode에 따른다.",
                )
                .content(
                    "application/problem+json",
                    Content::new(Some(Ref::from_schema_name("WorkbenchFault"))),
                )
                .build(),
        )
        .build();
    let operation = OperationBuilder::new()
        .operation_id(Some("callWorkbench"))
        .summary(Some("Workbench.call"))
        .description(Some(
            "단일 operation을 호출한다. operation·input·output의 상관관계는 components.schemas.CallRequest / CallReplyByOperation의 oneOf variant로 표현된다.",
        ))
        .request_body(Some(request_body))
        .responses(responses)
        .build();
    PathItem::new(HttpMethod::Post, operation)
}

/// 전체 문서. 결정적 출력을 위해 registry 순서대로 variant를 넣는다.
pub fn build_openapi() -> openapi::OpenApi {
    let mut doc = ApiDoc::openapi();
    let components = doc.components.get_or_insert_with(Default::default);
    components.schemas.insert(
        CALL_REQUEST_SCHEMA.to_owned(),
        one_of(OPERATIONS.iter().map(request_variant)),
    );
    components.schemas.insert(
        CALL_REPLY_BY_OPERATION_SCHEMA.to_owned(),
        one_of(OPERATIONS.iter().map(reply_variant)),
    );
    components.schemas.insert(
        EVENT_BY_SCHEMA.to_owned(),
        one_of(EVENT_SCHEMAS.iter().filter_map(|spec| {
            spec.body_schema
                .map(|body_schema| event_variant(spec, body_schema))
        })),
    );
    doc.paths
        .paths
        .insert(CALLS_PATH.to_owned(), calls_path_item());
    doc
}

/// `export_openapi` bin과 golden test가 같은 문자열을 쓴다(끝 개행 포함).
pub fn render_openapi() -> String {
    let mut json = serde_json::to_string_pretty(&build_openapi()).expect("openapi serializes");
    json.push('\n');
    json
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn doc_json() -> Value {
        serde_json::from_str(&render_openapi()).unwrap()
    }

    #[test]
    fn targets_openapi_3_1_and_declares_calls_path() {
        let doc = doc_json();
        assert_eq!(doc["openapi"], "3.1.0");
        assert!(doc["paths"][CALLS_PATH]["post"].is_object());
        assert_eq!(
            doc["paths"][CALLS_PATH]["post"]["operationId"],
            "callWorkbench"
        );
    }

    /// 200 응답이 참조하는 generic `CallReply`는 어떤 operation 결과도 거절하면 안 된다(`project.list`는 배열).
    #[test]
    fn generic_reply_envelope_accepts_any_output_and_uses_camel_case() {
        let doc = doc_json();
        let variants = doc["components"]["schemas"]["CallReply"]["oneOf"]
            .as_array()
            .expect("CallReply oneOf");
        let complete = variants
            .iter()
            .find(|v| v["properties"]["kind"]["enum"] == serde_json::json!(["complete"]))
            .expect("complete variant");
        assert!(
            complete["properties"]["output"].get("type").is_none(),
            "output must be unconstrained JSON, got {}",
            complete["properties"]["output"]
        );
        let accepted = variants
            .iter()
            .find(|v| v["properties"]["kind"]["enum"] == serde_json::json!(["accepted"]))
            .expect("accepted variant");
        assert!(accepted["properties"]["executionId"].is_object());
        assert!(accepted["properties"].get("execution_id").is_none());
        assert_eq!(
            doc["paths"][CALLS_PATH]["post"]["responses"]["200"]["content"]["application/json"]
                ["schema"]["$ref"],
            "#/components/schemas/CallReply"
        );
    }

    #[test]
    fn request_union_has_one_variant_per_operation_with_literal_operation() {
        let doc = doc_json();
        let variants = doc["components"]["schemas"][CALL_REQUEST_SCHEMA]["oneOf"]
            .as_array()
            .expect("oneOf");
        assert_eq!(variants.len(), OPERATIONS.len());
        for (variant, spec) in variants.iter().zip(OPERATIONS.iter()) {
            let literal = &variant["properties"]["operation"]["enum"];
            assert_eq!(literal, &serde_json::json!([spec.id.as_str()]));
            let required = variant["required"].as_array().unwrap();
            assert_eq!(
                required.iter().any(|r| r == "idempotencyKey"),
                spec.kind == OperationKind::Command,
                "{}",
                spec.id
            );
            assert_eq!(
                variant["properties"]["input"]["$ref"],
                format!("#/components/schemas/{}", input_schema_name(spec.id))
            );
        }
    }

    #[test]
    fn reply_union_pairs_each_operation_with_its_output() {
        let doc = doc_json();
        let variants = doc["components"]["schemas"][CALL_REPLY_BY_OPERATION_SCHEMA]["oneOf"]
            .as_array()
            .expect("oneOf");
        assert_eq!(variants.len(), OPERATIONS.len());
        let output_of = |operation: &str| -> serde_json::Value {
            variants
                .iter()
                .find(|variant| variant["properties"]["operation"]["enum"][0] == operation)
                .unwrap_or_else(|| panic!("variant for {operation}"))["properties"]["output"]
                .clone()
        };
        assert_eq!(output_of("project.list")["type"], "array");
        assert_eq!(
            output_of("project.create")["$ref"],
            "#/components/schemas/ProjectDto"
        );
        assert_eq!(
            output_of("system.describe")["$ref"],
            "#/components/schemas/DescribeOutput"
        );
        // 038: null 출력과 nullable 출력
        assert_eq!(output_of("project.delete")["type"], "null");
        let nullable = output_of("goal.get");
        assert_eq!(nullable["oneOf"][0]["$ref"], "#/components/schemas/GoalDto");
        assert_eq!(nullable["oneOf"][1]["type"], "null");
        assert_eq!(
            output_of("savedPrompt.list")["items"]["$ref"],
            "#/components/schemas/SavedPromptDto"
        );
    }

    #[test]
    fn rendering_is_deterministic() {
        assert_eq!(render_openapi(), render_openapi());
    }

    /// 커밋된 생성물과 코드가 어긋나면 실패한다. `pnpm run generate:contracts`로 갱신한다.
    #[test]
    fn committed_openapi_matches_registry() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("openapi")
            .join("workbench.openapi.json");
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "{} 없음 ({error}). `pnpm run generate:contracts`를 실행해 생성물을 커밋하세요.",
                path.display()
            )
        });
        assert_eq!(
            committed,
            render_openapi(),
            "openapi drift: `pnpm run generate:contracts`를 실행하세요."
        );
    }

    #[test]
    fn event_by_schema_correlates_schema_id_with_typed_body() {
        let doc = doc_json();
        let variants = doc["components"]["schemas"][EVENT_BY_SCHEMA]["oneOf"]
            .as_array()
            .expect("EventBySchema oneOf");
        let pairs: Vec<(String, String)> = variants
            .iter()
            .map(|variant| {
                (
                    variant["properties"]["schema"]["enum"][0]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                    variant["properties"]["body"]["$ref"]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                )
            })
            .collect();
        assert_eq!(
            pairs,
            [
                ("run.event.v1", "#/components/schemas/RunEventDto"),
                (
                    "worktree.changed.v1",
                    "#/components/schemas/WorktreeChangedDto"
                ),
                (
                    "orchestration.workspaceUpdated.v1",
                    "#/components/schemas/OrchestrationEventDto"
                ),
                (
                    "exchange.requested.v1",
                    "#/components/schemas/ExchangeRequestedDto"
                ),
                (
                    "exchange.status.v1",
                    "#/components/schemas/AgentExchangeDto"
                ),
                (
                    "bench.titleRequested.v1",
                    "#/components/schemas/TitleRequestedDto"
                ),
            ]
            .map(|(schema, body)| (schema.to_owned(), body.to_owned()))
        );
        for name in ["EventFrame", "GapNotice", "EventItem", "RunEventDto"] {
            assert!(
                doc["components"]["schemas"][name].is_object(),
                "{name} component"
            );
        }
    }
}
