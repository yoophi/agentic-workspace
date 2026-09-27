//! orchestration 자식 run 기계(041 research R8): AW `AcpAgentWorkerAdapter`·`TauriAcpWorkerRuntime`을 core
//! `RunEngine` 위로 옮겼다. 자식 run의 소유자는 작업 영역이 묶인 작업대이고, 기동은 그 작업대 입장권 안에서 한다
//! (닫기와 직렬화). MCP 권한은 `RunLaunchDecorator`가 `LaunchContext.orchestration`으로 만든다(창 label 없음).
//! 오류 문구는 오늘과 같다.

use std::sync::Arc;

use acp_agent_core::domain::run::{AgentRunRequest, PermissionMode, ResumePolicy};
use workbench_protocol::RequestId;

use crate::{
    application::bench_service::BenchServices,
    domain::agent_orchestration::{
        CoordinatorNotification, OrchestrationError, OrchestrationErrorCode, PromptDelivery,
    },
    infrastructure::orchestration::worktree_guard::{
        fingerprint_worktree, WorktreeGuard, WorktreeGuards,
    },
    ports::{
        agent_worker::{
            AgentWorkerPort, StartWorkerOutcome, WorkerAssignment, WorkerBinding,
            WorkerCommandOutcome,
        },
        coordinator_notification::{CoordinatorNotificationPort, CoordinatorNotificationReceipt},
        desktop_bridge::{LaunchContext, OrchestrationLaunchRole},
    },
};

/// 작업대가 없어 자식을 띄울 수 없을 때(오늘 AW `desktop_benches::MESSAGE_WINDOW_UNAVAILABLE`와 같은 문구).
pub const MESSAGE_OWNER_UNAVAILABLE: &str = "Owner Worktree Session window is unavailable.";

#[derive(Clone)]
pub struct EngineAgentWorker {
    benches: Arc<BenchServices>,
    guards: Arc<WorktreeGuards>,
}

impl EngineAgentWorker {
    pub fn new(benches: Arc<BenchServices>, guards: Arc<WorktreeGuards>) -> Self {
        Self { benches, guards }
    }

    fn command_error(message: impl Into<String>) -> OrchestrationError {
        OrchestrationError::new(OrchestrationErrorCode::WorkerUnavailable, message).retryable()
    }

    fn accepted() -> WorkerCommandOutcome {
        WorkerCommandOutcome {
            accepted: true,
            reason: None,
        }
    }

    async fn launch(&self, assignment: &WorkerAssignment, goal: String) -> Result<String, String> {
        let mut request = build_worker_request(assignment, goal);
        // 입장권: 작업대가 닫히는 중이면 띄우지 않는다. 엔진이 run을 등록할 때까지 쥔다(research R2·R8).
        let admission = self
            .benches
            .admit(&RequestId::random(), None, &assignment.bench_id)
            .map_err(|_| MESSAGE_OWNER_UNAVAILABLE.to_owned())?;
        if let Some(decorator) = &self.benches.launch_decorator {
            decorator.decorate(
                &mut request,
                &LaunchContext {
                    bench_id: assignment.bench_id.clone(),
                    panel_id: Some(assignment.node_id.clone()),
                    run_id: assignment.planned_run_id.clone(),
                    orchestration: Some(OrchestrationLaunchRole::Child {
                        workspace_id: assignment.workspace_id.clone(),
                        node_id: assignment.node_id.clone(),
                        task_id: assignment.task_id.clone(),
                    }),
                },
            )?;
        }
        if !self
            .benches
            .hub
            .claim_run(&assignment.planned_run_id, &assignment.bench_id)
        {
            return Err(format!("duplicate run id: {}", assignment.planned_run_id));
        }
        let run = match self
            .benches
            .engine
            .start(
                request,
                &assignment.bench_id,
                self.benches.run_sink(&assignment.bench_id),
            )
            .await
        {
            Ok(run) => run,
            Err(error) => {
                self.benches
                    .hub
                    .release_run_claim(&assignment.planned_run_id, &assignment.bench_id);
                return Err(error.message);
            }
        };
        drop(admission);
        Ok(run.id)
    }
}

/// 오늘 `build_worker_request`와 같은 요청(MCP env는 decorator가 넣는다).
pub fn build_worker_request(assignment: &WorkerAssignment, goal: String) -> AgentRunRequest {
    AgentRunRequest {
        goal,
        agent_id: assignment.runtime_profile.agent_profile_id.clone(),
        workspace_id: None,
        checkout_id: None,
        cwd: Some(assignment.worktree_path.clone()),
        agent_command: None,
        agent_env: None,
        mcp_servers: Vec::new(),
        stdio_buffer_limit_mb: None,
        auto_allow: Some(true),
        permission_mode: Some(PermissionMode::ReadOnly),
        model_id: assignment.runtime_profile.model_id.clone(),
        effort_id: None,
        context_size: None,
        run_id: Some(assignment.planned_run_id.clone()),
        resume_session_id: None,
        resume_policy: Some(ResumePolicy::Fresh),
        ralph_loop: None,
    }
}

