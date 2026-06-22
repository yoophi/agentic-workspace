#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::run::AgentRunRequest;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpSessionRecord {
    pub run_id: String,
    pub session_id: String,
    pub workspace_id: Option<String>,
    pub checkout_id: Option<String>,
    pub workdir: Option<String>,
    pub agent_id: String,
    pub agent_command: Option<String>,
    pub task: String,
    pub created_at: String,
    pub updated_at: String,
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AcpSessionLookup {
    pub workspace_id: Option<String>,
    pub checkout_id: Option<String>,
    pub workdir: Option<String>,
    pub agent_id: String,
    pub agent_command: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpSessionListQuery {
    pub workspace_id: Option<String>,
    pub checkout_id: Option<String>,
    pub workdir: Option<String>,
    pub agent_id: Option<String>,
    pub agent_command: Option<String>,
    pub limit: Option<u32>,
}

impl AcpSessionRecord {
    pub fn from_request_with_agent_command(
        run_id: &str,
        session_id: &str,
        request: &AgentRunRequest,
        agent_command: Option<&str>,
    ) -> Self {
        let now = timestamp();
        Self {
            run_id: run_id.to_string(),
            session_id: session_id.to_string(),
            workspace_id: request.workspace_id.clone(),
            checkout_id: request.checkout_id.clone(),
            workdir: request.cwd.clone(),
            agent_id: request.agent_id.clone(),
            agent_command: normalize_agent_command_lossy(agent_command),
            task: request.goal.clone(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

impl AcpSessionLookup {
    pub fn from_request(request: &AgentRunRequest) -> Self {
        Self {
            workspace_id: request.workspace_id.clone(),
            checkout_id: request.checkout_id.clone(),
            workdir: request.cwd.clone(),
            agent_id: request.agent_id.clone(),
            agent_command: normalize_agent_command_lossy(request.agent_command.as_deref()),
        }
    }
}

pub fn normalize_agent_command(command: &str) -> Result<Option<String>> {
    let command = command.trim();
    if command.is_empty() {
        return Ok(None);
    }

    let argv = shell_words::split(command).context("agent command cannot be parsed")?;
    if argv.is_empty() {
        return Ok(None);
    }
    Ok(Some(shell_words::join(argv)))
}

fn normalize_agent_command_lossy(command: Option<&str>) -> Option<String> {
    command.and_then(|value| {
        normalize_agent_command(value)
            .ok()
            .flatten()
            .or_else(|| Some(value.trim().to_string()).filter(|value| !value.is_empty()))
    })
}

#[cfg(test)]
mod tests {
    use super::normalize_agent_command;

    #[test]
    fn normalizes_equivalent_agent_commands() {
        assert_eq!(
            normalize_agent_command(" npx   -y   @zed-industries/codex-acp ")
                .unwrap()
                .as_deref(),
            Some("npx -y @zed-industries/codex-acp")
        );
        assert_eq!(
            normalize_agent_command("agent '--flag=value with spaces'")
                .unwrap()
                .as_deref(),
            Some("agent '--flag=value with spaces'")
        );
        assert_eq!(normalize_agent_command("  ").unwrap(), None);
    }
}
