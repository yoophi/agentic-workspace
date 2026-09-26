//! `project.update`: 수정(upsert 아님, 대상 없으면 notFound). 예약 없음 → 재시작 판정 unknown.

use std::sync::Arc;

use async_trait::async_trait;
use workbench_protocol::{
    operations::project::ProjectUpdateInput, CallReply, OperationId, WorkbenchFault,
};

use crate::{
    application::{
        handlers::{project_fault, to_dto},
        intent_first::{Applied, IntentFirst, MutationSpec, Reservation},
        project_service,
        registry::{decode_input, CallContext, OperationHandler},
    },
    domain::{project::ProjectDraft, project_error::ProjectError},
    infrastructure::storage_coordinator::PROJECTS_AGGREGATE,
};

pub struct ProjectUpdateHandler {
    runner: Arc<IntentFirst>,
}

impl ProjectUpdateHandler {
    pub fn new(runner: Arc<IntentFirst>) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl OperationHandler for ProjectUpdateHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let input: ProjectUpdateInput = decode_input(&ctx.request_id, &input)?;
        let id = input.id;
        let draft = project_service::normalize_draft(ProjectDraft {
            name: input.name,
            working_directory: input.working_directory,
            description: input.description,
        })
        .map_err(|error| project_fault(&ctx.request_id, error))?;
        let normalized = serde_json::json!({
            "id": id,
            "name": draft.name,
            "workingDirectory": draft.working_directory,
            "description": draft.description,
        });
        let repository = Arc::clone(self.runner.coordinator().projects_repository());
        let recover_repository = Arc::clone(&repository);

        let spec = MutationSpec {
            operation: OperationId::ProjectUpdate,
            aggregate: PROJECTS_AGGREGATE.to_owned(),
            normalized_input: normalized,
            reservation: Reservation::None,
            tracks_revision: true,
            apply: Box::new(move |_| {
                let project = project_service::update_project(
                    repository.as_ref(),
                    id.clone(),
                    draft.clone(),
                )?;
                Ok(Applied::Ok(to_dto(&project)))
            }),
            is_store_corrupt: |error| matches!(error, ProjectError::StoreCorrupt(_)),
            recover: Box::new(move || recover_repository.recover_from_backup()),
            fault: Box::new(project_fault),
        };
        self.runner.run(ctx, spec).await
    }
}
