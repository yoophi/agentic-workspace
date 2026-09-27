//! 교환 어댑터(040): 작업 영역 registry, 교환 스트림 발행 sink, run 엔진 기반 소유 조회.

pub mod hub_event_sink;
pub mod in_memory_workspace_registry;

use std::sync::Arc;

use crate::ports::{agent_workspace_registry::AgentRunOwnerLookup, run_engine::RunEngine};

/// run의 살아 있는 소유 작업대(run 엔진).
#[derive(Clone)]
pub struct EngineRunOwners(pub Arc<dyn RunEngine>);

impl AgentRunOwnerLookup for EngineRunOwners {
    async fn active_owner_for_exchange(&self, run_id: &str) -> Option<String> {
        self.0.active_owner_of(run_id).await
    }
}
