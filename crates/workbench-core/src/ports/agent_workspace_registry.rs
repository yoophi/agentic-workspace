//! 교환 작업 영역 포트(040: AW에서 이동). 키는 작업대 id다.

use crate::domain::agent_exchange::{
    AgentExchange, AgentExchangeError, AgentExchangeStatus, AgentWorkspaceSnapshot,
    AgentWorkspaceSyncResponse,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreExchangeOutcome {
    Stored(AgentExchange),
    Existing(AgentExchange),
}

// 구현은 한 crate 안의 구체 타입뿐이라 반환 future에 auto-trait 제약을 따로 두지 않는다.
#[allow(async_fn_in_trait)]
pub trait AgentWorkspaceRegistry: Clone + Send + Sync + 'static {
    async fn sync_snapshot(
        &self,
        snapshot: AgentWorkspaceSnapshot,
    ) -> Result<AgentWorkspaceSyncResponse, AgentExchangeError>;
    async fn snapshot(&self, bench_id: &str) -> Option<AgentWorkspaceSnapshot>;
    async fn store_exchange(
        &self,
        exchange: AgentExchange,
    ) -> Result<StoreExchangeOutcome, AgentExchangeError>;
    async fn exchange(&self, bench_id: &str, request_id: &str) -> Option<AgentExchange>;
    async fn transition_exchange(
        &self,
        bench_id: &str,
        request_id: &str,
        status: AgentExchangeStatus,
        failure_code: Option<String>,
        failure_reason: Option<String>,
    ) -> Result<AgentExchange, AgentExchangeError>;
    async fn list_exchanges(&self, bench_id: &str) -> Vec<AgentExchange>;
    async fn remove_bench(&self, bench_id: &str);
}

#[allow(async_fn_in_trait)]
pub trait AgentRunOwnerLookup: Clone + Send + Sync + 'static {
    async fn active_owner_for_exchange(&self, run_id: &str) -> Option<String>;
}

pub trait AgentExchangeEventSink: Clone + Send + Sync + 'static {
    fn emit_requested(&self, exchange: &AgentExchange) -> Result<(), AgentExchangeError>;
    fn emit_status(&self, exchange: &AgentExchange) -> Result<(), AgentExchangeError>;
}
