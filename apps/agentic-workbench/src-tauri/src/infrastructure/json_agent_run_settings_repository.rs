use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use crate::domain::{
    agent_run_settings::AgentRunSettings, agent_run_settings_repository::AgentRunSettingsRepository,
};
use crate::infrastructure::json_store::{load_json_vec, save_json_vec};

pub struct JsonAgentRunSettingsRepository {
    store_path: PathBuf,
}

impl JsonAgentRunSettingsRepository {
    pub fn from_app(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;

        fs::create_dir_all(&dir)
            .map_err(|error| format!("Failed to create app data directory: {error}"))?;

        Ok(Self {
            store_path: dir.join("agent-run-settings.json"),
        })
    }
}

impl AgentRunSettingsRepository for JsonAgentRunSettingsRepository {
    fn load_settings(&self) -> Result<Vec<AgentRunSettings>, String> {
        load_json_vec(&self.store_path, "agent run settings")
    }

    fn save_settings(&self, settings: &[AgentRunSettings]) -> Result<(), String> {
        save_json_vec(&self.store_path, "agent run settings", settings)
    }
}
