//! Port for publishing orchestration state changes.

use serde::{Deserialize, Serialize};

use crate::domain::agent_orchestration::OrchestrationError;

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationEvent {
    pub workspace_id: String,
    pub revision: u64,
    pub reason: String,
    pub task_id: Option<String>,
    pub node_id: Option<String>,
}

pub trait OrchestrationEventSink: Send + Sync {
    fn emit(&self, window_label: &str, event: OrchestrationEvent)
    -> Result<(), OrchestrationError>;
}
