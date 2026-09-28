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
///
/// `null`인 수는 아직 파생하지 않은 값이다(0 = 없음이 아니다). 어떤 필드가 그런지는 [`ServerStatusOutput::not_yet_derived`]에
/// 싣는다. 정지 판정은 모르는 값을 활동 작업으로 본다([`ActiveWorkDto::blocks_stop`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActiveWorkDto {
    /// 진행 중 turn·엔진 대기열 prompt·권한 대기가 있는 run.
    pub busy_runs: u64,
    pub orchestration_tasks: Option<u64>,
    /// 비우기 시작 전에 만든 대기 task(K로 배정 가능).
    pub queued_tasks: Option<u64>,
    /// 전달 prompt가 소비되지 않은 교환(데스크톱 임대가 있을 때만 셈).
    pub pending_exchanges: Option<u64>,
    /// 저장된 미전달 coordinator 알림(대상 coordinator run이 살아 있음).
    pub pending_notifications: Option<u64>,
    /// 이 프로세스가 적용 중인 ledger `pending`.
    pub pending_operations: Option<u64>,
    /// 받아들인 분리 호출(HTTP·MCP).
    pub accepted_calls: u64,
    /// 작업 관문의 활동 예약 수(A-turn·X-deliver·T-start·N-notify).
    pub reservations: u64,
}

impl ActiveWorkDto {
    /// 정지(유휴·`default`·`wait`)를 막는가. 파생하지 않은 수(`null`)는 보수적으로 활동 작업이 있다고 본다.
    pub fn blocks_stop(&self) -> bool {
        let known = [self.busy_runs, self.accepted_calls, self.reservations];
        let optional = [
            self.orchestration_tasks,
            self.queued_tasks,
            self.pending_exchanges,
            self.pending_notifications,
            self.pending_operations,
        ];
        known.iter().any(|count| *count > 0)
            || optional
                .iter()
                .any(|count| count.is_none_or(|count| count > 0))
    }
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
    /// 이전 세대에서 판정하지 못한 ledger `unknown`(활동 작업이 아님). `null` = 아직 파생하지 않음.
    pub unresolved_operations: Option<u64>,
    /// 임대가 없어 전달할 클라이언트가 없는 미소비 교환. `null` = 아직 파생하지 않음.
    pub undeliverable_exchanges: Option<Vec<String>>,
    /// 엔진 대기열 전달이 run 종료로 실패한 교환. `null` = 아직 파생하지 않음.
    pub failed_exchange_deliveries: Option<Vec<String>>,
    /// 비우기 전 준비 task 중 배정할 쪽(바쁜 coordinator·미전달 알림)이 없어 활동으로 세지 않은 것(task id). 저장돼 있어
    /// 복구할 수 있다(044 메인 세션 검토). `null` = 아직 파생하지 않음.
    #[serde(default)]
    pub deferred_tasks: Option<Vec<String>>,
    /// 전달 시도 상한을 넘어 재시도를 기다리는 실패 coordinator 알림 id(활동 작업 아님, 저장된 채 재시도 가능한 실패로
    /// 남음). `None`은 아직 파생하지 않음.
    #[serde(default)]
    pub stalled_notifications: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle_since: Option<String>,
    /// 이 서버가 아직 파생하지 않는 필드의 JSON 경로(예: `activeWork.pendingExchanges`, `idleSince`). 목록에 있는 필드의
    /// `null`·부재는 "없음"이 아니라 "모름"이다. 모두 파생하면 빈 배열.
    pub not_yet_derived: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn known_zero() -> ActiveWorkDto {
        ActiveWorkDto {
            orchestration_tasks: Some(0),
            queued_tasks: Some(0),
            pending_exchanges: Some(0),
            pending_notifications: Some(0),
            pending_operations: Some(0),
            ..ActiveWorkDto::default()
        }
    }

    #[test]
    fn unknown_counts_block_a_stop_and_known_zeros_do_not() {
        assert!(!known_zero().blocks_stop());
        assert!(ActiveWorkDto::default().blocks_stop(), "all unknown blocks");
        let one_unknown = ActiveWorkDto {
            pending_exchanges: None,
            ..known_zero()
        };
        assert!(one_unknown.blocks_stop());
        let busy = ActiveWorkDto {
            busy_runs: 1,
            ..known_zero()
        };
        assert!(busy.blocks_stop());
    }

    #[test]
    fn unknown_counts_serialize_as_null_not_zero() {
        let json = serde_json::to_value(ActiveWorkDto::default()).unwrap();
        assert_eq!(json["pendingExchanges"], serde_json::Value::Null);
        assert_eq!(json["busyRuns"], 0);
    }
}
