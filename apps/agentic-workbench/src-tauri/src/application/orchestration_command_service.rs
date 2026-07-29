//! Durable Child command outbox shared by UI and MCP inbound adapters.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    domain::agent_orchestration::{
        CommandFailure, CoordinatorNotificationStatus, OrchestrationError, OrchestrationErrorCode,
        OrchestrationSession, PresentationStatus, PromptDelivery, TaskCommand, TaskCommandKind,
        TaskCommandSource, TaskCommandStatus, TaskReportType, TaskStatus, full_payload_fingerprint,
    },
    ports::{
        agent_worker::{AgentWorkerPort, WorkerBinding, WorkerCommandOutcome},
        orchestration_repository::OrchestrationRepository,
    },
};

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliverTaskCommandRequest {
    pub request_id: String,
    pub task_id: String,
    pub kind: TaskCommandKind,
    pub message: Option<String>,
    pub input_report_id: Option<String>,
    pub delivery: PromptDelivery,
    pub source: TaskCommandSource,
    pub expected_task_revision: Option<u64>,
}

pub struct OrchestrationCommandService<R, W> {
    repository: R,
    worker: W,
}

impl<R, W> OrchestrationCommandService<R, W>
where
    R: OrchestrationRepository,
    W: AgentWorkerPort,
{
    pub fn new(repository: R, worker: W) -> Self {
        Self { repository, worker }
    }

    pub async fn deliver(
        &self,
        window_label: &str,
        request: DeliverTaskCommandRequest,
    ) -> Result<TaskCommand, OrchestrationError> {
        validate_request(&request)?;
        let fingerprint = full_payload_fingerprint(&request)?;
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;

        if let Some(existing) = session
            .commands
            .iter()
            .find(|command| command.request_id == request.request_id)
        {
            return if existing.payload_fingerprint == fingerprint {
                Ok(existing.clone())
            } else {
                Err(duplicate_conflict())
            };
        }

        let task_index = session
            .tasks
            .iter()
            .position(|task| task.id == request.task_id)
            .ok_or_else(|| not_found("Task"))?;
        if let Some(expected) = request.expected_task_revision
            && session.tasks[task_index].revision != expected
        {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::RevisionConflict,
                "The task revision is stale.",
            )
            .retryable());
        }
        if session.tasks[task_index].status.is_terminal() {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidTransition,
                "A terminal task cannot receive a runtime command.",
            ));
        }
        validate_input_report(session, task_index, &request)?;

        let node_id = session.tasks[task_index]
            .assigned_node_id
            .clone()
            .ok_or_else(|| not_found("Assigned child node"))?;
        let node_index = session
            .nodes
            .iter()
            .position(|node| node.id == node_id)
            .ok_or_else(|| not_found("Assigned child node"))?;
        let run_id = session.nodes[node_index]
            .current_run_id
            .clone()
            .ok_or_else(|| {
                OrchestrationError::new(
                    OrchestrationErrorCode::RuntimeLost,
                    "The assigned child has no active run.",
                )
                .retryable()
            })?;
        let now = now();
        let command = TaskCommand {
            id: Uuid::new_v4().to_string(),
            request_id: request.request_id,
            payload_fingerprint: fingerprint,
            task_id: request.task_id,
            node_id,
            run_id,
            attempt: session.tasks[task_index].attempt,
            kind: request.kind,
            message: request.message,
            input_report_id: request.input_report_id,
            delivery: request.delivery,
            source: request.source,
            status: TaskCommandStatus::Pending,
            failure: None,
            created_at: now.clone(),
            updated_at: now,
        };
        command.assert_current_binding(&session.tasks[task_index], &session.nodes[node_index])?;
        session.commands.push(command.clone());
        touch(session);
        self.repository.save_sessions(&sessions)?;

        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        transition_command(session, &command.id, TaskCommandStatus::Dispatching, None)?;
        touch(session);
        let binding = WorkerBinding {
            workspace_id: session.id.clone(),
            window_label: window_label.into(),
            node_id: command.node_id.clone(),
            task_id: command.task_id.clone(),
            run_id: command.run_id.clone(),
        };
        self.repository.save_sessions(&sessions)?;
        let delivery = match command.kind {
            TaskCommandKind::Message | TaskCommandKind::InputResponse => {
                self.worker
                    .send_prompt(
                        &binding,
                        command.message.as_deref().unwrap_or_default(),
                        command.delivery,
                    )
                    .await
            }
            TaskCommandKind::Interrupt => self.worker.interrupt_worker(&binding).await,
            TaskCommandKind::Cancel => self.worker.cancel_worker(&binding).await,
        };

        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        let outcome = match delivery {
            Ok(receipt) if receipt.accepted => match accept_command(session, &command.id) {
                Ok(()) => WorkerCommandOutcome {
                    accepted: true,
                    reason: receipt.reason,
                },
                Err(error) => {
                    fail_command(session, &command.id, error.clone())?;
                    WorkerCommandOutcome {
                        accepted: false,
                        reason: Some(error.message),
                    }
                }
            },
            Ok(receipt) => {
                fail_command(
                    session,
                    &command.id,
                    OrchestrationError::new(
                        OrchestrationErrorCode::WorkerUnavailable,
                        receipt
                            .reason
                            .clone()
                            .unwrap_or_else(|| "The worker rejected the command.".into()),
                    )
                    .retryable(),
                )?;
                receipt
            }
            Err(error) => {
                fail_command(session, &command.id, error.clone())?;
                WorkerCommandOutcome {
                    accepted: false,
                    reason: Some(error.message),
                }
            }
        };
        touch(session);
        let command = session
            .commands
            .iter()
            .find(|candidate| candidate.id == command.id)
            .cloned()
            .ok_or_else(|| not_found("Task command"))?;
        self.repository.save_sessions(&sessions)?;
        debug_assert_eq!(
            command.status == TaskCommandStatus::Accepted,
            outcome.accepted
        );
        Ok(command)
    }

    /// Reconciles interrupted dispatches without resending accepted commands.
    pub fn reconcile_pending(
        &self,
        window_label: &str,
    ) -> Result<Vec<TaskCommand>, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        let mut recovered = Vec::new();
        let mut changed = false;
        for command in &mut session.commands {
            if command.status == TaskCommandStatus::Dispatching {
                command.status = TaskCommandStatus::Pending;
                command.failure = Some(CommandFailure {
                    code: OrchestrationErrorCode::RuntimeLost,
                    message: "Delivery was interrupted and requires an explicit retry.".into(),
                    retryable: true,
                });
                command.updated_at = now();
                recovered.push(command.clone());
                changed = true;
            }
        }
        for notification in &mut session.coordinator_notifications {
            if notification.status == CoordinatorNotificationStatus::Dispatching {
                notification.status = CoordinatorNotificationStatus::Pending;
                notification.updated_at = now();
                changed = true;
            }
        }
        if changed {
            touch(session);
            self.repository.save_sessions(&sessions)?;
        }
        Ok(recovered)
    }
}

