//! `agent-run-settings.json` 어댑터. AW `infrastructure/json_agent_run_settings_repository.rs`에서 이동.

use crate::{
    domain::{agent_run_settings::AgentRunSettings, errors::AgentRunSettingsError},
    infrastructure::{
        data_paths::DataPaths, json_collection_store::JsonCollectionStore, json_store::StoreError,
    },
    ports::agent_run_settings_repository::AgentRunSettingsRepository,
};

const LABEL: &str = "agent run settings";

pub struct JsonAgentRunSettingsRepository {
    store: JsonCollectionStore<AgentRunSettings>,
}

impl JsonAgentRunSettingsRepository {
    pub fn new(paths: &DataPaths) -> Self {
        Self {
            store: JsonCollectionStore::new(paths.agent_run_settings_file(), LABEL),
        }
    }
}

fn map_store_error(error: StoreError) -> AgentRunSettingsError {
    match error {
        StoreError::PrimaryCorrupt { .. } => AgentRunSettingsError::StoreCorrupt(error.to_string()),
        other => AgentRunSettingsError::Storage(other.to_string()),
    }
}

impl AgentRunSettingsRepository for JsonAgentRunSettingsRepository {
    fn load_settings(&self) -> Result<Vec<AgentRunSettings>, AgentRunSettingsError> {
        self.store.load().map_err(map_store_error)
    }

    fn save_settings(&self, settings: &[AgentRunSettings]) -> Result<(), AgentRunSettingsError> {
        self.store.save(settings).map_err(map_store_error)
    }

    fn recover_from_backup(&self) -> Result<(), AgentRunSettingsError> {
        self.store
            .recover_from_backup()
            .map(|_| ())
            .map_err(map_store_error)
    }
}
