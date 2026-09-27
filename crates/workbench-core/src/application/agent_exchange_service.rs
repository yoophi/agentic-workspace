//! 교환 서비스(040: AW에서 이동). 작업 영역은 작업대에 묶이고, 소유 조회는 run 엔진(작업대 id)이다. 백엔드는
//! 프롬프트를 보내지 않는다 — 요청 이벤트를 받은 화면이 패널에 라우팅하고 확인한다.

use crate::{
    domain::agent_exchange::{
        validate_exchange_message, validate_workspace_request, AgentExchange,
        AgentExchangeAckRequest, AgentExchangeEndpointRef, AgentExchangeError, AgentExchangeStatus,
        AgentPanelEndpoint, AgentPanelStatus, AgentWorkspaceSnapshot, AgentWorkspaceSyncRequest,
        AgentWorkspaceSyncResponse, SendAgentExchangeRequest,
    },
    ports::agent_workspace_registry::{
        AgentExchangeEventSink, AgentRunOwnerLookup, AgentWorkspaceRegistry, StoreExchangeOutcome,
    },
};

pub struct AgentExchangeService<R, O, S>
where
    R: AgentWorkspaceRegistry,
    O: AgentRunOwnerLookup,
    S: AgentExchangeEventSink,
{
    registry: R,
    owners: O,
    sink: S,
}

