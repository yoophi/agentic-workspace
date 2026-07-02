use std::collections::BTreeMap;

use crate::domain::{
    agent_run_settings::{
        AgentCommandOverrides, AgentCommandSource, AgentProfile, AgentRunSettings,
        AgentRunSettingsRalphLoop, BUILT_IN_AGENT_TYPES, CommandResolutionResult,
    },
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
    settings.command_overrides = normalize_command_overrides(settings.command_overrides);
    ensure_active_built_in_profile(&settings.command_overrides)?;
    Ok(settings)
}

/// 활성 기본 프로필 최소 1개 불변식(specs/008 FR-010). 오류 메시지에 env value를
/// 포함하지 않는다.
fn ensure_active_built_in_profile(overrides: &AgentCommandOverrides) -> Result<(), String> {
    let has_active_built_in = effective_profiles(overrides)
        .iter()
        .any(|profile| profile.built_in && profile.enabled);

    if has_active_built_in {
        Ok(())
    } else {
        Err("At least one built-in agent profile must stay enabled.".to_string())
    }
}

/// 저장 프로필 + seed(누락 기본 프로필 자동 채움)를 합친 "유효 프로필" 목록.
/// seed 시 legacy `agent_commands[agent_type]`을 command 초기값으로 사용한다.
/// 저장 데이터는 변경하지 않는다(specs/008 research R2).
pub fn effective_profiles(overrides: &AgentCommandOverrides) -> Vec<AgentProfile> {
    let normalized = normalize_command_overrides(overrides.clone());
    let mut profiles = normalized.profiles.clone();
    for agent_type in BUILT_IN_AGENT_TYPES {
        if profiles.iter().any(|profile| profile.id == *agent_type) {
            continue;
        }
        profiles.push(AgentProfile {
            id: (*agent_type).to_string(),
            name: built_in_profile_default_name(agent_type),
            agent_type: (*agent_type).to_string(),
            command: normalized.agent_commands.get(*agent_type).cloned(),
            env: BTreeMap::new(),
            enabled: true,
            built_in: true,
        });
    }
    profiles
}

fn built_in_profile_default_name(agent_type: &str) -> String {
    match agent_type {
        "codex" => "Codex".to_string(),
        "claude-code" => "Claude Code".to_string(),
        "opencode" => "OpenCode".to_string(),
        "pi-coding-agent" => "Pi Coding Agent".to_string(),
        other => other.to_string(),
    }
}

pub fn resolve_agent_command(
    agent_id: &str,
    overrides: &AgentCommandOverrides,
    default_command: Option<String>,
) -> Result<CommandResolutionResult, String> {
    let agent_id = normalize_required(agent_id.to_string(), "Agent id")?;
    let overrides = normalize_command_overrides(overrides.clone());

    if let Some(command) = overrides.agent_commands.get(&agent_id) {
        return Ok(CommandResolutionResult {
            agent_id,
            command: command.clone(),
            source: AgentCommandSource::AgentOverride,
        });
    }

    if let Some(command) = overrides.global_command {
        return Ok(CommandResolutionResult {
            agent_id,
            command,
            source: AgentCommandSource::GlobalOverride,
        });
    }

    let command = normalize_optional(default_command.unwrap_or_default())
        .ok_or_else(|| format!("No command is configured for agent {agent_id}."))?;
    Ok(CommandResolutionResult {
        agent_id,
        command,
        source: AgentCommandSource::DefaultCommand,
    })
}

fn normalize_ralph_loop(mut ralph_loop: AgentRunSettingsRalphLoop) -> AgentRunSettingsRalphLoop {
    ralph_loop.max_iterations = ralph_loop.max_iterations.clamp(1, MAX_RALPH_ITERATIONS);
    ralph_loop.delay_ms = ralph_loop.delay_ms.min(MAX_RALPH_DELAY_MS);
    ralph_loop.prompt_template = ralph_loop.prompt_template.trim().to_string();
    ralph_loop
}

fn normalize_command_overrides(mut overrides: AgentCommandOverrides) -> AgentCommandOverrides {
    overrides.global_command = overrides.global_command.and_then(normalize_optional);
    overrides.agent_commands = overrides
        .agent_commands
        .into_iter()
        .filter_map(|(agent_id, command)| {
            let agent_id = agent_id.trim().to_string();
            normalize_optional(command).and_then(|command| {
                if agent_id.is_empty() {
                    None
                } else {
                    Some((agent_id, command))
                }
            })
        })
        .collect();
    overrides.global_env = normalize_env(overrides.global_env);
    overrides.profiles = overrides
        .profiles
        .into_iter()
        .filter_map(|profile| {
            let profile = normalize_profile(profile);
            (!profile.id.is_empty()).then_some(profile)
        })
        .collect();
    overrides
}

