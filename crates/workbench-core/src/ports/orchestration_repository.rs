//! Persistence port for orchestration workspace aggregates.

use crate::domain::agent_orchestration::{OrchestrationError, OrchestrationSession};

pub trait OrchestrationRepository: Send + Sync {
    fn load_sessions(&self) -> Result<Vec<OrchestrationSession>, OrchestrationError>;

    fn save_sessions(&self, sessions: &[OrchestrationSession]) -> Result<(), OrchestrationError>;
}
