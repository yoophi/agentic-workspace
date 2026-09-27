#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AgentRunSettingsError {
    /// `"Working directory is required."` / `"Agent id is required."`
    #[error("{0} is required.")]
    Required(&'static str),
    /// specs/008 FR-010 불변식. 오류 메시지에 env value를 포함하지 않는다.
    #[error("At least one built-in agent profile must stay enabled.")]
    NoBuiltInProfile,
    #[error("No command is configured for agent {0}.")]
    NoCommandConfigured(String),
    #[error("{0}")]
    Storage(String),
    #[error("{0}")]
    StoreCorrupt(String),
}

impl AgentRunSettingsError {
    pub fn field_path(&self) -> Option<&'static str> {
        match self {
            AgentRunSettingsError::Required("Working directory") => {
                Some("/settings/workingDirectory")
            }
            AgentRunSettingsError::NoBuiltInProfile => Some("/settings/commandOverrides/profiles"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_matches_legacy_strings() {
        assert_eq!(
            AgentRunSettingsError::Required("Working directory").to_string(),
            "Working directory is required."
        );
        assert_eq!(
            AgentRunSettingsError::NoBuiltInProfile.to_string(),
            "At least one built-in agent profile must stay enabled."
        );
        assert_eq!(
            AgentRunSettingsError::NoCommandConfigured("codex".into()).to_string(),
            "No command is configured for agent codex."
        );
    }
}
