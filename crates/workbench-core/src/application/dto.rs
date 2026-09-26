//! 도메인 타입 ↔ protocol wire DTO 변환(research R1).
//!
//! DTO는 protocol crate에 **미러**로 정의되고 여기서 변환한다. wire가 이전(AW가 도메인 타입을 그대로 직렬화)과
//! 같음은 `assert_wire_parity`가 고정한다: 도메인 값을 serde로 직렬화한 JSON == DTO를 직렬화한 JSON.

use acp_agent_core::domain::run as acp_run;
use workbench_protocol::operations::{
    agent_run_settings as wire_settings,
    goal::{GoalDto, GoalStatus as WireGoalStatus},
    project::ProjectDto,
    saved_prompt::SavedPromptDto,
};

use crate::domain::{
    agent_run_settings::{
        AgentCommandOverrides, AgentProfile, AgentRunSessionMode, AgentRunSettings,
        AgentRunSettingsRalphLoop,
    },
    goal::{GoalStatus, ThreadGoal},
    project::Project,
    saved_prompt::SavedPrompt,
};

pub fn to_dto(project: &Project) -> ProjectDto {
    ProjectDto {
        id: project.id.clone(),
        name: project.name.clone(),
        working_directory: project.working_directory.clone(),
        description: project.description.clone(),
    }
}

pub fn saved_prompt_dto(prompt: &SavedPrompt) -> SavedPromptDto {
    SavedPromptDto {
        id: prompt.id.clone(),
        label: prompt.label.clone(),
        prompt: prompt.prompt.clone(),
    }
}

pub fn goal_status_dto(status: &GoalStatus) -> WireGoalStatus {
    match status {
        GoalStatus::Active => WireGoalStatus::Active,
        GoalStatus::Paused => WireGoalStatus::Paused,
        GoalStatus::Blocked => WireGoalStatus::Blocked,
        GoalStatus::UsageLimited => WireGoalStatus::UsageLimited,
        GoalStatus::BudgetLimited => WireGoalStatus::BudgetLimited,
        GoalStatus::Complete => WireGoalStatus::Complete,
    }
}

pub fn goal_status_domain(status: WireGoalStatus) -> GoalStatus {
    match status {
        WireGoalStatus::Active => GoalStatus::Active,
        WireGoalStatus::Paused => GoalStatus::Paused,
        WireGoalStatus::Blocked => GoalStatus::Blocked,
        WireGoalStatus::UsageLimited => GoalStatus::UsageLimited,
        WireGoalStatus::BudgetLimited => GoalStatus::BudgetLimited,
        WireGoalStatus::Complete => GoalStatus::Complete,
    }
}

pub fn goal_dto(goal: &ThreadGoal) -> GoalDto {
    GoalDto {
        working_directory: goal.working_directory.clone(),
        objective: goal.objective.clone(),
        status: goal_status_dto(&goal.status),
        token_budget: goal.token_budget.map(|budget| budget as u64),
        tokens_used: goal.tokens_used as u64,
        time_used_seconds: goal.time_used_seconds,
        created_at: goal.created_at.clone(),
        updated_at: goal.updated_at.clone(),
    }
}

fn permission_mode_dto(mode: acp_run::PermissionMode) -> wire_settings::PermissionMode {
    match mode {
        acp_run::PermissionMode::Default => wire_settings::PermissionMode::Default,
        acp_run::PermissionMode::Auto => wire_settings::PermissionMode::Auto,
        acp_run::PermissionMode::ReadOnly => wire_settings::PermissionMode::ReadOnly,
        acp_run::PermissionMode::Plan => wire_settings::PermissionMode::Plan,
        acp_run::PermissionMode::AcceptEdits => wire_settings::PermissionMode::AcceptEdits,
        acp_run::PermissionMode::DangerouslySkipAllPermissions => {
            wire_settings::PermissionMode::DangerouslySkipAllPermissions
        }
    }
}

fn permission_mode_domain(mode: wire_settings::PermissionMode) -> acp_run::PermissionMode {
    match mode {
        wire_settings::PermissionMode::Default => acp_run::PermissionMode::Default,
        wire_settings::PermissionMode::Auto => acp_run::PermissionMode::Auto,
        wire_settings::PermissionMode::ReadOnly => acp_run::PermissionMode::ReadOnly,
        wire_settings::PermissionMode::Plan => acp_run::PermissionMode::Plan,
        wire_settings::PermissionMode::AcceptEdits => acp_run::PermissionMode::AcceptEdits,
        wire_settings::PermissionMode::DangerouslySkipAllPermissions => {
            acp_run::PermissionMode::DangerouslySkipAllPermissions
        }
    }
}

