//! `orchestration.workspaceUpdated.v1` 본문: AW `ports::orchestration_event_sink::OrchestrationEvent`의 미러(039).
//! 041: 작업 영역의 현재 묶임 스트림 `orchestration:<bindingId>`로 발행한다(분류 상태 복원용).

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
