//! Builds ACP launch requests for foreground and background agent runs.

use std::collections::BTreeMap;

use crate::{
    domain::run::{AgentRunRequest, RalphLoopRequest, ResumePolicy},
    infrastructure::{
        acp_agent_worker_adapter::AgentWorkerLaunchRequest,
        mcp::{AW_MCP_RUN_ID_ENV, AW_MCP_TOKEN_ENV, AW_MCP_URL_ENV, McpLaunchEnv},
    },
};

pub fn normalize_run_request(mut request: AgentRunRequest) -> AgentRunRequest {
    if request.run_id.as_deref().is_none_or(str::is_empty) {
        request.run_id = Some(uuid::Uuid::new_v4().to_string());
    }
    request.workspace_id = None;
    request.checkout_id = None;
    request.ralph_loop = request.ralph_loop.map(RalphLoopRequest::sanitized);
    request
}

pub fn inject_mcp_launch_env(request: &mut AgentRunRequest, env: McpLaunchEnv) {
    let agent_env = request.agent_env.get_or_insert_with(BTreeMap::new);
    agent_env.insert(AW_MCP_URL_ENV.to_string(), env.url.clone());
    agent_env.insert(AW_MCP_TOKEN_ENV.to_string(), env.token.clone());
    agent_env.insert(AW_MCP_RUN_ID_ENV.to_string(), env.run_id.clone());
    request.mcp_servers.push(env.server_config());
    request.goal = with_mcp_agent_instructions(&request.goal, &env.agent_instructions());
}

pub fn with_mcp_agent_instructions(goal: &str, instructions: &str) -> String {
    format!(
        "{instructions}\n---\n\nUser request:\n{goal}",
        instructions = instructions.trim(),
        goal = goal.trim()
    )
}

pub fn build_worker_request(
    launch: &AgentWorkerLaunchRequest,
    env: McpLaunchEnv,
) -> AgentRunRequest {
    let assignment = &launch.assignment;
    let mut request = AgentRunRequest {
        goal: launch.goal.clone(),
        agent_id: assignment.runtime_profile.agent_profile_id.clone(),
        workspace_id: None,
        checkout_id: None,
        cwd: Some(assignment.worktree_path.clone()),
        agent_command: None,
        agent_env: None,
        mcp_servers: Vec::new(),
        stdio_buffer_limit_mb: None,
        auto_allow: Some(launch.auto_allow),
        permission_mode: Some(launch.permission_mode),
        model_id: assignment.runtime_profile.model_id.clone(),
        context_size: None,
        run_id: Some(assignment.planned_run_id.clone()),
        resume_session_id: None,
        resume_policy: Some(ResumePolicy::Fresh),
        ralph_loop: None,
    };
    inject_mcp_launch_env(&mut request, env);
    request
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            agent_orchestration::{AccessPolicy, AgentRoleProfile, WorkerRuntimeProfile},
            run::PermissionMode,
        },
        ports::agent_worker::WorkerAssignment,
    };

    fn worker_launch() -> AgentWorkerLaunchRequest {
        AgentWorkerLaunchRequest {
            assignment: WorkerAssignment {
                workspace_id: "workspace-1".into(),
                window_label: "window-1".into(),
                worktree_path: "/repo".into(),
                node_id: "child-1".into(),
                task_id: "task-1".into(),
                attempt: 1,
                planned_run_id: "run-1".into(),
                role: AgentRoleProfile::new("researcher", "Researcher", "조사", "구조화 결과")
                    .unwrap(),
                objective: "구조를 조사한다.".into(),
                constraints: vec!["read-only".into()],
                expected_result: "근거 목록".into(),
                runtime_profile: WorkerRuntimeProfile {
                    agent_profile_id: "codex".into(),
                    provider_id: "codex".into(),
                    model_id: None,
                    access_policy: AccessPolicy::ReadOnly,
                    supports_read_only: true,
                },
                mcp_capability: "awcap_test".into(),
            },
            permission_mode: PermissionMode::ReadOnly,
            auto_allow: true,
            goal: "조사한다.".into(),
            worktree_fingerprint: "fingerprint".into(),
        }
    }

    #[test]
    fn preserves_background_permission_policy_in_agent_run_request() {
        let request = build_worker_request(
            &worker_launch(),
            McpLaunchEnv {
                url: "http://127.0.0.1:1234/".into(),
                token: "secret".into(),
                run_id: "run-1".into(),
            },
        );

        assert_eq!(request.permission_mode, Some(PermissionMode::ReadOnly));
        assert_eq!(request.auto_allow, Some(true));
        assert_eq!(request.run_id.as_deref(), Some("run-1"));
        assert!(
            request
                .agent_env
                .as_ref()
                .is_some_and(|env| env.contains_key(AW_MCP_TOKEN_ENV))
        );
    }
}
