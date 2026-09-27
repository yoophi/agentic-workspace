//! 재시작 판정(reconciler, research R6). `bootstrap`이 `pending` 기록마다 operation에 등록된 `Reconciler`에게
//! 묻고, `Some(result)`면 `applied`, `None`이면 `unknown`으로 닫는다. 자동 재실행은 없다(FR-009).
//!
//! 규칙 3종:
//! - 서버가 id를 만드는 생성: 예약 id가 컬렉션에 있으면 applied (`JsonCreateReconciler`)
//! - 삭제: 대상 id가 컬렉션에 없으면 applied (`JsonDeleteReconciler`)
//! - Git worktree 생성·삭제: 종료 상태 규칙 (`git_worktree.rs`)
//! - upsert·수정: 등록하지 않는다 → 항상 unknown

pub mod git_worktree;

use std::{collections::HashMap, sync::Arc};

use serde_json::Value;
use workbench_protocol::OperationId;

use crate::ports::operation_ledger::LedgerRecord;

pub trait Reconciler: Send + Sync {
    /// `Some(result_json)` = 적용됨(그 결과로 `applied`), `None` = 불명.
    fn resolve(&self, record: &LedgerRecord) -> Option<Value>;
}

#[derive(Default)]
pub struct ReconcilerRegistry {
    reconcilers: HashMap<OperationId, Arc<dyn Reconciler>>,
}

impl ReconcilerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, id: OperationId, reconciler: Arc<dyn Reconciler>) {
        self.reconcilers.insert(id, reconciler);
    }

    pub fn has(&self, id: OperationId) -> bool {
        self.reconcilers.contains_key(&id)
    }

    /// 등록되지 않은 operation(upsert·수정)은 `None` → unknown.
    pub fn resolve(&self, record: &LedgerRecord) -> Option<Value> {
        self.reconcilers.get(&record.key.operation)?.resolve(record)
    }
}

/// 컬렉션을 DTO JSON 배열로 읽는 closure. lock은 closure 안(coordinator)에서 잡는다. 읽기 실패는 `None`(→ unknown).
pub type LoadCollection = Box<dyn Fn() -> Option<Vec<Value>> + Send + Sync>;

/// 예약 id(`reserved_resource_id`)와 `id_field`가 같은 항목이 있으면 그 항목을 결과로 applied.
pub struct JsonCreateReconciler {
    load: LoadCollection,
    id_field: &'static str,
}

impl JsonCreateReconciler {
    pub fn new(load: LoadCollection, id_field: &'static str) -> Self {
        Self { load, id_field }
    }
}

impl Reconciler for JsonCreateReconciler {
    fn resolve(&self, record: &LedgerRecord) -> Option<Value> {
        let reserved = record.reserved_resource_id.as_deref()?;
        let items = (self.load)()?;
        items
            .into_iter()
            .find(|item| item.get(self.id_field).and_then(Value::as_str) == Some(reserved))
    }
}

/// 대상 id(`reserved_resource_id`)가 컬렉션에 **없으면** applied(결과 `null`). 있으면 unknown.
pub struct JsonDeleteReconciler {
    load: LoadCollection,
    id_field: &'static str,
}

impl JsonDeleteReconciler {
    pub fn new(load: LoadCollection, id_field: &'static str) -> Self {
        Self { load, id_field }
    }
}

impl Reconciler for JsonDeleteReconciler {
    fn resolve(&self, record: &LedgerRecord) -> Option<Value> {
        let target = record.reserved_resource_id.as_deref()?;
        let items = (self.load)()?;
        let still_present = items
            .iter()
            .any(|item| item.get(self.id_field).and_then(Value::as_str) == Some(target));
        (!still_present).then_some(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use workbench_protocol::{IdempotencyKey, PrincipalKind, RequestId, CONTRACT_REVISION};

    use super::*;
    use crate::ports::operation_ledger::{LedgerKey, LedgerState};

    fn record(operation: OperationId, reserved: Option<&str>) -> LedgerRecord {
        LedgerRecord {
            execution_id: "exec".into(),
            key: LedgerKey {
                principal_kind: PrincipalKind::Desktop,
                operation,
                contract_revision: CONTRACT_REVISION,
                idempotency_key: IdempotencyKey::new("k").unwrap(),
            },
            input_fingerprint: "fp".into(),
            aggregate: "projects".into(),
            reserved_resource_id: reserved.map(str::to_owned),
            state: LedgerState::Pending,
            result_json: None,
            revision: None,
            request_id: RequestId::new("r").unwrap(),
            created_at: String::new(),
            updated_at: String::new(),
            expires_at: None,
        }
    }

    fn load(items: Vec<Value>) -> LoadCollection {
        Box::new(move || Some(items.clone()))
    }

    #[test]
    fn create_reconciler_applies_only_when_reserved_id_exists() {
        let reconciler =
            JsonCreateReconciler::new(load(vec![json!({"id": "p1", "name": "A"})]), "id");
        assert_eq!(
            reconciler.resolve(&record(OperationId::ProjectCreate, Some("p1"))),
            Some(json!({"id": "p1", "name": "A"}))
        );
        assert_eq!(
            reconciler.resolve(&record(OperationId::ProjectCreate, Some("p2"))),
            None
        );
        assert_eq!(
            reconciler.resolve(&record(OperationId::ProjectCreate, None)),
            None
        );
    }

    #[test]
    fn delete_reconciler_applies_when_target_is_gone() {
        let reconciler = JsonDeleteReconciler::new(load(vec![json!({"id": "p1"})]), "id");
        assert_eq!(
            reconciler.resolve(&record(OperationId::ProjectCreate, Some("p2"))),
            Some(Value::Null)
        );
        assert_eq!(
            reconciler.resolve(&record(OperationId::ProjectCreate, Some("p1"))),
            None
        );
    }

    #[test]
    fn unregistered_operation_is_unknown_and_load_failure_is_unknown() {
        let mut registry = ReconcilerRegistry::new();
        assert_eq!(
            registry.resolve(&record(OperationId::ProjectCreate, Some("p1"))),
            None
        );
        registry.register(
            OperationId::ProjectCreate,
            Arc::new(JsonCreateReconciler::new(Box::new(|| None), "id")),
        );
        assert!(registry.has(OperationId::ProjectCreate));
        assert_eq!(
            registry.resolve(&record(OperationId::ProjectCreate, Some("p1"))),
            None
        );
    }
}
