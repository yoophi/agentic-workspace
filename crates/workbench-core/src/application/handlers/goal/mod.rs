//! `goal.*` handler(038 US1). `goal.create`는 같은 worktree의 기존 목표를 교체하는 upsert라 예약이 없고 재시작
//! 판정은 unknown이다(research R6, Codex 리뷰). `goal.clear`는 종료 상태 규칙(대상 없음 = applied).

use std::sync::Arc;

use async_trait::async_trait;
use workbench_protocol::{
    operations::{
        common::EmptyOutput,
        goal::{
            GoalClearInput, GoalCreateInput, GoalGetInput, GoalRecordProgressInput, GoalUpdateInput,
        },
    },
    CallReply, OperationId, WorkbenchFault,
};

use crate::{
    application::{
        dto::{goal_dto, goal_status_domain},
        goal_service,
        handlers::{blocking, complete, goal_fault, goals_as_dto_json, json_delete_reconciler},
        intent_first::{Applied, IntentFirst, MutationSpec, Reservation},
        reconcilers::ReconcilerRegistry,
        registry::{decode_input, CallContext, OperationHandler, Registry},
    },
    domain::{
        errors::GoalError,
        goal::{GoalDraft, GoalProgressUpdate, GoalUpdate},
    },
    infrastructure::storage_coordinator::{StorageCoordinator, GOALS_AGGREGATE},
};

pub fn register(
    registry: &mut Registry,
    reconcilers: &mut ReconcilerRegistry,
    coordinator: &Arc<StorageCoordinator>,
    runner: &Arc<IntentFirst>,
) {
    registry.register(
        OperationId::GoalGet,
        Arc::new(GetHandler {
            coordinator: Arc::clone(coordinator),
        }),
    );
    for (id, handler) in [
        (OperationId::GoalCreate, Mutation::Create),
        (OperationId::GoalUpdate, Mutation::Update),
        (OperationId::GoalClear, Mutation::Clear),
        (OperationId::GoalRecordProgress, Mutation::RecordProgress),
    ] {
        registry.register(
            id,
            Arc::new(MutationHandler {
                runner: Arc::clone(runner),
                kind: handler,
            }),
        );
    }
    reconcilers.register(
        OperationId::GoalClear,
        json_delete_reconciler(
            goals_as_dto_json(Arc::clone(coordinator)),
            "workingDirectory",
        ),
    );
}

fn is_corrupt(error: &GoalError) -> bool {
    matches!(error, GoalError::StoreCorrupt(_))
}

struct GetHandler {
    coordinator: Arc<StorageCoordinator>,
}

#[async_trait]
impl OperationHandler for GetHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let input: GoalGetInput = decode_input(&ctx.request_id, &input)?;
        let coordinator = Arc::clone(&self.coordinator);
        let goal = blocking(&ctx.request_id, goal_fault, move || {
            coordinator
                .with_goals(|repo| goal_service::get_goal(repo, input.working_directory.clone()))
        })
        .await?;
        Ok(complete(goal.as_ref().map(goal_dto)))
    }
}

#[derive(Clone, Copy)]
enum Mutation {
    Create,
    Update,
    Clear,
    RecordProgress,
}

struct MutationHandler {
    runner: Arc<IntentFirst>,
    kind: Mutation,
}

