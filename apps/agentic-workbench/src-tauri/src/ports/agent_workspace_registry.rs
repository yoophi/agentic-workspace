use crate::domain::agent_exchange::{
    AgentExchange, AgentExchangeError, AgentExchangeStatus, AgentWorkspaceSnapshot,
    AgentWorkspaceSyncResponse,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreExchangeOutcome {
    Stored(AgentExchange),
    Existing(AgentExchange),
}

// These ports are internal to the Tauri application, so callers do not need to
// impose additional auto-trait bounds on the returned futures.
#[allow(async_fn_in_trait)]
pub trait AgentWorkspaceRegistry: Clone + Send + Sync + 'static {
    async fn sync_snapshot(
        &self,
        snapshot: AgentWorkspaceSnapshot,
    ) -> Result<AgentWorkspaceSyncResponse, AgentExchangeError>;
    async fn snapshot(&self, window_label: &str) -> Option<AgentWorkspaceSnapshot>;
    async fn store_exchange(
        &self,
        exchange: AgentExchange,
    ) -> Result<StoreExchangeOutcome, AgentExchangeError>;
    async fn exchange(&self, window_label: &str, request_id: &str) -> Option<AgentExchange>;
    async fn transition_exchange(
        &self,
        window_label: &str,
        request_id: &str,
        status: AgentExchangeStatus,
        failure_code: Option<String>,
        failure_reason: Option<String>,
    ) -> Result<AgentExchange, AgentExchangeError>;
    async fn list_exchanges(&self, window_label: &str) -> Vec<AgentExchange>;
    async fn remove_window(&self, window_label: &str);
}

#[allow(async_fn_in_trait)]
pub trait AgentRunOwnerLookup: Clone + Send + Sync + 'static {
    async fn active_owner_for_exchange(&self, run_id: &str) -> Option<String>;
}

pub trait AgentExchangeEventSink: Clone + Send + Sync + 'static {
    fn emit_requested(&self, exchange: &AgentExchange) -> Result<(), AgentExchangeError>;
    fn emit_status(&self, exchange: &AgentExchange) -> Result<(), AgentExchangeError>;
}
