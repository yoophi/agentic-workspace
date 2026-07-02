use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::run::{ContextSizePreset, PermissionMode};

pub const APP_COMMAND_OVERRIDE_SETTINGS_KEY: &str = "__app_agent_command_overrides__";

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
    #[serde(default)]
    pub command_overrides: AgentCommandOverrides,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCommandOverrides {
    #[serde(default)]
    pub global_command: Option<String>,
    /// legacy(command-only) 데이터. 신규 UI는 편집하지 않고 기본 프로필 seed의
    /// command 초기값 매핑에만 사용한다(specs/008 research R2).
    #[serde(default)]
    pub agent_commands: BTreeMap<String, String>,
    #[serde(default)]
    pub global_env: BTreeMap<String, String>,
    #[serde(default)]
    pub profiles: Vec<AgentProfile>,
}

/// 기본 프로필 4종. 실제 agent catalog id와 동일해야 legacy agentCommands 매핑과
/// 세션 재사용(agent id 기반)이 무변경으로 호환된다(specs/008 research R3).
pub const BUILT_IN_AGENT_TYPES: &[&str] = &["codex", "claude-code", "opencode", "pi-coding-agent"];

/// agent 실행 프로필(specs/008). 기본 프로필의 id는 catalog agent id와 동일해
/// 세션 재사용 등 agent id 기반 기존 흐름과 호환된다.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProfile {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub agent_type: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub built_in: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentCommandSource {
    AgentOverride,
    GlobalOverride,
    DefaultCommand,
    ProfileCommand,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResolutionResult {
    pub agent_id: String,
    pub command: String,
    pub source: AgentCommandSource,
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