fn validate_request(request: &DeliverTaskCommandRequest) -> Result<(), OrchestrationError> {
    if request.request_id.trim().is_empty() || request.task_id.trim().is_empty() {
        return Err(OrchestrationError::new(
            OrchestrationErrorCode::InvalidInput,
            "Command request and task ids are required.",
        ));
    }
    if matches!(
        request.kind,
        TaskCommandKind::Message | TaskCommandKind::InputResponse
    ) && request
        .message
        .as_deref()
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .is_none()
    {
        return Err(OrchestrationError::new(
            OrchestrationErrorCode::InvalidInput,
            "Message commands require non-empty text.",
        ));
    }
    Ok(())
}

fn validate_input_report(
    session: &OrchestrationSession,
    task_index: usize,
    request: &DeliverTaskCommandRequest,
) -> Result<(), OrchestrationError> {
    if request.kind != TaskCommandKind::InputResponse {
        return Ok(());
    }
    if session.tasks[task_index].status != TaskStatus::InputRequired {
        return Err(OrchestrationError::new(
            OrchestrationErrorCode::InvalidTransition,
            "Only a task waiting for input can receive a response.",
        ));
    }
    let latest_input = session
        .reports
        .iter()
        .rev()
        .find(|report| {
            report.task_id == session.tasks[task_index].id
                && report.report_type == TaskReportType::InputRequest
        })
        .map(|report| report.id.as_str());
    if latest_input != request.input_report_id.as_deref() {
        return Err(OrchestrationError::new(
            OrchestrationErrorCode::RevisionConflict,
            "The input request is stale.",
        ));
    }
    Ok(())
}

