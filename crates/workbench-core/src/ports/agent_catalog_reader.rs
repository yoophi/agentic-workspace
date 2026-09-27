//! agent catalog 조회 포트. acp-agent-core의 `AgentCatalog`는 `Clone` bound 때문에 `dyn`으로 쓸 수 없어서,
//! runtime이 주입받을 수 있도록 object-safe한 읽기 전용 포장을 둔다(acp-agent-core는 바꾸지 않는다).

use acp_agent_core::{domain::agent::AgentDescriptor, ports::agent_catalog::AgentCatalog};

pub trait AgentCatalogReader: Send + Sync {
    fn list_agents(&self) -> Vec<AgentDescriptor>;
}

impl<T: AgentCatalog> AgentCatalogReader for T {
    fn list_agents(&self) -> Vec<AgentDescriptor> {
        AgentCatalog::list_agents(self)
    }
}
