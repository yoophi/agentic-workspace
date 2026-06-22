use crate::domain::agent::AgentDescriptor;

pub trait AgentCatalog: Clone + Send + Sync + 'static {
    fn list_agents(&self) -> Vec<AgentDescriptor>;

    fn command_for_agent(&self, agent_id: &str) -> Option<String> {
        self.list_agents()
            .into_iter()
            .find(|agent| agent.id == agent_id)
            .map(|agent| agent.command)
    }
}
