//! Application service shared by Tauri and MCP inbound adapters.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    domain::agent_orchestration::{
        AccessPolicy, AgentNode, AgentRoleProfile, ArtifactReference, CoordinatorGeneration,
        CoordinatorGenerationStatus, CoordinatorNotification, CoordinatorNotificationStatus,
        ExecutionStatus, IdempotencyRecord, MAIN_AGENT_NODE_ID, MAX_ORCHESTRATION_NODES,
        OrchestrationError, OrchestrationErrorCode, OrchestrationSession, OrchestrationTask,
        PresentationStatus, PromptDelivery, PromptDispatch, PromptDispatchIntent,
        PromptDispatchTarget, PromptDispatchTargetStatus, PromptTargetMode, TaskFinding,
        TaskReport, TaskReportType, TaskStatus, full_payload_fingerprint,
    },
    ports::{
        orchestration_event_sink::{OrchestrationEvent, OrchestrationEventSink},
        orchestration_repository::OrchestrationRepository,
    },
};

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MainRunBindingState {
    Active,
    Ended,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindMainRunRequest {
    pub request_id: String,
    pub panel_id: String,
    pub run_id: String,
    pub state: MainRunBindingState,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChildTaskRequest {
    pub request_id: String,
    pub title: String,
    pub role: AgentRoleProfile,
    pub objective: String,
    pub constraints: Vec<String>,
    pub expected_result: String,
    pub dependency_task_ids: Vec<String>,
    pub preferred_node_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChildTaskOutcome {
    pub task_id: String,
    pub node_id: String,
    pub status: TaskStatus,
    pub execution_status: ExecutionStatus,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportTaskRequest {
    pub request_id: String,
    pub task_id: String,
    pub reporter_node_id: String,
    pub reporter_run_id: String,
    pub report_type: TaskReportType,
    pub progress_percent: Option<u8>,
    pub summary: String,
    pub findings: Vec<TaskFinding>,
    pub artifact_refs: Vec<ArtifactReference>,
    pub unresolved: Vec<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateGoalRequest {
    pub request_id: String,
    pub goal: String,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateGoalOutcome {
    pub root_task_id: String,
    pub generation_id: String,
    pub dispatch_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPresentationRequest {
    pub request_id: String,
    pub node_id: String,
    pub presentation_status: PresentationStatus,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskActionRequest {
    pub request_id: String,
    pub task_id: String,
    pub expected_revision: u64,
    pub message: Option<String>,
    pub target_node_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorHandoffRequest {
    pub request_id: String,
    pub successor_run_id: String,
    pub summary: String,
    pub confirmed: bool,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchPromptRequest {
    pub request_id: String,
    pub intent: PromptDispatchIntent,
    pub target_mode: PromptTargetMode,
    pub message: String,
    pub delivery: PromptDelivery,
    pub panel_ids: Vec<String>,
    pub expected_revision: u64,
}

pub struct OrchestrationService<R, E> {
    repository: R,
    event_sink: E,
}

impl<R, E> OrchestrationService<R, E>
where
    R: OrchestrationRepository,
    E: OrchestrationEventSink,
{
    pub fn new(repository: R, event_sink: E) -> Self {
        Self {
            repository,
            event_sink,
        }
    }

    pub fn bootstrap(
        &self,
        worktree_path: &str,
        window_label: &str,
        resume_workspace_id: Option<&str>,
    ) -> Result<OrchestrationSession, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        if let Some(existing) = sessions
            .iter()
            .find(|session| session.bound_window_label.as_deref() == Some(window_label))
        {
            if existing.worktree_path != worktree_path {
                return Err(OrchestrationError::new(
                    OrchestrationErrorCode::ScopeMismatch,
                    "The window is already bound to another worktree.",
                ));
            }
            return Ok(existing.clone());
        }

        let now = now();
        let session = if let Some(workspace_id) = resume_workspace_id {
            let existing = sessions
                .iter_mut()
                .find(|session| session.id == workspace_id)
                .ok_or_else(|| {
                    OrchestrationError::new(
                        OrchestrationErrorCode::NotFound,
                        "Recoverable orchestration workspace was not found.",
                    )
                })?;
            if existing.worktree_path != worktree_path || existing.bound_window_label.is_some() {
                return Err(OrchestrationError::new(
                    OrchestrationErrorCode::ScopeMismatch,
                    "The workspace cannot be bound to this window.",
                ));
            }
            existing.bound_window_label = Some(window_label.into());
            existing.revision += 1;
            existing.updated_at = now;
            existing.clone()
        } else {
            let session = OrchestrationSession::new(
                Uuid::new_v4().to_string(),
                worktree_path,
                window_label,
                now,
            );
            sessions.push(session.clone());
            session
        };

        self.repository.save_sessions(&sessions)?;
        self.emit_workspace_changed(&session, "bootstrap");
        Ok(session)
    }

    pub fn get_for_window(
        &self,
        window_label: &str,
    ) -> Result<Option<OrchestrationSession>, OrchestrationError> {
        Ok(self
            .repository
            .load_sessions()?
            .into_iter()
            .find(|session| session.bound_window_label.as_deref() == Some(window_label)))
    }

    pub fn list_for_worktree(
        &self,
        worktree_path: &str,
    ) -> Result<Vec<OrchestrationSession>, OrchestrationError> {
        Ok(self
            .repository
            .load_sessions()?
            .into_iter()
            .filter(|session| session.worktree_path == worktree_path)
            .collect())
    }

    pub fn list_recoverable(
        &self,
        worktree_path: &str,
    ) -> Result<Vec<OrchestrationSession>, OrchestrationError> {
        let mut sessions: Vec<_> = self
            .list_for_worktree(worktree_path)?
            .into_iter()
            .filter(|session| {
                session.bound_window_label.is_none()
                    && (!session.tasks.is_empty()
                        || !session.generations.is_empty()
                        || session.nodes.len() > 1
                        || !session.dispatches.is_empty())
            })
            .collect();
        sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(sessions)
    }

    pub fn release_window(
        &self,
        window_label: &str,
    ) -> Result<Option<OrchestrationSession>, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let Some(session) = sessions
            .iter_mut()
            .find(|session| session.bound_window_label.as_deref() == Some(window_label))
        else {
            return Ok(None);
        };
        reconcile_session_runtime(session, &[]);
        let open_task_ids: Vec<_> = session
            .tasks
            .iter()
            .filter(|task| !task.status.is_terminal())
            .map(|task| task.id.clone())
            .collect();
        for node in session
            .nodes
            .iter_mut()
            .filter(|node| node.kind == crate::domain::agent_orchestration::AgentNodeKind::Child)
        {
            if node
                .assigned_task_id
                .as_ref()
                .is_some_and(|task_id| open_task_ids.contains(task_id))
            {
                node.presentation_status = PresentationStatus::AttentionRequired;
            } else if matches!(
                node.presentation_status,
                PresentationStatus::Panel | PresentationStatus::Promoting
            ) {
                node.presentation_status = PresentationStatus::Background;
            }
        }
        session.bound_window_label = None;
        session.revision += 1;
        session.updated_at = now();
        let snapshot = session.clone();
        self.repository.save_sessions(&sessions)?;
        Ok(Some(snapshot))
    }

    pub fn bind_main_run(
        &self,
        window_label: &str,
        request: BindMainRunRequest,
    ) -> Result<OrchestrationSession, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = sessions
            .iter_mut()
            .find(|session| session.bound_window_label.as_deref() == Some(window_label))
            .ok_or_else(|| {
                OrchestrationError::new(
                    OrchestrationErrorCode::NotFound,
                    "Orchestration workspace is not bootstrapped.",
                )
            })?;
        session.assert_scope(window_label)?;
        if request.panel_id != MAIN_AGENT_NODE_ID {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidTopology,
                "Only the Main panel can own a Coordinator generation.",
            ));
        }
        let fingerprint = format!(
            "{}:{}:{:?}",
            request.panel_id, request.run_id, request.state
        );
        if let Some(existing) = session.idempotency_records.iter().find(|record| {
            record.actor_key == window_label
                && record.operation == "bindMainRun"
                && record.request_id == request.request_id
        }) {
            if existing.payload_fingerprint != fingerprint {
                return Err(OrchestrationError::new(
                    OrchestrationErrorCode::DuplicateConflict,
                    "Request id was already used with another payload.",
                ));
            }
            return Ok(session.clone());
        }
        if session.revision != request.expected_revision {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::RevisionConflict,
                format!(
                    "Expected revision {}, current revision is {}.",
                    request.expected_revision, session.revision
                ),
            )
            .retryable());
        }

        let now = now();
        let result_ref = match request.state {
            MainRunBindingState::Active => {
                if let Some(active_id) = session.active_coordinator_generation_id.clone() {
                    let active = session
                        .generations
                        .iter_mut()
                        .find(|generation| generation.id == active_id)
                        .ok_or_else(|| not_found("Active Coordinator generation"))?;
                    if active.run_id == request.run_id {
                        active_id
                    } else {
                        return Err(OrchestrationError::new(
                            OrchestrationErrorCode::InvalidTransition,
                            "A different Main run requires an explicit Coordinator handoff.",
                        ));
                    }
                } else {
                    let generation_id = Uuid::new_v4().to_string();
                    let generation = CoordinatorGeneration {
                        id: generation_id.clone(),
                        ordinal: session.generations.len() as u32 + 1,
                        main_node_id: MAIN_AGENT_NODE_ID.into(),
                        run_id: request.run_id.clone(),
                        previous_generation_id: None,
                        status: CoordinatorGenerationStatus::Active,
                        started_at: now.clone(),
                        ended_at: None,
                        handoff_summary: None,
                        successor_generation_id: None,
                    };
                    session.generations.push(generation);
                    session.active_coordinator_generation_id = Some(generation_id.clone());
                    if let Some(main) = session
                        .nodes
                        .iter_mut()
                        .find(|node| node.id == MAIN_AGENT_NODE_ID)
                    {
                        main.current_run_id = Some(request.run_id.clone());
                        main.execution_status = ExecutionStatus::Active;
                        main.last_activity_at = Some(now.clone());
                    }
                    generation_id
                }
            }
            MainRunBindingState::Ended => {
                let generation = session
                    .generations
                    .iter_mut()
                    .find(|generation| {
                        generation.run_id == request.run_id
                            && generation.status == CoordinatorGenerationStatus::Active
                    })
                    .ok_or_else(|| {
                        OrchestrationError::new(
                            OrchestrationErrorCode::NotFound,
                            "Active Coordinator generation was not found.",
                        )
                    })?;
                generation.status = CoordinatorGenerationStatus::Ended;
                generation.ended_at = Some(now.clone());
                let generation_id = generation.id.clone();
                for task in session.tasks.iter_mut().filter(|task| {
                    task.coordinator_generation_id == generation_id && !task.status.is_terminal()
                }) {
                    task.awaiting_handoff = true;
                }
                session.active_coordinator_generation_id = None;
                if let Some(main) = session
                    .nodes
                    .iter_mut()
                    .find(|node| node.id == MAIN_AGENT_NODE_ID)
                {
                    main.current_run_id = None;
                    main.execution_status = ExecutionStatus::Stopped;
                    main.last_activity_at = Some(now.clone());
                }
                generation_id
            }
        };

        session.revision += 1;
        session.updated_at = now.clone();
        session.idempotency_records.push(IdempotencyRecord {
            actor_key: window_label.into(),
            operation: "bindMainRun".into(),
            request_id: request.request_id,
            payload_fingerprint: fingerprint,
            result_ref,
            created_at: now,
        });
        let snapshot = session.clone();
        self.repository.save_sessions(&sessions)?;
        self.emit_workspace_changed(&snapshot, "mainRunBinding");
        Ok(snapshot)
    }

    pub fn create_child_task(
        &self,
        window_label: &str,
        generation_id: &str,
        request: CreateChildTaskRequest,
    ) -> Result<CreateChildTaskOutcome, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        if session.active_coordinator_generation_id.as_deref() != Some(generation_id) {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::Unauthorized,
                "Only the active Main Coordinator can create child tasks.",
            ));
        }
        validate_task_input(&request.title, &request.objective, &request.expected_result)?;
        let fingerprint = serde_json::to_string(&request).map_err(|error| {
            OrchestrationError::new(
                OrchestrationErrorCode::InvalidInput,
                format!("Failed to normalize task request: {error}"),
            )
        })?;
        if let Some(record) = session.idempotency_records.iter().find(|record| {
            record.actor_key == generation_id
                && record.operation == "createChildTask"
                && record.request_id == request.request_id
        }) {
            if record.payload_fingerprint != fingerprint {
                return Err(OrchestrationError::new(
                    OrchestrationErrorCode::DuplicateConflict,
                    "Request id was already used with another task payload.",
                ));
            }
            let (task_id, node_id) = record.result_ref.split_once('|').ok_or_else(|| {
                OrchestrationError::new(
                    OrchestrationErrorCode::NotFound,
                    "Stored task result is unavailable.",
                )
            })?;
            let task = session
                .tasks
                .iter()
                .find(|task| task.id == task_id)
                .ok_or_else(|| {
                    OrchestrationError::new(
                        OrchestrationErrorCode::NotFound,
                        "Stored task is unavailable.",
                    )
                })?;
            let node = session
                .nodes
                .iter()
                .find(|node| node.id == node_id)
                .ok_or_else(|| {
                    OrchestrationError::new(
                        OrchestrationErrorCode::NotFound,
                        "Stored child node is unavailable.",
                    )
                })?;
            return Ok(CreateChildTaskOutcome {
                task_id: task.id.clone(),
                node_id: node.id.clone(),
                status: task.status,
                execution_status: node.execution_status,
            });
        }

        for dependency_id in &request.dependency_task_ids {
            if !session.tasks.iter().any(|task| {
                task.id == *dependency_id && task.coordinator_generation_id == generation_id
            }) {
                return Err(OrchestrationError::new(
                    OrchestrationErrorCode::InvalidInput,
                    "Every dependency must belong to the active generation.",
                ));
            }
        }
        let now = now();
        let node_id = if let Some(preferred) = request.preferred_node_id.as_deref() {
            let node = session
                .nodes
                .iter()
                .find(|node| {
                    node.id == preferred
                        && node.parent_node_id.as_deref() == Some(MAIN_AGENT_NODE_ID)
                        && node.assigned_task_id.is_none()
                })
                .ok_or_else(|| {
                    OrchestrationError::new(
                        OrchestrationErrorCode::InvalidTopology,
                        "Preferred node must be an idle direct child of Main.",
                    )
                })?;
            node.id.clone()
        } else {
            if session.nodes.len() >= MAX_ORCHESTRATION_NODES {
                return Err(OrchestrationError::new(
                    OrchestrationErrorCode::CapacityExceeded,
                    "The workspace already has the maximum number of agent nodes.",
                ));
            }
            let mut sequence = 1;
            let node_id = loop {
                let candidate = format!("extra-agent-run-{sequence}");
                if !session.nodes.iter().any(|node| node.id == candidate) {
                    break candidate;
                }
                sequence += 1;
            };
            session.nodes.push(AgentNode::child(
                node_id.clone(),
                MAIN_AGENT_NODE_ID,
                request.role.clone(),
                now.clone(),
            )?);
            node_id
        };
        let dependencies_completed = request.dependency_task_ids.iter().all(|dependency_id| {
            session
                .tasks
                .iter()
                .any(|task| task.id == *dependency_id && task.status == TaskStatus::Completed)
        });
        let task_id = Uuid::new_v4().to_string();
        let status = if dependencies_completed {
            TaskStatus::Ready
        } else {
            TaskStatus::Pending
        };
        let task = OrchestrationTask {
            id: task_id.clone(),
            parent_task_id: None,
            coordinator_generation_id: generation_id.into(),
            assigned_node_id: Some(node_id.clone()),
            title: request.title,
            objective: request.objective,
            constraints: request.constraints,
            expected_result: request.expected_result,
            dependency_task_ids: request.dependency_task_ids,
            status,
            awaiting_handoff: false,
            access_policy: AccessPolicy::ReadOnly,
            attempt: 1,
            latest_result_report_id: None,
            failure: None,
            revision: 0,
            created_at: now.clone(),
            started_at: None,
            completed_at: None,
            updated_at: now.clone(),
        };
        let node = session
            .nodes
            .iter_mut()
            .find(|node| node.id == node_id)
            .expect("assigned child node");
        node.assigned_task_id = Some(task_id.clone());
        node.execution_status = ExecutionStatus::Starting;
        node.last_activity_at = Some(now.clone());
        let execution_status = node.execution_status;
        session.tasks.push(task);
        session.idempotency_records.push(IdempotencyRecord {
            actor_key: generation_id.into(),
            operation: "createChildTask".into(),
            request_id: request.request_id,
            payload_fingerprint: fingerprint,
            result_ref: format!("{task_id}|{node_id}"),
            created_at: now.clone(),
        });
        session.revision += 1;
        session.updated_at = now;
        let snapshot = session.clone();
        self.repository.save_sessions(&sessions)?;
        self.emit_workspace_changed(&snapshot, "childTaskCreated");
        Ok(CreateChildTaskOutcome {
            task_id,
            node_id,
            status,
            execution_status,
        })
    }

    pub fn delegate_goal(
        &self,
        window_label: &str,
        request: DelegateGoalRequest,
    ) -> Result<DelegateGoalOutcome, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        let generation_id = session
            .active_coordinator_generation_id
            .clone()
            .ok_or_else(|| {
                OrchestrationError::new(
                    OrchestrationErrorCode::CoordinatorInactive,
                    "활성 Main Coordinator 실행이 없습니다. Main 실행을 시작한 뒤 다시 위임하세요.",
                )
            })?;
        if request.goal.trim().is_empty()
            || request.goal.as_bytes().len() > crate::domain::agent_orchestration::MAX_PROMPT_BYTES
        {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidInput,
                "Delegation goal must be between 1 and 16384 UTF-8 bytes.",
            ));
        }
        let fingerprint = request.goal.trim().to_string();
        if let Some(record) = session.idempotency_records.iter().find(|record| {
            record.actor_key == window_label
                && record.operation == "delegateGoal"
                && record.request_id == request.request_id
        }) {
            if record.payload_fingerprint != fingerprint {
                return Err(OrchestrationError::new(
                    OrchestrationErrorCode::DuplicateConflict,
                    "Request id was already used with another goal.",
                ));
            }
            let parts: Vec<_> = record.result_ref.split('|').collect();
            if parts.len() == 3 {
                return Ok(DelegateGoalOutcome {
                    root_task_id: parts[0].into(),
                    generation_id: parts[1].into(),
                    dispatch_id: parts[2].into(),
                    status: "accepted".into(),
                });
            }
        }
        if session.revision != request.expected_revision {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::RevisionConflict,
                "Delegation used a stale workspace revision.",
            )
            .retryable());
        }
        let now = now();
        let root_task_id = Uuid::new_v4().to_string();
        let dispatch_id = Uuid::new_v4().to_string();
        let target_request_id = Uuid::new_v4().to_string();
        let main_run_id = session
            .nodes
            .iter()
            .find(|node| node.id == MAIN_AGENT_NODE_ID)
            .and_then(|node| node.current_run_id.clone())
            .ok_or_else(|| {
                OrchestrationError::new(
                    OrchestrationErrorCode::CoordinatorInactive,
                    "Main Coordinator 실행이 연결되어 있지 않습니다. Main 실행을 시작한 뒤 다시 위임하세요.",
                )
            })?;
        session.tasks.push(OrchestrationTask {
            id: root_task_id.clone(),
            parent_task_id: None,
            coordinator_generation_id: generation_id.clone(),
            assigned_node_id: Some(MAIN_AGENT_NODE_ID.into()),
            title: request.goal.trim().chars().take(120).collect(),
            objective: request.goal.trim().into(),
            constraints: vec!["same-worktree".into()],
            expected_result: "하위 에이전트 결과의 출처와 충돌을 구분한 종합 응답".into(),
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
        });
        session.dispatches.push(PromptDispatch {
            id: dispatch_id.clone(),
            intent: PromptDispatchIntent::Delegate,
            target_mode: PromptTargetMode::Coordinator,
            message: request.goal.trim().into(),
            delivery: PromptDelivery::Send,
            targets: vec![PromptDispatchTarget {
                panel_id: MAIN_AGENT_NODE_ID.into(),
                run_id: Some(main_run_id),
                request_id: target_request_id,
                status: PromptDispatchTargetStatus::Accepted,
                failure_code: None,
                failure_reason: None,
            }],
            created_by: "user".into(),
            created_at: now.clone(),
            updated_at: now.clone(),
        });
        session.idempotency_records.push(IdempotencyRecord {
            actor_key: window_label.into(),
            operation: "delegateGoal".into(),
            request_id: request.request_id,
            payload_fingerprint: fingerprint,
            result_ref: format!("{root_task_id}|{generation_id}|{dispatch_id}"),
            created_at: now.clone(),
        });
        session.revision += 1;
        session.updated_at = now;
        let snapshot = session.clone();
        self.repository.save_sessions(&sessions)?;
        self.emit_workspace_changed(&snapshot, "goalDelegated");
        Ok(DelegateGoalOutcome {
            root_task_id,
            generation_id,
            dispatch_id,
            status: "accepted".into(),
        })
    }

    pub fn adopt_manual_child(
        &self,
        window_label: &str,
        panel_id: &str,
        title: &str,
    ) -> Result<OrchestrationSession, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        if let Some(existing) = session.nodes.iter().find(|node| node.id == panel_id) {
            if existing.kind == crate::domain::agent_orchestration::AgentNodeKind::Child {
                return Ok(session.clone());
            }
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidTopology,
                "The panel id is reserved by Main.",
            ));
        }
        if session.nodes.len() >= MAX_ORCHESTRATION_NODES {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::CapacityExceeded,
                "The workspace already has the maximum number of agent nodes.",
            ));
        }
        let now = now();
        let mut node = AgentNode::child(
            panel_id,
            MAIN_AGENT_NODE_ID,
            AgentRoleProfile::new(
                format!("manual-{panel_id}"),
                title,
                "사용자가 만든 직접 하위 에이전트",
                "할당된 작업의 구조화 결과",
            )?,
            now.clone(),
        )?;
        node.created_by = crate::domain::agent_orchestration::AgentNodeCreator::User;
        node.presentation_status = PresentationStatus::Panel;
        session.nodes.push(node);
        session.revision += 1;
        session.updated_at = now;
        let snapshot = session.clone();
        self.repository.save_sessions(&sessions)?;
        self.emit_workspace_changed(&snapshot, "manualChildAdopted");
        Ok(snapshot)
    }

    pub fn bind_child_run(
        &self,
        window_label: &str,
        task_id: &str,
        node_id: &str,
        run_id: &str,
    ) -> Result<OrchestrationSession, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        let now = now();
        let task = session
            .tasks
            .iter_mut()
            .find(|task| task.id == task_id && task.assigned_node_id.as_deref() == Some(node_id))
            .ok_or_else(|| {
                OrchestrationError::new(
                    OrchestrationErrorCode::NotFound,
                    "Assigned orchestration task was not found.",
                )
            })?;
        if task.status == TaskStatus::Ready {
            task.transition(TaskStatus::Running, now.clone())?;
        }
        let node = session
            .nodes
            .iter_mut()
            .find(|node| node.id == node_id)
            .ok_or_else(|| {
                OrchestrationError::new(
                    OrchestrationErrorCode::NotFound,
                    "Assigned child node was not found.",
                )
            })?;
        node.current_run_id = Some(run_id.into());
        node.execution_status = ExecutionStatus::Active;
        node.last_activity_at = Some(now.clone());
        session.revision += 1;
        session.updated_at = now;
        let snapshot = session.clone();
        self.repository.save_sessions(&sessions)?;
        self.emit_workspace_changed(&snapshot, "childRunBound");
        Ok(snapshot)
    }

    pub fn report_task(
        &self,
        window_label: &str,
        request: ReportTaskRequest,
    ) -> Result<TaskReport, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        if let Some(existing) = session
            .reports
            .iter()
            .find(|report| report.request_id == request.request_id)
        {
            let existing_payload = ReportTaskRequest {
                request_id: existing.request_id.clone(),
                task_id: existing.task_id.clone(),
                reporter_node_id: existing.reporter_node_id.clone(),
                reporter_run_id: existing.reporter_run_id.clone(),
                report_type: existing.report_type,
                progress_percent: existing.progress_percent,
                summary: existing.summary.clone(),
                findings: existing.findings.clone(),
                artifact_refs: existing.artifact_refs.clone(),
                unresolved: existing.unresolved.clone(),
                confidence: existing.confidence,
            };
            if full_payload_fingerprint(&existing_payload)? == full_payload_fingerprint(&request)? {
                return Ok(existing.clone());
            }
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::DuplicateConflict,
                "Report request id was already used with another payload.",
            ));
        }
        if request.summary.trim().is_empty()
            || request
                .progress_percent
                .is_some_and(|progress| progress > 100)
            || request
                .confidence
                .is_some_and(|confidence| !(0.0..=1.0).contains(&confidence))
        {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidInput,
                "Task report is invalid.",
            ));
        }
        // FR-047: reject only the offending artifact reference and keep the report body.
        // Rejections are recorded in `unresolved` so the violation stays visible.
        let mut artifact_refs = Vec::with_capacity(request.artifact_refs.len());
        let mut rejected_artifacts = Vec::new();
        for artifact in request.artifact_refs {
            match artifact.validate_for_workspace(&session.worktree_path) {
                Ok(()) => artifact_refs.push(artifact),
                Err(error) => rejected_artifacts.push(format!(
                    "거부된 산출물 참조 {}: {}",
                    artifact.uri, error.message
                )),
            }
        }
        let task_index = session
            .tasks
            .iter()
            .position(|task| {
                task.id == request.task_id
                    && task.assigned_node_id.as_deref() == Some(request.reporter_node_id.as_str())
            })
            .ok_or_else(|| {
                OrchestrationError::new(
                    OrchestrationErrorCode::Unauthorized,
                    "The caller is not assigned to this task.",
                )
            })?;
        let node_index = session
            .nodes
            .iter()
            .position(|node| node.id == request.reporter_node_id)
            .ok_or_else(|| {
                OrchestrationError::new(
                    OrchestrationErrorCode::Unauthorized,
                    "The caller node is not assigned to this workspace.",
                )
            })?;
        // MCP authenticates the concrete run capability before this use case is
        // reached. A late report from a revoked/previous run is retained as
        // evidence, but it must never advance the current attempt or notify the
        // successor Main generation.
        let is_current_run = session.nodes[node_index].current_run_id.as_deref()
            == Some(request.reporter_run_id.as_str());
        let now = now();
        let report = TaskReport {
            id: Uuid::new_v4().to_string(),
            request_id: request.request_id,
            task_id: request.task_id,
            reporter_node_id: request.reporter_node_id,
            reporter_run_id: request.reporter_run_id,
            report_type: request.report_type,
            progress_percent: request.progress_percent,
            summary: request.summary.trim().into(),
            findings: request.findings,
            artifact_refs,
            unresolved: request
                .unresolved
                .into_iter()
                .chain(rejected_artifacts)
                .collect(),
            confidence: request.confidence,
            created_at: now.clone(),
        };
        if is_current_run && !session.tasks[task_index].status.is_terminal() {
            match report.report_type {
                TaskReportType::Result => {
                    session.tasks[task_index].latest_result_report_id = Some(report.id.clone());
                    session.tasks[task_index].transition(TaskStatus::Completed, now.clone())?;
                    session.nodes[node_index].execution_status = ExecutionStatus::Idle;
                }
                TaskReportType::InputRequest => {
                    session.tasks[task_index].transition(TaskStatus::InputRequired, now.clone())?;
                    session.nodes[node_index].presentation_status =
                        PresentationStatus::AttentionRequired;
                }
                TaskReportType::Blocked => {
                    session.tasks[task_index].transition(TaskStatus::Blocked, now.clone())?;
                    session.nodes[node_index].presentation_status =
                        PresentationStatus::AttentionRequired;
                }
                TaskReportType::Progress | TaskReportType::Message => {}
            }
        }
        if is_current_run {
            session.nodes[node_index].last_activity_at = Some(now.clone());
        }
        session.reports.push(report.clone());
        if is_current_run {
            let generation_id = session.tasks[task_index].coordinator_generation_id.clone();
            let main_run_id = (session.active_coordinator_generation_id.as_deref()
                == Some(generation_id.as_str()))
            .then(|| {
                session
                    .nodes
                    .iter()
                    .find(|node| node.id == MAIN_AGENT_NODE_ID)
                    .and_then(|node| node.current_run_id.clone())
            })
            .flatten();
            session
                .coordinator_notifications
                .push(CoordinatorNotification {
                    id: Uuid::new_v4().to_string(),
                    report_id: report.id.clone(),
                    task_id: report.task_id.clone(),
                    report_type: report.report_type,
                    generation_id,
                    main_run_id,
                    status: CoordinatorNotificationStatus::Pending,
                    attempt_count: 0,
                    failure: None,
                    collected_at: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                });
        }
        session.revision += 1;
        session.updated_at = now;
        let snapshot = session.clone();
        self.repository.save_sessions(&sessions)?;
        self.emit_workspace_changed(&snapshot, "taskReport");
        Ok(report)
    }

    pub fn list_child_tasks(
        &self,
        window_label: &str,
        generation_id: &str,
    ) -> Result<Vec<OrchestrationTask>, OrchestrationError> {
        let session = self.get_for_window(window_label)?.ok_or_else(|| {
            OrchestrationError::new(
                OrchestrationErrorCode::NotFound,
                "Orchestration workspace is not bootstrapped.",
            )
        })?;
        Ok(session
            .tasks
            .into_iter()
            .filter(|task| task.coordinator_generation_id == generation_id)
            .collect())
    }

    pub fn collect_child_results(
        &self,
        window_label: &str,
        generation_id: &str,
        task_ids: &[String],
    ) -> Result<Vec<TaskReport>, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        if session.active_coordinator_generation_id.as_deref() != Some(generation_id) {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::Unauthorized,
                "Only the active Main Coordinator can collect child results.",
            ));
        }
        let generation_task_ids = session
            .tasks
            .iter()
            .filter(|task| {
                task.coordinator_generation_id == generation_id
                    && (task_ids.is_empty() || task_ids.contains(&task.id))
            })
            .map(|task| task.id.clone())
            .collect::<std::collections::HashSet<_>>();
        if !task_ids
            .iter()
            .all(|task_id| generation_task_ids.contains(task_id))
        {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::NotFound,
                "Every requested task must belong to the active Coordinator generation.",
            ));
        }
        let reports = session
            .reports
            .iter()
            .filter(|report| generation_task_ids.contains(&report.task_id))
            .cloned()
            .collect::<Vec<_>>();
        let report_ids = reports
            .iter()
            .map(|report| report.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let mut changed = false;
        let collected_at = now();
        for notification in &mut session.coordinator_notifications {
            if notification.generation_id == generation_id
                && report_ids.contains(notification.report_id.as_str())
            {
                if notification.collected_at.is_none() {
                    notification.collected_at = Some(collected_at.clone());
                    notification.updated_at = collected_at.clone();
                    changed = true;
                }
                if notification.status == CoordinatorNotificationStatus::Delivered {
                    notification.transition(
                        CoordinatorNotificationStatus::Processed,
                        collected_at.clone(),
                    )?;
                    notification.failure = None;
                    changed = true;
                }
            }
        }
        if changed {
            session.revision += 1;
            session.updated_at = collected_at;
            let snapshot = session.clone();
            self.repository.save_sessions(&sessions)?;
            self.emit_workspace_changed(&snapshot, "coordinatorResultsCollected");
        }
        Ok(reports)
    }

    pub fn set_presentation(
        &self,
        window_label: &str,
        request: SetPresentationRequest,
    ) -> Result<OrchestrationSession, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        if session.revision != request.expected_revision {
            return Err(revision_conflict());
        }
        if !matches!(
            request.presentation_status,
            PresentationStatus::Panel
                | PresentationStatus::Background
                | PresentationStatus::Detached
        ) {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidTransition,
                "Only panel, background, and detached presentation states can be selected.",
            ));
        }
        let fingerprint = format!("{}:{:?}", request.node_id, request.presentation_status);
        if let Some(record) = session.idempotency_records.iter().find(|record| {
            record.actor_key == window_label
                && record.operation == "setPresentation"
                && record.request_id == request.request_id
        }) {
            if record.payload_fingerprint == fingerprint {
                return Ok(session.clone());
            }
            return Err(duplicate_conflict());
        }
        let node = session
            .nodes
            .iter_mut()
            .find(|node| node.id == request.node_id && node.id != MAIN_AGENT_NODE_ID)
            .ok_or_else(|| {
                OrchestrationError::new(
                    OrchestrationErrorCode::NotFound,
                    "A direct child node was not found.",
                )
            })?;
        node.presentation_status = request.presentation_status;
        let now = now();
        node.last_activity_at = Some(now.clone());
        session.idempotency_records.push(IdempotencyRecord {
            actor_key: window_label.into(),
            operation: "setPresentation".into(),
            request_id: request.request_id,
            payload_fingerprint: fingerprint,
            result_ref: request.node_id,
            created_at: now.clone(),
        });
        session.revision += 1;
        session.updated_at = now;
        let snapshot = session.clone();
        self.repository.save_sessions(&sessions)?;
        self.emit_workspace_changed(&snapshot, "presentationChanged");
        Ok(snapshot)
    }

    pub fn record_prompt_dispatch(
        &self,
        window_label: &str,
        request: DispatchPromptRequest,
    ) -> Result<PromptDispatch, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        let message = request.message.trim();
        if message.is_empty()
            || message.as_bytes().len() > crate::domain::agent_orchestration::MAX_PROMPT_BYTES
        {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidInput,
                "Prompt must be between 1 and 16384 UTF-8 bytes.",
            ));
        }
        if request.panel_ids.is_empty() {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidInput,
                "At least one prompt target is required.",
            ));
        }
        let fingerprint = serde_json::to_string(&request).map_err(|error| {
            OrchestrationError::new(
                OrchestrationErrorCode::InvalidInput,
                format!("Failed to normalize prompt dispatch: {error}"),
            )
        })?;
        if let Some(record) = session.idempotency_records.iter().find(|record| {
            record.actor_key == window_label
                && record.operation == "dispatchPrompt"
                && record.request_id == request.request_id
        }) {
            if record.payload_fingerprint != fingerprint {
                return Err(duplicate_conflict());
            }
            return session
                .dispatches
                .iter()
                .find(|dispatch| dispatch.id == record.result_ref)
                .cloned()
                .ok_or_else(|| not_found("Prompt dispatch"));
        }
        ensure_revision(session, request.expected_revision)?;
        let mut seen = std::collections::HashSet::new();
        let targets = request
            .panel_ids
            .iter()
            .map(|panel_id| {
                if !seen.insert(panel_id) {
                    return Err(OrchestrationError::new(
                        OrchestrationErrorCode::InvalidInput,
                        "Prompt targets must be unique.",
                    ));
                }
                let node = session
                    .nodes
                    .iter()
                    .find(|node| node.id == *panel_id)
                    .ok_or_else(|| not_found("Prompt target node"))?;
                Ok(PromptDispatchTarget {
                    panel_id: panel_id.clone(),
                    run_id: node.current_run_id.clone(),
                    request_id: format!("{}:{panel_id}", request.request_id),
                    status: if node.current_run_id.is_some() {
                        PromptDispatchTargetStatus::Accepted
                    } else {
                        PromptDispatchTargetStatus::Failed
                    },
                    failure_code: node
                        .current_run_id
                        .is_none()
                        .then(|| "workerUnavailable".into()),
                    failure_reason: node
                        .current_run_id
                        .is_none()
                        .then(|| "Target run is unavailable.".into()),
                })
            })
            .collect::<Result<Vec<_>, OrchestrationError>>()?;
        let now = now();
        let dispatch = PromptDispatch {
            id: Uuid::new_v4().to_string(),
            intent: request.intent,
            target_mode: request.target_mode,
            message: message.into(),
            delivery: request.delivery,
            targets,
            created_by: "user".into(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        session.idempotency_records.push(IdempotencyRecord {
            actor_key: window_label.into(),
            operation: "dispatchPrompt".into(),
            request_id: request.request_id,
            payload_fingerprint: fingerprint,
            result_ref: dispatch.id.clone(),
            created_at: now.clone(),
        });
        session.dispatches.push(dispatch.clone());
        session.revision += 1;
        session.updated_at = now;
        let snapshot = session.clone();
        self.repository.save_sessions(&sessions)?;
        self.emit_workspace_changed(&snapshot, "promptDispatched");
        Ok(dispatch)
    }

    pub fn update_prompt_dispatch_target(
        &self,
        window_label: &str,
        dispatch_id: &str,
        request_id: &str,
        status: PromptDispatchTargetStatus,
        failure: Option<(String, String)>,
    ) -> Result<PromptDispatch, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        let dispatch = session
            .dispatches
            .iter_mut()
            .find(|dispatch| dispatch.id == dispatch_id)
            .ok_or_else(|| not_found("Prompt dispatch"))?;
        let target = dispatch
            .targets
            .iter_mut()
            .find(|target| target.request_id == request_id)
            .ok_or_else(|| not_found("Prompt dispatch target"))?;
        if matches!(
            target.status,
            PromptDispatchTargetStatus::Delivered
                | PromptDispatchTargetStatus::Rejected
                | PromptDispatchTargetStatus::Failed
                | PromptDispatchTargetStatus::Cancelled
        ) {
            return Ok(dispatch.clone());
        }
        target.status = status;
        target.failure_code = failure.as_ref().map(|failure| failure.0.clone());
        target.failure_reason = failure.map(|failure| failure.1);
        dispatch.updated_at = now();
        let result = dispatch.clone();
        session.revision += 1;
        session.updated_at = now();
        let snapshot = session.clone();
        self.repository.save_sessions(&sessions)?;
        self.emit_workspace_changed(&snapshot, "promptDispatchTargetUpdated");
        Ok(result)
    }

    pub fn respond_to_input(
        &self,
        window_label: &str,
        request: TaskActionRequest,
    ) -> Result<OrchestrationSession, OrchestrationError> {
        let response = request
            .message
            .as_deref()
            .map(str::trim)
            .filter(|message| !message.is_empty())
            .ok_or_else(|| {
                OrchestrationError::new(
                    OrchestrationErrorCode::InvalidInput,
                    "An input response is required.",
                )
            })?;
        self.mutate_task(window_label, &request, "respondInput", |task, node, now| {
            if task.status != TaskStatus::InputRequired {
                return Err(OrchestrationError::new(
                    OrchestrationErrorCode::InvalidTransition,
                    "Only a task waiting for input can receive a response.",
                ));
            }
            task.transition(TaskStatus::Running, now.to_owned())?;
            node.presentation_status = PresentationStatus::Background;
            node.last_activity_at = Some(now.to_owned());
            task.failure = None;
            let _ = response;
            Ok(())
        })
    }

    pub fn cancel_task(
        &self,
        window_label: &str,
        request: TaskActionRequest,
    ) -> Result<OrchestrationSession, OrchestrationError> {
        self.mutate_task(window_label, &request, "cancelTask", |task, node, now| {
            if task.status.is_terminal() {
                return Ok(());
            }
            task.transition(TaskStatus::Cancelled, now.to_owned())?;
            node.execution_status = ExecutionStatus::Stopped;
            node.last_activity_at = Some(now.to_owned());
            Ok(())
        })
    }

    pub fn retry_task(
        &self,
        window_label: &str,
        request: TaskActionRequest,
    ) -> Result<OrchestrationSession, OrchestrationError> {
        self.mutate_task(window_label, &request, "retryTask", |task, node, now| {
            if !matches!(task.status, TaskStatus::Failed | TaskStatus::Blocked) {
                return Err(OrchestrationError::new(
                    OrchestrationErrorCode::InvalidTransition,
                    "Only failed or blocked tasks can be retried.",
                ));
            }
            task.transition(TaskStatus::Ready, now.to_owned())?;
            task.attempt += 1;
            task.failure = None;
            task.latest_result_report_id = None;
            node.current_run_id = None;
            node.execution_status = ExecutionStatus::Starting;
            node.presentation_status = PresentationStatus::Background;
            Ok(())
        })
    }

    pub fn reassign_task(
        &self,
        window_label: &str,
        request: TaskActionRequest,
    ) -> Result<OrchestrationSession, OrchestrationError> {
        let target_node_id = request.target_node_id.clone().ok_or_else(|| {
            OrchestrationError::new(
                OrchestrationErrorCode::InvalidInput,
                "A target child node is required.",
            )
        })?;
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        ensure_revision(session, request.expected_revision)?;
        let task_index = session
            .tasks
            .iter()
            .position(|task| task.id == request.task_id)
            .ok_or_else(|| not_found("Task"))?;
        let old_node_id = session.tasks[task_index].assigned_node_id.clone();
        if !matches!(
            session.tasks[task_index].status,
            TaskStatus::Failed | TaskStatus::Blocked | TaskStatus::Ready
        ) {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidTransition,
                "Only ready, blocked, or failed tasks can be reassigned.",
            ));
        }
        let target = session
            .nodes
            .iter_mut()
            .find(|node| {
                node.id == target_node_id
                    && node.parent_node_id.as_deref() == Some(MAIN_AGENT_NODE_ID)
                    && (node.assigned_task_id.is_none()
                        || node.assigned_task_id.as_deref() == Some(request.task_id.as_str()))
            })
            .ok_or_else(|| not_found("Idle target child node"))?;
        target.assigned_task_id = Some(request.task_id.clone());
        target.execution_status = ExecutionStatus::Starting;
        target.presentation_status = PresentationStatus::Background;
        if let Some(old_id) = old_node_id
            && old_id != target_node_id
            && let Some(old) = session.nodes.iter_mut().find(|node| node.id == old_id)
        {
            old.assigned_task_id = None;
            old.current_run_id = None;
            old.execution_status = ExecutionStatus::Unassigned;
        }
        let task = &mut session.tasks[task_index];
        task.assigned_node_id = Some(target_node_id);
        if task.status != TaskStatus::Ready {
            task.transition(TaskStatus::Ready, now())?;
        }
        task.attempt += 1;
        task.failure = None;
        persist_mutation(
            &self.repository,
            &self.event_sink,
            &mut sessions,
            window_label,
            "taskReassigned",
        )
    }

    pub fn handoff_coordinator(
        &self,
        window_label: &str,
        request: CoordinatorHandoffRequest,
    ) -> Result<OrchestrationSession, OrchestrationError> {
        if !request.confirmed {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::Unauthorized,
                "Coordinator handoff requires explicit confirmation.",
            ));
        }
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        ensure_revision(session, request.expected_revision)?;
        let previous_id = session
            .active_coordinator_generation_id
            .clone()
            .ok_or_else(|| not_found("Active Coordinator generation"))?;
        let now = now();
        let successor_id = Uuid::new_v4().to_string();
        let previous = session
            .generations
            .iter_mut()
            .find(|generation| generation.id == previous_id)
            .ok_or_else(|| not_found("Active Coordinator generation"))?;
        previous.status = CoordinatorGenerationStatus::Superseded;
        previous.ended_at = Some(now.clone());
        previous.handoff_summary = Some(request.summary.clone());
        previous.successor_generation_id = Some(successor_id.clone());
        session.generations.push(CoordinatorGeneration {
            id: successor_id.clone(),
            ordinal: session.generations.len() as u32 + 1,
            main_node_id: MAIN_AGENT_NODE_ID.into(),
            run_id: request.successor_run_id.clone(),
            previous_generation_id: Some(previous_id.clone()),
            status: CoordinatorGenerationStatus::Active,
            started_at: now.clone(),
            ended_at: None,
            handoff_summary: Some(request.summary),
            successor_generation_id: None,
        });
        for task in session.tasks.iter_mut().filter(|task| {
            task.coordinator_generation_id == previous_id && !task.status.is_terminal()
        }) {
            task.coordinator_generation_id = successor_id.clone();
            task.awaiting_handoff = false;
            task.revision += 1;
            task.updated_at = now.clone();
        }
        session.active_coordinator_generation_id = Some(successor_id);
        if let Some(main) = session
            .nodes
            .iter_mut()
            .find(|node| node.id == MAIN_AGENT_NODE_ID)
        {
            main.current_run_id = Some(request.successor_run_id);
            main.execution_status = ExecutionStatus::Active;
        }
        persist_mutation(
            &self.repository,
            &self.event_sink,
            &mut sessions,
            window_label,
            "coordinatorHandoff",
        )
    }

    pub fn reconcile_runtime(
        &self,
        window_label: &str,
        live_run_ids: &[String],
    ) -> Result<OrchestrationSession, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        reconcile_session_runtime(session, live_run_ids);
        persist_mutation(
            &self.repository,
            &self.event_sink,
            &mut sessions,
            window_label,
            "runtimeReconciled",
        )
    }

    pub fn fail_task_for_runtime(
        &self,
        window_label: &str,
        task_id: &str,
        node_id: &str,
        code: OrchestrationErrorCode,
        message: &str,
    ) -> Result<OrchestrationSession, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        let now = now();
        let task = session
            .tasks
            .iter_mut()
            .find(|task| task.id == task_id && task.assigned_node_id.as_deref() == Some(node_id))
            .ok_or_else(|| not_found("Assigned task"))?;
        task.status = TaskStatus::Failed;
        task.completed_at = None;
        task.revision += 1;
        task.updated_at = now.clone();
        task.failure = Some(crate::domain::agent_orchestration::TaskFailure {
            code,
            message: message.into(),
            retryable: true,
            partial_result_report_ids: session
                .reports
                .iter()
                .filter(|report| report.task_id == task_id)
                .map(|report| report.id.clone())
                .collect(),
        });
        if let Some(node) = session.nodes.iter_mut().find(|node| node.id == node_id) {
            node.execution_status = ExecutionStatus::Stopped;
            node.presentation_status = PresentationStatus::AttentionRequired;
            node.last_activity_at = Some(now);
        }
        persist_mutation(
            &self.repository,
            &self.event_sink,
            &mut sessions,
            window_label,
            "runtimePolicyViolation",
        )
    }

    fn mutate_task<F>(
        &self,
        window_label: &str,
        request: &TaskActionRequest,
        operation: &str,
        mutate: F,
    ) -> Result<OrchestrationSession, OrchestrationError>
    where
        F: FnOnce(&mut OrchestrationTask, &mut AgentNode, &str) -> Result<(), OrchestrationError>,
    {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        ensure_revision(session, request.expected_revision)?;
        if let Some(record) = session.idempotency_records.iter().find(|record| {
            record.actor_key == window_label
                && record.operation == operation
                && record.request_id == request.request_id
        }) {
            if record.result_ref == request.task_id {
                return Ok(session.clone());
            }
            return Err(duplicate_conflict());
        }
        let task_index = session
            .tasks
            .iter()
            .position(|task| task.id == request.task_id)
            .ok_or_else(|| not_found("Task"))?;
        let node_id = session.tasks[task_index]
            .assigned_node_id
            .clone()
            .ok_or_else(|| not_found("Assigned child node"))?;
        let node_index = session
            .nodes
            .iter()
            .position(|node| node.id == node_id)
            .ok_or_else(|| not_found("Assigned child node"))?;
        let now = now();
        mutate(
            &mut session.tasks[task_index],
            &mut session.nodes[node_index],
            &now,
        )?;
        session.idempotency_records.push(IdempotencyRecord {
            actor_key: window_label.into(),
            operation: operation.into(),
            request_id: request.request_id.clone(),
            payload_fingerprint: serde_json::to_string(request).unwrap_or_default(),
            result_ref: request.task_id.clone(),
            created_at: now,
        });
        persist_mutation(
            &self.repository,
            &self.event_sink,
            &mut sessions,
            window_label,
            operation,
        )
    }

    fn emit_workspace_changed(&self, session: &OrchestrationSession, reason: &str) {
        let Some(window_label) = session.bound_window_label.as_deref() else {
            return;
        };
        let _ = self.event_sink.emit(
            window_label,
            OrchestrationEvent {
                workspace_id: session.id.clone(),
                revision: session.revision,
                reason: reason.into(),
                task_id: None,
                node_id: None,
            },
        );
    }
}

