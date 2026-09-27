//! `agentRunSettings.*` handler(038 US1). `save`는 같은 worktree의 설정을 교체하는 upsert — 예약 없음, 재시작
//! 판정 unknown.

use std::sync::Arc;

use async_trait::async_trait;
use workbench_protocol::{
    operations::agent_run_settings::{AgentRunSettingsGetInput, AgentRunSettingsSaveInput},
    CallReply, OperationId, WorkbenchFault,
};

use crate::{
    application::{
        agent_run_settings_service,
        dto::{agent_run_settings_domain, agent_run_settings_dto},
        handlers::{agent_run_settings_fault, blocking, complete},
        intent_first::{Applied, IntentFirst, MutationSpec, Reservation},
        registry::{decode_input, CallContext, OperationHandler, Registry},
    },
    domain::errors::AgentRunSettingsError,
    infrastructure::storage_coordinator::{StorageCoordinator, AGENT_RUN_SETTINGS_AGGREGATE},
};

pub fn register(
    registry: &mut Registry,
    coordinator: &Arc<StorageCoordinator>,
    runner: &Arc<IntentFirst>,
) {
    registry.register(
        OperationId::AgentRunSettingsGet,
        Arc::new(GetHandler {
            coordinator: Arc::clone(coordinator),
        }),
    );
    registry.register(
        OperationId::AgentRunSettingsSave,
        Arc::new(SaveHandler {
            runner: Arc::clone(runner),
        }),
    );
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
        let input: AgentRunSettingsGetInput = decode_input(&ctx.request_id, &input)?;
        let coordinator = Arc::clone(&self.coordinator);
        let settings = blocking(&ctx.request_id, agent_run_settings_fault, move || {
            coordinator.with_agent_run_settings(|repo| {
                agent_run_settings_service::get_settings(repo, input.working_directory.clone())
            })
        })
        .await?;
        Ok(complete(settings.as_ref().map(agent_run_settings_dto)))
    }
}

struct SaveHandler {
    runner: Arc<IntentFirst>,
}

#[async_trait]
impl OperationHandler for SaveHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let input: AgentRunSettingsSaveInput = decode_input(&ctx.request_id, &input)?;
        let settings = agent_run_settings_service::normalize_settings(agent_run_settings_domain(
            input.settings,
        ))
        .map_err(|error| agent_run_settings_fault(&ctx.request_id, error))?;
        let normalized =
            serde_json::to_value(agent_run_settings_dto(&settings)).expect("settings serialize");
        let repository = Arc::clone(self.runner.coordinator().agent_run_settings_repository());
        let recover_repository = Arc::clone(&repository);

        let spec = MutationSpec {
            operation: OperationId::AgentRunSettingsSave,
            aggregate: AGENT_RUN_SETTINGS_AGGREGATE.to_owned(),
            normalized_input: normalized,
            reservation: Reservation::None,
            tracks_revision: true,
            apply: Box::new(move |_| {
                let saved = agent_run_settings_service::save_settings(
                    repository.as_ref(),
                    settings.clone(),
                )?;
                Ok(Applied::Ok(agent_run_settings_dto(&saved)))
            }),
            is_store_corrupt: |error| matches!(error, AgentRunSettingsError::StoreCorrupt(_)),
            recover: Box::new(move || recover_repository.recover_from_backup()),
            fault: Box::new(agent_run_settings_fault),
        };
        self.runner.run(ctx, spec).await
    }
}
