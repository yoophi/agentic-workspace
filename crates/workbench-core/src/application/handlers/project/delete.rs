//! `project.delete`: 대상 id를 예약(진행 중 같은 id 변경 배제) → 재시작 판정은 종료 상태(대상 없음 = applied).

use std::sync::Arc;

use async_trait::async_trait;
use workbench_protocol::{
    operations::{common::EmptyOutput, project::ProjectDeleteInput},
    CallReply, OperationId, WorkbenchFault,
};

use crate::{
    application::{
        handlers::project_fault,
        intent_first::{Applied, IntentFirst, MutationSpec, Reservation},
        project_service,
        registry::{decode_input, CallContext, OperationHandler},
    },
    domain::project_error::ProjectError,
    infrastructure::storage_coordinator::PROJECTS_AGGREGATE,
};

pub struct ProjectDeleteHandler {
    runner: Arc<IntentFirst>,
}

impl ProjectDeleteHandler {
    pub fn new(runner: Arc<IntentFirst>) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl OperationHandler for ProjectDeleteHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let input: ProjectDeleteInput = decode_input(&ctx.request_id, &input)?;
        let id = input.id;
        let repository = Arc::clone(self.runner.coordinator().projects_repository());
        let recover_repository = Arc::clone(&repository);
        let target = id.clone();

        let spec = MutationSpec {
            operation: OperationId::ProjectDelete,
            aggregate: PROJECTS_AGGREGATE.to_owned(),
            normalized_input: serde_json::json!({ "id": id }),
            reservation: Reservation::CallerProvided(target),
            tracks_revision: true,
            apply: Box::new(move |_| {
                project_service::delete_project(repository.as_ref(), id.clone())?;
                Ok(Applied::Ok(EmptyOutput))
            }),
            is_store_corrupt: |error| matches!(error, ProjectError::StoreCorrupt(_)),
            recover: Box::new(move || recover_repository.recover_from_backup()),
            fault: Box::new(project_fault),
        };
        self.runner.run(ctx, spec).await
    }
}