fn reconcile_session_runtime(session: &mut OrchestrationSession, live_run_ids: &[String]) {
    let now = now();
    for node in &mut session.nodes {
        let Some(run_id) = node.current_run_id.as_ref() else {
            continue;
        };
        if node.execution_status == ExecutionStatus::Active && !live_run_ids.contains(run_id) {
            node.execution_status = ExecutionStatus::Stopped;
            node.last_activity_at = Some(now.clone());
            if node.kind == crate::domain::agent_orchestration::AgentNodeKind::Child {
                node.presentation_status = PresentationStatus::AttentionRequired;
                if let Some(task_id) = node.assigned_task_id.as_ref()
                    && let Some(task) = session.tasks.iter_mut().find(|task| task.id == *task_id)
                    && !task.status.is_terminal()
                {
                    task.status = TaskStatus::Blocked;
                    task.failure = Some(crate::domain::agent_orchestration::TaskFailure {
                        code: OrchestrationErrorCode::RuntimeLost,
                        message: "The worker runtime was lost and can be retried.".into(),
                        retryable: true,
                        partial_result_report_ids: session
                            .reports
                            .iter()
                            .filter(|report| report.task_id == *task_id)
                            .map(|report| report.id.clone())
                            .collect(),
                    });
                    task.revision += 1;
                    task.updated_at = now.clone();
                }
            }
        }
    }
}

