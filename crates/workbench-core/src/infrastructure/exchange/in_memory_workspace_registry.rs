//! 교환 작업 영역 registry(040: AW `in_memory_agent_workspace_registry.rs`에서 이동). 작업대별 패널 스냅샷과
//! 교환 이력(최근 500개)을 메모리에 둔다. 이벤트 발행은 `hub_event_sink`(교환 스트림 + 데스크톱 전달)가 한다.

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    domain::agent_exchange::{
        can_transition_exchange, AgentExchange, AgentExchangeError, AgentExchangeStatus,
        AgentWorkspaceSnapshot, AgentWorkspaceSyncResponse,
    },
    ports::agent_workspace_registry::{AgentWorkspaceRegistry, StoreExchangeOutcome},
};

const MAX_RETAINED_EXCHANGES: usize = 500;

#[derive(Clone, Default)]
pub struct InMemoryAgentWorkspaceRegistry {
    inner: Arc<Mutex<RegistryData>>,
}

impl InMemoryAgentWorkspaceRegistry {
    fn data(&self) -> MutexGuard<'_, RegistryData> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 작업대 닫기 hook(동기)용.
    pub fn remove_bench_now(&self, bench_id: &str) {
        let mut inner = self.data();
        inner.snapshots.remove(bench_id);
        inner.exchanges.remove(bench_id);
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
        let mut inner = self.data();
        if let Some(current) = inner.snapshots.get(&snapshot.bench_id) {
            if current.revision > snapshot.revision {
                return Ok(AgentWorkspaceSyncResponse {
                    revision: current.revision,
                    accepted_panels: current.panels.len(),
                });
            }
        }
        let response = AgentWorkspaceSyncResponse {
            revision: snapshot.revision,
            accepted_panels: snapshot.panels.len(),
        };
        inner.snapshots.insert(snapshot.bench_id.clone(), snapshot);
        Ok(response)
    }

    async fn snapshot(&self, bench_id: &str) -> Option<AgentWorkspaceSnapshot> {
        self.data().snapshots.get(bench_id).cloned()
    }

    async fn store_exchange(
        &self,
        exchange: AgentExchange,
    ) -> Result<StoreExchangeOutcome, AgentExchangeError> {
        let mut inner = self.data();
        // 작업 영역이 없는(닫혀 지워진) 작업대에 큐를 새로 만들지 않는다. 서비스가 스냅샷을 읽은 뒤 닫기가 끼어들어도
        // 닫힌 작업대에 교환이 남지 않는다(040).
        if !inner.snapshots.contains_key(&exchange.bench_id) {
            return Err(AgentExchangeError::new(
                "unknownWorkspace",
                "Agent workspace is not registered.",
            ));
        }
        let queue = inner
            .exchanges
            .entry(exchange.bench_id.clone())
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

    async fn exchange(&self, bench_id: &str, request_id: &str) -> Option<AgentExchange> {
        self.data()
            .exchanges
            .get(bench_id)?
            .iter()
            .find(|item| item.request_id == request_id)
            .cloned()
    }

    async fn transition_exchange(
        &self,
        bench_id: &str,
        request_id: &str,
        status: AgentExchangeStatus,
        failure_code: Option<String>,
        failure_reason: Option<String>,
    ) -> Result<AgentExchange, AgentExchangeError> {
        let mut inner = self.data();
        let exchange = inner
            .exchanges
            .get_mut(bench_id)
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

    async fn list_exchanges(&self, bench_id: &str) -> Vec<AgentExchange> {
        self.data()
            .exchanges
            .get(bench_id)
            .map(|queue| queue.iter().cloned().collect())
            .unwrap_or_default()
    }

    async fn remove_bench(&self, bench_id: &str) {
        self.remove_bench_now(bench_id);
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
            bench_id: "session-a".into(),
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
            bench_id: "session-a".into(),
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
        registry.sync_snapshot(snapshot(1)).await.unwrap();
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

    #[tokio::test]
    async fn does_not_recreate_a_removed_bench_queue() {
        let registry = InMemoryAgentWorkspaceRegistry::default();
        registry.sync_snapshot(snapshot(1)).await.unwrap();
        registry.remove_bench_now("session-a");
        assert_eq!(
            registry
                .store_exchange(exchange("hello"))
                .await
                .unwrap_err()
                .code,
            "unknownWorkspace"
        );
        assert!(registry.data().exchanges.is_empty());
    }
}