pub fn worker_goal(assignment: &WorkerAssignment) -> String {
    format!(
        "Role: {role}\nResponsibility: {responsibility}\n\nObjective:\n{objective}\n\nConstraints:\n{constraints}\n\nExpected result:\n{expected}\n\nYou must report the final structured result with aw_report_result.",
        role = assignment.role.name,
        responsibility = assignment.role.responsibility,
        objective = assignment.objective,
        constraints = assignment.constraints.join("\n- "),
        expected = assignment.expected_result,
    )
}

impl AgentWorkerPort for EngineAgentWorker {
    async fn start_worker(
        &self,
        assignment: WorkerAssignment,
    ) -> Result<StartWorkerOutcome, OrchestrationError> {
        if !assignment.runtime_profile.supports_read_only {
            return Ok(StartWorkerOutcome::Failed {
                code: "unsupportedReadOnlyProfile".into(),
                message: "The selected agent profile cannot enforce read-only access.".into(),
                retryable: false,
            });
        }
        let path = assignment.worktree_path.clone();
        let baseline = tokio::task::spawn_blocking(move || fingerprint_worktree(&path))
            .await
            .map_err(|error| Self::command_error(error.to_string()))??;
        let goal = worker_goal(&assignment);
        match self.launch(&assignment, goal).await {
            Ok(run_id) => {
                self.guards.insert(
                    &run_id,
                    WorktreeGuard {
                        bench_id: assignment.bench_id.clone(),
                        workspace_id: assignment.workspace_id.clone(),
                        node_id: assignment.node_id.clone(),
                        task_id: assignment.task_id.clone(),
                        worktree_path: assignment.worktree_path.clone(),
                        baseline,
                    },
                );
                Ok(StartWorkerOutcome::Started { run_id })
            }
            Err(message) => Ok(StartWorkerOutcome::Failed {
                code: "workerLaunchFailed".into(),
                message,
                retryable: true,
            }),
        }
    }

    async fn send_prompt(
        &self,
        binding: &WorkerBinding,
        message: &str,
        delivery: PromptDelivery,
    ) -> Result<WorkerCommandOutcome, OrchestrationError> {
        let sink = self.benches.run_sink(&binding.bench_id);
        let engine = &self.benches.engine;
        let result = match delivery {
            PromptDelivery::Draft => {
                return Err(Self::command_error(
                    "Background workers do not accept draft delivery.",
                ))
            }
            PromptDelivery::Queue => engine.queue_prompt(&binding.run_id, message.into(), sink),
            PromptDelivery::Send => engine.send_prompt(&binding.run_id, message.into(), sink),
        }
        .await;
        result.map_err(|error| Self::command_error(error.message))?;
        Ok(Self::accepted())
    }

    async fn interrupt_worker(
        &self,
        binding: &WorkerBinding,
    ) -> Result<WorkerCommandOutcome, OrchestrationError> {
        self.benches
            .engine
            .cancel(&binding.run_id, self.benches.run_sink(&binding.bench_id))
            .await;
        Ok(Self::accepted())
    }

    async fn cancel_worker(
        &self,
        binding: &WorkerBinding,
    ) -> Result<WorkerCommandOutcome, OrchestrationError> {
        self.benches
            .engine
            .cancel(&binding.run_id, self.benches.run_sink(&binding.bench_id))
            .await;
        Ok(Self::accepted())
    }

    async fn is_active(&self, binding: &WorkerBinding) -> bool {
        self.benches
            .engine
            .active_owner_of(&binding.run_id)
            .await
            .as_deref()
            == Some(binding.bench_id.as_str())
    }
}

impl CoordinatorNotificationPort for EngineAgentWorker {
    async fn notify_coordinator(
        &self,
        binding: &WorkerBinding,
        notification: &CoordinatorNotification,
    ) -> Result<CoordinatorNotificationReceipt, OrchestrationError> {
        let message = format!(
            "Child report available: workspace={}, task={}, report={}, type={:?}. Call aw_collect_child_results with taskIds=[\"{}\"] now, then incorporate the structured report into the parent task.",
            binding.workspace_id,
            notification.task_id,
            notification.report_id,
            notification.report_type,
            notification.task_id,
        );
        self.benches
            .engine
            .send_and_wait(
                &binding.run_id,
                message,
                true,
                self.benches.run_sink(&binding.bench_id),
            )
            .await
            .map_err(|error| Self::command_error(error.message))?;
        Ok(CoordinatorNotificationReceipt {
            accepted: true,
            reason: None,
        })
    }
}
