//! Injects the AW MCP server into ACP launch requests (041: background worker requests are built by core).
//!
//! 044(research R2): 주입은 창과 무관하다. `McpLaunchDecorator`는 모든 `run.start`(데스크톱 창이 없는 작업대 포함)에
//! run에 묶인 MCP 토큰·끝점·run id와 MCP 서버 설정·안내문을 넣는다. 작업대가 닫히면 core가 토큰을 폐기한다.

use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

use acp_agent_core::domain::run::AgentRunRequest;
use workbench_core::ports::desktop_bridge::{LaunchContext, RunLaunchDecorator};

use crate::mcp::{
    AW_MCP_RUN_ID_ENV, AW_MCP_TOKEN_ENV, AW_MCP_URL_ENV, McpLaunchEnv, McpServerState,
};

pub const MESSAGE_MCP_NOT_READY: &str = "MCP server is not ready.";

/// run 시작 보강: 런타임이 MCP 서버보다 먼저 만들어지므로(MCP가 런타임을 쓴다) 시작 뒤 [`Self::bind`]로 묶는다.
#[derive(Default)]
pub struct McpLaunchDecorator {
    mcp: OnceLock<McpServerState>,
}

impl McpLaunchDecorator {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn bind(&self, mcp: McpServerState) {
        let _ = self.mcp.set(mcp);
    }
}

impl RunLaunchDecorator for McpLaunchDecorator {
    fn decorate(
        &self,
        request: &mut AgentRunRequest,
        context: &LaunchContext,
    ) -> Result<(), String> {
        let mcp = self
            .mcp
            .get()
            .ok_or_else(|| MESSAGE_MCP_NOT_READY.to_owned())?;
        inject_mcp_launch_env(request, mcp.launch_env(&context.run_id));
        Ok(())
    }

    fn revoke_run(&self, run_id: &str) {
        if let Some(mcp) = self.mcp.get() {
            mcp.revoke_run_capability(run_id);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_run_scoped_mcp_env_server_and_instructions() {
        let mut request: AgentRunRequest = serde_json::from_value(serde_json::json!({
            "goal": "조사한다.",
            "agentId": "codex"
        }))
        .unwrap();
        inject_mcp_launch_env(
            &mut request,
            McpLaunchEnv {
                url: "http://127.0.0.1:1234/".into(),
                token: "secret".into(),
                run_id: "run-1".into(),
            },
        );
        let env = request.agent_env.as_ref().unwrap();
        assert_eq!(
            env.get(AW_MCP_TOKEN_ENV).map(String::as_str),
            Some("secret")
        );
        assert_eq!(
            env.get(AW_MCP_RUN_ID_ENV).map(String::as_str),
            Some("run-1")
        );
        assert_eq!(request.mcp_servers.len(), 1);
        assert!(request.goal.ends_with("User request:\n조사한다."));
    }
}
