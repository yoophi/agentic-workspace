//! Worktree-window scoped orchestration domain.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const ORCHESTRATION_SCHEMA_VERSION: u32 = 2;
pub const MAIN_AGENT_NODE_ID: &str = "main-agent-run";
pub const MAX_ORCHESTRATION_NODES: usize = 8;
pub const MAX_PROMPT_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OrchestrationErrorCode {
    InvalidInput,
    InvalidTopology,
    InvalidTransition,
    ScopeMismatch,
    RevisionConflict,
    DuplicateConflict,
    NotFound,
    Unauthorized,
    CapacityExceeded,
    RuntimeLost,
    WorkerUnavailable,
    ReadOnlyViolation,
    /// No Main Coordinator run is bound, so orchestration cannot start at all.
    /// The user must start a Main run first (FR-022).
    CoordinatorInactive,
    /// A Main Coordinator run exists but cannot accept the request right now.
    /// The user should wait and retry (FR-022).
    CoordinatorBusy,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationError {
    pub code: OrchestrationErrorCode,
    pub message: String,
    pub retryable: bool,
}

impl OrchestrationError {
    pub fn new(code: OrchestrationErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retryable: false,
        }
    }

    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }
}

impl std::fmt::Display for OrchestrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for OrchestrationError {}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentNodeKind {
    Main,
    Child,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentNodeCreator {
    User,
    Coordinator,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Pending,
    Ready,
    Running,
    InputRequired,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn can_transition_to(self, next: Self) -> bool {
        if self == next {
            return true;
        }
        matches!(
            (self, next),
            (Self::Pending, Self::Ready)
                | (Self::Ready, Self::Running | Self::Cancelled)
                | (
                    Self::Running,
                    Self::InputRequired
                        | Self::Blocked
                        | Self::Completed
                        | Self::Failed
                        | Self::Cancelled
                )
                | (
                    Self::InputRequired,
                    Self::Running | Self::Blocked | Self::Failed | Self::Cancelled
                )
                | (Self::Blocked, Self::Ready | Self::Failed | Self::Cancelled)
                | (Self::Failed, Self::Ready)
        )
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionStatus {
    Unassigned,
    Starting,
    Active,
    Idle,
    Stopped,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PresentationStatus {
    Background,
    AttentionRequired,
    Promoting,
    Panel,
    Detached,
    Archived,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PromotionPolicy {
    Manual,
    OnAttention,
    Always,
    OnFailure,
    OnCompletion,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AccessPolicy {
    ReadOnly,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRoleProfile {
    pub id: String,
    pub name: String,
    pub responsibility: String,
    pub expected_output: String,
    pub system_instructions: Option<String>,
}

impl AgentRoleProfile {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        responsibility: impl Into<String>,
        expected_output: impl Into<String>,
    ) -> Result<Self, OrchestrationError> {
        let profile = Self {
            id: id.into(),
            name: name.into(),
            responsibility: responsibility.into(),
            expected_output: expected_output.into(),
            system_instructions: None,
        };
        if profile.id.trim().is_empty()
            || profile.name.trim().is_empty()
            || profile.name.chars().count() > 80
            || profile.responsibility.trim().is_empty()
            || profile.expected_output.trim().is_empty()
        {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidInput,
                "Agent role profile is incomplete.",
            ));
        }
        Ok(profile)
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerRuntimeProfile {
    pub agent_profile_id: String,
    pub provider_id: String,
    pub model_id: Option<String>,
    pub access_policy: AccessPolicy,
    pub supports_read_only: bool,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentNode {
    pub id: String,
    pub kind: AgentNodeKind,
    pub parent_node_id: Option<String>,
    pub role: AgentRoleProfile,
    pub current_run_id: Option<String>,
    pub assigned_task_id: Option<String>,
    pub execution_status: ExecutionStatus,
    pub presentation_status: PresentationStatus,
    pub promotion_policy: PromotionPolicy,
    pub runtime_profile: Option<WorkerRuntimeProfile>,
    pub last_activity_at: Option<String>,
    pub created_by: AgentNodeCreator,
    pub created_at: String,
}

impl AgentNode {
    pub fn main(responsibility: impl Into<String>, created_at: impl Into<String>) -> Self {
        Self {
            id: MAIN_AGENT_NODE_ID.into(),
            kind: AgentNodeKind::Main,
            parent_node_id: None,
            role: AgentRoleProfile {
                id: "main-coordinator".into(),
                name: "Main".into(),
                responsibility: responsibility.into(),
                expected_output: "하위 작업의 출처와 충돌을 구분한 종합 결과".into(),
                system_instructions: None,
            },
            current_run_id: None,
            assigned_task_id: None,
            execution_status: ExecutionStatus::Unassigned,
            presentation_status: PresentationStatus::Panel,
            promotion_policy: PromotionPolicy::Always,
            runtime_profile: None,
            last_activity_at: None,
            created_by: AgentNodeCreator::User,
            created_at: created_at.into(),
        }
    }

    pub fn child(
        id: impl Into<String>,
        parent_node_id: impl Into<String>,
        role: AgentRoleProfile,
        created_at: impl Into<String>,
    ) -> Result<Self, OrchestrationError> {
        let id = id.into();
        let parent_node_id = parent_node_id.into();
        if id == MAIN_AGENT_NODE_ID || parent_node_id != MAIN_AGENT_NODE_ID {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidTopology,
                "Every child must be a direct child of Main.",
            ));
        }
        Ok(Self {
            id,
            kind: AgentNodeKind::Child,
            parent_node_id: Some(parent_node_id),
            role,
            current_run_id: None,
            assigned_task_id: None,
            execution_status: ExecutionStatus::Unassigned,
            presentation_status: PresentationStatus::Background,
            promotion_policy: PromotionPolicy::OnAttention,
            runtime_profile: None,
            last_activity_at: None,
            created_by: AgentNodeCreator::Coordinator,
            created_at: created_at.into(),
        })
    }

    pub fn can_delete(&self) -> bool {
        self.kind == AgentNodeKind::Child
    }

    pub fn detach(&mut self) {
        if self.kind == AgentNodeKind::Child {
            self.presentation_status = PresentationStatus::Detached;
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CoordinatorGenerationStatus {
    Active,
    Ended,
    Superseded,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorGeneration {
    pub id: String,
    pub ordinal: u32,
    pub main_node_id: String,
    pub run_id: String,
    pub previous_generation_id: Option<String>,
    pub status: CoordinatorGenerationStatus,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub handoff_summary: Option<String>,
    pub successor_generation_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFailure {
    pub code: OrchestrationErrorCode,
    pub message: String,
    pub retryable: bool,
    pub partial_result_report_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationTask {
    pub id: String,
    pub parent_task_id: Option<String>,
    pub coordinator_generation_id: String,
    pub assigned_node_id: Option<String>,
    pub title: String,
    pub objective: String,
    pub constraints: Vec<String>,
    pub expected_result: String,
    pub dependency_task_ids: Vec<String>,
    pub status: TaskStatus,
    pub awaiting_handoff: bool,
    pub access_policy: AccessPolicy,
    pub attempt: u32,
    pub latest_result_report_id: Option<String>,
    pub failure: Option<TaskFailure>,
    pub revision: u64,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub updated_at: String,
}

impl OrchestrationTask {
    pub fn transition(
        &mut self,
        status: TaskStatus,
        now: impl Into<String>,
    ) -> Result<(), OrchestrationError> {
        if !self.status.can_transition_to(status) {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidTransition,
                "Task status transition is not allowed.",
            ));
        }
        if self.status == status {
            return Ok(());
        }
        let now = now.into();
        if status == TaskStatus::Running && self.started_at.is_none() {
            self.started_at = Some(now.clone());
        }
        if status.is_terminal() {
            self.completed_at = Some(now.clone());
        }
        self.status = status;
        self.updated_at = now;
        self.revision += 1;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskReportType {
    Progress,
    Result,
    InputRequest,
    Blocked,
    Message,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FindingSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFinding {
    pub title: String,
    pub detail: String,
    pub evidence: Vec<String>,
    pub severity: FindingSeverity,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKind {
    File,
    Url,
    Text,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactReference {
    pub kind: ArtifactKind,
    pub uri: String,
    pub label: String,
    pub description: Option<String>,
}

impl ArtifactReference {
    pub fn validate_for_workspace(&self, worktree_path: &str) -> Result<(), OrchestrationError> {
        if self.kind != ArtifactKind::File {
            return Ok(());
        }
        let relative = std::path::Path::new(&self.uri);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::ReadOnlyViolation,
                "Artifact file references must be workspace-relative.",
            ));
        }
        let root = std::fs::canonicalize(worktree_path).map_err(|error| {
            OrchestrationError::new(
                OrchestrationErrorCode::InvalidInput,
                format!("Workspace path cannot be resolved: {error}"),
            )
        })?;
        let candidate = root.join(relative);
        if candidate.exists() {
            let canonical = std::fs::canonicalize(&candidate).map_err(|error| {
                OrchestrationError::new(
                    OrchestrationErrorCode::InvalidInput,
                    format!("Artifact path cannot be resolved: {error}"),
                )
            })?;
            if !canonical.starts_with(&root) {
                return Err(OrchestrationError::new(
                    OrchestrationErrorCode::ReadOnlyViolation,
                    "Artifact file reference escapes the workspace.",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskReport {
    pub id: String,
    pub request_id: String,
    pub task_id: String,
    pub reporter_node_id: String,
    pub reporter_run_id: String,
    #[serde(rename = "type")]
    pub report_type: TaskReportType,
    pub progress_percent: Option<u8>,
    pub summary: String,
    pub findings: Vec<TaskFinding>,
    pub artifact_refs: Vec<ArtifactReference>,
    pub unresolved: Vec<String>,
    pub confidence: Option<f64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskCommandKind {
    Message,
    InputResponse,
    Interrupt,
    Cancel,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskCommandSource {
    User,
    Coordinator,
    Recovery,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskCommandStatus {
    Pending,
    Dispatching,
    Accepted,
    Failed,
    Cancelled,
}

impl TaskCommandStatus {
    pub fn can_transition_to(self, next: Self) -> bool {
        self == next
            || matches!(
                (self, next),
                (Self::Pending, Self::Dispatching | Self::Cancelled)
                    | (
                        Self::Dispatching,
                        Self::Accepted | Self::Failed | Self::Cancelled
                    )
                    | (Self::Failed, Self::Pending | Self::Cancelled)
            )
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Accepted | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandFailure {
    pub code: OrchestrationErrorCode,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCommand {
    pub id: String,
    pub request_id: String,
    pub payload_fingerprint: String,
    pub task_id: String,
    pub node_id: String,
    pub run_id: String,
    pub attempt: u32,
    pub kind: TaskCommandKind,
    pub message: Option<String>,
    pub input_report_id: Option<String>,
    pub delivery: PromptDelivery,
    pub source: TaskCommandSource,
    pub status: TaskCommandStatus,
    pub failure: Option<CommandFailure>,
    pub created_at: String,
    pub updated_at: String,
}

impl TaskCommand {
    pub fn assert_current_binding(
        &self,
        task: &OrchestrationTask,
        node: &AgentNode,
    ) -> Result<(), OrchestrationError> {
        if task.id != self.task_id
            || task.attempt != self.attempt
            || task.assigned_node_id.as_deref() != Some(self.node_id.as_str())
            || node.id != self.node_id
            || node.current_run_id.as_deref() != Some(self.run_id.as_str())
        {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::RuntimeLost,
                "The command belongs to a stale task attempt or run.",
            ));
        }
        Ok(())
    }

    pub fn transition(
        &mut self,
        status: TaskCommandStatus,
        now: impl Into<String>,
    ) -> Result<(), OrchestrationError> {
        if !self.status.can_transition_to(status) {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidTransition,
                "Task command delivery transition is not allowed.",
            ));
        }
        self.status = status;
        self.updated_at = now.into();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CoordinatorNotificationStatus {
    Pending,
    Dispatching,
    /// Legacy status written before delivery and collection were tracked separately.
    Accepted,
    Delivered,
    Processed,
    Failed,
    Superseded,
}

impl CoordinatorNotificationStatus {
    pub fn can_transition_to(self, next: Self) -> bool {
        self == next
            || matches!(
                (self, next),
                (Self::Pending, Self::Dispatching | Self::Superseded)
                    | (
                        Self::Dispatching,
                        Self::Delivered | Self::Processed | Self::Failed | Self::Superseded
                    )
                    | (
                        Self::Accepted,
                        Self::Pending | Self::Processed | Self::Superseded
                    )
                    | (Self::Delivered, Self::Processed)
                    | (Self::Failed, Self::Pending | Self::Superseded)
            )
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorNotification {
    pub id: String,
    pub report_id: String,
    pub task_id: String,
    pub report_type: TaskReportType,
    pub generation_id: String,
    pub main_run_id: Option<String>,
    pub status: CoordinatorNotificationStatus,
    pub attempt_count: u32,
    pub failure: Option<CommandFailure>,
    #[serde(default)]
    pub collected_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl CoordinatorNotification {
    pub fn transition(
        &mut self,
        status: CoordinatorNotificationStatus,
        now: impl Into<String>,
    ) -> Result<(), OrchestrationError> {
        if !self.status.can_transition_to(status) {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidTransition,
                "Coordinator notification transition is not allowed.",
            ));
        }
        self.status = status;
        self.updated_at = now.into();
        Ok(())
    }
}

pub fn full_payload_fingerprint<T: Serialize>(payload: &T) -> Result<String, OrchestrationError> {
    serde_json::to_string(payload).map_err(|error| {
        OrchestrationError::new(
            OrchestrationErrorCode::InvalidInput,
            format!("Failed to normalize idempotent payload: {error}"),
        )
    })
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PromptDispatchIntent {
    Direct,
    Delegate,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PromptTargetMode {
    Focused,
    Selected,
    All,
    Coordinator,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PromptDelivery {
    Send,
    Queue,
    Draft,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PromptDispatchTargetStatus {
    Pending,
    Accepted,
    Delivered,
    Rejected,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptDispatchTarget {
    pub panel_id: String,
    pub run_id: Option<String>,
    pub request_id: String,
    pub status: PromptDispatchTargetStatus,
    pub failure_code: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptDispatch {
    pub id: String,
    pub intent: PromptDispatchIntent,
    pub target_mode: PromptTargetMode,
    pub message: String,
    pub delivery: PromptDelivery,
    pub targets: Vec<PromptDispatchTarget>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdempotencyRecord {
    pub actor_key: String,
    pub operation: String,
    pub request_id: String,
    pub payload_fingerprint: String,
    pub result_ref: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationSession {
    pub schema_version: u32,
    pub id: String,
    pub worktree_path: String,
    pub bound_window_label: Option<String>,
    pub main_node_id: String,
    pub active_coordinator_generation_id: Option<String>,
    pub nodes: Vec<AgentNode>,
    pub generations: Vec<CoordinatorGeneration>,
    pub tasks: Vec<OrchestrationTask>,
    pub reports: Vec<TaskReport>,
    #[serde(default)]
    pub commands: Vec<TaskCommand>,
    #[serde(default)]
    pub coordinator_notifications: Vec<CoordinatorNotification>,
    pub dispatches: Vec<PromptDispatch>,
    pub idempotency_records: Vec<IdempotencyRecord>,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl OrchestrationSession {
    pub fn new(
        id: impl Into<String>,
        worktree_path: impl Into<String>,
        window_label: impl Into<String>,
        now: impl Into<String>,
    ) -> Self {
        let now = now.into();
        Self {
            schema_version: ORCHESTRATION_SCHEMA_VERSION,
            id: id.into(),
            worktree_path: worktree_path.into(),
            bound_window_label: Some(window_label.into()),
            main_node_id: MAIN_AGENT_NODE_ID.into(),
            active_coordinator_generation_id: None,
            nodes: vec![AgentNode::main(
                "Coordinate direct child agents",
                now.clone(),
            )],
            generations: Vec::new(),
            tasks: Vec::new(),
            reports: Vec::new(),
            commands: Vec::new(),
            coordinator_notifications: Vec::new(),
            dispatches: Vec::new(),
            idempotency_records: Vec::new(),
            revision: 0,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn validate(&self) -> Result<(), OrchestrationError> {
        if self.nodes.len() > MAX_ORCHESTRATION_NODES {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::CapacityExceeded,
                "An orchestration workspace supports at most eight nodes.",
            ));
        }
        let main_count = self
            .nodes
            .iter()
            .filter(|node| node.kind == AgentNodeKind::Main)
            .count();
        if main_count != 1
            || self.main_node_id != MAIN_AGENT_NODE_ID
            || self.nodes.iter().any(|node| match node.kind {
                AgentNodeKind::Main => {
                    node.id != MAIN_AGENT_NODE_ID || node.parent_node_id.is_some()
                }
                AgentNodeKind::Child => {
                    node.id == MAIN_AGENT_NODE_ID
                        || node.parent_node_id.as_deref() != Some(MAIN_AGENT_NODE_ID)
                }
            })
        {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidTopology,
                "Session must contain exactly one Main and direct children only.",
            ));
        }
        let mut node_ids = HashSet::new();
        if self.nodes.iter().any(|node| !node_ids.insert(&node.id)) {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidTopology,
                "Agent node ids must be unique.",
            ));
        }
        Ok(())
    }

    pub fn assert_scope(&self, window_label: &str) -> Result<(), OrchestrationError> {
        if self.bound_window_label.as_deref() != Some(window_label) {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::ScopeMismatch,
                "The orchestration session belongs to another window.",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_is_immutable_and_children_form_a_single_level_star() {
        let main = AgentNode::main("Research coordinator", "2026-07-27T00:00:00Z");
        assert_eq!(main.id, MAIN_AGENT_NODE_ID);
        assert_eq!(main.kind, AgentNodeKind::Main);
        assert_eq!(main.parent_node_id, None);
        assert!(!main.can_delete());

        let child = AgentNode::child(
            "child-1",
            MAIN_AGENT_NODE_ID,
            AgentRoleProfile::new("researcher", "Researcher", "조사", "근거가 있는 결과").unwrap(),
            "2026-07-27T00:00:00Z",
        )
        .unwrap();
        assert_eq!(child.parent_node_id.as_deref(), Some(MAIN_AGENT_NODE_ID));
        assert!(
            AgentNode::child(
                "grandchild",
                "child-1",
                AgentRoleProfile::new("reviewer", "Reviewer", "검토", "위험 보고").unwrap(),
                "2026-07-27T00:00:00Z",
            )
            .is_err()
        );
    }

    #[test]
    fn task_execution_and_presentation_states_are_independent() {
        assert!(TaskStatus::Pending.can_transition_to(TaskStatus::Ready));
        assert!(TaskStatus::Running.can_transition_to(TaskStatus::InputRequired));
        assert!(TaskStatus::Running.can_transition_to(TaskStatus::Completed));
        assert!(!TaskStatus::Completed.can_transition_to(TaskStatus::Running));
        assert!(!TaskStatus::Cancelled.can_transition_to(TaskStatus::Ready));

        let mut node = AgentNode::child(
            "child-1",
            MAIN_AGENT_NODE_ID,
            AgentRoleProfile::new("tester", "Tester", "검증", "테스트 결과").unwrap(),
            "2026-07-27T00:00:00Z",
        )
        .unwrap();
        node.current_run_id = Some("run-1".into());
        node.execution_status = ExecutionStatus::Active;
        node.presentation_status = PresentationStatus::Panel;
        node.detach();

        assert_eq!(node.current_run_id.as_deref(), Some("run-1"));
        assert_eq!(node.execution_status, ExecutionStatus::Active);
        assert_eq!(node.presentation_status, PresentationStatus::Detached);
    }

    #[test]
    fn session_rejects_a_second_main_and_more_than_eight_nodes() {
        let mut session =
            OrchestrationSession::new("session-1", "/repo", "window-1", "2026-07-27T00:00:00Z");
        assert!(session.validate().is_ok());
        session
            .nodes
            .push(AgentNode::main("Duplicate", "2026-07-27T00:00:00Z"));
        assert_eq!(
            session.validate().unwrap_err().code,
            OrchestrationErrorCode::InvalidTopology
        );
    }

    #[test]
    fn artifact_file_references_reject_absolute_and_parent_paths() {
        let workspace = std::env::temp_dir();
        let valid = ArtifactReference {
            kind: ArtifactKind::File,
            uri: "docs/result.md".into(),
            label: "result".into(),
            description: None,
        };
        assert!(
            valid
                .validate_for_workspace(workspace.to_string_lossy().as_ref())
                .is_ok()
        );
        for uri in ["/etc/passwd", "../outside.txt"] {
            let invalid = ArtifactReference {
                uri: uri.into(),
                ..valid.clone()
            };
            assert_eq!(
                invalid
                    .validate_for_workspace(workspace.to_string_lossy().as_ref())
                    .unwrap_err()
                    .code,
                OrchestrationErrorCode::ReadOnlyViolation
            );
        }
    }

    /// FR-047: a symlink that resolves outside the workspace must be rejected even though
    /// the reference itself looks workspace-relative.
    #[test]
    fn artifact_file_references_reject_symlink_escapes() {
        let root = std::env::temp_dir().join(format!(
            "aw-artifact-symlink-{}",
            std::process::id() as u64 + 1
        ));
        let outside = std::env::temp_dir().join(format!(
            "aw-artifact-outside-{}",
            std::process::id() as u64 + 1
        ));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&root).expect("create workspace root");
        std::fs::create_dir_all(&outside).expect("create outside dir");
        std::fs::write(outside.join("secret.txt"), b"secret").expect("write outside file");

        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.join("secret.txt"), root.join("linked.txt"))
            .expect("create symlink");

        let escaping = ArtifactReference {
            kind: ArtifactKind::File,
            // Looks relative, but resolves outside the workspace.
            uri: "linked.txt".into(),
            label: "linked".into(),
            description: None,
        };

        #[cfg(unix)]
        assert_eq!(
            escaping
                .validate_for_workspace(root.to_string_lossy().as_ref())
                .unwrap_err()
                .code,
            OrchestrationErrorCode::ReadOnlyViolation
        );

        // A real file inside the workspace stays valid.
        std::fs::write(root.join("inside.txt"), b"ok").expect("write inside file");
        let inside = ArtifactReference {
            uri: "inside.txt".into(),
            ..escaping.clone()
        };
        assert!(
            inside
                .validate_for_workspace(root.to_string_lossy().as_ref())
                .is_ok()
        );

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn command_and_notification_transitions_enforce_attempt_and_run_fences() {
        let now = "2026-07-27T00:00:00Z".to_string();
        let mut session = OrchestrationSession::new("session-1", "/repo", "window-1", now.clone());
        let role = AgentRoleProfile::new("researcher", "Researcher", "조사", "근거 목록").unwrap();
        let mut node = AgentNode::child("child-1", MAIN_AGENT_NODE_ID, role, now.clone()).unwrap();
        node.current_run_id = Some("run-1".into());
        node.assigned_task_id = Some("task-1".into());
        let task = OrchestrationTask {
            id: "task-1".into(),
            parent_task_id: None,
            coordinator_generation_id: "generation-1".into(),
            assigned_node_id: Some("child-1".into()),
            title: "Research".into(),
            objective: "Inspect".into(),
            constraints: vec![],
            expected_result: "Findings".into(),
            dependency_task_ids: vec![],
            status: TaskStatus::Running,
            awaiting_handoff: false,
            access_policy: AccessPolicy::ReadOnly,
            attempt: 1,
            latest_result_report_id: None,
            failure: None,
            revision: 0,
            created_at: now.clone(),
            started_at: Some(now.clone()),
            completed_at: None,
            updated_at: now.clone(),
        };
        let mut command = TaskCommand {
            id: "command-1".into(),
            request_id: "request-1".into(),
            payload_fingerprint: "full-payload".into(),
            task_id: task.id.clone(),
            node_id: node.id.clone(),
            run_id: "run-1".into(),
            attempt: 1,
            kind: TaskCommandKind::Message,
            message: Some("status?".into()),
            input_report_id: None,
            delivery: PromptDelivery::Queue,
            source: TaskCommandSource::Coordinator,
            status: TaskCommandStatus::Pending,
            failure: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        command.assert_current_binding(&task, &node).unwrap();
        command
            .transition(TaskCommandStatus::Dispatching, now.clone())
            .unwrap();
        command
            .transition(TaskCommandStatus::Accepted, now.clone())
            .unwrap();
        assert!(
            command
                .transition(TaskCommandStatus::Pending, now.clone())
                .is_err()
        );

        let mut notification = CoordinatorNotification {
            id: "notification-1".into(),
            report_id: "report-1".into(),
            task_id: task.id.clone(),
            report_type: TaskReportType::Result,
            generation_id: "generation-1".into(),
            main_run_id: Some("main-run".into()),
            status: CoordinatorNotificationStatus::Pending,
            attempt_count: 0,
            failure: None,
            collected_at: None,
            created_at: now.clone(),
            updated_at: now,
        };
        notification
            .transition(CoordinatorNotificationStatus::Dispatching, "later")
            .unwrap();
        notification
            .transition(CoordinatorNotificationStatus::Delivered, "later")
            .unwrap();
        notification
            .transition(CoordinatorNotificationStatus::Processed, "later")
            .unwrap();
        session.commands.push(command);
        session.coordinator_notifications.push(notification);
        assert_eq!(session.commands.len(), 1);
        assert_eq!(session.coordinator_notifications.len(), 1);
    }
}
