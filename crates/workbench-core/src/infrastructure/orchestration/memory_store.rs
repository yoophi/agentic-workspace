//! 메모리 orchestration 저장소(테스트·가짜 조립용). 인스턴스 하나가 저장 단위 하나이고 clone은 같은 단위를 공유한다.

use std::sync::{Arc, Mutex};

use crate::{
    domain::agent_orchestration::{OrchestrationError, OrchestrationSession},
    infrastructure::orchestration::store_boundary::{self, BoundaryTx, SessionStorage},
    ports::orchestration_repository::OrchestrationRepository,
};

#[derive(Clone)]
pub struct InMemoryOrchestrationRepository {
    sessions: Arc<Mutex<Vec<OrchestrationSession>>>,
    boundary: &'static Mutex<()>,
}

impl Default for InMemoryOrchestrationRepository {
    fn default() -> Self {
        Self::from_sessions(Vec::new())
    }
}

impl InMemoryOrchestrationRepository {
    pub fn from_sessions(sessions: Vec<OrchestrationSession>) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(sessions)),
            boundary: store_boundary::new_boundary(),
        }
    }
}

impl SessionStorage for InMemoryOrchestrationRepository {
    fn read_all(&self) -> Result<Vec<OrchestrationSession>, OrchestrationError> {
        Ok(self
            .sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone())
    }

    fn write_all(&self, sessions: &[OrchestrationSession]) -> Result<(), OrchestrationError> {
        *self
            .sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = sessions.to_vec();
        Ok(())
    }

    fn boundary(&self) -> &'static Mutex<()> {
        self.boundary
    }
}

impl OrchestrationRepository for InMemoryOrchestrationRepository {
    type Tx<'a> = BoundaryTx<'a, Self>;

    fn begin(&self) -> Result<Self::Tx<'_>, OrchestrationError> {
        store_boundary::begin(self)
    }

    fn snapshot(&self) -> Result<Vec<OrchestrationSession>, OrchestrationError> {
        store_boundary::snapshot(self)
    }
}
