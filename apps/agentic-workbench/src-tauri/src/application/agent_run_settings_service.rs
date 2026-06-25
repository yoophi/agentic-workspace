use crate::domain::{
    agent_run_settings::{AgentRunSettings, AgentRunSettingsRalphLoop},
    agent_run_settings_repository::AgentRunSettingsRepository,
    run::{MAX_RALPH_DELAY_MS, MAX_RALPH_ITERATIONS},
};

pub fn get_settings(
    repository: &impl AgentRunSettingsRepository,
    working_directory: String,
) -> Result<Option<AgentRunSettings>, String> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    Ok(repository
        .load_settings()?
        .into_iter()
        .find(|settings| settings.working_directory == working_directory))
}

pub fn save_settings(
    repository: &impl AgentRunSettingsRepository,
    settings: AgentRunSettings,
) -> Result<AgentRunSettings, String> {
    let settings = normalize_settings(settings)?;
    let mut all_settings = repository.load_settings()?;

    all_settings.retain(|existing| existing.working_directory != settings.working_directory);
    all_settings.push(settings.clone());
    repository.save_settings(&all_settings)?;

    Ok(settings)
}

fn normalize_settings(mut settings: AgentRunSettings) -> Result<AgentRunSettings, String> {
    settings.working_directory =
        normalize_required(settings.working_directory, "Working directory")?;
    settings.agent_id = settings.agent_id.trim().to_string();
    settings.model_id = normalize_optional_with_default(settings.model_id, "providerDefault");
    settings.ralph_loop = normalize_ralph_loop(settings.ralph_loop);
    Ok(settings)
}

fn normalize_ralph_loop(mut ralph_loop: AgentRunSettingsRalphLoop) -> AgentRunSettingsRalphLoop {
    ralph_loop.max_iterations = ralph_loop.max_iterations.clamp(1, MAX_RALPH_ITERATIONS);
    ralph_loop.delay_ms = ralph_loop.delay_ms.min(MAX_RALPH_DELAY_MS);
    ralph_loop.prompt_template = ralph_loop.prompt_template.trim().to_string();
    ralph_loop
}

fn normalize_optional_with_default(value: String, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_required(value: String, label: &str) -> Result<String, String> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(format!("{label} is required."));
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::domain::{
        agent_run_settings::AgentRunSessionMode,
        run::{ContextSizePreset, PermissionMode},
    };

    #[derive(Default)]
    struct MemoryAgentRunSettingsRepository {
        settings: RefCell<Vec<AgentRunSettings>>,
    }

    impl AgentRunSettingsRepository for MemoryAgentRunSettingsRepository {
        fn load_settings(&self) -> Result<Vec<AgentRunSettings>, String> {
            Ok(self.settings.borrow().clone())
        }

        fn save_settings(&self, settings: &[AgentRunSettings]) -> Result<(), String> {
            self.settings.replace(settings.to_vec());
            Ok(())
        }
    }

    fn settings(agent_id: &str) -> AgentRunSettings {
        AgentRunSettings {
            working_directory: " /repo/worktree ".into(),
            agent_id: agent_id.into(),
            permission_mode: PermissionMode::Plan,
            model_id: " gpt-5 ".into(),
            context_size: ContextSizePreset::Large,
            session_mode: AgentRunSessionMode::Reuse,
            ralph_loop: AgentRunSettingsRalphLoop {
                enabled: true,
                max_iterations: 999,
                delay_ms: 999_999,
                stop_on_permission: true,
                stop_on_error: true,
                prompt_template: " continue ".into(),
            },
        }
    }

    #[test]
    fn save_settings_replaces_worktree_entry_and_sanitizes_values() {
        let repository = MemoryAgentRunSettingsRepository::default();

        save_settings(&repository, settings("codex")).expect("settings should save");
        let saved =
            save_settings(&repository, settings("claude-code")).expect("settings should replace");

        assert_eq!(saved.working_directory, "/repo/worktree");
        assert_eq!(saved.agent_id, "claude-code");
        assert_eq!(saved.model_id, "gpt-5");
        assert_eq!(saved.ralph_loop.max_iterations, MAX_RALPH_ITERATIONS);
        assert_eq!(saved.ralph_loop.delay_ms, MAX_RALPH_DELAY_MS);
        assert!(saved.ralph_loop.stop_on_permission);
        assert_eq!(saved.ralph_loop.prompt_template, "continue");
        assert_eq!(repository.load_settings().expect("load settings").len(), 1);
    }

    #[test]
    fn get_settings_returns_only_matching_worktree() {
        let repository = MemoryAgentRunSettingsRepository::default();
        save_settings(&repository, settings("codex")).expect("settings should save");

        assert!(
            get_settings(&repository, "/other".into())
                .expect("lookup should succeed")
                .is_none()
        );
        assert!(
            get_settings(&repository, "/repo/worktree".into())
                .expect("lookup should succeed")
                .is_some()
        );
    }
}
