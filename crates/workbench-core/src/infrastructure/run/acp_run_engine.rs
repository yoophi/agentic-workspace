//! 운영 run 엔진(040, research R3): acp-agent-core `AppState`(세션 registry·권한 broker) + `AcpAgentRunner` +
//! 세션 저장소로 기존 유스케이스를 그대로 호출한다. 오류 문구는 유스케이스의 `Display` 그대로다.

use std::sync::Arc;

use acp_agent_core::{
    application::{
        agent_run_errors::{
            SendPromptError, SetPermissionModeError, StartAgentRunError, SteerPromptError,
        },
        cancel_agent_run::CancelAgentRunUseCase,
        cancel_prompt_and_send::CancelPromptAndSendUseCase,
        send_prompt::SendPromptUseCase,
        set_permission_mode::SetPermissionModeUseCase,
        start_agent_run::StartAgentRunUseCase,
        steer_prompt::SteerPromptUseCase,
    },
    domain::events::RunEvent,
    domain::run::{AgentRun, AgentRunRequest, PermissionMode},
    infrastructure::{
        acp::runner::AcpAgentRunner, agent_catalog::ConfigurableAgentCatalog,
        agent_session_registry::AppState,
    },
    ports::{
        event_sink::RunEventSink,
        permission::PermissionDecision,
        session_handle::SessionHandle,
        session_registry::{ReserveRunError, SessionRegistry},
    },
};
use async_trait::async_trait;

use crate::{
    infrastructure::{
        fs::acp_session_store::JsonAcpSessionStore, run::workbench_run_sink::WorkbenchRunSink,
    },
    ports::run_engine::{RunEngine, RunEngineError, RunErrorKind},
};

pub struct AcpRunEngine {
    registry: AppState,
    session_store: Arc<JsonAcpSessionStore>,
}

impl AcpRunEngine {
    pub fn new(registry: AppState, session_store: Arc<JsonAcpSessionStore>) -> Self {
        Self {
            registry,
            session_store,
        }
    }
}

fn start_error(error: StartAgentRunError) -> RunEngineError {
    let kind = match &error {
        StartAgentRunError::ReserveRun(ReserveRunError::DuplicateRunId { .. }) => {
            RunErrorKind::Conflict
        }
        StartAgentRunError::ReserveRun(ReserveRunError::ConcurrentLimit { .. }) => {
            RunErrorKind::RateLimited
        }
        StartAgentRunError::AttachRunHandle(_) => RunErrorKind::Internal,
    };
    RunEngineError::new(kind, error.to_string())
}

pub(crate) fn send_error(error: SendPromptError) -> RunEngineError {
    let kind = match &error {
        SendPromptError::EmptyPrompt => RunErrorKind::InvalidArgument,
        SendPromptError::RunNotActive => RunErrorKind::NotFound,
        SendPromptError::DispatchFailed(_) => RunErrorKind::Internal,
    };
    RunEngineError::new(kind, error.to_string())
}

fn steer_error(error: SteerPromptError) -> RunEngineError {
    let kind = match &error {
        SteerPromptError::EmptyPrompt => RunErrorKind::InvalidArgument,
        SteerPromptError::RunNotActive => RunErrorKind::NotFound,
        SteerPromptError::Unsupported(_) => RunErrorKind::PreconditionFailed,
        SteerPromptError::DispatchFailed(_) => RunErrorKind::Internal,
    };
    RunEngineError::new(kind, error.to_string())
}

fn permission_mode_error(error: SetPermissionModeError) -> RunEngineError {
    let kind = match &error {
        SetPermissionModeError::RunNotActive => RunErrorKind::NotFound,
        SetPermissionModeError::Apply(_) => RunErrorKind::Internal,
    };
    RunEngineError::new(kind, error.to_string())
}