fn accept_command(
    session: &mut OrchestrationSession,
    command_id: &str,
) -> Result<(), OrchestrationError> {
    let command_index = session
        .commands
        .iter()
        .position(|command| command.id == command_id)
        .ok_or_else(|| not_found("Task command"))?;
    let command = session.commands[command_index].clone();
    let task_index = session
        .tasks
        .iter()
        .position(|task| task.id == command.task_id)
        .ok_or_else(|| not_found("Task"))?;
    let node_index = session
        .nodes
        .iter()
        .position(|node| node.id == command.node_id)
        .ok_or_else(|| not_found("Assigned child node"))?;
    command.assert_current_binding(&session.tasks[task_index], &session.nodes[node_index])?;
    session.commands[command_index].failure = None;
    session.commands[command_index].transition(TaskCommandStatus::Accepted, now())?;
    match command.kind {
        TaskCommandKind::InputResponse => {
            if session.tasks[task_index].status == TaskStatus::InputRequired {
                session.tasks[task_index].transition(TaskStatus::Running, now())?;
                session.tasks[task_index].failure = None;
                session.nodes[node_index].presentation_status = PresentationStatus::Background;
            }
        }
        TaskCommandKind::Cancel => {
            if !session.tasks[task_index].status.is_terminal() {
                session.tasks[task_index].transition(TaskStatus::Cancelled, now())?;
                session.nodes[node_index].execution_status =
                    crate::domain::agent_orchestration::ExecutionStatus::Stopped;
            }
        }
        TaskCommandKind::Message | TaskCommandKind::Interrupt => {}
    }
    session.nodes[node_index].last_activity_at = Some(now());
    Ok(())
}

fn fail_command(
    session: &mut OrchestrationSession,
    command_id: &str,
    error: OrchestrationError,
) -> Result<(), OrchestrationError> {
    let command = session
        .commands
        .iter_mut()
        .find(|command| command.id == command_id)
        .ok_or_else(|| not_found("Task command"))?;
    command.failure = Some(CommandFailure {
        code: error.code,
        message: error.message,
        retryable: error.retryable,
    });
    command.transition(TaskCommandStatus::Failed, now())
}

fn transition_command(
    session: &mut OrchestrationSession,
    command_id: &str,
    status: TaskCommandStatus,
    failure: Option<CommandFailure>,
) -> Result<(), OrchestrationError> {
    let command = session
        .commands
        .iter_mut()
        .find(|command| command.id == command_id)
        .ok_or_else(|| not_found("Task command"))?;
    command.failure = failure;
    command.transition(status, now())
}

fn session_for_window_mut<'a>(
    sessions: &'a mut [OrchestrationSession],
    window_label: &str,
) -> Result<&'a mut OrchestrationSession, OrchestrationError> {
    let session = sessions
        .iter_mut()
        .find(|session| session.bound_window_label.as_deref() == Some(window_label))
        .ok_or_else(|| not_found("Orchestration workspace"))?;
    session.assert_scope(window_label)?;
    Ok(session)
}