fn context_size_dto(preset: acp_run::ContextSizePreset) -> wire_settings::ContextSizePreset {
    match preset {
        acp_run::ContextSizePreset::Default => wire_settings::ContextSizePreset::Default,
        acp_run::ContextSizePreset::Medium => wire_settings::ContextSizePreset::Medium,
        acp_run::ContextSizePreset::Large => wire_settings::ContextSizePreset::Large,
        acp_run::ContextSizePreset::XLarge => wire_settings::ContextSizePreset::XLarge,
    }
}

fn context_size_domain(preset: wire_settings::ContextSizePreset) -> acp_run::ContextSizePreset {
    match preset {
        wire_settings::ContextSizePreset::Default => acp_run::ContextSizePreset::Default,
        wire_settings::ContextSizePreset::Medium => acp_run::ContextSizePreset::Medium,
        wire_settings::ContextSizePreset::Large => acp_run::ContextSizePreset::Large,
        wire_settings::ContextSizePreset::XLarge => acp_run::ContextSizePreset::XLarge,
    }
}

fn session_mode_dto(mode: AgentRunSessionMode) -> wire_settings::AgentRunSessionMode {
    match mode {
        AgentRunSessionMode::New => wire_settings::AgentRunSessionMode::New,
        AgentRunSessionMode::Reuse => wire_settings::AgentRunSessionMode::Reuse,
    }
}

fn session_mode_domain(mode: wire_settings::AgentRunSessionMode) -> AgentRunSessionMode {
    match mode {
        wire_settings::AgentRunSessionMode::New => AgentRunSessionMode::New,
        wire_settings::AgentRunSessionMode::Reuse => AgentRunSessionMode::Reuse,
    }
}

fn profile_dto(profile: &AgentProfile) -> wire_settings::AgentProfileDto {
    wire_settings::AgentProfileDto {
        id: profile.id.clone(),
        name: profile.name.clone(),
        agent_type: profile.agent_type.clone(),
        command: profile.command.clone(),
        env: profile.env.clone(),
        enabled: profile.enabled,
        built_in: profile.built_in,
    }
}

fn profile_domain(profile: wire_settings::AgentProfileDto) -> AgentProfile {
    AgentProfile {
        id: profile.id,
        name: profile.name,
        agent_type: profile.agent_type,
        command: profile.command,
        env: profile.env,
        enabled: profile.enabled,
        built_in: profile.built_in,
    }
}

fn overrides_dto(overrides: &AgentCommandOverrides) -> wire_settings::AgentCommandOverridesDto {
    wire_settings::AgentCommandOverridesDto {
        global_command: overrides.global_command.clone(),
        agent_commands: overrides.agent_commands.clone(),
        global_env: overrides.global_env.clone(),
        profiles: overrides.profiles.iter().map(profile_dto).collect(),
    }
}

fn overrides_domain(overrides: wire_settings::AgentCommandOverridesDto) -> AgentCommandOverrides {
    AgentCommandOverrides {
        global_command: overrides.global_command,
        agent_commands: overrides.agent_commands,
        global_env: overrides.global_env,
        profiles: overrides.profiles.into_iter().map(profile_domain).collect(),
    }
}

fn ralph_loop_dto(
    ralph_loop: &AgentRunSettingsRalphLoop,
) -> wire_settings::AgentRunSettingsRalphLoopDto {
    wire_settings::AgentRunSettingsRalphLoopDto {
        enabled: ralph_loop.enabled,
        max_iterations: ralph_loop.max_iterations as u64,
        delay_ms: ralph_loop.delay_ms,
        stop_on_permission: ralph_loop.stop_on_permission,
        stop_on_error: ralph_loop.stop_on_error,
        prompt_template: ralph_loop.prompt_template.clone(),
    }
}