#[async_trait]
impl OperationHandler for MutationHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let request_id = &ctx.request_id;
        let repository = Arc::clone(self.runner.coordinator().goals_repository());
        let recover_repository = Arc::clone(&repository);
        let recover = Box::new(move || recover_repository.recover_from_backup());

        match self.kind {
            Mutation::Create => {
                let input: GoalCreateInput = decode_input(request_id, &input)?;
                let draft = goal_service::normalize_draft(GoalDraft {
                    working_directory: input.working_directory,
                    objective: input.objective,
                    token_budget: input.token_budget.map(|budget| budget as usize),
                })
                .map_err(|error| goal_fault(request_id, error))?;
                let spec = MutationSpec {
                    operation: OperationId::GoalCreate,
                    aggregate: GOALS_AGGREGATE.to_owned(),
                    normalized_input: serde_json::json!({
                        "workingDirectory": draft.working_directory,
                        "objective": draft.objective,
                        "tokenBudget": draft.token_budget,
                    }),
                    // upsert: 기존 목표가 있어도 증거가 아니므로 예약하지 않는다.
                    reservation: Reservation::None,
                    tracks_revision: true,
                    apply: Box::new(move |_| {
                        let goal = goal_service::create_goal(repository.as_ref(), draft.clone())?;
                        Ok(Applied::Ok(goal_dto(&goal)))
                    }),
                    is_store_corrupt: is_corrupt,
                    recover,
                    fault: Box::new(goal_fault),
                };
                self.runner.run(ctx, spec).await
            }
            Mutation::Update => {
                let input: GoalUpdateInput = decode_input(request_id, &input)?;
                let working_directory =
                    goal_service::normalize_required(input.working_directory, "Working directory")
                        .map_err(|error| goal_fault(request_id, error))?;
                let update = goal_service::normalize_update(GoalUpdate {
                    objective: input.objective,
                    status: input.status.map(goal_status_domain),
                    token_budget: input
                        .token_budget
                        .map(|budget| budget.map(|value| value as usize)),
                })
                .map_err(|error| goal_fault(request_id, error))?;
                let spec = MutationSpec {
                    operation: OperationId::GoalUpdate,
                    aggregate: GOALS_AGGREGATE.to_owned(),
                    normalized_input: serde_json::json!({
                        "workingDirectory": working_directory,
                        "objective": update.objective,
                        "status": update.status.as_ref().map(crate::application::dto::goal_status_dto),
                        "tokenBudget": update.token_budget,
                    }),
                    reservation: Reservation::None,
                    tracks_revision: true,
                    apply: Box::new(move |_| {
                        let goal = goal_service::update_goal(
                            repository.as_ref(),
                            working_directory.clone(),
                            update.clone(),
                        )?;
                        Ok(Applied::Ok(goal_dto(&goal)))
                    }),
                    is_store_corrupt: is_corrupt,
                    recover,
                    fault: Box::new(goal_fault),
                };
                self.runner.run(ctx, spec).await
            }
            Mutation::Clear => {
                let input: GoalClearInput = decode_input(request_id, &input)?;
                let working_directory =
                    goal_service::normalize_required(input.working_directory, "Working directory")
                        .map_err(|error| goal_fault(request_id, error))?;
                let spec = MutationSpec {
                    operation: OperationId::GoalClear,
                    aggregate: GOALS_AGGREGATE.to_owned(),
                    normalized_input: serde_json::json!({ "workingDirectory": working_directory }),
                    reservation: Reservation::CallerProvided(working_directory.clone()),
                    tracks_revision: true,
                    apply: Box::new(move |_| {
                        goal_service::clear_goal(repository.as_ref(), working_directory.clone())?;
                        Ok(Applied::Ok(EmptyOutput))
                    }),
                    is_store_corrupt: is_corrupt,
                    recover,
                    fault: Box::new(goal_fault),
                };
                self.runner.run(ctx, spec).await
            }
            Mutation::RecordProgress => {
                let input: GoalRecordProgressInput = decode_input(request_id, &input)?;
                let working_directory =
                    goal_service::normalize_required(input.working_directory, "Working directory")
                        .map_err(|error| goal_fault(request_id, error))?;
                let progress = GoalProgressUpdate {
                    tokens_used: input.tokens_used as usize,
                    time_used_seconds: input.time_used_seconds,
                };
                let spec = MutationSpec {
                    operation: OperationId::GoalRecordProgress,
                    aggregate: GOALS_AGGREGATE.to_owned(),
                    normalized_input: serde_json::json!({
                        "workingDirectory": working_directory,
                        "tokensUsed": input.tokens_used,
                        "timeUsedSeconds": input.time_used_seconds,
                    }),
                    reservation: Reservation::None,
                    tracks_revision: true,
                    apply: Box::new(move |_| {
                        let goal = goal_service::record_goal_progress(
                            repository.as_ref(),
                            working_directory.clone(),
                            progress.clone(),
                        )?;
                        Ok(Applied::Ok(goal_dto(&goal)))
                    }),
                    is_store_corrupt: is_corrupt,
                    recover,
                    fault: Box::new(goal_fault),
                };
                self.runner.run(ctx, spec).await
            }
        }
    }
}
