//! operation registry의 정적 표. `system.describe`, OpenAPI `oneOf`, authorization이 모두 이 표를 읽는다.

pub mod agent;
pub mod agent_run_settings;
pub mod bench;
pub mod common;
pub mod desktop;
pub mod exchange;
pub mod git;
pub mod goal;
pub mod lease;
pub mod orchestration;
pub mod orchestration_dto;
pub mod project;
pub mod run;
pub mod saved_prompt;
pub mod server;
pub mod system;
pub mod worktree;

use utoipa::PartialSchema;

use crate::{
    call::OperationId,
    descriptor::{Effect, IdempotencyScope, OperationKind},
    principal::Scope,
};

/// operation 하나의 정적 계약(스키마 제외).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationSpec {
    pub id: OperationId,
    pub kind: OperationKind,
    pub effect: Effect,
    pub idempotent: bool,
    /// command만 `Some`(040).
    pub idempotency_scope: Option<IdempotencyScope>,
    pub required_scopes: &'static [Scope],
}

const fn query(id: OperationId, scope: &'static [Scope]) -> OperationSpec {
    OperationSpec {
        id,
        kind: OperationKind::Query,
        effect: Effect::Read,
        idempotent: false,
        idempotency_scope: None,
        required_scopes: scope,
    }
}

const fn command(id: OperationId, scope: &'static [Scope]) -> OperationSpec {
    OperationSpec {
        id,
        kind: OperationKind::Command,
        effect: Effect::Modify,
        idempotent: true,
        idempotency_scope: Some(IdempotencyScope::Durable),
        required_scopes: scope,
    }
}

/// 세대 범위 멱등성 command(040): 메모리 상태를 바꾸는 작업대·run 제어·교환.
const fn epoch_command(id: OperationId, scope: &'static [Scope]) -> OperationSpec {
    OperationSpec {
        id,
        kind: OperationKind::Command,
        effect: Effect::Modify,
        idempotent: true,
        idempotency_scope: Some(IdempotencyScope::Epoch),
        required_scopes: scope,
    }
}

