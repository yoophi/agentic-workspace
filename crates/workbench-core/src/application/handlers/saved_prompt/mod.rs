//! `savedPrompt.*` handler(038 US1). 조회는 aggregate lock 안 읽기, 변경은 intent-first runner.

use std::sync::Arc;

use async_trait::async_trait;
use workbench_protocol::{
    operations::{
        common::EmptyOutput,
        saved_prompt::{
            SavedPromptCreateInput, SavedPromptDeleteInput, SavedPromptListInput,
            SavedPromptUpdateInput,
        },
    },
    CallReply, OperationId, WorkbenchFault,
};

use crate::{
    application::{
        dto::saved_prompt_dto,
        handlers::{
            blocking, complete, json_create_reconciler, json_delete_reconciler, saved_prompt_fault,
            saved_prompts_as_dto_json,
        },
        intent_first::{Applied, IntentFirst, MutationSpec, Reservation},
        reconcilers::ReconcilerRegistry,
        registry::{decode_input, CallContext, OperationHandler, Registry},
        saved_prompt_service,
    },
    domain::{errors::SavedPromptError, saved_prompt::SavedPromptDraft},
    infrastructure::storage_coordinator::{StorageCoordinator, SAVED_PROMPTS_AGGREGATE},
};

pub fn register(
    registry: &mut Registry,
    reconcilers: &mut ReconcilerRegistry,
    coordinator: &Arc<StorageCoordinator>,
    runner: &Arc<IntentFirst>,
) {
    registry.register(
        OperationId::SavedPromptList,
        Arc::new(ListHandler {
            coordinator: Arc::clone(coordinator),
        }),
    );
    registry.register(
        OperationId::SavedPromptCreate,
        Arc::new(CreateHandler {
            runner: Arc::clone(runner),
        }),
    );
    registry.register(
        OperationId::SavedPromptUpdate,
        Arc::new(UpdateHandler {
            runner: Arc::clone(runner),
        }),
    );
    registry.register(
        OperationId::SavedPromptDelete,
        Arc::new(DeleteHandler {
            runner: Arc::clone(runner),
        }),
    );
    reconcilers.register(
        OperationId::SavedPromptCreate,
        json_create_reconciler(saved_prompts_as_dto_json(Arc::clone(coordinator)), "id"),
    );
    reconcilers.register(
        OperationId::SavedPromptDelete,
        json_delete_reconciler(saved_prompts_as_dto_json(Arc::clone(coordinator)), "id"),
    );
}

fn is_corrupt(error: &SavedPromptError) -> bool {
    matches!(error, SavedPromptError::StoreCorrupt(_))
}

struct ListHandler {
    coordinator: Arc<StorageCoordinator>,
}

#[async_trait]
impl OperationHandler for ListHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        decode_input::<SavedPromptListInput>(&ctx.request_id, &input)?;
        let coordinator = Arc::clone(&self.coordinator);
        let prompts = blocking(&ctx.request_id, saved_prompt_fault, move || {
            coordinator.with_saved_prompts(saved_prompt_service::list_saved_prompts)
        })
        .await?;
        Ok(complete(
            prompts.iter().map(saved_prompt_dto).collect::<Vec<_>>(),
        ))
    }
}

struct CreateHandler {
    runner: Arc<IntentFirst>,
}

#[async_trait]
impl OperationHandler for CreateHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let input: SavedPromptCreateInput = decode_input(&ctx.request_id, &input)?;
        let draft = saved_prompt_service::normalize_draft(SavedPromptDraft {
            label: input.label,
            prompt: input.prompt,
        })
        .map_err(|error| saved_prompt_fault(&ctx.request_id, error))?;
        let normalized = serde_json::json!({ "label": draft.label, "prompt": draft.prompt });
        let repository = Arc::clone(self.runner.coordinator().saved_prompts_repository());
        let recover_repository = Arc::clone(&repository);
        let generate_request_id = ctx.request_id.clone();

        let spec = MutationSpec {
            operation: OperationId::SavedPromptCreate,
            aggregate: SAVED_PROMPTS_AGGREGATE.to_owned(),
            normalized_input: normalized,
            reservation: Reservation::ServerGenerated(Box::new(move || {
                saved_prompt_service::new_saved_prompt_id()
                    .map_err(|error| saved_prompt_fault(&generate_request_id, error))
            })),
            tracks_revision: true,
            apply: Box::new(move |apply_ctx| {
                let id = apply_ctx
                    .reserved_id
                    .expect("server-generated reservation")
                    .to_owned();
                let prompt = saved_prompt_service::create_saved_prompt_with_id(
                    repository.as_ref(),
                    id,
                    draft.clone(),
                )?;
                Ok(Applied::Ok(saved_prompt_dto(&prompt)))
            }),
            is_store_corrupt: is_corrupt,
            recover: Box::new(move || recover_repository.recover_from_backup()),
            fault: Box::new(saved_prompt_fault),
        };
        self.runner.run(ctx, spec).await
    }
}

struct UpdateHandler {
    runner: Arc<IntentFirst>,
}

#[async_trait]
impl OperationHandler for UpdateHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let input: SavedPromptUpdateInput = decode_input(&ctx.request_id, &input)?;
        let id = input.id;
        let draft = saved_prompt_service::normalize_draft(SavedPromptDraft {
            label: input.label,
            prompt: input.prompt,
        })
        .map_err(|error| saved_prompt_fault(&ctx.request_id, error))?;
        let normalized =
            serde_json::json!({ "id": id, "label": draft.label, "prompt": draft.prompt });
        let repository = Arc::clone(self.runner.coordinator().saved_prompts_repository());
        let recover_repository = Arc::clone(&repository);

        let spec = MutationSpec {
            operation: OperationId::SavedPromptUpdate,
            aggregate: SAVED_PROMPTS_AGGREGATE.to_owned(),
            normalized_input: normalized,
            reservation: Reservation::None,
            tracks_revision: true,
            apply: Box::new(move |_| {
                let prompt = saved_prompt_service::update_saved_prompt(
                    repository.as_ref(),
                    id.clone(),
                    draft.clone(),
                )?;
                Ok(Applied::Ok(saved_prompt_dto(&prompt)))
            }),
            is_store_corrupt: is_corrupt,
            recover: Box::new(move || recover_repository.recover_from_backup()),
            fault: Box::new(saved_prompt_fault),
        };
        self.runner.run(ctx, spec).await
    }
}

struct DeleteHandler {
    runner: Arc<IntentFirst>,
}

#[async_trait]
impl OperationHandler for DeleteHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let input: SavedPromptDeleteInput = decode_input(&ctx.request_id, &input)?;
        let id = input.id;
        let repository = Arc::clone(self.runner.coordinator().saved_prompts_repository());
        let recover_repository = Arc::clone(&repository);

        let spec = MutationSpec {
            operation: OperationId::SavedPromptDelete,
            aggregate: SAVED_PROMPTS_AGGREGATE.to_owned(),
            normalized_input: serde_json::json!({ "id": id }),
            reservation: Reservation::CallerProvided(id.clone()),
            tracks_revision: true,
            apply: Box::new(move |_| {
                saved_prompt_service::delete_saved_prompt(repository.as_ref(), id.clone())?;
                Ok(Applied::Ok(EmptyOutput))
            }),
            is_store_corrupt: is_corrupt,
            recover: Box::new(move || recover_repository.recover_from_backup()),
            fault: Box::new(saved_prompt_fault),
        };
        self.runner.run(ctx, spec).await
    }
}