impl<R, O, S> AgentExchangeService<R, O, S>
where
    R: AgentWorkspaceRegistry,
    O: AgentRunOwnerLookup,
    S: AgentExchangeEventSink,
{
    pub fn new(registry: R, owners: O, sink: S) -> Self {
        Self {
            registry,
            owners,
            sink,
        }
    }

    pub async fn sync_workspace(
        &self,
        bench_id: String,
        request: AgentWorkspaceSyncRequest,
    ) -> Result<AgentWorkspaceSyncResponse, AgentExchangeError> {
        validate_workspace_request(&request)?;
        if request.worktree_path.trim().is_empty() {
            return Err(AgentExchangeError::new(
                "invalidWorktree",
                "Canonical worktree path is required.",
            ));
        }
        for panel in &request.panels {
            if let Some(run_id) = &panel.run_id {
                let owner = self.owners.active_owner_for_exchange(run_id).await;
                if owner.as_deref() != Some(bench_id.as_str()) {
                    return Err(AgentExchangeError::new(
                        "staleSourceRun",
                        "Panel run is inactive or owned by another window.",
                    ));
                }
            }
        }
        self.registry
            .sync_snapshot(AgentWorkspaceSnapshot {
                bench_id,
                worktree_path: request.worktree_path,
                revision: request.revision,
                focused_panel_id: request.focused_panel_id,
                panels: request.panels,
            })
            .await
    }

    pub async fn list_peers_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<AgentPanelEndpoint>, AgentExchangeError> {
        let bench_id = self
            .owners
            .active_owner_for_exchange(run_id)
            .await
            .ok_or_else(|| {
                AgentExchangeError::new("unknownSource", "Source agent run is not active.")
            })?;
        let snapshot = self.registry.snapshot(&bench_id).await.ok_or_else(|| {
            AgentExchangeError::new("unknownWorkspace", "Agent workspace is not registered.")
        })?;
        Ok(snapshot
            .panels
            .into_iter()
            .filter(|panel| {
                panel.run_id.as_deref() != Some(run_id) && panel.status != AgentPanelStatus::Closing
            })
            .collect())
    }

    pub async fn send_user_exchange(
        &self,
        bench_id: &str,
        request: SendAgentExchangeRequest,
    ) -> Result<AgentExchange, AgentExchangeError> {
        self.send(bench_id, request, None).await
    }

    pub async fn send_agent_exchange(
        &self,
        source_run_id: &str,
        mut request: SendAgentExchangeRequest,
    ) -> Result<AgentExchange, AgentExchangeError> {
        let bench_id = self
            .owners
            .active_owner_for_exchange(source_run_id)
            .await
            .ok_or_else(|| {
                AgentExchangeError::new("unknownSource", "Source agent run is not active.")
            })?;
        let snapshot = self.registry.snapshot(&bench_id).await.ok_or_else(|| {
            AgentExchangeError::new("unknownWorkspace", "Agent workspace is not registered.")
        })?;
        let source = snapshot
            .panels
            .iter()
            .find(|panel| panel.run_id.as_deref() == Some(source_run_id))
            .ok_or_else(|| {
                AgentExchangeError::new(
                    "staleSourceRun",
                    "Source run is not attached to a registered panel.",
                )
            })?;
        request.source_panel_id = source.panel_id.clone();
        request.source_run_id = Some(source_run_id.to_string());
        self.send(&bench_id, request, Some(source_run_id)).await
    }

    async fn send(
        &self,
        bench_id: &str,
        request: SendAgentExchangeRequest,
        required_source_run: Option<&str>,
    ) -> Result<AgentExchange, AgentExchangeError> {
        let snapshot = self.registry.snapshot(bench_id).await.ok_or_else(|| {
            AgentExchangeError::new("unknownWorkspace", "Agent workspace is not registered.")
        })?;
        let source = find_panel(&snapshot.panels, &request.source_panel_id, "unknownSource")?;
        let target = find_panel(&snapshot.panels, &request.target_panel_id, "unknownTarget")?;
        if target.status == AgentPanelStatus::Closing {
            return Err(AgentExchangeError::new(
                "targetClosing",
                "Target panel is closing.",
            ));
        }
        if required_source_run.is_some_and(|required| source.run_id.as_deref() != Some(required)) {
            return Err(AgentExchangeError::new(
                "staleSourceRun",
                "Source run no longer matches the source panel.",
            ));
        }
        if request.source_run_id.is_some() && request.source_run_id != source.run_id {
            return Err(AgentExchangeError::new(
                "staleSourceRun",
                "Source run no longer matches the source panel.",
            ));
        }
        if request.target_run_id.is_some() && request.target_run_id != target.run_id {
            return Err(AgentExchangeError::new(
                "staleTargetRun",
                "Target run no longer matches the target panel.",
            ));
        }
        let message = validate_exchange_message(&request.message)?;
        let now = chrono::Utc::now().to_rfc3339();
        let exchange = AgentExchange {
            request_id: request.request_id,
            bench_id: snapshot.bench_id,
            worktree_path: snapshot.worktree_path,
            source: endpoint_ref(source),
            target: endpoint_ref(target),
            message,
            delivery: request.delivery,
            status: AgentExchangeStatus::Accepted,
            failure_code: None,
            failure_reason: None,
            created_at: now.clone(),
            updated_at: now,
        };
        match self.registry.store_exchange(exchange).await? {
            StoreExchangeOutcome::Existing(existing) => Ok(existing),
            StoreExchangeOutcome::Stored(stored) => {
                if let Err(error) = self.sink.emit_requested(&stored) {
                    let failed = self
                        .registry
                        .transition_exchange(
                            bench_id,
                            &stored.request_id,
                            AgentExchangeStatus::Failed,
                            Some(error.code),
                            Some(error.message),
                        )
                        .await?;
                    let _ = self.sink.emit_status(&failed);
                    return Ok(failed);
                }
                self.sink.emit_status(&stored)?;
                Ok(stored)
            }
        }
    }

    pub async fn acknowledge(
        &self,
        bench_id: &str,
        request: AgentExchangeAckRequest,
    ) -> Result<AgentExchange, AgentExchangeError> {
        if !matches!(
            request.outcome,
            AgentExchangeStatus::Delivered
                | AgentExchangeStatus::Rejected
                | AgentExchangeStatus::Failed
                | AgentExchangeStatus::Cancelled
        ) {
            return Err(AgentExchangeError::new(
                "invalidTransition",
                "Acknowledgement must use a terminal outcome.",
            ));
        }
        let current = self
            .registry
            .exchange(bench_id, &request.request_id)
            .await
            .ok_or_else(|| AgentExchangeError::new("unknownExchange", "Exchange was not found."))?;
        if current.target.panel_id != request.target_panel_id {
            return Err(AgentExchangeError::new(
                "unknownTarget",
                "Acknowledgement target does not match the exchange.",
            ));
        }
        // 같은 결과의 두 번째 확인은 값만 돌려주고 상태 이벤트를 다시 내지 않는다(040 Q5, 요청 id 기준 멱등).
        if current.status == request.outcome {
            return Ok(current);
        }
        let next = self
            .registry
            .transition_exchange(
                bench_id,
                &request.request_id,
                request.outcome,
                request
                    .reason
                    .as_ref()
                    .map(|_| "deliveryRejected".to_string()),
                request.reason,
            )
            .await?;
        self.sink.emit_status(&next)?;
        Ok(next)
    }

    pub async fn exchange_for_source_run(
        &self,
        source_run_id: &str,
        request_id: &str,
    ) -> Result<AgentExchange, AgentExchangeError> {
        let bench_id = self
            .owners
            .active_owner_for_exchange(source_run_id)
            .await
            .ok_or_else(|| {
                AgentExchangeError::new("unknownSource", "Source agent run is not active.")
            })?;
        let exchange = self
            .registry
            .exchange(&bench_id, request_id)
            .await
            .ok_or_else(|| AgentExchangeError::new("unknownExchange", "Exchange was not found."))?;
        if exchange.source.run_id.as_deref() != Some(source_run_id) {
            return Err(AgentExchangeError::new(
                "unknownExchange",
                "Exchange does not belong to the source run.",
            ));
        }
        Ok(exchange)
    }

    pub async fn list_exchanges(&self, bench_id: &str) -> Vec<AgentExchange> {
        self.registry.list_exchanges(bench_id).await
    }
}

