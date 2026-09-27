//! `orchestration.workspaceUpdated.v1` 본문: AW `ports::orchestration_event_sink::OrchestrationEvent`의 미러(039).
//! 2b 예약: 스키마만 계약에 싣고 구독은 아직 받지 않는다(`stream kind is not available yet.`).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationEventDto {
    pub workspace_id: String,
    pub revision: u64,
    pub reason: String,
    pub task_id: Option<String>,
    pub node_id: Option<String>,
}