fn ralph_loop_domain(
    ralph_loop: wire_settings::AgentRunSettingsRalphLoopDto,
) -> AgentRunSettingsRalphLoop {
    AgentRunSettingsRalphLoop {
        enabled: ralph_loop.enabled,
        max_iterations: usize::try_from(ralph_loop.max_iterations).unwrap_or(usize::MAX),
        delay_ms: ralph_loop.delay_ms,
        stop_on_permission: ralph_loop.stop_on_permission,
        stop_on_error: ralph_loop.stop_on_error,
        prompt_template: ralph_loop.prompt_template,
    }
}

pub fn agent_run_settings_dto(settings: &AgentRunSettings) -> wire_settings::AgentRunSettingsDto {
    wire_settings::AgentRunSettingsDto {
        working_directory: settings.working_directory.clone(),
        agent_id: settings.agent_id.clone(),
        permission_mode: permission_mode_dto(settings.permission_mode),
        model_id: settings.model_id.clone(),
        effort_id: settings.effort_id.clone(),
        context_size: context_size_dto(settings.context_size),
        session_mode: session_mode_dto(settings.session_mode),
        ralph_loop: ralph_loop_dto(&settings.ralph_loop),
        command_overrides: overrides_dto(&settings.command_overrides),
    }
}

pub fn agent_run_settings_domain(dto: wire_settings::AgentRunSettingsDto) -> AgentRunSettings {
    AgentRunSettings {
        working_directory: dto.working_directory,
        agent_id: dto.agent_id,
        permission_mode: permission_mode_domain(dto.permission_mode),
        model_id: dto.model_id,
        effort_id: dto.effort_id,
        context_size: context_size_domain(dto.context_size),
        session_mode: session_mode_domain(dto.session_mode),
        ralph_loop: ralph_loop_domain(dto.ralph_loop),
        command_overrides: overrides_domain(dto.command_overrides),
    }
}

/// 테스트 helper: 도메인 값과 DTO의 JSON이 같은지 확인한다. 다르면 두 JSON을 함께 보여 준다.
#[cfg(test)]
pub(crate) fn assert_wire_parity<D: serde::Serialize, W: serde::Serialize>(
    label: &str,
    domain: &D,
    dto: &W,
) {
    let domain_json = serde_json::to_value(domain).expect("domain serializes");
    let dto_json = serde_json::to_value(dto).expect("dto serializes");
    assert_eq!(
        domain_json,
        dto_json,
        "{label}: wire mismatch\n domain = {}\n dto    = {}",
        serde_json::to_string_pretty(&domain_json).unwrap(),
        serde_json::to_string_pretty(&dto_json).unwrap()
    );
}

