//! `project.list`: aggregate lock 안에서 읽기(손상 시 같은 lock에서 복구) → `ProjectDto[]`.

use std::sync::Arc;

use async_trait::async_trait;
use workbench_protocol::{operations::project::ProjectListInput, CallReply, WorkbenchFault};

use crate::{
    application::{
        handlers::{project_fault, to_dto},
        project_service,
        registry::{decode_input, CallContext, OperationHandler},
    },
    infrastructure::storage_coordinator::StorageCoordinator,
};

pub struct ProjectListHandler {
    coordinator: Arc<StorageCoordinator>,
}

impl ProjectListHandler {
    pub fn new(coordinator: Arc<StorageCoordinator>) -> Self {
        Self { coordinator }
    }
}

#[async_trait]
impl OperationHandler for ProjectListHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        decode_input::<ProjectListInput>(&ctx.request_id, &input)?;

        let coordinator = Arc::clone(&self.coordinator);
        let request_id = ctx.request_id.clone();
        let projects = tokio::task::spawn_blocking(move || {
            coordinator.with_projects(|repo| project_service::list_projects(repo))
        })
        .await
        .map_err(|error| WorkbenchFault::internal(request_id.clone(), error.to_string()))?
        .map_err(|error| project_fault(&request_id, error))?;

        let output = projects.iter().map(to_dto).collect::<Vec<_>>();
        Ok(CallReply::complete(
            serde_json::to_value(output).expect("dto serializes"),
            None,
        ))
    }
}
