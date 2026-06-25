use serde::{Deserialize, Serialize};

use crate::domain::run::{ContextSizePreset, PermissionMode};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentRunSessionMode {
    #[default]
    New,
    Reuse,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunSettings {
    pub working_directory: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub permission_mode: PermissionMode,
    #[serde(default = "default_model_id")]
    pub model_id: String,
    #[serde(default)]
    pub context_size: ContextSizePreset,
    #[serde(default)]
    pub session_mode: AgentRunSessionMode,
    #[serde(default)]
    pub ralph_loop: AgentRunSettingsRalphLoop,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunSettingsRalphLoop {
    pub enabled: bool,
    pub max_iterations: usize,
    pub delay_ms: u64,
    #[serde(default)]
    pub stop_on_permission: bool,
    pub stop_on_error: bool,
    pub prompt_template: String,
}

impl Default for AgentRunSettingsRalphLoop {
    fn default() -> Self {
        Self {
            enabled: false,
            max_iterations: 5,
            delay_ms: 0,
            stop_on_permission: false,
            stop_on_error: true,
            prompt_template: String::new(),
        }
    }
}

fn default_model_id() -> String {
    "providerDefault".to_string()
}
