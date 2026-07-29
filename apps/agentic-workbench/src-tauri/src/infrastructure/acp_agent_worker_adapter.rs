//! `AgentWorkerPort` adapter backed by the existing ACP runtime.

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    process::Command,
    sync::{Arc, Mutex, OnceLock},
};
use tauri::AppHandle;

use crate::{
    application::{
        cancel_agent_run::CancelAgentRunUseCase, send_prompt::SendPromptUseCase,
        start_agent_run::StartAgentRunUseCase,
    },
    domain::{
        agent_orchestration::{OrchestrationError, OrchestrationErrorCode, PromptDelivery},
        events::RunEvent,
    },
    infrastructure::{
        acp::runner::AcpAgentRunner,
        acp_agent_launch_factory::build_worker_request,
        agent_catalog::ConfigurableAgentCatalog,
        agent_session_registry::AppState,
        json_acp_session_store::JsonAcpSessionStore,
        mcp::{McpServerState, capability_registry::CapabilityPrincipal},
        tauri_run_event_sink::TauriRunEventSink,
    },
    ports::agent_worker::{
        AgentWorkerPort, StartWorkerOutcome, WorkerAssignment, WorkerBinding, WorkerCommandOutcome,
    },
    ports::coordinator_notification::{
        CoordinatorNotificationPort, CoordinatorNotificationReceipt,
    },
    ports::{
        event_sink::RunEventSink, session_handle::SessionHandle, session_registry::SessionRegistry,
    },
};

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkerLaunchRequest {
    pub assignment: WorkerAssignment,
    pub permission_mode: crate::domain::run::PermissionMode,
    pub auto_allow: bool,
    pub goal: String,
    pub worktree_fingerprint: String,
}

#[derive(Debug, Clone)]
pub struct WorktreeMutationGuard {
    pub window_label: String,
    pub node_id: String,
    pub task_id: String,
    pub worktree_path: String,
    pub baseline: String,
}

static WORKTREE_GUARDS: OnceLock<Mutex<HashMap<String, WorktreeMutationGuard>>> = OnceLock::new();

fn worktree_guards() -> &'static Mutex<HashMap<String, WorktreeMutationGuard>> {
    WORKTREE_GUARDS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn take_worktree_guard(run_id: &str) -> Option<WorktreeMutationGuard> {
    worktree_guards().lock().ok()?.remove(run_id)
}

pub fn fingerprint_worktree(path: &str) -> Result<String, OrchestrationError> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for arguments in [
        vec!["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        vec!["diff", "--binary", "HEAD"],
        vec!["diff", "--cached", "--binary", "HEAD"],
    ] {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(arguments)
            .output()
            .map_err(|error| {
                OrchestrationError::new(
                    OrchestrationErrorCode::WorkerUnavailable,
                    format!("Failed to fingerprint worktree: {error}"),
                )
            })?;
        if !output.status.success() {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::InvalidInput,
                "Background workers require a valid Git worktree.",
            ));
        }
        output.stdout.hash(&mut hasher);
    }
    Ok(format!("{:016x}", hasher.finish()))
}

pub fn verify_worktree_unchanged(path: &str, baseline: &str) -> Result<(), OrchestrationError> {
    let current = fingerprint_worktree(path)?;
    if current == baseline {
        Ok(())
    } else {
        Err(OrchestrationError::new(
            OrchestrationErrorCode::ReadOnlyViolation,
            "The read-only child changed the worktree; changes were preserved for review.",
        ))
    }
}

pub trait AcpWorkerRuntime: Clone + Send + Sync + 'static {
    async fn start(&self, request: AgentWorkerLaunchRequest) -> Result<String, String>;

    async fn send(
        &self,
        binding: &WorkerBinding,
        message: &str,
        delivery: PromptDelivery,
    ) -> Result<(), String>;

    async fn send_and_wait(
        &self,
        binding: &WorkerBinding,
        message: &str,
        delivery: PromptDelivery,
    ) -> Result<(), String> {
        self.send(binding, message, delivery).await
    }

    async fn interrupt(&self, binding: &WorkerBinding) -> Result<(), String>;

    async fn cancel(&self, binding: &WorkerBinding) -> Result<(), String>;

    async fn is_active(&self, binding: &WorkerBinding) -> bool;
}

#[derive(Clone)]
pub struct AcpAgentWorkerAdapter<T> {
    runtime: T,
}

#[derive(Clone)]
pub struct TauriAcpWorkerRuntime {
    app: AppHandle,
    registry: AppState,
    mcp: McpServerState,
}

impl TauriAcpWorkerRuntime {
    pub fn new(app: AppHandle, registry: AppState, mcp: McpServerState) -> Self {
        Self { app, registry, mcp }
    }