fn touch(session: &mut OrchestrationSession) {
    session.revision += 1;
    session.updated_at = now();
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::{
        domain::agent_orchestration::{
            AccessPolicy, AgentNode, AgentRoleProfile, ArtifactReference, ExecutionStatus,
            OrchestrationTask, TaskReport,
        },
        ports::agent_worker::{StartWorkerOutcome, WorkerAssignment},
    };

    #[derive(Clone)]
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

    #[derive(Clone)]
    struct FakeWorker {
        calls: Arc<Mutex<Vec<(TaskCommandKind, String)>>>,
        accepts: bool,
    }

    impl AgentWorkerPort for FakeWorker {
        async fn start_worker(
            &self,
            _assignment: WorkerAssignment,
        ) -> Result<StartWorkerOutcome, OrchestrationError> {
            unreachable!()
        }

        async fn send_prompt(
            &self,
            _binding: &WorkerBinding,
            message: &str,
            _delivery: PromptDelivery,
        ) -> Result<WorkerCommandOutcome, OrchestrationError> {
            self.calls
                .lock()
                .unwrap()
                .push((TaskCommandKind::Message, message.into()));
            Ok(WorkerCommandOutcome {
                accepted: self.accepts,
                reason: (!self.accepts).then(|| "offline".into()),
            })
        }

        async fn interrupt_worker(
            &self,
            _binding: &WorkerBinding,
        ) -> Result<WorkerCommandOutcome, OrchestrationError> {
            Ok(WorkerCommandOutcome {
                accepted: self.accepts,
                reason: None,
            })
        }

        async fn cancel_worker(
            &self,
            _binding: &WorkerBinding,
        ) -> Result<WorkerCommandOutcome, OrchestrationError> {
            Ok(WorkerCommandOutcome {
                accepted: self.accepts,
                reason: None,
            })
        }

        async fn is_active(&self, _binding: &WorkerBinding) -> bool {
            true
        }
    }

    fn repository(status: TaskStatus) -> MemoryRepository {
        let now = "2026-07-27T00:00:00Z".to_string();
        let mut session =
            OrchestrationSession::new("workspace-1", "/repo", "window-1", now.clone());
        let role =
            AgentRoleProfile::new("researcher", "Researcher", "Inspect", "Findings").unwrap();
        let mut node = AgentNode::child("child-1", "main-agent-run", role, now.clone()).unwrap();
        node.current_run_id = Some("run-1".into());
        node.assigned_task_id = Some("task-1".into());
        node.execution_status = ExecutionStatus::Active;
        session.nodes.push(node);
        session.tasks.push(OrchestrationTask {
            id: "task-1".into(),
            parent_task_id: None,
            coordinator_generation_id: "generation-1".into(),
            assigned_node_id: Some("child-1".into()),
            title: "Research".into(),
            objective: "Inspect".into(),
            constraints: vec![],
            expected_result: "Findings".into(),
            dependency_task_ids: vec![],
            status,
            awaiting_handoff: false,
            access_policy: AccessPolicy::ReadOnly,
            attempt: 1,
            latest_result_report_id: None,
            failure: None,
            revision: 3,
            created_at: now.clone(),
            started_at: Some(now.clone()),
            completed_at: None,
            updated_at: now.clone(),
        });
        if status == TaskStatus::InputRequired {
            session.reports.push(TaskReport {
                id: "input-1".into(),
                request_id: "input-request".into(),
                task_id: "task-1".into(),
                reporter_node_id: "child-1".into(),
                reporter_run_id: "run-1".into(),
                report_type: TaskReportType::InputRequest,
                progress_percent: None,
                summary: "Need input".into(),
                findings: vec![],
                artifact_refs: Vec::<ArtifactReference>::new(),
                unresolved: vec![],
                confidence: None,
                created_at: now,
            });
        }
        MemoryRepository(Arc::new(Mutex::new(vec![session])))
    }

    fn message_request(message: &str) -> DeliverTaskCommandRequest {
        DeliverTaskCommandRequest {
            request_id: "request-1".into(),
            task_id: "task-1".into(),
            kind: TaskCommandKind::Message,
            message: Some(message.into()),
            input_report_id: None,
            delivery: PromptDelivery::Queue,
            source: TaskCommandSource::Coordinator,
            expected_task_revision: Some(3),
        }
    }

    #[tokio::test]
    async fn persists_before_send_and_replays_exact_duplicates_without_resending() {
        let repository = repository(TaskStatus::Running);
        let calls = Arc::new(Mutex::new(vec![]));
        let service = OrchestrationCommandService::new(
            repository.clone(),
            FakeWorker {
                calls: calls.clone(),
                accepts: true,
            },
        );
        let first = service
            .deliver("window-1", message_request("status"))
            .await
            .unwrap();
        assert_eq!(first.status, TaskCommandStatus::Accepted);
        let repeated = service
            .deliver("window-1", message_request("status"))
            .await
            .unwrap();
        assert_eq!(repeated.id, first.id);
        assert_eq!(calls.lock().unwrap().len(), 1);

        let conflict = service
            .deliver("window-1", message_request("different"))
            .await
            .unwrap_err();
        assert_eq!(conflict.code, OrchestrationErrorCode::DuplicateConflict);
    }

    #[tokio::test]
    async fn failed_input_delivery_preserves_input_required_and_response_text() {
        let repository = repository(TaskStatus::InputRequired);
        let service = OrchestrationCommandService::new(
            repository.clone(),
            FakeWorker {
                calls: Arc::new(Mutex::new(vec![])),
                accepts: false,
            },
        );
        let command = service
            .deliver(
                "window-1",
                DeliverTaskCommandRequest {
                    request_id: "response-1".into(),
                    task_id: "task-1".into(),
                    kind: TaskCommandKind::InputResponse,
                    message: Some("Use read-only".into()),
                    input_report_id: Some("input-1".into()),
                    delivery: PromptDelivery::Queue,
                    source: TaskCommandSource::User,
                    expected_task_revision: Some(3),
                },
            )
            .await
            .unwrap();
        assert_eq!(command.status, TaskCommandStatus::Failed);
        assert_eq!(command.message.as_deref(), Some("Use read-only"));
        assert_eq!(
            repository.load_sessions().unwrap()[0].tasks[0].status,
            TaskStatus::InputRequired
        );
    }

    #[tokio::test]
    async fn rejects_a_response_for_an_old_input_report() {
        let repository = repository(TaskStatus::InputRequired);
        let service = OrchestrationCommandService::new(
            repository,
            FakeWorker {
                calls: Arc::new(Mutex::new(vec![])),
                accepts: true,
            },
        );
        let error = service
            .deliver(
                "window-1",
                DeliverTaskCommandRequest {
                    request_id: "stale-response".into(),
                    task_id: "task-1".into(),
                    kind: TaskCommandKind::InputResponse,
                    message: Some("old answer".into()),
                    input_report_id: Some("older-input".into()),
                    delivery: PromptDelivery::Queue,
                    source: TaskCommandSource::User,
                    expected_task_revision: Some(3),
                },
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, OrchestrationErrorCode::RevisionConflict);
    }

    #[derive(Clone)]
    struct CompletingDuringCancelWorker {
        repository: MemoryRepository,
    }

    impl AgentWorkerPort for CompletingDuringCancelWorker {
        async fn start_worker(
            &self,
            _assignment: WorkerAssignment,
        ) -> Result<StartWorkerOutcome, OrchestrationError> {
            unreachable!()
        }

        async fn send_prompt(
            &self,
            _binding: &WorkerBinding,
            _message: &str,
            _delivery: PromptDelivery,
        ) -> Result<WorkerCommandOutcome, OrchestrationError> {
            unreachable!()
        }

        async fn interrupt_worker(
            &self,
            _binding: &WorkerBinding,
        ) -> Result<WorkerCommandOutcome, OrchestrationError> {
            unreachable!()
        }

        async fn cancel_worker(
            &self,
            _binding: &WorkerBinding,
        ) -> Result<WorkerCommandOutcome, OrchestrationError> {
            let mut sessions = self.repository.load_sessions()?;
            sessions[0].tasks[0].status = TaskStatus::Completed;
            self.repository.save_sessions(&sessions)?;
            Ok(WorkerCommandOutcome {
                accepted: true,
                reason: None,
            })
        }

        async fn is_active(&self, _binding: &WorkerBinding) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn terminal_result_wins_when_it_arrives_before_cancel_ack() {
        let repository = repository(TaskStatus::Running);
        let service = OrchestrationCommandService::new(
            repository.clone(),
            CompletingDuringCancelWorker {
                repository: repository.clone(),
            },
        );
        let command = service
            .deliver(
                "window-1",
                DeliverTaskCommandRequest {
                    request_id: "cancel-race".into(),
                    task_id: "task-1".into(),
                    kind: TaskCommandKind::Cancel,
                    message: None,
                    input_report_id: None,
                    delivery: PromptDelivery::Queue,
                    source: TaskCommandSource::Coordinator,
                    expected_task_revision: Some(3),
                },
            )
            .await
            .unwrap();
        assert_eq!(command.status, TaskCommandStatus::Accepted);
        assert_eq!(
            repository.load_sessions().unwrap()[0].tasks[0].status,
            TaskStatus::Completed
        );
    }
}
