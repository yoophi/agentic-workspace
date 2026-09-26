//! 변경 진행 기록(operation ledger) port. 상태 전이와 재요청 규칙은 `specs/037-workbench-seam/data-model.md` §3.

use chrono::{DateTime, Utc};
use workbench_protocol::{IdempotencyKey, OperationId, PrincipalKind, RequestId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerState {
    Pending,
    Applied,
    Failed,
    Unknown,
}

impl LedgerState {
    pub fn as_str(self) -> &'static str {
        match self {
            LedgerState::Pending => "pending",
            LedgerState::Applied => "applied",
            LedgerState::Failed => "failed",
            LedgerState::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(LedgerState::Pending),
            "applied" => Some(LedgerState::Applied),
            "failed" => Some(LedgerState::Failed),
            "unknown" => Some(LedgerState::Unknown),
            _ => None,
        }
    }
}

/// 멱등성 namespace: principal 종류 · operation · 계약 revision · 키.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LedgerKey {
    pub principal_kind: PrincipalKind,
    pub operation: OperationId,
    pub contract_revision: u32,
    pub idempotency_key: IdempotencyKey,
}

/// `begin`에 넘기는 새 항목.
#[derive(Debug, Clone)]
pub struct NewLedgerEntry {
    pub key: LedgerKey,
    pub input_fingerprint: String,
    pub aggregate: String,
    pub reserved_resource_id: Option<String>,
    pub request_id: RequestId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LedgerRecord {
    pub execution_id: String,
    pub key: LedgerKey,
    pub input_fingerprint: String,
    pub aggregate: String,
    pub reserved_resource_id: Option<String>,
    pub state: LedgerState,
    pub result_json: Option<serde_json::Value>,
    pub revision: Option<u64>,
    pub request_id: RequestId,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReconcileSummary {
    pub applied: usize,
    pub unknown: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LedgerError {
    #[error("변경 기록 저장소를 사용할 수 없습니다: {0}")]
    Storage(String),
    #[error("같은 멱등성 키의 기록이 이미 있습니다.")]
    DuplicateKey,
    #[error("같은 resource id가 이미 예약되어 있습니다.")]
    DuplicateReservation,
    #[error("변경 기록을 찾을 수 없습니다: {0}")]
    NotFound(String),
    #[error("변경 기록 상태가 {0}이라 이 전이를 적용할 수 없습니다.")]
    InvalidState(&'static str),
    #[error("지원하지 않는 저장소 스키마 버전 {found}입니다 (지원: {supported}).")]
    UnsupportedSchema { found: i64, supported: i64 },
}

pub type LedgerResult<T> = Result<T, LedgerError>;

pub trait OperationLedger: Send + Sync {
    /// 스키마를 만들거나 버전을 확인한다. 기동 시 한 번.
    fn migrate(&self) -> LedgerResult<()>;

    fn find(&self, key: &LedgerKey) -> LedgerResult<Option<LedgerRecord>>;

    /// `pending` 항목을 commit하고 execution id를 돌려준다. 부작용보다 **먼저** 호출한다.
    fn begin(&self, entry: NewLedgerEntry) -> LedgerResult<String>;

    /// `pending → applied`. 같은 트랜잭션에서 aggregate revision을 +1 하고 그 값을 돌려준다.
    fn complete(&self, execution_id: &str, result: &serde_json::Value) -> LedgerResult<u64>;

    /// `pending → failed`. 적용되지 않은 확정 실패.
    fn fail(&self, execution_id: &str, fault: &serde_json::Value) -> LedgerResult<()>;

    /// 모든 `pending` 항목을 `resolve`로 판정한다. `Some(result)`면 `applied`(revision +1), `None`이면 `unknown`.
    /// 자동 재실행은 하지 않는다.
    fn reconcile_pending(
        &self,
        resolve: &mut dyn FnMut(&LedgerRecord) -> Option<serde_json::Value>,
    ) -> LedgerResult<ReconcileSummary>;

    /// 만료된 `applied`/`failed` 항목을 지운다. `pending`/`unknown`과 `aggregate_revision`은 건드리지 않는다.
    fn gc_expired(&self, now: DateTime<Utc>) -> LedgerResult<usize>;

    /// aggregate의 현재 revision. 기록이 없으면 0을 만들고 돌려준다.
    fn current_revision(&self, aggregate: &str) -> LedgerResult<u64>;
}