    fn sink(&self, window_label: &str) -> TauriRunEventSink {
        TauriRunEventSink::with_target(self.app.clone(), self.registry.clone(), window_label.into())
    }
}

impl AcpWorkerRuntime for TauriAcpWorkerRuntime {
    async fn start(&self, request: AgentWorkerLaunchRequest) -> Result<String, String> {
        let assignment = &request.assignment;
        let env = self
            .mcp
            .launch_env_for_principal(CapabilityPrincipal::child(
                assignment.workspace_id.clone(),
                assignment.window_label.clone(),
                assignment.node_id.clone(),
                assignment.planned_run_id.clone(),
                assignment.task_id.clone(),
            ))
            .map_err(|error| error.to_string())?;
        let run_request = build_worker_request(&request, env);
        let session_store = JsonAcpSessionStore::from_app(&self.app)?;
        let runner = AcpAgentRunner::new(
            ConfigurableAgentCatalog::from_env(),
            self.registry.permissions(),
            Arc::new(session_store),
        );
        let run = StartAgentRunUseCase::new(self.registry.clone())
            .execute(
                runner,
                self.sink(&assignment.window_label),
                run_request,
                Some(assignment.window_label.clone()),
            )
            .await
            .map_err(String::from)?;
        Ok(run.id)
    }

    async fn send(
        &self,
        binding: &WorkerBinding,
        message: &str,
        delivery: PromptDelivery,
    ) -> Result<(), String> {
        if delivery == PromptDelivery::Draft {
            return Err("Background workers do not accept draft delivery.".into());
        }
        if delivery == PromptDelivery::Queue {
            let session = self
                .registry
                .active_session(&binding.run_id)
                .await
                .ok_or_else(|| format!("unknown or finished run: {}", binding.run_id))?;
            let sink = self.sink(&binding.window_label);
            let run_id = binding.run_id.clone();
            let message = message.trim().to_string();
            tokio::spawn(async move {
                if let Err(error) = session.queue_prompt(sink.clone(), message).await {
                    sink.emit(
                        &run_id,
                        RunEvent::Error {
                            message: format!("queued prompt delivery failed: {error}"),
                        },
                    );
                }
            });
            return Ok(());
        }
        SendPromptUseCase::new(self.registry.clone())
            .execute(
                self.sink(&binding.window_label),
                binding.run_id.clone(),
                message.into(),
            )
            .await
            .map_err(String::from)
    }

    async fn interrupt(&self, binding: &WorkerBinding) -> Result<(), String> {
        CancelAgentRunUseCase::new(self.registry.clone())
            .execute(self.sink(&binding.window_label), binding.run_id.clone())
            .await;
        Ok(())
    }

