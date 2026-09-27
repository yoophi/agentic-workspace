//! `agentRunSettings.*` operation의 input/output wire 타입. core `domain::agent_run_settings::AgentRunSettings`와
//! acp-agent-core `PermissionMode`·`ContextSizePreset`의 미러(research R1). serde 속성은 원본과 같다.
//!
//! 최상위 input(`AgentRunSettingsSaveInput`)만 `deny_unknown_fields`다. 중첩 DTO는 프론트가 보내는 여분 필드를
//! 원본처럼 무시한다(research R2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::{
    openapi::{RefOr, Schema},
    PartialSchema, ToSchema,
};

use super::common::nullable_schema;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    #[default]
    Default,
    Auto,
    ReadOnly,
    Plan,
    AcceptEdits,
    DangerouslySkipAllPermissions,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ContextSizePreset {
    #[default]
    Default,
    Medium,
    Large,
    XLarge,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AgentRunSessionMode {
    #[default]
    New,
    Reuse,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunSettingsDto {
    pub working_directory: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub permission_mode: PermissionMode,
    #[serde(default = "default_model_id")]
    pub model_id: String,
    #[serde(default = "default_model_id")]
    pub effort_id: String,
    #[serde(default)]
    pub context_size: ContextSizePreset,
    #[serde(default)]
    pub session_mode: AgentRunSessionMode,
    #[serde(default)]
    pub ralph_loop: AgentRunSettingsRalphLoopDto,
    #[serde(default)]
    pub command_overrides: AgentCommandOverridesDto,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentCommandOverridesDto {
    #[serde(default)]
    pub global_command: Option<String>,
    #[serde(default)]
    pub agent_commands: BTreeMap<String, String>,
    #[serde(default)]
    pub global_env: BTreeMap<String, String>,
    #[serde(default)]
    pub profiles: Vec<AgentProfileDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentProfileDto {
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunSettingsRalphLoopDto {
    pub enabled: bool,
    pub max_iterations: u64,
    pub delay_ms: u64,
    #[serde(default)]
    pub stop_on_permission: bool,
    pub stop_on_error: bool,
    pub prompt_template: String,
}

impl Default for AgentRunSettingsRalphLoopDto {
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

fn default_true() -> bool {
    true
}

fn default_model_id() -> String {
    "providerDefault".to_string()
}

pub fn agent_run_settings_get_output_schema() -> RefOr<Schema> {
    nullable_schema(AgentRunSettingsDto::schema())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentRunSettingsGetInput {
    pub working_directory: String,
}

/// `agentRunSettings.save` input. AW `save_agent_run_settings(settings)`의 파라미터를 `settings` 아래에 둔다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentRunSettingsSaveInput {
    pub settings: AgentRunSettingsDto,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn legacy_settings_without_overrides_deserialize_with_defaults() {
        let value = json!({
            "workingDirectory": "/repo/worktree",
            "agentId": "codex",
            "permissionMode": "plan",
            "modelId": "providerDefault",
            "contextSize": "default",
            "sessionMode": "new",
            "ralphLoop": {
                "enabled": false, "maxIterations": 5, "delayMs": 0,
                "stopOnPermission": false, "stopOnError": true, "promptTemplate": ""
            },
            "futureField": 1
        });
        let dto: AgentRunSettingsDto = serde_json::from_value(value).unwrap();
        assert_eq!(dto.effort_id, "providerDefault");
        assert_eq!(dto.command_overrides, AgentCommandOverridesDto::default());
    }

    #[test]
    fn save_input_rejects_unknown_top_level_fields_only() {
        let settings = json!({"workingDirectory": "/w", "ralphLoop": {"enabled": false, "maxIterations": 1, "delayMs": 0, "stopOnError": true, "promptTemplate": ""}});
        assert!(serde_json::from_value::<AgentRunSettingsSaveInput>(
            json!({"settings": settings, "extra": 1})
        )
        .is_err());
        assert!(
            serde_json::from_value::<AgentRunSettingsSaveInput>(json!({"settings": settings}))
                .is_ok()
        );
    }
}
