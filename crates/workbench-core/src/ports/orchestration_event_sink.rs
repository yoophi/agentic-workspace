//! Port for publishing orchestration state changes.

use serde::{Deserialize, Serialize};

use crate::domain::agent_orchestration::OrchestrationError;

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationEvent {
    pub workspace_id: String,
    pub revision: u64,
    pub reason: String,
    pub task_id: Option<String>,
    pub node_id: Option<String>,
}

pub trait OrchestrationEventSink: Send + Sync {
    fn emit(&self, bench_id: &str, event: OrchestrationEvent) -> Result<(), OrchestrationError>;
}

#[cfg(test)]
mod tests {
    use workbench_protocol::events::orchestration::OrchestrationEventDto;

    use super::OrchestrationEvent;

    /// 039: protocol `OrchestrationEventDto`는 스키마 생성용 미러다. 지금 창으로 보내는 JSON과 같아야 한다.
    #[test]
    fn orchestration_event_wire_parity() {
        for (task_id, node_id) in [(None, None), (Some("t1".to_owned()), Some("n1".to_owned()))] {
            let domain = serde_json::to_value(OrchestrationEvent {
                workspace_id: "w1".into(),
                revision: 7,
                reason: "taskUpdated".into(),
                task_id,
                node_id,
            })
            .unwrap();
            let dto: OrchestrationEventDto = serde_json::from_value(domain.clone()).unwrap();
            assert_eq!(serde_json::to_value(&dto).unwrap(), domain);
        }
    }
}