    async fn send_and_wait(
        &self,
        binding: &WorkerBinding,
        message: &str,
        delivery: PromptDelivery,
    ) -> Result<(), String> {
        if delivery == PromptDelivery::Draft {
            return Err("Background workers do not accept draft delivery.".into());
        }
        let session = self
            .registry
            .active_session(&binding.run_id)
            .await
            .ok_or_else(|| format!("unknown or finished run: {}", binding.run_id))?;
        if delivery == PromptDelivery::Queue {
            session
                .queue_prompt(self.sink(&binding.window_label), message.trim().to_string())
                .await
                .map(|_| ())
                .map_err(|error| error.to_string())
        } else {
            session
                .send_prompt(self.sink(&binding.window_label), message.trim().to_string())
                .await
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
    }

    async fn cancel(&self, binding: &WorkerBinding) -> Result<(), String> {
        CancelAgentRunUseCase::new(self.registry.clone())
            .execute(self.sink(&binding.window_label), binding.run_id.clone())
            .await;
        Ok(())
    }

    async fn is_active(&self, binding: &WorkerBinding) -> bool {
        self.registry
            .active_owner_of(&binding.run_id)
            .await
            .as_deref()
            == Some(binding.window_label.as_str())
    }
}

impl<T> AcpAgentWorkerAdapter<T>
where
    T: AcpWorkerRuntime,
{
    pub fn new(runtime: T) -> Self {
        Self { runtime }
    }

    fn command_error(error: String) -> OrchestrationError {
        OrchestrationError::new(OrchestrationErrorCode::WorkerUnavailable, error).retryable()
    }
}

impl<T> AgentWorkerPort for AcpAgentWorkerAdapter<T>
where
    T: AcpWorkerRuntime,
{
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
        let goal = format!(
            "Role: {role}\nResponsibility: {responsibility}\n\nObjective:\n{objective}\n\nConstraints:\n{constraints}\n\nExpected result:\n{expected}\n\nYou must report the final structured result with aw_report_result.",
            role = assignment.role.name,
            responsibility = assignment.role.responsibility,
            objective = assignment.objective,
            constraints = assignment.constraints.join("\n- "),
            expected = assignment.expected_result,
        );
        let worktree_fingerprint = fingerprint_worktree(&assignment.worktree_path)?;
        let request = AgentWorkerLaunchRequest {
            assignment,
            permission_mode: crate::domain::run::PermissionMode::ReadOnly,
            auto_allow: true,
            goal,
            worktree_fingerprint,
        };
        let guard = WorktreeMutationGuard {
            window_label: request.assignment.window_label.clone(),
            node_id: request.assignment.node_id.clone(),
            task_id: request.assignment.task_id.clone(),
            worktree_path: request.assignment.worktree_path.clone(),
            baseline: request.worktree_fingerprint.clone(),
        };
        match self.runtime.start(request).await {
            Ok(run_id) => {
                if let Ok(mut guards) = worktree_guards().lock() {
                    guards.insert(run_id.clone(), guard);
                }
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
        self.runtime
            .send(binding, message, delivery)
            .await
            .map_err(Self::command_error)?;
        Ok(WorkerCommandOutcome {
            accepted: true,
            reason: None,
        })
    }

    async fn interrupt_worker(
        &self,
        binding: &WorkerBinding,
    ) -> Result<WorkerCommandOutcome, OrchestrationError> {
        self.runtime
            .interrupt(binding)
            .await
            .map_err(Self::command_error)?;
        Ok(WorkerCommandOutcome {
            accepted: true,
            reason: None,
        })
    }

    async fn cancel_worker(
        &self,
        binding: &WorkerBinding,
    ) -> Result<WorkerCommandOutcome, OrchestrationError> {
        self.runtime
            .cancel(binding)
            .await
            .map_err(Self::command_error)?;
        Ok(WorkerCommandOutcome {
            accepted: true,
            reason: None,
        })
    }

    async fn is_active(&self, binding: &WorkerBinding) -> bool {
        self.runtime.is_active(binding).await
    }
}

impl<T> CoordinatorNotificationPort for AcpAgentWorkerAdapter<T>
where
    T: AcpWorkerRuntime,
{
    async fn notify_coordinator(
        &self,
        binding: &WorkerBinding,
        notification: &crate::domain::agent_orchestration::CoordinatorNotification,
    ) -> Result<CoordinatorNotificationReceipt, OrchestrationError> {
        let message = format!(
            "Child report available: workspace={}, task={}, report={}, type={:?}. Call aw_collect_child_results with taskIds=[\"{}\"] now, then incorporate the structured report into the parent task.",
            binding.workspace_id,
            notification.task_id,
            notification.report_id,
            notification.report_type,
            notification.task_id,
        );
        self.runtime
            .send_and_wait(binding, &message, PromptDelivery::Queue)
            .await
            .map_err(Self::command_error)?;
        Ok(CoordinatorNotificationReceipt {
            accepted: true,
            reason: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::{
        domain::agent_orchestration::{
            AccessPolicy, AgentRoleProfile, CoordinatorNotification, CoordinatorNotificationStatus,
            PromptDelivery, TaskReportType, WorkerRuntimeProfile,
        },
        ports::agent_worker::{
            AgentWorkerPort, StartWorkerOutcome, WorkerAssignment, WorkerBinding,
            WorkerCommandOutcome,
        },
    };

    #[derive(Clone, Default)]
    struct FakeRuntime {
        requests: Arc<Mutex<Vec<AgentWorkerLaunchRequest>>>,
        sent: Arc<Mutex<Vec<(String, PromptDelivery)>>>,
    }

    impl AcpWorkerRuntime for FakeRuntime {
        async fn start(&self, request: AgentWorkerLaunchRequest) -> Result<String, String> {
            let run_id = request.assignment.planned_run_id.clone();
            self.requests.lock().unwrap().push(request);
            Ok(run_id)
        }

        async fn send(
            &self,
            _binding: &WorkerBinding,
            message: &str,
            delivery: PromptDelivery,
        ) -> Result<(), String> {
            self.sent.lock().unwrap().push((message.into(), delivery));
            Ok(())
        }

        async fn interrupt(&self, _binding: &WorkerBinding) -> Result<(), String> {
            Ok(())
        }

        async fn cancel(&self, _binding: &WorkerBinding) -> Result<(), String> {
            Ok(())
        }

        async fn is_active(&self, _binding: &WorkerBinding) -> bool {
            true
        }
    }

    fn assignment(supports_read_only: bool) -> WorkerAssignment {
        WorkerAssignment {
            workspace_id: "workspace-1".into(),
            window_label: "window-1".into(),
            worktree_path: std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            node_id: "child-1".into(),
            task_id: "task-1".into(),
            attempt: 1,
            planned_run_id: "run-1".into(),
            role: AgentRoleProfile::new("researcher", "Researcher", "조사", "근거 목록").unwrap(),
            objective: "구조를 조사한다.".into(),
            constraints: vec!["read-only".into()],
            expected_result: "구조화 결과".into(),
            runtime_profile: WorkerRuntimeProfile {
                agent_profile_id: "codex".into(),
                provider_id: "codex".into(),
                model_id: None,
                access_policy: AccessPolicy::ReadOnly,
                supports_read_only,
            },
            mcp_capability: "awcap_test".into(),
        }
    }

    #[tokio::test]
    async fn launches_with_read_only_profile_and_controls_the_same_binding() {
        let runtime = FakeRuntime::default();
        let adapter = AcpAgentWorkerAdapter::new(runtime.clone());
        let outcome = adapter.start_worker(assignment(true)).await.unwrap();
        assert_eq!(
            outcome,
            StartWorkerOutcome::Started {
                run_id: "run-1".into()
            }
        );
        let recorded = runtime.requests.lock().unwrap();
        assert_eq!(
            recorded[0].permission_mode,
            crate::domain::run::PermissionMode::ReadOnly
        );
        assert!(recorded[0].auto_allow);
        drop(recorded);

        let binding = WorkerBinding {
            workspace_id: "workspace-1".into(),
            window_label: "window-1".into(),
            node_id: "child-1".into(),
            task_id: "task-1".into(),
            run_id: "run-1".into(),
        };
        assert_eq!(
            adapter
                .send_prompt(&binding, "continue", PromptDelivery::Queue)
                .await
                .unwrap(),
            WorkerCommandOutcome {
                accepted: true,
                reason: None
            }
        );
        assert!(adapter.is_active(&binding).await);
    }

    #[tokio::test]
    async fn rejects_profiles_without_read_only_support() {
        let adapter = AcpAgentWorkerAdapter::new(FakeRuntime::default());
        let outcome = adapter.start_worker(assignment(false)).await.unwrap();
        assert!(matches!(
            outcome,
            StartWorkerOutcome::Failed { code, .. }
                if code == "unsupportedReadOnlyProfile"
        ));
    }

    #[tokio::test]
    async fn queues_actionable_child_report_notification_for_main() {
        let runtime = FakeRuntime::default();
        let adapter = AcpAgentWorkerAdapter::new(runtime.clone());
        let binding = WorkerBinding {
            workspace_id: "workspace-1".into(),
            window_label: "window-1".into(),
            node_id: "main-agent-run".into(),
            task_id: "task-1".into(),
            run_id: "main-run".into(),
        };
        let notification = CoordinatorNotification {
            id: "notification-1".into(),
            report_id: "report-1".into(),
            task_id: "task-1".into(),
            report_type: TaskReportType::Result,
            generation_id: "generation-1".into(),
            main_run_id: Some("main-run".into()),
            status: CoordinatorNotificationStatus::Pending,
            attempt_count: 0,
            failure: None,
            collected_at: None,
            created_at: "2026-07-27T00:00:00Z".into(),
            updated_at: "2026-07-27T00:00:00Z".into(),
        };

        let receipt = adapter
            .notify_coordinator(&binding, &notification)
            .await
            .unwrap();

        assert!(receipt.accepted);
        let sent = runtime.sent.lock().unwrap();
        assert_eq!(sent[0].1, PromptDelivery::Queue);
        assert!(sent[0].0.contains("aw_collect_child_results"));
        assert!(sent[0].0.contains(r#"taskIds=["task-1"]"#));
    }

    #[test]
    fn detects_worktree_mutation_without_reverting_it() {
        let directory = tempfile::tempdir().unwrap();
        for arguments in [
            vec!["init"],
            vec!["config", "user.email", "test@example.com"],
            vec!["config", "user.name", "Test"],
        ] {
            assert!(
                Command::new("git")
                    .arg("-C")
                    .arg(directory.path())
                    .args(arguments)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        std::fs::write(directory.path().join("tracked.txt"), "before").unwrap();
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(directory.path())
                .args(["add", "."])
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(directory.path())
                .args(["commit", "-m", "initial"])
                .status()
                .unwrap()
                .success()
        );
        let path = directory.path().to_string_lossy();
        let baseline = fingerprint_worktree(&path).unwrap();
        std::fs::write(directory.path().join("tracked.txt"), "after").unwrap();

        assert_eq!(
            verify_worktree_unchanged(&path, &baseline)
                .unwrap_err()
                .code,
            OrchestrationErrorCode::ReadOnlyViolation
        );
        assert_eq!(
            std::fs::read_to_string(directory.path().join("tracked.txt")).unwrap(),
            "after"
        );
    }
}