/// `OperationId::ALL`과 같은 순서.
pub const OPERATIONS: [OperationSpec; 94] = [
    query(OperationId::ProjectList, &[Scope::ProjectRead]),
    command(OperationId::ProjectCreate, &[Scope::ProjectWrite]),
    command(OperationId::ProjectUpdate, &[Scope::ProjectWrite]),
    command(OperationId::ProjectDelete, &[Scope::ProjectWrite]),
    query(OperationId::SavedPromptList, &[Scope::SavedPromptRead]),
    command(OperationId::SavedPromptCreate, &[Scope::SavedPromptWrite]),
    command(OperationId::SavedPromptUpdate, &[Scope::SavedPromptWrite]),
    command(OperationId::SavedPromptDelete, &[Scope::SavedPromptWrite]),
    query(OperationId::GoalGet, &[Scope::GoalRead]),
    command(OperationId::GoalCreate, &[Scope::GoalWrite]),
    command(OperationId::GoalUpdate, &[Scope::GoalWrite]),
    command(OperationId::GoalClear, &[Scope::GoalWrite]),
    command(OperationId::GoalRecordProgress, &[Scope::GoalWrite]),
    query(
        OperationId::AgentRunSettingsGet,
        &[Scope::AgentRunSettingsRead],
    ),
    command(
        OperationId::AgentRunSettingsSave,
        &[Scope::AgentRunSettingsWrite],
    ),
    query(OperationId::GitListRemotes, &[Scope::GitRead]),
    query(OperationId::GitListBranches, &[Scope::GitRead]),
    query(OperationId::GitListWorktrees, &[Scope::GitRead]),
    command(OperationId::GitCreateWorktree, &[Scope::GitWrite]),
    command(OperationId::GitDeleteWorktree, &[Scope::GitWrite]),
    query(OperationId::WorktreeListChanges, &[Scope::WorktreeRead]),
    query(OperationId::WorktreeGetChanges, &[Scope::WorktreeRead]),
    query(OperationId::WorktreeGetFileDiff, &[Scope::WorktreeRead]),
    query(OperationId::WorktreeListFiles, &[Scope::WorktreeRead]),
    query(OperationId::WorktreeReadTextFile, &[Scope::WorktreeRead]),
    query(OperationId::WorktreeListHistory, &[Scope::WorktreeRead]),
    query(OperationId::WorktreeGetGraph, &[Scope::WorktreeRead]),
    query(OperationId::WorktreeGetCommitDetail, &[Scope::WorktreeRead]),
    query(
        OperationId::WorktreeGetCommitFileDiff,
        &[Scope::WorktreeRead],
    ),
    query(OperationId::AgentList, &[Scope::AgentRead]),
    query(OperationId::AgentListProviderSessions, &[Scope::AgentRead]),
    epoch_command(OperationId::BenchOpen, &[Scope::BenchWrite]),
    epoch_command(OperationId::BenchClose, &[Scope::BenchWrite]),
    epoch_command(OperationId::BenchRequestTitle, &[Scope::PresentationWrite]),
    query(OperationId::RunListToolCandidates, &[Scope::RunRead]),
    command(OperationId::RunStart, &[Scope::RunWrite]),
    epoch_command(OperationId::RunSendPrompt, &[Scope::RunWrite]),
    epoch_command(OperationId::RunSteer, &[Scope::RunWrite]),
    epoch_command(OperationId::RunCancelAndSend, &[Scope::RunWrite]),
    epoch_command(OperationId::RunSetPermissionMode, &[Scope::RunWrite]),
    epoch_command(OperationId::RunCancel, &[Scope::RunWrite]),
    epoch_command(OperationId::RunRespondPermission, &[Scope::RunWrite]),
    epoch_command(OperationId::ExchangeSyncWorkspace, &[Scope::ExchangeWrite]),
    epoch_command(OperationId::ExchangeSend, &[Scope::ExchangeWrite]),
    epoch_command(OperationId::ExchangeAcknowledge, &[Scope::ExchangeWrite]),
    epoch_command(
        OperationId::ExchangeDiscardDelivery,
        &[Scope::ExchangeWrite],
    ),
    query(OperationId::ExchangeList, &[Scope::ExchangeRead]),
    query(OperationId::ExchangeListPeers, &[Scope::ExchangeRead]),
    epoch_command(OperationId::ExchangeSendFromRun, &[Scope::ExchangeWrite]),
    query(OperationId::ExchangeGetForRun, &[Scope::ExchangeRead]),
    epoch_command(
        OperationId::OrchestrationBootstrap,
        &[Scope::OrchestrationWrite],
    ),
    query(OperationId::OrchestrationGet, &[Scope::OrchestrationRead]),
    query(
        OperationId::OrchestrationListRecoverable,
        &[Scope::OrchestrationRead],
    ),
    epoch_command(
        OperationId::OrchestrationBindCoordinator,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationDelegateGoal,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationAdoptManualChild,
        &[Scope::OrchestrationWrite],
    ),
    query(
        OperationId::OrchestrationListTasks,
        &[Scope::OrchestrationRead],
    ),
    query(
        OperationId::OrchestrationCollectReports,
        &[Scope::OrchestrationRead],
    ),
    epoch_command(
        OperationId::OrchestrationSetPresentation,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationSendChildCommand,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationRespondInput,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationCancelTask,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationRetryTask,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationReassignTask,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationHandoffCoordinator,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationDispatchPrompt,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationRecover,
        &[Scope::OrchestrationWrite],
    ),
    query(OperationId::RunReplay, &[Scope::RunRead]),
    epoch_command(
        OperationId::OrchestrationCreateChildTask,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationAssignChildTask,
        &[Scope::OrchestrationWrite],
    ),
    query(
        OperationId::OrchestrationListChildTasks,
        &[Scope::OrchestrationRead],
    ),
    epoch_command(
        OperationId::OrchestrationSendChildMessage,
        &[Scope::OrchestrationWrite],
    ),
    query(
        OperationId::OrchestrationWaitChildTasks,
        &[Scope::OrchestrationRead],
    ),
    epoch_command(
        OperationId::OrchestrationCollectChildResults,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationInterruptChildTask,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationCancelChildTask,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationRetryChildTask,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationReassignChildTask,
        &[Scope::OrchestrationWrite],
    ),
    query(
        OperationId::OrchestrationGetOwnTask,
        &[Scope::OrchestrationRead],
    ),
    epoch_command(
        OperationId::OrchestrationReportProgress,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationReportResult,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationRequestParentInput,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationReportBlocked,
        &[Scope::OrchestrationWrite],
    ),
    epoch_command(
        OperationId::OrchestrationSendParentMessage,
        &[Scope::OrchestrationWrite],
    ),
    query(
        OperationId::OrchestrationGetAgentRole,
        &[Scope::OrchestrationRead],
    ),
    query(OperationId::SystemDescribe, &[Scope::SystemDescribe]),
    query(OperationId::ServerStatus, &[Scope::ServerRead]),
    epoch_command(OperationId::ServerStop, &[Scope::ServerAdmin]),
    epoch_command(OperationId::LeaseAcquire, &[Scope::ServerAdmin]),
    epoch_command(OperationId::LeaseRenew, &[Scope::ServerAdmin]),
    epoch_command(OperationId::LeaseRelease, &[Scope::ServerAdmin]),
    epoch_command(OperationId::DesktopIssueWindowToken, &[Scope::ServerAdmin]),
    epoch_command(OperationId::DesktopRetireWindow, &[Scope::ServerAdmin]),
    query(OperationId::BenchList, &[Scope::BenchRead]),
];

