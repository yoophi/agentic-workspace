//! 운영 run 엔진(040, research R3): acp-agent-core `AppState`(세션 registry·권한 broker) + `AcpAgentRunner` +
//! 세션 저장소로 기존 유스케이스를 그대로 호출한다. 오류 문구는 유스케이스의 `Display` 그대로다.
//!
//! 044 실행 수명 계약(research R14): 작업 관문이 붙으면 모든 prompt 실행 진입점이 A-turn을 **동기적으로** 예약하고, 그
//! 실행 future가 끝날 때(성공·오류·취소·abort drop) guard가 해제한다. 시작의 초기 prompt 순서는 runner가 순서 끝에서
//! guard를 놓는다. 이벤트로 추정하지 않는다(대기열 등록 이벤트가 없고, RPC 오류는 완료 이벤트 없이 끝난다).

use std::sync::{Arc, OnceLock};

use acp_agent_core::{
    application::{
        agent_run_errors::{
            SendPromptError, SetPermissionModeError, StartAgentRunError, SteerPromptError,
        },
        cancel_agent_run::CancelAgentRunUseCase,
        cancel_prompt_and_send::CancelPromptAndSendUseCase,
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
    application::work_gate::{Reservation, ReservationKind, WorkGate, MESSAGE_STOPPING},
    infrastructure::{
        fs::acp_session_store::JsonAcpSessionStore, run::workbench_run_sink::WorkbenchRunSink,
    },
    ports::run_engine::{QueuePromptCompletion, RunEngine, RunEngineError, RunErrorKind},
};

pub struct AcpRunEngine {
    registry: AppState,
    session_store: Arc<JsonAcpSessionStore>,
    work_gate: OnceLock<Arc<WorkGate>>,
}

impl AcpRunEngine {
    pub fn new(registry: AppState, session_store: Arc<JsonAcpSessionStore>) -> Self {
        Self {
            registry,
            session_store,
            work_gate: OnceLock::new(),
        }
    }

    /// `start`·`start_gated` 공통: `start_gate`가 있으면 유스케이스의 시작 장벽으로 넘긴다(R14).
    async fn start_with(
        &self,
        request: AgentRunRequest,
        owner: &str,
        sink: WorkbenchRunSink,
        start_gate: Option<tokio::sync::oneshot::Receiver<()>>,
    ) -> Result<AgentRun, RunEngineError> {
        let mut request = request;
        // 초기 prompt 순서의 예약은 run id로 센다: id가 없으면 여기서 정한다(유스케이스의 `build_run`과 같은 uuid).
        let run_id = match request.run_id.clone().filter(|id| !id.trim().is_empty()) {
            Some(id) => id,
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                request.run_id = Some(id.clone());
                id
            }
        };
        let mut runner = AcpAgentRunner::new(
            ConfigurableAgentCatalog::from_env(),
            self.registry.permissions(),
            self.session_store.clone(),
        );
        if let Some(guard) = self.reserve_turn(&run_id)? {
            runner = runner.with_initial_turn_guard(Box::new(guard));
        }
        StartAgentRunUseCase::new(self.registry.clone())
            .execute_gated(runner, sink, request, Some(owner.to_owned()), start_gate)
            .await
            .map_err(start_error)
    }

    /// A-turn을 동기 예약한다. 관문이 없으면(시험 조립 등) 예약 없이 진행한다. 정지 중이면 거절한다.
    fn reserve_turn(&self, run_id: &str) -> Result<Option<Reservation>, RunEngineError> {
        match self.work_gate.get() {
            None => Ok(None),
            Some(gate) => gate
                .reserve(ReservationKind::Turn, Some(run_id))
                .map(Some)
                .map_err(|_| RunEngineError::new(RunErrorKind::Unavailable, MESSAGE_STOPPING)),
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
    fn attach_work_gate(&self, gate: Arc<WorkGate>) {
        let _ = self.work_gate.set(gate);
    }

    async fn start(
        &self,
        request: AgentRunRequest,
        owner: &str,
        sink: WorkbenchRunSink,
    ) -> Result<AgentRun, RunEngineError> {
        self.start_with(request, owner, sink, None).await
    }

    async fn start_gated(
        &self,
        request: AgentRunRequest,
        owner: &str,
        sink: WorkbenchRunSink,
        start_gate: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<AgentRun, RunEngineError> {
        self.start_with(request, owner, sink, Some(start_gate))
            .await
    }

    async fn send_prompt(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        // `SendPromptUseCase`와 같은 검사·오류 문구. 전송 future를 여기서 spawn해 A-turn guard를 그 future에 묶는다.
        let trimmed = prompt.trim().to_owned();
        if trimmed.is_empty() {
            return Err(send_error(SendPromptError::EmptyPrompt));
        }
        let session = self
            .registry
            .active_session(run_id)
            .await
            .ok_or_else(|| send_error(SendPromptError::RunNotActive))?;
        let guard = self.reserve_turn(run_id)?;
        let run_id = run_id.to_owned();
        tokio::spawn(async move {
            let _guard = guard;
            if let Err(error) = session.send_prompt(sink.clone(), trimmed).await {
                sink.emit(
                    &run_id,
                    RunEvent::Error {
                        message: error.to_string(),
                    },
                );
            }
        });
        Ok(())
    }

    async fn queue_prompt(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        self.queue_prompt_with_completion(run_id, prompt, sink, Box::new(|_| {}))
            .await
    }

    async fn queue_prompt_with_completion(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
        completion: QueuePromptCompletion,
    ) -> Result<(), RunEngineError> {
        let session = self
            .registry
            .active_session(run_id)
            .await
            .ok_or_else(|| unknown_or_finished(run_id))?;
        let guard = self.reserve_turn(run_id)?;
        let run_id = run_id.to_owned();
        let message = prompt.trim().to_owned();
        tokio::spawn(async move {
            // 대기열 차례를 기다리는 동안도 바쁘다(현재 turn 뒤 차례로 보냄).
            let _guard = guard;
            let result = session
                .queue_prompt(sink.clone(), message)
                .await
                .map(|_| ())
                .map_err(|error| RunEngineError::new(RunErrorKind::Internal, error.to_string()));
            if let Err(error) = &result {
                sink.emit(
                    &run_id,
                    RunEvent::Error {
                        message: format!("queued prompt delivery failed: {error}"),
                    },
                );
            }
            completion(result);
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
        let _guard = self.reserve_turn(run_id)?;
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
        let _guard = self.reserve_turn(run_id)?;
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
        let _guard = self.reserve_turn(run_id)?;
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
}

fn unknown_or_finished(run_id: &str) -> RunEngineError {
    RunEngineError::new(
        RunErrorKind::NotFound,
        format!("unknown or finished run: {run_id}"),
    )
}