/// 테스트 helper: 프론트가 보내는 형태의 JSON이 input DTO로 역직렬화되고 다시 같은 JSON이 되는지 확인한다.
#[cfg(test)]
pub(crate) fn assert_input_roundtrip<I: serde::de::DeserializeOwned + serde::Serialize>(
    label: &str,
    json: serde_json::Value,
) -> I {
    let parsed: I = serde_json::from_value(json.clone())
        .unwrap_or_else(|error| panic!("{label}: input does not deserialize: {error}\n{json}"));
    let back = serde_json::to_value(&parsed).expect("input serializes");
    assert_eq!(back, json, "{label}: input roundtrip changed the JSON");
    parsed
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;
    use workbench_protocol::operations::{
        agent_run_settings::AgentRunSettingsSaveInput, goal::GoalUpdateInput,
    };

    use super::*;

    #[test]
    fn project_wire_parity() {
        for description in [None, Some("desc".to_owned())] {
            let project = Project {
                id: "project-1".into(),
                name: "AW".into(),
                working_directory: "/tmp/aw".into(),
                description,
            };
            assert_wire_parity("project", &project, &to_dto(&project));
        }
    }

    #[test]
    fn saved_prompt_wire_parity() {
        let prompt = SavedPrompt {
            id: "saved-prompt-1".into(),
            label: "Continue".into(),
            prompt: "keep going".into(),
        };
        assert_wire_parity("saved prompt", &prompt, &saved_prompt_dto(&prompt));
    }

    #[test]
    fn goal_wire_parity_for_every_status_and_budget() {
        for (status, budget) in [
            (GoalStatus::Active, None),
            (GoalStatus::Paused, Some(0)),
            (GoalStatus::Blocked, Some(100)),
            (GoalStatus::UsageLimited, None),
            (GoalStatus::BudgetLimited, Some(1)),
            (GoalStatus::Complete, None),
        ] {
            let goal = ThreadGoal {
                working_directory: "/repo/wt".into(),
                objective: "Ship".into(),
                status: status.clone(),
                token_budget: budget,
                tokens_used: 7,
                time_used_seconds: 9,
                created_at: "2026-01-01T00:00:00+00:00".into(),
                updated_at: "2026-01-02T00:00:00+00:00".into(),
            };
            assert_wire_parity(&format!("goal {status:?}"), &goal, &goal_dto(&goal));
            assert_eq!(goal_status_domain(goal_status_dto(&status)), status);
        }
    }

    #[test]
    fn goal_update_input_null_budget_means_no_change_like_aw() {
        let parsed: GoalUpdateInput = serde_json::from_value(
            json!({"workingDirectory": "/w", "objective": "x", "tokenBudget": null}),
        )
        .unwrap();
        assert_eq!(parsed.token_budget, None);
        let parsed: GoalUpdateInput =
            serde_json::from_value(json!({"workingDirectory": "/w", "tokenBudget": 3})).unwrap();
        assert_eq!(parsed.token_budget, Some(Some(3)));
    }

    fn full_settings() -> AgentRunSettings {
        AgentRunSettings {
            working_directory: "/repo/wt".into(),
            agent_id: "codex".into(),
            permission_mode: acp_run::PermissionMode::DangerouslySkipAllPermissions,
            model_id: "gpt-5".into(),
            effort_id: "high".into(),
            context_size: acp_run::ContextSizePreset::XLarge,
            session_mode: AgentRunSessionMode::Reuse,
            ralph_loop: AgentRunSettingsRalphLoop {
                enabled: true,
                max_iterations: 3,
                delay_ms: 10,
                stop_on_permission: true,
                stop_on_error: false,
                prompt_template: "go".into(),
            },
            command_overrides: AgentCommandOverrides {
                global_command: Some("npx acp".into()),
                agent_commands: BTreeMap::from([("codex".to_string(), "codex-acp".to_string())]),
                global_env: BTreeMap::from([("FOO".to_string(), "1".to_string())]),
                profiles: vec![AgentProfile {
                    id: "codex".into(),
                    name: "Codex".into(),
                    agent_type: "codex".into(),
                    command: None,
                    env: BTreeMap::from([("K".to_string(), String::new())]),
                    enabled: false,
                    built_in: true,
                }],
            },
        }
    }

    #[test]
    fn agent_run_settings_wire_parity_and_roundtrip() {
        let empty = AgentRunSettings {
            working_directory: "/repo/wt".into(),
            agent_id: String::new(),
            permission_mode: acp_run::PermissionMode::Default,
            model_id: "providerDefault".into(),
            effort_id: "providerDefault".into(),
            context_size: acp_run::ContextSizePreset::Default,
            session_mode: AgentRunSessionMode::New,
            ralph_loop: AgentRunSettingsRalphLoop::default(),
            command_overrides: AgentCommandOverrides::default(),
        };
        for (label, settings) in [("empty", empty), ("full", full_settings())] {
            let dto = agent_run_settings_dto(&settings);
            assert_wire_parity(label, &settings, &dto);
            assert_eq!(agent_run_settings_domain(dto), settings, "{label}");
        }
    }

    /// 프론트 `entities/agent-run/model/types.ts`의 `AgentRunSettings` 필드 집합 그대로(`commandOverrides` 선택).
    #[test]
    fn agent_run_settings_save_input_accepts_frontend_shape() {
        let json = json!({
            "settings": {
                "workingDirectory": "/repo/wt",
                "agentId": "codex",
                "permissionMode": "plan",
                "modelId": "providerDefault",
                "effortId": "providerDefault",
                "contextSize": "default",
                "sessionMode": "new",
                "ralphLoop": {
                    "enabled": false, "maxIterations": 5, "delayMs": 0,
                    "stopOnPermission": false, "stopOnError": true, "promptTemplate": ""
                },
                "commandOverrides": {
                    "globalCommand": null, "agentCommands": {}, "globalEnv": {}, "profiles": []
                }
            }
        });
        let input: AgentRunSettingsSaveInput =
            assert_input_roundtrip("agentRunSettings.save", json);
        let domain = agent_run_settings_domain(input.settings);
        assert_eq!(domain.permission_mode, acp_run::PermissionMode::Plan);
    }
}
