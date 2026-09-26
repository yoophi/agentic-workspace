//! `worktree.*` handler(038 US2). 전부 조회: lock 없이 blocking pool에서 실행한다. 안전 규칙(root 검사·512KB·
//! UTF-8·숨김/제외 디렉터리)은 어댑터가 값 그대로 적용한다(FR-007).

use workbench_protocol::{
    operations::worktree::{
        GitCommitDetailDto, GitCommitGraphDto, GitCommitHistoryDto, GitFileDiffDto,
        GitWorktreeChangesDto, GitWorktreeFileDiffDto, WorktreeChangeDto, WorktreeFileEntryDto,
        WorktreeGetChangesInput, WorktreeGetCommitDetailInput, WorktreeGetCommitFileDiffInput,
        WorktreeGetFileDiffInput, WorktreeGetGraphInput, WorktreeListChangesInput,
        WorktreeListFilesInput, WorktreeListHistoryInput, WorktreeReadTextFileInput,
        WorktreeTextFileDto,
    },
    OperationId,
};

use crate::{
    application::{
        git_dto::{
            git_commit_detail_dto, git_commit_graph_dto, git_commit_history_dto, git_file_diff_dto,
            git_worktree_changes_dto, git_worktree_file_diff_dto, saturating_usize,
            worktree_change_dto, worktree_file_entry_dto, worktree_file_list_scope_domain,
            worktree_text_file_dto,
        },
        handlers::{git_fault, query_handler, worktree_file_fault},
        registry::Registry,
        worktree_changes_service, worktree_file_service, worktree_git_service,
    },
    domain::errors::{GitError, WorktreeFileError},
    infrastructure::{
        fs::worktree_file_provider::FsWorktreeFileProvider,
        git::{
            cli_worktree_change_provider::GitCliWorktreeChangeProvider,
            cli_worktree_git_provider::{GitCliWorktreeGitProvider, GitCliWorktreeStatusProvider},
        },
    },
};

pub fn register(registry: &mut Registry) {
    registry.register(
        OperationId::WorktreeListChanges,
        query_handler(git_fault, |input: WorktreeListChangesInput| {
            let changes = worktree_changes_service::list_worktree_changes(
                &GitCliWorktreeChangeProvider,
                input.working_directory,
            )?;
            Ok::<Vec<WorktreeChangeDto>, GitError>(
                changes.iter().map(worktree_change_dto).collect(),
            )
        }),
    );
    registry.register(
        OperationId::WorktreeGetChanges,
        query_handler(git_fault, |input: WorktreeGetChangesInput| {
            let changes = worktree_changes_service::get_worktree_changes(
                &GitCliWorktreeStatusProvider,
                input.working_directory,
            )?;
            Ok::<GitWorktreeChangesDto, GitError>(git_worktree_changes_dto(&changes))
        }),
    );
    registry.register(
        OperationId::WorktreeGetFileDiff,
        query_handler(git_fault, |input: WorktreeGetFileDiffInput| {
            let diff = worktree_changes_service::get_worktree_file_diff(
                &GitCliWorktreeStatusProvider,
                input.working_directory,
                input.path,
            )?;
            Ok::<GitWorktreeFileDiffDto, GitError>(git_worktree_file_diff_dto(&diff))
        }),
    );
    registry.register(
        OperationId::WorktreeListFiles,
        query_handler(worktree_file_fault, |input: WorktreeListFilesInput| {
            let entries = worktree_file_service::list_worktree_files(
                &FsWorktreeFileProvider,
                input.working_directory,
                input.scope.map(worktree_file_list_scope_domain),
            )?;
            Ok::<Vec<WorktreeFileEntryDto>, WorktreeFileError>(
                entries.iter().map(worktree_file_entry_dto).collect(),
            )
        }),
    );
    registry.register(
        OperationId::WorktreeReadTextFile,
        query_handler(worktree_file_fault, |input: WorktreeReadTextFileInput| {
            let file = worktree_file_service::read_worktree_text_file(
                &FsWorktreeFileProvider,
                input.working_directory,
                input.path,
            )?;
            Ok::<WorktreeTextFileDto, WorktreeFileError>(worktree_text_file_dto(&file))
        }),
    );
    registry.register(
        OperationId::WorktreeListHistory,
        query_handler(git_fault, |input: WorktreeListHistoryInput| {
            let history = worktree_git_service::list_worktree_git_history(
                &GitCliWorktreeGitProvider,
                input.working_directory,
                input.max_count.map(saturating_usize),
                input.offset.map(saturating_usize),
                input.cursor,
            )?;
            Ok::<GitCommitHistoryDto, GitError>(git_commit_history_dto(&history))
        }),
    );
    registry.register(
        OperationId::WorktreeGetGraph,
        query_handler(git_fault, |input: WorktreeGetGraphInput| {
            let graph = worktree_git_service::get_worktree_git_graph(
                &GitCliWorktreeGitProvider,
                input.working_directory,
                input.max_count.map(saturating_usize),
                input.offset.map(saturating_usize),
                input.cursor,
            )?;
            Ok::<GitCommitGraphDto, GitError>(git_commit_graph_dto(&graph))
        }),
    );
    registry.register(
        OperationId::WorktreeGetCommitDetail,
        query_handler(git_fault, |input: WorktreeGetCommitDetailInput| {
            let detail = worktree_git_service::get_worktree_commit_detail(
                &GitCliWorktreeGitProvider,
                input.working_directory,
                input.commit_hash,
            )?;
            Ok::<GitCommitDetailDto, GitError>(git_commit_detail_dto(&detail))
        }),
    );
    registry.register(
        OperationId::WorktreeGetCommitFileDiff,
        query_handler(git_fault, |input: WorktreeGetCommitFileDiffInput| {
            let diff = worktree_git_service::get_worktree_commit_file_diff(
                &GitCliWorktreeGitProvider,
                input.working_directory,
                input.commit_hash,
                input.path,
            )?;
            Ok::<GitFileDiffDto, GitError>(git_file_diff_dto(&diff))
        }),
    );
}