#[async_trait]
impl RunEngine for AcpRunEngine {
    async fn start(
        &self,
        request: AgentRunRequest,
        owner: &str,
        sink: WorkbenchRunSink,
    ) -> Result<AgentRun, RunEngineError> {
        let runner = AcpAgentRunner::new(
            ConfigurableAgentCatalog::from_env(),
            self.registry.permissions(),
            self.session_store.clone(),
        );
        StartAgentRunUseCase::new(self.registry.clone())
            .execute(runner, sink, request, Some(owner.to_owned()))
            .await
            .map_err(start_error)
    }

    async fn send_prompt(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        SendPromptUseCase::new(self.registry.clone())
            .execute(sink, run_id.to_owned(), prompt)
            .await
            .map_err(send_error)
    }

    async fn queue_prompt(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        let session = self
            .registry
            .active_session(run_id)
            .await
            .ok_or_else(|| unknown_or_finished(run_id))?;
        let run_id = run_id.to_owned();
        let message = prompt.trim().to_owned();
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
        Ok(())
    }

    async fn send_and_wait(
        &self,
        run_id: &str,
        prompt: String,
        queue: bool,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        let session = self
            .registry
            .active_session(run_id)
            .await
            .ok_or_else(|| unknown_or_finished(run_id))?;
        let message = prompt.trim().to_owned();
        let result = if queue {
            session.queue_prompt(sink, message).await
        } else {
            session.send_prompt(sink, message).await
        };
        result
            .map(|_| ())
            .map_err(|error| RunEngineError::new(RunErrorKind::Internal, error.to_string()))
    }

    async fn steer_prompt(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        SteerPromptUseCase::new(self.registry.clone())
            .execute(sink, run_id.to_owned(), prompt)
            .await
            .map_err(steer_error)
    }

    async fn cancel_current_prompt_and_send(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        CancelPromptAndSendUseCase::new(self.registry.clone())
            .execute(sink, run_id.to_owned(), prompt)
            .await
            .map_err(send_error)
    }

    async fn set_permission_mode(
        &self,
        run_id: &str,
        mode: PermissionMode,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        SetPermissionModeUseCase::new(self.registry.clone())
            .execute(sink, run_id.to_owned(), mode)
            .await
            .map_err(permission_mode_error)
    }

    async fn cancel(&self, run_id: &str, sink: WorkbenchRunSink) {
        CancelAgentRunUseCase::new(self.registry.clone())
            .execute(sink, run_id.to_owned())
            .await;
    }

    async fn respond_permission(
        &self,
        run_id: &str,
        permission_id: &str,
        option_id: &str,
    ) -> Result<(), RunEngineError> {
        self.registry
            .permissions()
            .respond_for_run(
                run_id,
                permission_id,
                PermissionDecision {
                    option_id: option_id.to_owned(),
                },
            )
            .await
            .map_err(|error| {
                let message = error.to_string();
                let kind = if message.starts_with("unknown or already answered permission") {
                    RunErrorKind::NotFound
                } else if message.contains("belongs to a different run") {
                    RunErrorKind::InvalidArgument
                } else {
                    RunErrorKind::PreconditionFailed
                };
                RunEngineError::new(kind, message)
            })
    }

    async fn owner_of(&self, run_id: &str) -> Option<String> {
        self.registry.owner_of(run_id).await
    }

    async fn active_owner_of(&self, run_id: &str) -> Option<String> {
        self.registry.active_owner_of(run_id).await
    }

    async fn cancel_runs_owned_by(&self, owner: &str) -> Vec<String> {
        self.registry.cancel_runs_owned_by(owner).await
    }

    fn acp_registry(&self) -> Option<AppState> {
        Some(self.registry.clone())
    }

    fn acp_session_store(&self) -> Option<Arc<JsonAcpSessionStore>> {
        Some(self.session_store.clone())
    }
}

fn unknown_or_finished(run_id: &str) -> RunEngineError {
    RunEngineError::new(
        RunErrorKind::NotFound,
        format!("unknown or finished run: {run_id}"),
    )
}
