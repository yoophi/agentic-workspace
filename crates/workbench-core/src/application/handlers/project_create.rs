//! `project.create`: intent-first 경로(research R6). 절차는 `intent_first::run_command`가 맡고,
//! 이 handler는 입력 정규화와 `MutationSpec`(예약 = 서버 생성 `project-{nanos}`, aggregate = projects)만 만든다.

use std::sync::Arc;

use async_trait::async_trait;
use workbench_protocol::{
    operations::project::ProjectCreateInput, CallReply, OperationId, WorkbenchFault,
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

/// 037 골든 문구. `intent_first::key_required_message(ProjectCreate)`와 같다.
pub const MESSAGE_KEY_REQUIRED: &str = "idempotencyKey is required for project.create.";

pub struct ProjectCreateHandler {
    runner: Arc<IntentFirst>,
}

impl ProjectCreateHandler {
    pub fn new(runner: Arc<IntentFirst>) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl OperationHandler for ProjectCreateHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let input: ProjectCreateInput = decode_input(&ctx.request_id, &input)?;
        let draft = project_service::normalize_draft(ProjectDraft {
            name: input.name,
            working_directory: input.working_directory,
            description: input.description,
        })
        .map_err(|error| project_fault(&ctx.request_id, error))?;

        let normalized = serde_json::json!({
            "name": draft.name,
            "workingDirectory": draft.working_directory,
            "description": draft.description,
        });
        let repository = Arc::clone(self.runner.coordinator().projects_repository());
        let recover_repository = Arc::clone(&repository);
        let generate_request_id = ctx.request_id.clone();

        let spec = MutationSpec {
            operation: OperationId::ProjectCreate,
            aggregate: PROJECTS_AGGREGATE.to_owned(),
            normalized_input: normalized,
            // 예약 id는 `project-{unix_nanos}`라 동시 호출이 같은 나노초에 떨어지면 충돌한다. runner가 id만
            // 다시 뽑아 재시도한다(기존 id 형식은 저장 파일 호환 때문에 유지).
            reservation: Reservation::ServerGenerated(Box::new(move || {
                project_service::new_project_id()
                    .map_err(|error| project_fault(&generate_request_id, error))
            })),
            tracks_revision: true,
            apply: Box::new(move |apply_ctx| {
                let id = apply_ctx
                    .reserved_id
                    .expect("server-generated reservation")
                    .to_owned();
                let project = project_service::create_project_with_id(
                    repository.as_ref(),
                    id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::intent_first::key_required_message;

    #[test]
    fn key_required_message_matches_037_golden() {
        assert_eq!(
            key_required_message(OperationId::ProjectCreate),
            MESSAGE_KEY_REQUIRED
        );
    }
}
