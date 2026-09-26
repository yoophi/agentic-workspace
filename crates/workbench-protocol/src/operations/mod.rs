//! operation registry의 정적 표. `system.describe`, OpenAPI `oneOf`, authorization이 모두 이 표를 읽는다.

pub mod agent_run_settings;
pub mod common;
pub mod git;
pub mod goal;
pub mod project;
pub mod saved_prompt;
pub mod system;
pub mod worktree;

use utoipa::PartialSchema;

use crate::{
    call::OperationId,
    descriptor::{Effect, OperationKind},
    principal::Scope,
};

/// operation 하나의 정적 계약(스키마 제외).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationSpec {
    pub id: OperationId,
    pub kind: OperationKind,
    pub effect: Effect,
    pub idempotent: bool,
    pub required_scopes: &'static [Scope],
}

const fn query(id: OperationId, scope: &'static [Scope]) -> OperationSpec {
    OperationSpec {
        id,
        kind: OperationKind::Query,
        effect: Effect::Read,
        idempotent: false,
        required_scopes: scope,
    }
}

const fn command(id: OperationId, scope: &'static [Scope]) -> OperationSpec {
    OperationSpec {
        id,
        kind: OperationKind::Command,
        effect: Effect::Modify,
        idempotent: true,
        required_scopes: scope,
    }
}

/// `OperationId::ALL`과 같은 순서.
pub const OPERATIONS: [OperationSpec; 30] = [
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
    query(OperationId::SystemDescribe, &[Scope::SystemDescribe]),
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
        OperationId::SystemDescribe => (
            system::SystemDescribeInput::schema(),
            crate::descriptor::DescribeOutput::schema(),
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