/// env normalization(FR-004): key trim, 빈/공백 key 제거, 빈 value는 유지.
fn normalize_env(env: BTreeMap<String, String>) -> BTreeMap<String, String> {
    env.into_iter()
        .filter_map(|(key, value)| {
            let key = key.trim().to_string();
            (!key.is_empty()).then_some((key, value))
        })
        .collect()
}

fn normalize_profile(mut profile: AgentProfile) -> AgentProfile {
    profile.id = profile.id.trim().to_string();
    profile.agent_type = profile.agent_type.trim().to_string();
    let name = profile.name.trim().to_string();
    profile.name = if name.is_empty() {
        built_in_profile_default_name(&profile.agent_type)
    } else {
        name
    };
    profile.command = profile.command.and_then(normalize_optional);
    profile.env = normalize_env(profile.env);
    profile
}

fn normalize_optional_with_default(value: String, fallback: &str) -> String {
    normalize_optional(value).unwrap_or_else(|| fallback.to_string())
}

fn normalize_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
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
    use std::collections::BTreeMap;

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
            command_overrides: AgentCommandOverrides::default(),
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
        assert_eq!(saved.command_overrides, AgentCommandOverrides::default());
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

    #[test]
    fn save_settings_normalizes_command_overrides() {
        let repository = MemoryAgentRunSettingsRepository::default();
        let mut settings = settings("codex");
        settings.command_overrides = AgentCommandOverrides {
            global_env: BTreeMap::new(),
            profiles: Vec::new(),
            global_command: Some("  global-acp  ".into()),
            agent_commands: BTreeMap::from([
                (" codex ".into(), "  codex-acp  ".into()),
                ("claude-code".into(), "   ".into()),
                (" ".into(), "ignored".into()),
            ]),
        };

        let saved = save_settings(&repository, settings).expect("settings should save");

        assert_eq!(
            saved.command_overrides.global_command.as_deref(),
            Some("global-acp")
        );
        assert_eq!(
            saved.command_overrides.agent_commands,
            BTreeMap::from([("codex".into(), "codex-acp".into())])
        );
    }

    #[test]
    fn missing_command_overrides_deserializes_as_empty() {
        let value = serde_json::json!({
            "workingDirectory": "/repo/worktree",
            "agentId": "codex",
            "permissionMode": "plan",
            "modelId": "providerDefault",
            "contextSize": "default",
            "sessionMode": "new",
            "ralphLoop": {
                "enabled": false,
                "maxIterations": 5,
                "delayMs": 0,
                "stopOnPermission": false,
                "stopOnError": true,
                "promptTemplate": ""
            }
        });

        let settings: AgentRunSettings =
            serde_json::from_value(value).expect("legacy settings should deserialize");

        assert_eq!(settings.command_overrides, AgentCommandOverrides::default());
    }

    #[test]
    fn resolve_agent_command_prefers_agent_override_then_global_then_default() {
        let overrides = AgentCommandOverrides {
            global_command: Some("global-acp".into()),
            agent_commands: BTreeMap::from([("codex".into(), "codex-acp".into())]),
            ..Default::default()
        };

        assert_eq!(
            resolve_agent_command("codex", &overrides, Some("default-acp".into()))
                .expect("agent override")
                .source,
            AgentCommandSource::AgentOverride
        );
        assert_eq!(
            resolve_agent_command("claude-code", &overrides, Some("default-acp".into()))
                .expect("global override")
                .source,
            AgentCommandSource::GlobalOverride
        );
        assert_eq!(
            resolve_agent_command(
                "claude-code",
                &AgentCommandOverrides::default(),
                Some(" default-acp ".into())
            )
            .expect("default command"),
            CommandResolutionResult {
                agent_id: "claude-code".into(),
                command: "default-acp".into(),
                source: AgentCommandSource::DefaultCommand,
            }
        );
    }

    // ---- specs/008: env/프로필 normalization·seed·불변식 ----

    fn profile(id: &str, built_in: bool, enabled: bool) -> AgentProfile {
        AgentProfile {
            id: id.into(),
            name: id.into(),
            agent_type: id.into(),
            command: None,
            env: BTreeMap::new(),
            enabled,
            built_in,
        }
    }

    #[test]
    fn normalization_trims_env_keys_and_drops_blank_keys_keeping_empty_values() {
        let overrides = normalize_command_overrides(AgentCommandOverrides {
            global_env: BTreeMap::from([
                ("  FOO  ".to_string(), "bar".to_string()),
                ("".to_string(), "drop".to_string()),
                ("   ".to_string(), "drop".to_string()),
                ("EMPTY".to_string(), String::new()),
            ]),
            profiles: vec![AgentProfile {
                env: BTreeMap::from([(" KEY ".to_string(), "v".to_string())]),
                ..profile("codex", true, true)
            }],
            ..Default::default()
        });

        assert_eq!(
            overrides.global_env,
            BTreeMap::from([
                ("FOO".to_string(), "bar".to_string()),
                ("EMPTY".to_string(), String::new()),
            ])
        );
        assert_eq!(
            overrides.profiles[0].env,
            BTreeMap::from([("KEY".to_string(), "v".to_string())])
        );
    }

    #[test]
    fn effective_profiles_seed_missing_built_ins_with_legacy_commands() {
        // seed는 저장 데이터를 바꾸지 않고 읽기(유효 프로필 계산) 시 적용된다(R2).
        let overrides = AgentCommandOverrides {
            agent_commands: BTreeMap::from([(
                "claude-code".to_string(),
                "npx custom-claude".to_string(),
            )]),
            ..Default::default()
        };

        let profiles = effective_profiles(&overrides);

        let built_ins: Vec<&AgentProfile> =
            profiles.iter().filter(|profile| profile.built_in).collect();
        assert_eq!(built_ins.len(), 4);
        let claude = built_ins
            .iter()
            .find(|profile| profile.id == "claude-code")
            .expect("claude-code seeded");
        assert_eq!(claude.command.as_deref(), Some("npx custom-claude"));
        assert!(built_ins.iter().all(|profile| profile.enabled));
    }

    #[test]
    fn effective_profiles_keep_stored_entries_and_fill_missing_built_ins() {
        let overrides = AgentCommandOverrides {
            profiles: vec![
                AgentProfile {
                    name: "Claude 수정본".into(),
                    command: Some("npx modified".into()),
                    ..profile("claude-code", true, false)
                },
                profile("custom-1", false, true),
            ],
            ..Default::default()
        };

        let profiles = effective_profiles(&overrides);

        assert_eq!(
            profiles
                .iter()
                .find(|entry| entry.id == "claude-code")
                .map(|entry| entry.name.as_str()),
            Some("Claude 수정본"),
        );
        assert!(profiles.iter().any(|entry| entry.id == "custom-1"));
        assert_eq!(profiles.iter().filter(|entry| entry.built_in).count(), 4);
    }

    #[test]
    fn normalization_defaults_blank_profile_names_and_empty_commands() {
        let overrides = normalize_command_overrides(AgentCommandOverrides {
            profiles: vec![AgentProfile {
                name: "   ".into(),
                command: Some("   ".into()),
                ..profile("codex", true, true)
            }],
            ..Default::default()
        });

        let codex = overrides
            .profiles
            .iter()
            .find(|profile| profile.id == "codex")
            .expect("codex profile");
        assert!(!codex.name.trim().is_empty(), "name gets a default");
        assert_eq!(codex.command, None);
    }

    #[test]
    fn save_rejects_payload_with_no_active_built_in_profile() {
        let repository = MemoryAgentRunSettingsRepository::default();
        let mut settings = settings("codex");
        settings.command_overrides = AgentCommandOverrides {
            profiles: vec![
                AgentProfile {
                    env: BTreeMap::from([("SECRET_TOKEN".to_string(), "hunter2".to_string())]),
                    ..profile("codex", true, false)
                },
                profile("claude-code", true, false),
                profile("opencode", true, false),
                profile("pi-coding-agent", true, false),
            ],
            ..Default::default()
        };

        let error = save_settings(&repository, settings).expect_err("invariant violation");

        assert!(error.contains("built-in agent profile"));
        assert!(!error.contains("hunter2"), "env value must not leak into errors");
    }

    #[test]
    fn loads_legacy_command_only_overrides_without_migration() {
        // 구버전 형식(profiles/global_env 없는 JSON)이 그대로 역직렬화되는지 확인.
        let legacy_json = r#"{
            "globalCommand": "npx global",
            "agentCommands": { "codex": "npx codex" }
        }"#;
        let overrides: AgentCommandOverrides =
            serde_json::from_str(legacy_json).expect("legacy shape deserializes");

        assert_eq!(overrides.global_command.as_deref(), Some("npx global"));
        assert!(overrides.profiles.is_empty());
        assert!(overrides.global_env.is_empty());
    }
}
