//! `server.*` operation(044): 독립 서버의 상태 조회와 정지 요청. 소유자 주체(`server:admin`)만 부른다.
//! 계약: `specs/044-standalone-server/contracts/server-lifecycle.md` §4·§5.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerStatusInput {}

/// 서버 상태(research R7·R14). 준비 상태 = `serving`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ServerStateDto {
    Starting,
    Serving,
    DrainingIdle,
    DrainingWait,
    Stopping,
}

/// 유휴·wait 정지를 막는 활동 작업(research R14). 세션 수가 아니라 바쁜 run을 센다.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActiveWorkDto {
    /// 진행 중 turn·엔진 대기열 prompt·권한 대기가 있는 run.
    pub busy_runs: u64,
    pub orchestration_tasks: u64,
    /// 비우기 시작 전에 만든 대기 task(K로 배정 가능).
    pub queued_tasks: u64,
    /// 전달 prompt가 소비되지 않은 교환(데스크톱 임대가 있을 때만 셈).
    pub pending_exchanges: u64,
    /// 저장된 미전달 coordinator 알림(대상 coordinator run이 살아 있음).
    pub pending_notifications: u64,
    /// 이 프로세스가 적용 중인 ledger `pending`.
    pub pending_operations: u64,
    /// 받아들인 분리 호출(HTTP·MCP).
    pub accepted_calls: u64,
    /// 작업 관문의 활동 예약 수(A-turn·X-deliver·T-start·N-notify).
    pub reservations: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatusOutput {
    pub state: ServerStateDto,
    pub instance_id: String,
    pub server_epoch: String,
    pub active_work: ActiveWorkDto,
    /// 쉬는 세션(바쁘지 않은 run). 활동 작업이 아니다.
    pub idle_runs: u64,
    pub leases: u64,
    /// 이전 세대에서 판정하지 못한 ledger `unknown`(활동 작업이 아님).
    pub unresolved_operations: u64,
    /// 임대가 없어 전달할 클라이언트가 없는 미소비 교환.
    pub undeliverable_exchanges: Vec<String>,
    /// 엔진 대기열 전달이 run 종료로 실패한 교환.
    pub failed_exchange_deliveries: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle_since: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum StopModeDto {
    /// 활동 작업이 있으면 `conflict`.
    Default,
    /// 비운 뒤 활동 작업이 끝나면 정지.
    Wait,
    /// 작업대를 모두 닫고(run 취소) 정지.
    Force,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerStopInput {
    pub mode: StopModeDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerStopOutput {
    pub state: ServerStateDto,
}
