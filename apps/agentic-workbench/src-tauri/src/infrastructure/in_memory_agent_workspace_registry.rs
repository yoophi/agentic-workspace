use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

use crate::{
    domain::agent_exchange::{
        AgentExchange, AgentExchangeError, AgentExchangeStatus, AgentWorkspaceSnapshot,
        AgentWorkspaceSyncResponse, can_transition_exchange,
    },
    ports::agent_workspace_registry::{
        AgentExchangeEventSink, AgentRunOwnerLookup, AgentWorkspaceRegistry, StoreExchangeOutcome,
    },
};

const MAX_RETAINED_EXCHANGES: usize = 500;

#[derive(Clone, Default)]
pub struct InMemoryAgentWorkspaceRegistry {
    inner: Arc<Mutex<RegistryData>>,
}

pub const AGENT_EXCHANGE_REQUESTED_EVENT: &str = "agent-exchange-requested";
pub const AGENT_EXCHANGE_STATUS_EVENT: &str = "agent-exchange-status";

impl AgentRunOwnerLookup for crate::infrastructure::agent_session_registry::AppState {
    async fn active_owner_for_exchange(&self, run_id: &str) -> Option<String> {
        self.active_owner_of(run_id).await
    }
}

#[derive(Clone)]
pub struct TauriAgentExchangeEventSink {
    app: AppHandle,
}

impl TauriAgentExchangeEventSink {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    fn emit_payload<T: serde::Serialize>(
        &self,
        window_label: &str,
        event_name: &str,
        payload: &T,
    ) -> Result<(), AgentExchangeError> {
        let window = self.app.get_webview_window(window_label).ok_or_else(|| {
            AgentExchangeError::new(
                "windowUnavailable",
                "Owner Worktree Session window is unavailable.",
            )
        })?;
        window.emit(event_name, payload).map_err(|error| {
            AgentExchangeError::new(
                "deliveryFailed",
                format!("Failed to emit exchange: {error}"),
            )
        })?;
        if let Ok(serialized) = serde_json::to_string(payload) {
            let fallback = format!("{event_name}-fallback");
            let script = format!(
                "window.dispatchEvent(new CustomEvent('{fallback}', {{ detail: {serialized} }}));"
            );
            let _ = window.eval(&script);
        }
        Ok(())
    }
}

impl AgentExchangeEventSink for TauriAgentExchangeEventSink {
    fn emit_requested(&self, exchange: &AgentExchange) -> Result<(), AgentExchangeError> {
        self.emit_payload(
            &exchange.window_label,
            AGENT_EXCHANGE_REQUESTED_EVENT,
            &crate::domain::agent_exchange::AgentExchangeRequestedEvent::from(exchange),
        )
    }

    fn emit_status(&self, exchange: &AgentExchange) -> Result<(), AgentExchangeError> {
        self.emit_payload(
            &exchange.window_label,
            AGENT_EXCHANGE_STATUS_EVENT,
            exchange,
        )
    }
}

#[derive(Default)]
struct RegistryData {
    snapshots: HashMap<String, AgentWorkspaceSnapshot>,
    exchanges: HashMap<String, VecDeque<AgentExchange>>,
}

impl AgentWorkspaceRegistry for InMemoryAgentWorkspaceRegistry {
    async fn sync_snapshot(
        &self,
        snapshot: AgentWorkspaceSnapshot,
    ) -> Result<AgentWorkspaceSyncResponse, AgentExchangeError> {
        let mut inner = self.inner.lock().await;
        if let Some(current) = inner.snapshots.get(&snapshot.window_label)
            && current.revision > snapshot.revision
        {
            return Ok(AgentWorkspaceSyncResponse {
                revision: current.revision,
                accepted_panels: current.panels.len(),
            });
        }
        let response = AgentWorkspaceSyncResponse {
            revision: snapshot.revision,
            accepted_panels: snapshot.panels.len(),
        };
        inner
            .snapshots
            .insert(snapshot.window_label.clone(), snapshot);
        Ok(response)
    }

    async fn snapshot(&self, window_label: &str) -> Option<AgentWorkspaceSnapshot> {
        self.inner.lock().await.snapshots.get(window_label).cloned()
    }