pub fn spec_for(id: OperationId) -> &'static OperationSpec {
    OPERATIONS
        .iter()
        .find(|spec| spec.id == id)
        .expect("every OperationId has a spec")
}

/// operation의 (input, output) JSON Schema. descriptor와 OpenAPI가 공유하는 유일한 출처다.
pub fn schema_for(id: OperationId) -> (serde_json::Value, serde_json::Value) {
    use common::EmptyOutput;

    let (input, output) = match id {
        OperationId::ProjectList => (
            project::ProjectListInput::schema(),
            project::project_list_output_schema(),
        ),
        OperationId::ProjectCreate => (
            project::ProjectCreateInput::schema(),
            project::ProjectDto::schema(),
        ),
        OperationId::ProjectUpdate => (
            project::ProjectUpdateInput::schema(),
            project::ProjectDto::schema(),
        ),
        OperationId::ProjectDelete => {
            (project::ProjectDeleteInput::schema(), EmptyOutput::schema())
        }
        OperationId::SavedPromptList => (
            saved_prompt::SavedPromptListInput::schema(),
            saved_prompt::saved_prompt_list_output_schema(),
        ),
        OperationId::SavedPromptCreate => (
            saved_prompt::SavedPromptCreateInput::schema(),
            saved_prompt::SavedPromptDto::schema(),
        ),
        OperationId::SavedPromptUpdate => (
            saved_prompt::SavedPromptUpdateInput::schema(),
            saved_prompt::SavedPromptDto::schema(),
        ),
        OperationId::SavedPromptDelete => (
            saved_prompt::SavedPromptDeleteInput::schema(),
            EmptyOutput::schema(),
        ),
        OperationId::GoalGet => (goal::GoalGetInput::schema(), goal::goal_get_output_schema()),
        OperationId::GoalCreate => (goal::GoalCreateInput::schema(), goal::GoalDto::schema()),
        OperationId::GoalUpdate => (goal::GoalUpdateInput::schema(), goal::GoalDto::schema()),
        OperationId::GoalClear => (goal::GoalClearInput::schema(), EmptyOutput::schema()),
        OperationId::GoalRecordProgress => (
            goal::GoalRecordProgressInput::schema(),
            goal::GoalDto::schema(),
        ),
        OperationId::AgentRunSettingsGet => (
            agent_run_settings::AgentRunSettingsGetInput::schema(),
            agent_run_settings::agent_run_settings_get_output_schema(),
        ),
        OperationId::AgentRunSettingsSave => (
            agent_run_settings::AgentRunSettingsSaveInput::schema(),
            agent_run_settings::AgentRunSettingsDto::schema(),
        ),
        OperationId::GitListRemotes => (
            git::GitListRemotesInput::schema(),
            common::array_schema(git::GitRemoteDto::schema()),
        ),
        OperationId::GitListBranches => (
            git::GitListBranchesInput::schema(),
            common::array_schema(git::GitBranchDto::schema()),
        ),
        OperationId::GitListWorktrees => (
            git::GitListWorktreesInput::schema(),
            common::array_schema(git::GitWorktreeDto::schema()),
        ),
        OperationId::GitCreateWorktree => {
            (git::GitCreateWorktreeInput::schema(), EmptyOutput::schema())
        }
        OperationId::GitDeleteWorktree => {
            (git::GitDeleteWorktreeInput::schema(), EmptyOutput::schema())
        }
        OperationId::WorktreeListChanges => (
            worktree::WorktreeListChangesInput::schema(),
            common::array_schema(worktree::WorktreeChangeDto::schema()),
        ),
        OperationId::WorktreeGetChanges => (
            worktree::WorktreeGetChangesInput::schema(),
            worktree::GitWorktreeChangesDto::schema(),
        ),
        OperationId::WorktreeGetFileDiff => (
            worktree::WorktreeGetFileDiffInput::schema(),
            worktree::GitWorktreeFileDiffDto::schema(),
        ),
        OperationId::WorktreeListFiles => (
            worktree::WorktreeListFilesInput::schema(),
            common::array_schema(worktree::WorktreeFileEntryDto::schema()),
        ),
        OperationId::WorktreeReadTextFile => (
            worktree::WorktreeReadTextFileInput::schema(),
            worktree::WorktreeTextFileDto::schema(),
        ),
        OperationId::WorktreeListHistory => (
            worktree::WorktreeListHistoryInput::schema(),
            worktree::GitCommitHistoryDto::schema(),
        ),
        OperationId::WorktreeGetGraph => (
            worktree::WorktreeGetGraphInput::schema(),
            worktree::GitCommitGraphDto::schema(),
        ),
        OperationId::WorktreeGetCommitDetail => (
            worktree::WorktreeGetCommitDetailInput::schema(),
            worktree::GitCommitDetailDto::schema(),
        ),
        OperationId::WorktreeGetCommitFileDiff => (
            worktree::WorktreeGetCommitFileDiffInput::schema(),
            worktree::GitFileDiffDto::schema(),
        ),
        OperationId::AgentList => (
            agent::AgentListInput::schema(),
            common::array_schema(agent::AgentDescriptorDto::schema()),
        ),
        OperationId::AgentListProviderSessions => (
            agent::AgentListProviderSessionsInput::schema(),
            common::array_schema(agent::ProviderSessionDto::schema()),
        ),
        OperationId::BenchOpen => (
            bench::BenchOpenInput::schema(),
            bench::BenchOpenOutput::schema(),
        ),
        OperationId::BenchClose => (
            bench::BenchCloseInput::schema(),
            bench::BenchCloseOutput::schema(),
        ),
        OperationId::BenchRequestTitle => (
            bench::BenchRequestTitleInput::schema(),
            bench::TitleChangeResultDto::schema(),
        ),
        OperationId::RunListToolCandidates => (
            run::RunListToolCandidatesInput::schema(),
            run::AgentToolCandidateResponseDto::schema(),
        ),
        OperationId::RunStart => (run::RunStartInput::schema(), run::AgentRunDto::schema()),
        OperationId::RunSendPrompt => (run::RunPromptInput::schema(), EmptyOutput::schema()),
        OperationId::RunSteer => (run::RunPromptInput::schema(), EmptyOutput::schema()),
        OperationId::RunCancelAndSend => (run::RunPromptInput::schema(), EmptyOutput::schema()),
        OperationId::RunSetPermissionMode => (
            run::RunSetPermissionModeInput::schema(),
            EmptyOutput::schema(),
        ),
        OperationId::RunCancel => (run::RunCancelInput::schema(), EmptyOutput::schema()),
        OperationId::RunRespondPermission => (
            run::RunRespondPermissionInput::schema(),
            EmptyOutput::schema(),
        ),
        OperationId::ExchangeSyncWorkspace => (
            exchange::ExchangeSyncWorkspaceInput::schema(),
            exchange::AgentWorkspaceSyncResponseDto::schema(),
        ),
        OperationId::ExchangeSend => (
            exchange::ExchangeSendInput::schema(),
            exchange::AgentExchangeDto::schema(),
        ),
        OperationId::ExchangeAcknowledge => (
            exchange::ExchangeAcknowledgeInput::schema(),
            exchange::AgentExchangeDto::schema(),
        ),
        OperationId::ExchangeDiscardDelivery => (
            exchange::ExchangeDiscardDeliveryInput::schema(),
            EmptyOutput::schema(),
        ),
        OperationId::ExchangeList => (
            exchange::ExchangeListInput::schema(),
            exchange::exchange_list_output_schema(),
        ),
        OperationId::ExchangeListPeers => (
            exchange::ExchangeListPeersInput::schema(),
            exchange::AgentPeersDto::schema(),
        ),
        OperationId::ExchangeSendFromRun => (
            exchange::ExchangeSendFromRunInput::schema(),
            exchange::AgentExchangeDto::schema(),
        ),
        OperationId::ExchangeGetForRun => (
            exchange::ExchangeGetForRunInput::schema(),
            exchange::AgentExchangeDto::schema(),
        ),
        OperationId::OrchestrationBootstrap => (
            orchestration::OrchestrationBootstrapInput::schema(),
            orchestration_dto::OrchestrationSessionDto::schema(),
        ),
        OperationId::OrchestrationGet => (
            orchestration::OrchestrationBenchInput::schema(),
            orchestration::session_or_null_schema(),
        ),
        OperationId::OrchestrationListRecoverable => (
            orchestration::OrchestrationListRecoverableInput::schema(),
            orchestration::sessions_schema(),
        ),
        OperationId::OrchestrationBindCoordinator => (
            orchestration::OrchestrationBindCoordinatorInput::schema(),
            orchestration_dto::OrchestrationSessionDto::schema(),
        ),
        OperationId::OrchestrationDelegateGoal => (
            orchestration::OrchestrationDelegateGoalInput::schema(),
            orchestration_dto::DelegateGoalOutcomeDto::schema(),
        ),
        OperationId::OrchestrationAdoptManualChild => (
            orchestration::OrchestrationAdoptManualChildInput::schema(),
            orchestration_dto::OrchestrationSessionDto::schema(),
        ),
        OperationId::OrchestrationListTasks => (
            orchestration::OrchestrationListTasksInput::schema(),
            orchestration::tasks_schema(),
        ),
        OperationId::OrchestrationCollectReports => (
            orchestration::OrchestrationBenchInput::schema(),
            orchestration::reports_schema(),
        ),
        OperationId::OrchestrationSetPresentation => (
            orchestration::OrchestrationSetPresentationInput::schema(),
            orchestration_dto::OrchestrationSessionDto::schema(),
        ),
        OperationId::OrchestrationSendChildCommand => (
            orchestration::OrchestrationSendChildCommandInput::schema(),
            orchestration_dto::TaskCommandDto::schema(),
        ),
        OperationId::OrchestrationRespondInput => (
            orchestration::OrchestrationTaskActionInput::schema(),
            orchestration_dto::TaskCommandDto::schema(),
        ),
        OperationId::OrchestrationCancelTask => (
            orchestration::OrchestrationTaskActionInput::schema(),
            orchestration_dto::OrchestrationSessionDto::schema(),
        ),
        OperationId::OrchestrationRetryTask => (
            orchestration::OrchestrationTaskActionInput::schema(),
            orchestration_dto::OrchestrationSessionDto::schema(),
        ),
        OperationId::OrchestrationReassignTask => (
            orchestration::OrchestrationTaskActionInput::schema(),
            orchestration_dto::OrchestrationSessionDto::schema(),
        ),
        OperationId::OrchestrationHandoffCoordinator => (
            orchestration::OrchestrationHandoffCoordinatorInput::schema(),
            orchestration_dto::OrchestrationSessionDto::schema(),
        ),
        OperationId::OrchestrationDispatchPrompt => (
            orchestration::OrchestrationDispatchPromptInput::schema(),
            orchestration_dto::PromptDispatchDto::schema(),
        ),
        OperationId::OrchestrationRecover => (
            orchestration::OrchestrationBenchInput::schema(),
            orchestration_dto::OrchestrationSessionDto::schema(),
        ),
        OperationId::RunReplay => (run::RunReplayInput::schema(), run::RunReplayDto::schema()),
        OperationId::OrchestrationCreateChildTask => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationAssignChildTask => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationListChildTasks => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationSendChildMessage => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationWaitChildTasks => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationCollectChildResults => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationInterruptChildTask => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationCancelChildTask => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationRetryChildTask => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationReassignChildTask => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationGetOwnTask => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationReportProgress => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationReportResult => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationRequestParentInput => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationReportBlocked => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationSendParentMessage => (
            orchestration::AgentToolInput::schema(),
            orchestration::AgentToolOutput::schema(),
        ),
        OperationId::OrchestrationGetAgentRole => (
            orchestration::AgentRoleInput::schema(),
            orchestration::AgentRoleDto::schema(),
        ),
        OperationId::SystemDescribe => (
            system::SystemDescribeInput::schema(),
            crate::descriptor::DescribeOutput::schema(),
        ),
        OperationId::ServerStatus => (
            server::ServerStatusInput::schema(),
            server::ServerStatusOutput::schema(),
        ),
        OperationId::ServerStop => (
            server::ServerStopInput::schema(),
            server::ServerStopOutput::schema(),
        ),
        OperationId::LeaseAcquire => (
            lease::LeaseAcquireInput::schema(),
            lease::LeaseAcquireOutput::schema(),
        ),
        OperationId::LeaseRenew => (
            lease::LeaseRenewInput::schema(),
            lease::LeaseRenewOutput::schema(),
        ),
        OperationId::LeaseRelease => (
            lease::LeaseReleaseInput::schema(),
            lease::LeaseReleaseOutput::schema(),
        ),
        OperationId::DesktopIssueWindowToken => (
            desktop::DesktopIssueWindowTokenInput::schema(),
            desktop::DesktopIssueWindowTokenOutput::schema(),
        ),
        OperationId::DesktopRetireWindow => (
            desktop::DesktopRetireWindowInput::schema(),
            desktop::DesktopRetireWindowOutput::schema(),
        ),
        OperationId::BenchList => (
            bench::BenchListInput::schema(),
            bench::bench_list_output_schema(),
        ),
    };
    (
        serde_json::to_value(input).expect("schema serializes"),
        serde_json::to_value(output).expect("schema serializes"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_covers_every_operation_in_order() {
        assert_eq!(OPERATIONS.len(), OperationId::ALL.len());
        for (spec, id) in OPERATIONS.iter().zip(OperationId::ALL) {
            assert_eq!(spec.id, id);
            assert_eq!(spec_for(id).id, id);
            let (input, output) = schema_for(id);
            assert!(input.is_object(), "{id}: input schema");
            assert!(output.is_object(), "{id}: output schema");
        }
    }

    #[test]
    fn commands_are_idempotent_and_need_write_scope() {
        for spec in OPERATIONS {
            match spec.kind {
                OperationKind::Command => {
                    assert!(spec.idempotent, "{}", spec.id);
                    assert!(
                        spec.required_scopes.iter().all(|scope| !scope.is_read()),
                        "{}",
                        spec.id
                    );
                }
                OperationKind::Query => {
                    assert!(!spec.idempotent, "{}", spec.id);
                    assert!(
                        spec.required_scopes.iter().all(|scope| scope.is_read()),
                        "{}",
                        spec.id
                    );
                }
            }
        }
    }
}
