//! Injects the AW MCP server into ACP launch requests (041: background worker requests are built by core).

use std::collections::BTreeMap;

use crate::{
    domain::run::AgentRunRequest,
    infrastructure::mcp::{AW_MCP_RUN_ID_ENV, AW_MCP_TOKEN_ENV, AW_MCP_URL_ENV, McpLaunchEnv},
};

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
