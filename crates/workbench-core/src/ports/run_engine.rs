//! run 기계 포트(040, research R3). 운영 구현은 `infrastructure::run::acp_run_engine::AcpRunEngine`(acp-agent-core
//! `AppState` + `AcpAgentRunner`), 테스트는 스크립트형 가짜 엔진을 주입한다. `AppState`의 세션 타입이
//! `AcpSession`으로 고정이라 가짜 `SessionLauncher`와 조합할 수 없어서 한 단계 위(이 포트)에서 가짜를 넣는다.
//!
//! run의 소유자는 작업대 id 문자열이다(acp-agent-core의 소유자는 원래 불투명한 문자열, ADR core 0004).

use std::fmt;

use acp_agent_core::domain::run::{AgentRun, AgentRunRequest, PermissionMode};
use async_trait::async_trait;

use crate::infrastructure::run::workbench_run_sink::WorkbenchRunSink;

/// 엔진 오류의 분류. fault 코드로 옮겨지고, `message`는 오늘 문자열 그대로다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunErrorKind {
    InvalidArgument,
    NotFound,
    Conflict,
    RateLimited,
    PreconditionFailed,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunEngineError {
    pub kind: RunErrorKind,
    pub message: String,
}

impl RunEngineError {
    pub fn new(kind: RunErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for RunEngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RunEngineError {}

#[async_trait]
pub trait RunEngine: Send + Sync {
    /// run을 예약(소유 기록)하고 spawn한 뒤 돌아온다. 완료는 기다리지 않는다.
    async fn start(
        &self,
        request: AgentRunRequest,
        owner: &str,
        sink: WorkbenchRunSink,
    ) -> Result<AgentRun, RunEngineError>;

    async fn send_prompt(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError>;

    async fn steer_prompt(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError>;

    async fn cancel_current_prompt_and_send(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError>;

    async fn set_permission_mode(
        &self,
        run_id: &str,
        mode: PermissionMode,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError>;

    /// orchestration(041): 지금 턴 뒤에 이어 붙이고 기다리지 않는다. 전달 실패는 run 오류 이벤트
    /// (`queued prompt delivery failed: …`)로 낸다. run이 없으면 `unknown or finished run: <id>`.
    async fn queue_prompt(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError>;

    /// orchestration(041): 턴이 끝날 때까지 기다린다(coordinator 알림 전달). `queue`면 지금 턴 뒤에 이어 붙인다.
    async fn send_and_wait(
        &self,
        run_id: &str,
        prompt: String,
        queue: bool,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError>;

    /// 이미 끝난 run도 성공(오늘과 같음). 취소 lifecycle 이벤트를 sink로 낸다.
    async fn cancel(&self, run_id: &str, sink: WorkbenchRunSink);

    async fn respond_permission(
        &self,
        run_id: &str,
        permission_id: &str,
        option_id: &str,
    ) -> Result<(), RunEngineError>;

    /// 예약된 run의 소유자(끝나면 사라진다).
    async fn owner_of(&self, run_id: &str) -> Option<String>;

    /// 세션이 붙어 살아 있는 run의 소유자.
    async fn active_owner_of(&self, run_id: &str) -> Option<String>;

    /// 소유자의 run을 모두 취소하고 취소한 run id를 돌려준다.
    async fn cancel_runs_owned_by(&self, owner: &str) -> Vec<String>;
}