fn find_panel<'a>(
    panels: &'a [AgentPanelEndpoint],
    panel_id: &str,
    code: &str,
) -> Result<&'a AgentPanelEndpoint, AgentExchangeError> {
    panels
        .iter()
        .find(|panel| panel.panel_id == panel_id)
        .ok_or_else(|| AgentExchangeError::new(code, format!("Panel was not found: {panel_id}")))
}

fn endpoint_ref(panel: &AgentPanelEndpoint) -> AgentExchangeEndpointRef {
    AgentExchangeEndpointRef {
        panel_id: panel.panel_id.clone(),
        title: panel.title.clone(),
        run_id: panel.run_id.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use tokio::sync::Mutex;

    use super::*;
    use crate::{
        infrastructure::exchange::in_memory_workspace_registry::InMemoryAgentWorkspaceRegistry,
        ports::agent_workspace_registry::{AgentExchangeEventSink, AgentRunOwnerLookup},
    };

    #[derive(Clone, Default)]
    struct Owners(Arc<Mutex<HashMap<String, String>>>);
    impl AgentRunOwnerLookup for Owners {
        async fn active_owner_for_exchange(&self, run_id: &str) -> Option<String> {
            self.0.lock().await.get(run_id).cloned()
        }
    }

    #[derive(Clone, Default)]
    struct Sink;
    impl AgentExchangeEventSink for Sink {
        fn emit_requested(&self, _exchange: &AgentExchange) -> Result<(), AgentExchangeError> {
            Ok(())
        }
        fn emit_status(&self, _exchange: &AgentExchange) -> Result<(), AgentExchangeError> {
            Ok(())
        }
    }

    fn panel(id: &str, run_id: Option<&str>) -> AgentPanelEndpoint {
        AgentPanelEndpoint {
            panel_id: id.into(),
            title: id.into(),
            run_id: run_id.map(str::to_string),
            status: if run_id.is_some() {
                AgentPanelStatus::Running
            } else {
                AgentPanelStatus::Idle
            },
        }
    }

    #[tokio::test]
    async fn validates_run_owners_and_stale_targets() {
        let owners = Owners::default();
        owners
            .0
            .lock()
            .await
            .insert("run-main".into(), "session-a".into());
        let registry = InMemoryAgentWorkspaceRegistry::default();
        let service = AgentExchangeService::new(registry, owners, Sink);
        service
            .sync_workspace(
                "session-a".into(),
                AgentWorkspaceSyncRequest {
                    worktree_path: "/repo".into(),
                    revision: 1,
                    focused_panel_id: "main".into(),
                    panels: vec![panel("main", Some("run-main")), panel("extra", None)],
                },
            )
            .await
            .unwrap();

        let error = service
            .send_user_exchange(
                "session-a",
                SendAgentExchangeRequest {
                    request_id: "request-1".into(),
                    source_panel_id: "main".into(),
                    source_run_id: Some("run-main".into()),
                    target_panel_id: "extra".into(),
                    target_run_id: Some("stale".into()),
                    message: "hello".into(),
                    delivery: crate::domain::agent_exchange::AgentExchangeDelivery::Queue,
                },
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, "staleTargetRun");
    }
}
