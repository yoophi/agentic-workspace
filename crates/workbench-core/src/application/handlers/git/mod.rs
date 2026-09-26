//! `git.*` handler(038 US2). 조회 3개는 lock 없이 blocking pool에서, worktree 생성·삭제는 intent-first runner로
//! 실행한다. 생성·삭제는 같은 저장소(`git-worktrees:<canonical root>`) 안에서 직렬화되고, 재시작 판정은 종료 상태
//! 규칙을 따른다(ADR `crates/workbench-core/docs/adr/0001`).

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use workbench_protocol::{
    operations::{
        common::EmptyOutput,
        git::{
            GitBranchDto, GitCreateWorktreeInput, GitDeleteWorktreeInput, GitListBranchesInput,
            GitListRemotesInput, GitListWorktreesInput, GitRemoteDto, GitWorktreeDto,
        },
    },
    CallReply, OperationId, WorkbenchFault,
};

use crate::{
    application::{
        git_dto::{git_branch_dto, git_remote_dto, git_worktree_dto},
        git_service, git_worktree_service,
        handlers::{git_fault, query_handler},
        intent_first::{Applied, IntentFirst, MutationSpec, Reservation},
        reconcilers::{git_worktree::GitWorktreeReconciler, ReconcilerRegistry},
        registry::{decode_input, CallContext, OperationHandler, Registry},
    },
    domain::{errors::GitError, git_worktree::GitWorktreeCreateDraft},
    infrastructure::{
        git::{
            cli_branch_provider::GitCliBranchProvider, cli_remote_provider::GitCliRemoteProvider,
            cli_worktree_provider::GitCliWorktreeProvider,
        },
        storage_coordinator::git_worktrees_aggregate,
    },
    ports::git_providers::GitWorktreeProvider,
};

pub fn register(
    registry: &mut Registry,
    reconcilers: &mut ReconcilerRegistry,
    runner: &Arc<IntentFirst>,
) {
    registry.register(
        OperationId::GitListRemotes,
        query_handler(git_fault, |input: GitListRemotesInput| {
            let remotes =
                git_service::list_git_remotes(&GitCliRemoteProvider, input.working_directory)?;
            Ok::<Vec<GitRemoteDto>, GitError>(remotes.iter().map(git_remote_dto).collect())
        }),
    );
    registry.register(
        OperationId::GitListBranches,
        query_handler(git_fault, |input: GitListBranchesInput| {
            let branches =
                git_service::list_git_branches(&GitCliBranchProvider, input.working_directory)?;
            Ok::<Vec<GitBranchDto>, GitError>(branches.iter().map(git_branch_dto).collect())
        }),
    );
    registry.register(
        OperationId::GitListWorktrees,
        query_handler(git_fault, |input: GitListWorktreesInput| {
            let worktrees = git_worktree_service::list_git_worktrees(
                &GitCliWorktreeProvider,
                input.working_directory,
                // 생략하면 오늘의 데스크톱 기본값(`include_status.unwrap_or(true)`)과 같다.
                input.include_status.unwrap_or(true),
            )?;
            Ok::<Vec<GitWorktreeDto>, GitError>(worktrees.iter().map(git_worktree_dto).collect())
        }),
    );
    registry.register(
        OperationId::GitCreateWorktree,
        Arc::new(CreateWorktreeHandler {
            runner: Arc::clone(runner),
        }),
    );
    registry.register(
        OperationId::GitDeleteWorktree,
        Arc::new(DeleteWorktreeHandler {
            runner: Arc::clone(runner),
        }),
    );
    reconcilers.register(
        OperationId::GitCreateWorktree,
        Arc::new(GitWorktreeReconciler::create()),
    );
    reconcilers.register(
        OperationId::GitDeleteWorktree,
        Arc::new(GitWorktreeReconciler::delete()),
    );
}

/// 예약·증거로 남길 worktree 경로. git은 상대 경로를 `-C <workingDirectory>` 기준으로 해석하므로 그 기준으로 절대화한다.
fn reserved_worktree_path(working_directory: &str, path: &str) -> String {
    let candidate = Path::new(path);
    let absolute: PathBuf = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        Path::new(working_directory).join(candidate)
    };
    absolute.to_string_lossy().into_owned()
}

struct CreateWorktreeHandler {
    runner: Arc<IntentFirst>,
}

#[async_trait]
impl OperationHandler for CreateWorktreeHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let input: GitCreateWorktreeInput = decode_input(&ctx.request_id, &input)?;
        let fault = |error| git_fault(&ctx.request_id, error);
        let request = git_worktree_service::normalize_create_request(
            input.working_directory,
            GitWorktreeCreateDraft {
                path: input.path,
                branch: input.branch,
                reference: input.reference,
            },
        )
        .map_err(fault)?;
        // 지문은 호출자 의도(기본값 채우기 전)로 계산한다 — 기본 branch 이름은 시각에서 만들어지므로.
        let normalized = serde_json::to_value(&request).expect("request serializes");
        let draft = git_worktree_service::resolve_create_draft(&request).map_err(fault)?;
        let working_directory = request.working_directory.clone();

        let spec = MutationSpec {
            operation: OperationId::GitCreateWorktree,
            aggregate: git_worktrees_aggregate(Path::new(&working_directory)),
            normalized_input: normalized,
            reservation: Reservation::CallerProvided(reserved_worktree_path(
                &working_directory,
                &draft.path,
            )),
            tracks_revision: false,
            apply: Box::new(move |_| {
                GitCliWorktreeProvider.create_worktree(&working_directory, draft.clone())?;
                Ok(Applied::Ok(EmptyOutput))
            }),
            is_store_corrupt: |_: &GitError| false,
            recover: Box::new(|| Ok(())),
            fault: Box::new(git_fault),
        };
        self.runner.run(ctx, spec).await
    }
}

struct DeleteWorktreeHandler {
    runner: Arc<IntentFirst>,
}

#[async_trait]
impl OperationHandler for DeleteWorktreeHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let input: GitDeleteWorktreeInput = decode_input(&ctx.request_id, &input)?;
        let (working_directory, path) =
            git_worktree_service::normalize_delete_request(input.working_directory, input.path)
                .map_err(|error| git_fault(&ctx.request_id, error))?;
        let normalized = serde_json::json!({ "workingDirectory": working_directory, "path": path });
        let reserved = reserved_worktree_path(&working_directory, &path);

        let spec = MutationSpec {
            operation: OperationId::GitDeleteWorktree,
            aggregate: git_worktrees_aggregate(Path::new(&working_directory)),
            normalized_input: normalized,
            // 삭제도 대상 경로를 예약한다: pending 동안 같은 경로의 생성·삭제와 배타이고, 재시작 판정의 증거가 된다.
            reservation: Reservation::CallerProvided(reserved),
            tracks_revision: false,
            apply: Box::new(move |apply_ctx| {
                match GitCliWorktreeProvider.delete_worktree(&working_directory, &path) {
                    Ok(()) => Ok(Applied::Ok(EmptyOutput)),
                    // 삭제 전 검사에서 거절됨 — 부작용 없음(`notApplied`).
                    Err(
                        error @ (GitError::WorktreeNotFound
                        | GitError::WorktreeHasChanges
                        | GitError::StatusUnresolved),
                    ) => Ok(Applied::Rejected(git_fault(apply_ctx.request_id, error))),
                    Err(error) => Err(error),
                }
            }),
            is_store_corrupt: |_: &GitError| false,
            recover: Box::new(|| Ok(())),
            fault: Box::new(git_fault),
        };
        self.runner.run(ctx, spec).await
    }
}
