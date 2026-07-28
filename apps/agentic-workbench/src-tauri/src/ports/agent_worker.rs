//! Port for launching and controlling background agent workers.

#![allow(async_fn_in_trait)]

use serde::{Deserialize, Serialize};

use crate::domain::agent_orchestration::{
    AgentRoleProfile, OrchestrationError, PromptDelivery, WorkerRuntimeProfile,
};

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerAssignment {
    pub workspace_id: String,
    pub window_label: String,
    pub worktree_path: String,
    pub node_id: String,
    pub task_id: String,
    pub attempt: u32,
    pub planned_run_id: String,
    pub role: AgentRoleProfile,
    pub objective: String,
    pub constraints: Vec<String>,
    pub expected_result: String,
    pub runtime_profile: WorkerRuntimeProfile,
    pub mcp_capability: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerBinding {
    pub workspace_id: String,
    pub window_label: String,
    pub node_id: String,
    pub task_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum StartWorkerOutcome {
    Started {
        run_id: String,
    },
    Queued {
        reason: String,
    },
    Failed {
        code: String,
        message: String,
        retryable: bool,
    },
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerCommandOutcome {
    pub accepted: bool,
    pub reason: Option<String>,
}

pub trait AgentWorkerPort: Send + Sync {
    async fn start_worker(
        &self,
        assignment: WorkerAssignment,
    ) -> Result<StartWorkerOutcome, OrchestrationError>;

    async fn send_prompt(
        &self,
        binding: &WorkerBinding,
        message: &str,
        delivery: PromptDelivery,
    ) -> Result<WorkerCommandOutcome, OrchestrationError>;

    async fn interrupt_worker(
        &self,
        binding: &WorkerBinding,
    ) -> Result<WorkerCommandOutcome, OrchestrationError>;

    async fn cancel_worker(
        &self,
        binding: &WorkerBinding,
    ) -> Result<WorkerCommandOutcome, OrchestrationError>;

    async fn is_active(&self, binding: &WorkerBinding) -> bool;
}
