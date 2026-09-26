//! agent 실행 설정 저장소 port. AW `domain/agent_run_settings_repository.rs`에서 이동.

use crate::domain::{agent_run_settings::AgentRunSettings, errors::AgentRunSettingsError};

pub trait AgentRunSettingsRepository: Send + Sync {
    fn load_settings(&self) -> Result<Vec<AgentRunSettings>, AgentRunSettingsError>;
    fn save_settings(&self, settings: &[AgentRunSettings]) -> Result<(), AgentRunSettingsError>;
    fn recover_from_backup(&self) -> Result<(), AgentRunSettingsError>;
}