fn session_for_window_mut<'a>(
    sessions: &'a mut [OrchestrationSession],
    window_label: &str,
) -> Result<&'a mut OrchestrationSession, OrchestrationError> {
    let session = sessions
        .iter_mut()
        .find(|session| session.bound_window_label.as_deref() == Some(window_label))
        .ok_or_else(|| {
            OrchestrationError::new(
                OrchestrationErrorCode::NotFound,
                "Orchestration workspace is not bootstrapped.",
            )
        })?;
    session.assert_scope(window_label)?;
    Ok(session)
}

fn validate_task_input(
    title: &str,
    objective: &str,
    expected_result: &str,
) -> Result<(), OrchestrationError> {
    if title.trim().is_empty()
        || title.chars().count() > 120
        || objective.trim().is_empty()
        || expected_result.trim().is_empty()
    {
        return Err(OrchestrationError::new(
            OrchestrationErrorCode::InvalidInput,
            "Task title, objective and expected result are required.",
        ));
    }
    Ok(())
}

fn ensure_revision(
    session: &OrchestrationSession,
    expected_revision: u64,
) -> Result<(), OrchestrationError> {
    if session.revision == expected_revision {
        Ok(())
    } else {
        Err(revision_conflict())
    }
}

fn revision_conflict() -> OrchestrationError {
    OrchestrationError::new(
        OrchestrationErrorCode::RevisionConflict,
        "The workspace revision is stale.",
    )
    .retryable()
}

