use serde::{Deserialize, Serialize};

pub const MAX_WORKSPACE_PANELS: usize = 8;
pub const MAX_EXCHANGE_MESSAGE_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentPanelStatus {
    Idle,
    Running,
    Closing,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentExchangeDelivery {
    Send,
    Queue,
    Draft,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentExchangeStatus {
    Pending,
    Accepted,
    Delivered,
    Rejected,
    Failed,
    Cancelled,
}

impl AgentExchangeStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Delivered | Self::Rejected | Self::Failed | Self::Cancelled
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPanelEndpoint {
    pub panel_id: String,
    pub title: String,
    pub run_id: Option<String>,
    pub status: AgentPanelStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkspaceSyncRequest {
    pub worktree_path: String,
    pub revision: u64,
    pub focused_panel_id: String,
    pub panels: Vec<AgentPanelEndpoint>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkspaceSyncResponse {
    pub revision: u64,
    pub accepted_panels: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentWorkspaceSnapshot {
    pub window_label: String,
    pub worktree_path: String,
    pub revision: u64,
    pub focused_panel_id: String,
    pub panels: Vec<AgentPanelEndpoint>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAgentExchangeRequest {
    pub request_id: String,
    pub source_panel_id: String,
    pub source_run_id: Option<String>,
    pub target_panel_id: String,
    pub target_run_id: Option<String>,
    pub message: String,
    pub delivery: AgentExchangeDelivery,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExchangeAckRequest {
    pub request_id: String,
    pub target_panel_id: String,
    pub outcome: AgentExchangeStatus,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExchangeEndpointRef {
    pub panel_id: String,
    pub title: String,
    pub run_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExchange {
    pub request_id: String,
    pub window_label: String,
    pub worktree_path: String,
    pub source: AgentExchangeEndpointRef,
    pub target: AgentExchangeEndpointRef,
    pub message: String,
    pub delivery: AgentExchangeDelivery,
    pub status: AgentExchangeStatus,
    pub failure_code: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExchangeRequestedEvent {
    pub request_id: String,
    pub source: AgentExchangeEndpointRef,
    pub target: AgentExchangeEndpointRef,
    pub message: String,
    pub delivery: AgentExchangeDelivery,
    pub created_at: String,
}

impl From<&AgentExchange> for AgentExchangeRequestedEvent {
    fn from(exchange: &AgentExchange) -> Self {
        Self {
            request_id: exchange.request_id.clone(),
            source: exchange.source.clone(),
            target: exchange.target.clone(),
            message: exchange.message.clone(),
            delivery: exchange.delivery,
            created_at: exchange.created_at.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExchangeError {
    pub code: String,
    pub message: String,
}

impl AgentExchangeError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AgentExchangeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AgentExchangeError {}

pub fn validate_workspace_request(
    request: &AgentWorkspaceSyncRequest,
) -> Result<(), AgentExchangeError> {
    if request.panels.is_empty() || request.panels.len() > MAX_WORKSPACE_PANELS {
        return Err(AgentExchangeError::new(
            "invalidPanels",
            format!("Workspace must contain 1..={MAX_WORKSPACE_PANELS} panels."),
        ));
    }
    let mut ids = std::collections::HashSet::new();
    for panel in &request.panels {
        if panel.panel_id.trim().is_empty() || !ids.insert(panel.panel_id.as_str()) {
            return Err(AgentExchangeError::new(
                "invalidPanels",
                "Panel ids must be non-empty and unique.",
            ));
        }
    }
    if !ids.contains(request.focused_panel_id.as_str()) {
        return Err(AgentExchangeError::new(
            "invalidFocus",
            "Focused panel must be present in the workspace.",
        ));
    }
    Ok(())
}

pub fn validate_exchange_message(raw: &str) -> Result<String, AgentExchangeError> {
    let message = raw.trim();
    if message.is_empty() {
        return Err(AgentExchangeError::new(
            "invalidMessage",
            "Exchange message must not be blank.",
        ));
    }
    if message.len() > MAX_EXCHANGE_MESSAGE_BYTES {
        return Err(AgentExchangeError::new(
            "messageTooLarge",
            format!("Exchange message must be at most {MAX_EXCHANGE_MESSAGE_BYTES} bytes."),
        ));
    }
    Ok(message.to_string())
}

pub fn can_transition_exchange(current: AgentExchangeStatus, next: AgentExchangeStatus) -> bool {
    if current.is_terminal() {
        return false;
    }
    match current {
        AgentExchangeStatus::Pending => {
            matches!(
                next,
                AgentExchangeStatus::Accepted | AgentExchangeStatus::Rejected
            )
        }
        AgentExchangeStatus::Accepted => matches!(
            next,
            AgentExchangeStatus::Delivered
                | AgentExchangeStatus::Rejected
                | AgentExchangeStatus::Failed
                | AgentExchangeStatus::Cancelled
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(id: &str) -> AgentPanelEndpoint {
        AgentPanelEndpoint {
            panel_id: id.into(),
            title: id.into(),
            run_id: None,
            status: AgentPanelStatus::Idle,
        }
    }

    #[test]
    fn validates_workspace_panel_identity_and_focus() {
        let valid = AgentWorkspaceSyncRequest {
            worktree_path: "/repo".into(),
            revision: 1,
            focused_panel_id: "main".into(),
            panels: vec![panel("main"), panel("extra")],
        };
        assert!(validate_workspace_request(&valid).is_ok());

        let duplicate = AgentWorkspaceSyncRequest {
            panels: vec![panel("main"), panel("main")],
            ..valid.clone()
        };
        assert_eq!(
            validate_workspace_request(&duplicate).unwrap_err().code,
            "invalidPanels"
        );
        let missing_focus = AgentWorkspaceSyncRequest {
            focused_panel_id: "missing".into(),
            ..valid
        };
        assert_eq!(
            validate_workspace_request(&missing_focus).unwrap_err().code,
            "invalidFocus"
        );
    }

    #[test]
    fn trims_and_limits_exchange_messages_by_utf8_bytes() {
        assert_eq!(validate_exchange_message("  hello  ").unwrap(), "hello");
        assert_eq!(
            validate_exchange_message(" \n ").unwrap_err().code,
            "invalidMessage"
        );
        assert_eq!(
            validate_exchange_message(&"가".repeat(MAX_EXCHANGE_MESSAGE_BYTES))
                .unwrap_err()
                .code,
            "messageTooLarge"
        );
    }

    #[test]
    fn exchange_states_only_move_forward() {
        assert!(can_transition_exchange(
            AgentExchangeStatus::Pending,
            AgentExchangeStatus::Accepted
        ));
        assert!(can_transition_exchange(
            AgentExchangeStatus::Accepted,
            AgentExchangeStatus::Delivered
        ));
        assert!(!can_transition_exchange(
            AgentExchangeStatus::Delivered,
            AgentExchangeStatus::Accepted
        ));
    }
}
