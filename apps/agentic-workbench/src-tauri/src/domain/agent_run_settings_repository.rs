use crate::domain::agent_run_settings::AgentRunSettings;

pub trait AgentRunSettingsRepository {
    fn load_settings(&self) -> Result<Vec<AgentRunSettings>, String>;
    fn save_settings(&self, settings: &[AgentRunSettings]) -> Result<(), String>;
}