fn duplicate_conflict() -> OrchestrationError {
    OrchestrationError::new(
        OrchestrationErrorCode::DuplicateConflict,
        "Request id was already used with another payload.",
    )
}

fn not_found(subject: &str) -> OrchestrationError {
    OrchestrationError::new(
        OrchestrationErrorCode::NotFound,
        format!("{subject} was not found."),
    )
}

fn persist_mutation<R, E>(
    repository: &R,
    event_sink: &E,
    sessions: &mut [OrchestrationSession],
    window_label: &str,
    reason: &str,
) -> Result<OrchestrationSession, OrchestrationError>
where
    R: OrchestrationRepository,
    E: OrchestrationEventSink,
{
    let session = session_for_window_mut(sessions, window_label)?;
    session.revision += 1;
    session.updated_at = now();
    let snapshot = session.clone();
    repository.save_sessions(sessions)?;
    let _ = event_sink.emit(
        window_label,
        OrchestrationEvent {
            workspace_id: snapshot.id.clone(),
            revision: snapshot.revision,
            reason: reason.into(),
            task_id: None,
            node_id: None,
        },
    );
    Ok(snapshot)
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::{
        domain::agent_orchestration::{
            ArtifactKind, MAIN_AGENT_NODE_ID, OrchestrationError, OrchestrationErrorCode,
            OrchestrationSession, TaskReportType, TaskStatus,
        },
        ports::{
            orchestration_event_sink::{OrchestrationEvent, OrchestrationEventSink},
            orchestration_repository::OrchestrationRepository,
        },
    };

    #[derive(Clone, Default)]
    struct MemoryRepository(Arc<Mutex<Vec<OrchestrationSession>>>);

    impl OrchestrationRepository for MemoryRepository {
        fn load_sessions(&self) -> Result<Vec<OrchestrationSession>, OrchestrationError> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn save_sessions(
            &self,
            sessions: &[OrchestrationSession],
        ) -> Result<(), OrchestrationError> {
            *self.0.lock().unwrap() = sessions.to_vec();
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct RecordingSink(Arc<Mutex<Vec<(String, OrchestrationEvent)>>>);

    impl OrchestrationEventSink for RecordingSink {
        fn emit(
            &self,
            window_label: &str,
            event: OrchestrationEvent,
        ) -> Result<(), OrchestrationError> {
            self.0
                .lock()
                .unwrap()
                .push((window_label.to_string(), event));
            Ok(())
        }
    }

    fn service_with_running_child() -> (
        OrchestrationService<MemoryRepository, RecordingSink>,
        OrchestrationSession,
        CreateChildTaskOutcome,
    ) {
        let service =
            OrchestrationService::new(MemoryRepository::default(), RecordingSink::default());
        let workspace = service.bootstrap("/repo", "window-1", None).unwrap();
        let workspace = service
            .bind_main_run(
                "window-1",
                BindMainRunRequest {
                    request_id: "bind-helper".into(),
                    panel_id: MAIN_AGENT_NODE_ID.into(),
                    run_id: "run-main".into(),
                    state: MainRunBindingState::Active,
                    expected_revision: workspace.revision,
                },
            )
            .unwrap();
        let created = service
            .create_child_task(
                "window-1",
                workspace
                    .active_coordinator_generation_id
                    .as_deref()
                    .unwrap(),
                CreateChildTaskRequest {
                    request_id: "create-helper".into(),
                    title: "조사".into(),
                    role: AgentRoleProfile::new("researcher", "Researcher", "조사", "결과")
                        .unwrap(),
                    objective: "구조를 조사한다.".into(),
                    constraints: vec!["read-only".into()],
                    expected_result: "구조화 결과".into(),
                    dependency_task_ids: vec![],
                    preferred_node_id: None,
                },
            )
            .unwrap();
        let workspace = service
            .bind_child_run("window-1", &created.task_id, &created.node_id, "run-child")
            .unwrap();
        (service, workspace, created)
    }

    /// FR-022: delegating without a Main run must be rejected with a reason the UI can
    /// tell apart from "Main is busy", must not create a task, and must not be retryable.
    #[test]
    fn rejects_delegation_without_active_main_run_using_a_distinct_reason() {
        let repository = MemoryRepository::default();
        let service = OrchestrationService::new(repository.clone(), RecordingSink::default());
        let workspace = service.bootstrap("/repo", "window-1", None).unwrap();

        let error = service
            .delegate_goal(
                "window-1",
                DelegateGoalRequest {
                    request_id: "delegate-no-main".into(),
                    goal: "세 역할로 조사한다.".into(),
                    expected_revision: workspace.revision,
                },
            )
            .expect_err("delegation must fail without an active Main run");

        assert_eq!(error.code, OrchestrationErrorCode::CoordinatorInactive);
        assert_ne!(error.code, OrchestrationErrorCode::CoordinatorBusy);
        assert!(
            !error.retryable,
            "starting a Main run is a user action, not an automatic retry"
        );
        assert!(!error.message.trim().is_empty(), "a reason must be shown");

        let stored = service.get_for_window("window-1").unwrap().unwrap();
        assert!(
            stored.tasks.is_empty(),
            "a rejected delegation must not create a task"
        );
        assert!(stored.dispatches.is_empty());
        assert_eq!(stored.revision, workspace.revision);
    }

    /// FR-022/FR-048: the two Coordinator reasons must serialize to distinct wire codes so
    /// the Composer can branch on `code` instead of parsing messages.
    #[test]
    fn coordinator_inactive_and_busy_serialize_to_distinct_codes() {
        let inactive = serde_json::to_string(&OrchestrationError::new(
            OrchestrationErrorCode::CoordinatorInactive,
            "no run",
        ))
        .unwrap();
        let busy = serde_json::to_string(
            &OrchestrationError::new(OrchestrationErrorCode::CoordinatorBusy, "busy").retryable(),
        )
        .unwrap();

        assert!(inactive.contains("\"coordinatorInactive\""));
        assert!(inactive.contains("\"retryable\":false"));
        assert!(busy.contains("\"coordinatorBusy\""));
        assert!(busy.contains("\"retryable\":true"));
    }

    /// FR-047: an out-of-workspace artifact reference is rejected on its own; the report
    /// body, findings, and result transition must survive and the rejection must stay visible.
    #[test]
    fn keeps_the_report_body_when_an_artifact_reference_is_rejected() {
        let (service, _workspace, created) = service_with_running_child();

        service
            .report_task(
                "window-1",
                ReportTaskRequest {
                    request_id: "result-with-bad-artifact".into(),
                    task_id: created.task_id.clone(),
                    reporter_node_id: created.node_id.clone(),
                    reporter_run_id: "run-child".into(),
                    report_type: TaskReportType::Result,
                    progress_percent: Some(100),
                    summary: "조사 결과 요약".into(),
                    findings: vec![],
                    artifact_refs: vec![
                        ArtifactReference {
                            kind: ArtifactKind::File,
                            uri: "../outside.txt".into(),
                            label: "escaping".into(),
                            description: None,
                        },
                        ArtifactReference {
                            kind: ArtifactKind::Text,
                            uri: "note://inline".into(),
                            label: "note".into(),
                            description: None,
                        },
                    ],
                    unresolved: vec!["원래 미해결 항목".into()],
                    confidence: Some(0.8),
                },
            )
            .expect("a rejected artifact must not drop the whole report");

        let stored = service.get_for_window("window-1").unwrap().unwrap();
        let report = stored
            .reports
            .iter()
            .find(|report| report.request_id == "result-with-bad-artifact")
            .expect("the report body must be preserved");

        assert_eq!(report.summary, "조사 결과 요약");
        assert_eq!(
            report.artifact_refs.len(),
            1,
            "only the escaping reference is dropped"
        );
        assert_eq!(report.artifact_refs[0].label, "note");
        assert!(
            report
                .unresolved
                .iter()
                .any(|entry| entry.contains("../outside.txt")),
            "the rejection must stay visible in unresolved: {:?}",
            report.unresolved
        );
        assert!(
            report
                .unresolved
                .iter()
                .any(|entry| entry == "원래 미해결 항목"),
            "existing unresolved entries must be kept"
        );
        assert_eq!(
            stored
                .tasks
                .iter()
                .find(|task| task.id == created.task_id)
                .unwrap()
                .status,
            TaskStatus::Completed
        );
    }

    #[test]
    fn bootstraps_window_scoped_workspaces_with_one_main() {
        let repository = MemoryRepository::default();
        let service = OrchestrationService::new(repository.clone(), RecordingSink::default());

        let first = service
            .bootstrap("/repo", "window-1", None)
            .expect("bootstrap first");
        let second = service
            .bootstrap("/repo", "window-2", None)
            .expect("bootstrap second");

        assert_ne!(first.id, second.id);
        assert_eq!(first.main_node_id, MAIN_AGENT_NODE_ID);
        assert_eq!(first.nodes.len(), 1);
        assert_eq!(
            service.get_for_window("window-1").unwrap().unwrap().id,
            first.id
        );
        assert!(service.get_for_window("unknown").unwrap().is_none());
    }

    #[test]
    fn releases_lost_window_as_explicitly_recoverable_runtime_lost_work() {
        let (service, workspace, created) = service_with_running_child();

        let released = service
            .release_window("window-1")
            .unwrap()
            .expect("released workspace");

        assert_eq!(released.id, workspace.id);
        assert_eq!(released.bound_window_label, None);
        assert_eq!(
            released
                .nodes
                .iter()
                .find(|node| node.id == MAIN_AGENT_NODE_ID)
                .unwrap()
                .execution_status,
            ExecutionStatus::Stopped
        );
        let child = released
            .nodes
            .iter()
            .find(|node| node.id == created.node_id)
            .unwrap();
        assert_eq!(child.execution_status, ExecutionStatus::Stopped);
        assert_eq!(
            child.presentation_status,
            PresentationStatus::AttentionRequired
        );
        let task = released
            .tasks
            .iter()
            .find(|task| task.id == created.task_id)
            .unwrap();
        assert_eq!(task.status, TaskStatus::Blocked);
        assert_eq!(
            task.failure.as_ref().unwrap().code,
            OrchestrationErrorCode::RuntimeLost
        );
        assert!(service.get_for_window("window-1").unwrap().is_none());
        assert_eq!(service.list_recoverable("/repo").unwrap().len(), 1);

        let resumed = service
            .bootstrap("/repo", "window-2", Some(&released.id))
            .unwrap();
        assert_eq!(resumed.bound_window_label.as_deref(), Some("window-2"));
        assert_eq!(resumed.tasks[0].status, TaskStatus::Blocked);
    }

    #[test]
    fn binds_main_generation_idempotently_and_rejects_stale_revision() {
        let repository = MemoryRepository::default();
        let service = OrchestrationService::new(repository, RecordingSink::default());
        let workspace = service.bootstrap("/repo", "window-1", None).unwrap();
        let request = BindMainRunRequest {
            request_id: "request-1".into(),
            panel_id: MAIN_AGENT_NODE_ID.into(),
            run_id: "run-main".into(),
            state: MainRunBindingState::Active,
            expected_revision: workspace.revision,
        };

        let first = service.bind_main_run("window-1", request.clone()).unwrap();
        let repeated = service.bind_main_run("window-1", request).unwrap();
        assert_eq!(
            first.active_coordinator_generation_id,
            repeated.active_coordinator_generation_id
        );
        assert_eq!(first.revision, repeated.revision);

        let error = service
            .bind_main_run(
                "window-1",
                BindMainRunRequest {
                    request_id: "request-2".into(),
                    panel_id: MAIN_AGENT_NODE_ID.into(),
                    run_id: "run-new".into(),
                    state: MainRunBindingState::Active,
                    expected_revision: 0,
                },
            )
            .unwrap_err();
        assert_eq!(error.code, OrchestrationErrorCode::RevisionConflict);
    }

    #[test]
    fn creates_direct_child_tasks_and_completes_only_on_result_report() {
        let repository = MemoryRepository::default();
        let service = OrchestrationService::new(repository, RecordingSink::default());
        let workspace = service.bootstrap("/repo", "window-1", None).unwrap();
        let workspace = service
            .bind_main_run(
                "window-1",
                BindMainRunRequest {
                    request_id: "bind-1".into(),
                    panel_id: MAIN_AGENT_NODE_ID.into(),
                    run_id: "run-main".into(),
                    state: MainRunBindingState::Active,
                    expected_revision: workspace.revision,
                },
            )
            .unwrap();
        let generation_id = workspace.active_coordinator_generation_id.clone().unwrap();
        let request = CreateChildTaskRequest {
            request_id: "create-research".into(),
            title: "구조 조사".into(),
            role: crate::domain::agent_orchestration::AgentRoleProfile::new(
                "researcher",
                "Researcher",
                "구조 조사",
                "근거 목록",
            )
            .unwrap(),
            objective: "현재 구조를 조사한다.".into(),
            constraints: vec!["read-only".into(), "same-worktree".into()],
            expected_result: "summary와 findings".into(),
            dependency_task_ids: vec![],
            preferred_node_id: None,
        };

        let created = service
            .create_child_task("window-1", &generation_id, request.clone())
            .unwrap();
        let repeated = service
            .create_child_task("window-1", &generation_id, request)
            .unwrap();
        assert_eq!(created.task_id, repeated.task_id);
        let snapshot = service.get_for_window("window-1").unwrap().unwrap();
        let node = snapshot
            .nodes
            .iter()
            .find(|node| node.id == created.node_id)
            .unwrap();
        assert_eq!(node.parent_node_id.as_deref(), Some(MAIN_AGENT_NODE_ID));
        assert_eq!(
            snapshot
                .tasks
                .iter()
                .find(|task| task.id == created.task_id)
                .unwrap()
                .status,
            TaskStatus::Ready
        );

        service
            .bind_child_run("window-1", &created.task_id, &created.node_id, "run-child")
            .unwrap();
        service
            .report_task(
                "window-1",
                ReportTaskRequest {
                    request_id: "progress-1".into(),
                    task_id: created.task_id.clone(),
                    reporter_node_id: created.node_id.clone(),
                    reporter_run_id: "run-child".into(),
                    report_type: TaskReportType::Progress,
                    progress_percent: Some(50),
                    summary: "절반 조사".into(),
                    findings: vec![],
                    artifact_refs: vec![],
                    unresolved: vec![],
                    confidence: None,
                },
            )
            .unwrap();
        assert_eq!(
            service.get_for_window("window-1").unwrap().unwrap().tasks[0].status,
            TaskStatus::Running
        );
        service
            .report_task(
                "window-1",
                ReportTaskRequest {
                    request_id: "result-1".into(),
                    task_id: created.task_id,
                    reporter_node_id: created.node_id,
                    reporter_run_id: "run-child".into(),
                    report_type: TaskReportType::Result,
                    progress_percent: Some(100),
                    summary: "조사 완료".into(),
                    findings: vec![],
                    artifact_refs: vec![],
                    unresolved: vec![],
                    confidence: Some(0.9),
                },
            )
            .unwrap();
        assert_eq!(
            service.get_for_window("window-1").unwrap().unwrap().tasks[0].status,
            TaskStatus::Completed
        );
    }

    #[test]
    fn promote_and_detach_keep_the_same_task_and_run() {
        let (service, workspace, created) = service_with_running_child();
        let promoted = service
            .set_presentation(
                "window-1",
                SetPresentationRequest {
                    request_id: "promote".into(),
                    node_id: created.node_id.clone(),
                    presentation_status: PresentationStatus::Panel,
                    expected_revision: workspace.revision,
                },
            )
            .unwrap();
        let detached = service
            .set_presentation(
                "window-1",
                SetPresentationRequest {
                    request_id: "detach".into(),
                    node_id: created.node_id.clone(),
                    presentation_status: PresentationStatus::Detached,
                    expected_revision: promoted.revision,
                },
            )
            .unwrap();
        let node = detached
            .nodes
            .iter()
            .find(|node| node.id == created.node_id)
            .unwrap();
        assert_eq!(node.current_run_id.as_deref(), Some("run-child"));
        assert_eq!(
            node.assigned_task_id.as_deref(),
            Some(created.task_id.as_str())
        );
        assert_eq!(node.execution_status, ExecutionStatus::Active);
    }

    #[test]
    fn cancellation_wins_a_late_result_and_retry_increments_attempt() {
        let (service, workspace, created) = service_with_running_child();
        let cancelled = service
            .cancel_task(
                "window-1",
                TaskActionRequest {
                    request_id: "cancel".into(),
                    task_id: created.task_id.clone(),
                    expected_revision: workspace.revision,
                    message: None,
                    target_node_id: None,
                },
            )
            .unwrap();
        service
            .report_task(
                "window-1",
                ReportTaskRequest {
                    request_id: "late-result".into(),
                    task_id: created.task_id.clone(),
                    reporter_node_id: created.node_id,
                    reporter_run_id: "run-child".into(),
                    report_type: TaskReportType::Result,
                    progress_percent: Some(100),
                    summary: "too late".into(),
                    findings: vec![],
                    artifact_refs: vec![],
                    unresolved: vec![],
                    confidence: Some(1.0),
                },
            )
            .unwrap();
        let snapshot = service.get_for_window("window-1").unwrap().unwrap();
        assert_eq!(cancelled.tasks[0].status, TaskStatus::Cancelled);
        assert_eq!(snapshot.tasks[0].status, TaskStatus::Cancelled);
        assert_eq!(snapshot.tasks[0].latest_result_report_id, None);
    }

    #[test]
    fn explicit_handoff_moves_open_tasks_to_a_new_generation() {
        let (service, workspace, created) = service_with_running_child();
        let previous = workspace.active_coordinator_generation_id.clone().unwrap();
        let handed_off = service
            .handoff_coordinator(
                "window-1",
                CoordinatorHandoffRequest {
                    request_id: "handoff".into(),
                    successor_run_id: "run-main-2".into(),
                    summary: "진행 중인 조사를 이어받는다.".into(),
                    confirmed: true,
                    expected_revision: workspace.revision,
                },
            )
            .unwrap();
        let successor = handed_off.active_coordinator_generation_id.clone().unwrap();
        assert_ne!(previous, successor);
        assert_eq!(
            handed_off
                .tasks
                .iter()
                .find(|task| task.id == created.task_id)
                .unwrap()
                .coordinator_generation_id,
            successor
        );
        assert_eq!(
            handed_off.generations[0].successor_generation_id,
            Some(successor)
        );
    }

    #[test]
    fn late_report_is_preserved_without_mutating_or_notifying_current_run() {
        let (service, _, created) = service_with_running_child();
        service
            .bind_child_run(
                "window-1",
                &created.task_id,
                &created.node_id,
                "run-child-2",
            )
            .unwrap();

        service
            .report_task(
                "window-1",
                ReportTaskRequest {
                    request_id: "late-old-attempt".into(),
                    task_id: created.task_id,
                    reporter_node_id: created.node_id,
                    reporter_run_id: "run-child".into(),
                    report_type: TaskReportType::Result,
                    progress_percent: Some(100),
                    summary: "이전 실행의 늦은 결과".into(),
                    findings: vec![],
                    artifact_refs: vec![],
                    unresolved: vec![],
                    confidence: Some(0.8),
                },
            )
            .unwrap();

        let snapshot = service.get_for_window("window-1").unwrap().unwrap();
        assert_eq!(snapshot.reports.len(), 1);
        assert_eq!(snapshot.tasks[0].status, TaskStatus::Running);
        assert!(snapshot.tasks[0].latest_result_report_id.is_none());
        assert!(snapshot.coordinator_notifications.is_empty());
    }

    #[test]
    fn report_and_notification_are_atomic_and_full_payload_idempotent() {
        let (service, _, created) = service_with_running_child();
        let request = ReportTaskRequest {
            request_id: "atomic-report".into(),
            task_id: created.task_id,
            reporter_node_id: created.node_id,
            reporter_run_id: "run-child".into(),
            report_type: TaskReportType::Progress,
            progress_percent: Some(40),
            summary: "진행 중".into(),
            findings: vec![TaskFinding {
                title: "근거 A".into(),
                detail: "첫 번째 근거".into(),
                evidence: vec!["source:A".into()],
                severity: crate::domain::agent_orchestration::FindingSeverity::Info,
            }],
            artifact_refs: vec![],
            unresolved: vec!["검증 필요".into()],
            confidence: Some(0.6),
        };
        let first = service.report_task("window-1", request.clone()).unwrap();
        let repeated = service.report_task("window-1", request.clone()).unwrap();
        assert_eq!(first.id, repeated.id);

        let snapshot = service.get_for_window("window-1").unwrap().unwrap();
        assert_eq!(snapshot.reports.len(), 1);
        assert_eq!(snapshot.coordinator_notifications.len(), 1);
        assert_eq!(
            snapshot.coordinator_notifications[0].report_id,
            snapshot.reports[0].id
        );

        let mut conflict = request;
        conflict.findings = vec![TaskFinding {
            title: "근거 B".into(),
            detail: "다른 근거".into(),
            evidence: vec!["source:B".into()],
            severity: crate::domain::agent_orchestration::FindingSeverity::Warning,
        }];
        assert_eq!(
            service.report_task("window-1", conflict).unwrap_err().code,
            OrchestrationErrorCode::DuplicateConflict
        );
    }

    #[test]
    fn collecting_child_results_records_collection_without_claiming_prompt_completion() {
        let (service, workspace, created) = service_with_running_child();
        let generation_id = workspace
            .active_coordinator_generation_id
            .as_deref()
            .unwrap()
            .to_string();
        let report = service
            .report_task(
                "window-1",
                ReportTaskRequest {
                    request_id: "collectable-report".into(),
                    task_id: created.task_id.clone(),
                    reporter_node_id: created.node_id,
                    reporter_run_id: "run-child".into(),
                    report_type: TaskReportType::Result,
                    progress_percent: Some(100),
                    summary: "수집 가능한 결과".into(),
                    findings: vec![],
                    artifact_refs: vec![],
                    unresolved: vec![],
                    confidence: Some(1.0),
                },
            )
            .unwrap();

        let reports = service
            .collect_child_results("window-1", &generation_id, &[created.task_id])
            .unwrap();

        assert_eq!(reports, vec![report]);
        let snapshot = service.get_for_window("window-1").unwrap().unwrap();
        assert_eq!(
            snapshot.coordinator_notifications[0].status,
            CoordinatorNotificationStatus::Pending
        );
        assert!(snapshot.coordinator_notifications[0].collected_at.is_some());
    }

    #[test]
    fn prompt_dispatch_is_exact_once_and_keeps_target_failures() {
        let (service, workspace, created) = service_with_running_child();
        let request = DispatchPromptRequest {
            request_id: "dispatch-1".into(),
            intent: PromptDispatchIntent::Direct,
            target_mode: PromptTargetMode::Selected,
            message: "계속 진행".into(),
            delivery: PromptDelivery::Queue,
            panel_ids: vec![MAIN_AGENT_NODE_ID.into(), created.node_id],
            expected_revision: workspace.revision,
        };
        let first = service
            .record_prompt_dispatch("window-1", request.clone())
            .unwrap();
        let repeated = service.record_prompt_dispatch("window-1", request).unwrap();
        assert_eq!(first.id, repeated.id);
        assert_eq!(first.targets.len(), 2);
        assert!(
            first
                .targets
                .iter()
                .all(|target| target.status == PromptDispatchTargetStatus::Accepted)
        );
    }

    #[test]
    fn reconciliation_marks_lost_child_runtime_recoverable() {
        let (service, workspace, created) = service_with_running_child();
        let reconciled = service
            .reconcile_runtime("window-1", &["run-main".into()])
            .unwrap();
        let task = reconciled
            .tasks
            .iter()
            .find(|task| task.id == created.task_id)
            .unwrap();
        let node = reconciled
            .nodes
            .iter()
            .find(|node| node.id == created.node_id)
            .unwrap();
        assert_eq!(workspace.tasks[0].status, TaskStatus::Running);
        assert_eq!(task.status, TaskStatus::Blocked);
        assert_eq!(
            task.failure.as_ref().unwrap().code,
            OrchestrationErrorCode::RuntimeLost
        );
        assert_eq!(
            node.presentation_status,
            PresentationStatus::AttentionRequired
        );
    }
}