    async fn store_exchange(
        &self,
        exchange: AgentExchange,
    ) -> Result<StoreExchangeOutcome, AgentExchangeError> {
        let mut inner = self.inner.lock().await;
        let queue = inner
            .exchanges
            .entry(exchange.window_label.clone())
            .or_default();
        if let Some(existing) = queue
            .iter()
            .find(|item| item.request_id == exchange.request_id)
        {
            if existing.source == exchange.source
                && existing.target == exchange.target
                && existing.message == exchange.message
                && existing.delivery == exchange.delivery
            {
                return Ok(StoreExchangeOutcome::Existing(existing.clone()));
            }
            return Err(AgentExchangeError::new(
                "duplicateConflict",
                "Request id was already used with a different exchange payload.",
            ));
        }
        queue.push_back(exchange.clone());
        while queue.len() > MAX_RETAINED_EXCHANGES {
            queue.pop_front();
        }
        Ok(StoreExchangeOutcome::Stored(exchange))
    }

    async fn exchange(&self, window_label: &str, request_id: &str) -> Option<AgentExchange> {
        self.inner
            .lock()
            .await
            .exchanges
            .get(window_label)?
            .iter()
            .find(|item| item.request_id == request_id)
            .cloned()
    }

    async fn transition_exchange(
        &self,
        window_label: &str,
        request_id: &str,
        status: AgentExchangeStatus,
        failure_code: Option<String>,
        failure_reason: Option<String>,
    ) -> Result<AgentExchange, AgentExchangeError> {
        let mut inner = self.inner.lock().await;
        let exchange = inner
            .exchanges
            .get_mut(window_label)
            .and_then(|queue| queue.iter_mut().find(|item| item.request_id == request_id))
            .ok_or_else(|| AgentExchangeError::new("unknownExchange", "Exchange was not found."))?;
        if exchange.status == status {
            return Ok(exchange.clone());
        }
        if !can_transition_exchange(exchange.status, status) {
            return Err(AgentExchangeError::new(
                "invalidTransition",
                "Exchange status cannot move backward or leave a terminal state.",
            ));
        }
        exchange.status = status;
        exchange.failure_code = failure_code;
        exchange.failure_reason = failure_reason;
        exchange.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(exchange.clone())
    }

    async fn list_exchanges(&self, window_label: &str) -> Vec<AgentExchange> {
        self.inner
            .lock()
            .await
            .exchanges
            .get(window_label)
            .map(|queue| queue.iter().cloned().collect())
            .unwrap_or_default()
    }

    async fn remove_window(&self, window_label: &str) {
        let mut inner = self.inner.lock().await;
        inner.snapshots.remove(window_label);
        inner.exchanges.remove(window_label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent_exchange::{
        AgentExchangeDelivery, AgentExchangeEndpointRef, AgentPanelEndpoint, AgentPanelStatus,
    };

    fn snapshot(revision: u64) -> AgentWorkspaceSnapshot {
        AgentWorkspaceSnapshot {
            window_label: "session-a".into(),
            worktree_path: "/repo".into(),
            revision,
            focused_panel_id: "main".into(),
            panels: vec![AgentPanelEndpoint {
                panel_id: "main".into(),
                title: "Main".into(),
                run_id: None,
                status: AgentPanelStatus::Idle,
            }],
        }
    }

    fn exchange(message: &str) -> AgentExchange {
        let endpoint = AgentExchangeEndpointRef {
            panel_id: "main".into(),
            title: "Main".into(),
            run_id: None,
        };
        AgentExchange {
            request_id: "request-1".into(),
            window_label: "session-a".into(),
            worktree_path: "/repo".into(),
            source: endpoint.clone(),
            target: endpoint,
            message: message.into(),
            delivery: AgentExchangeDelivery::Draft,
            status: AgentExchangeStatus::Accepted,
            failure_code: None,
            failure_reason: None,
            created_at: "now".into(),
            updated_at: "now".into(),
        }
    }

    #[tokio::test]
    async fn ignores_stale_snapshot_revisions() {
        let registry = InMemoryAgentWorkspaceRegistry::default();
        registry.sync_snapshot(snapshot(3)).await.unwrap();
        let response = registry.sync_snapshot(snapshot(2)).await.unwrap();
        assert_eq!(response.revision, 3);
        assert_eq!(registry.snapshot("session-a").await.unwrap().revision, 3);
    }

    #[tokio::test]
    async fn deduplicates_matching_payload_and_rejects_conflicts() {
        let registry = InMemoryAgentWorkspaceRegistry::default();
        assert!(matches!(
            registry.store_exchange(exchange("hello")).await.unwrap(),
            StoreExchangeOutcome::Stored(_)
        ));
        assert!(matches!(
            registry.store_exchange(exchange("hello")).await.unwrap(),
            StoreExchangeOutcome::Existing(_)
        ));
        assert_eq!(
            registry
                .store_exchange(exchange("different"))
                .await
                .unwrap_err()
                .code,
            "duplicateConflict"
        );
    }
}
