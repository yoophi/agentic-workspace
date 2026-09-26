//! 멱등성 규칙(순수 로직). 지문 계산과 "같은 키 재요청" 판정표(`data-model.md` §3)를 구현한다.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use workbench_protocol::{CallReply, OperationId, Outcome, RequestId, WorkbenchFault};

use crate::ports::operation_ledger::{LedgerRecord, LedgerState};

/// 객체 키를 정렬해 다시 직렬화한 JSON. 키 순서가 달라도 같은 문자열이 나온다.
pub fn canonical_json(value: &serde_json::Value) -> String {
    fn normalize(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let sorted: BTreeMap<&String, serde_json::Value> =
                    map.iter().map(|(k, v)| (k, normalize(v))).collect();
                serde_json::Value::Object(sorted.into_iter().map(|(k, v)| (k.clone(), v)).collect())
            }
            serde_json::Value::Array(items) => {
                serde_json::Value::Array(items.iter().map(normalize).collect())
            }
            other => other.clone(),
        }
    }
    normalize(value).to_string()
}

/// `sha256(operation + "\n" + canonical_json(input))`의 hex.
pub fn fingerprint(operation: OperationId, normalized_input: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(operation.as_str().as_bytes());
    hasher.update(b"\n");
    hasher.update(canonical_json(normalized_input).as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReplayDecision {
    /// 기존 기록이 없다. 새 실행을 시작한다.
    Proceed,
    /// 같은 키·같은 지문의 확정 결과를 그대로 돌려준다.
    ReturnStored(Result<CallReply, WorkbenchFault>),
    /// 적용하지 않고 충돌을 알린다.
    Conflict(WorkbenchFault),
}

pub const MESSAGE_IN_FLIGHT: &str = "같은 요청이 처리 중입니다.";
pub const MESSAGE_UNKNOWN: &str =
    "이전 요청의 적용 여부를 확인할 수 없습니다. 프로젝트 목록을 확인하세요.";
pub const MESSAGE_DIFFERENT_PAYLOAD: &str =
    "같은 멱등성 키로 다른 내용의 요청이 이미 처리되었습니다.";

/// data-model §3 "같은 키 재요청의 응답 규칙" 7행.
pub fn decide(
    existing: Option<&LedgerRecord>,
    fingerprint: &str,
    request_id: &RequestId,
) -> ReplayDecision {
    let Some(record) = existing else {
        return ReplayDecision::Proceed;
    };
    let same_payload = record.input_fingerprint == fingerprint;
    match record.state {
        LedgerState::Applied if same_payload => {
            ReplayDecision::ReturnStored(Ok(CallReply::complete(
                record.result_json.clone().unwrap_or_default(),
                record.revision,
            )))
        }
        LedgerState::Applied => ReplayDecision::Conflict(WorkbenchFault::conflict(
            request_id.clone(),
            MESSAGE_DIFFERENT_PAYLOAD,
            Outcome::Applied,
        )),
        LedgerState::Failed if same_payload => {
            let stored = record
                .result_json
                .clone()
                .and_then(|json| serde_json::from_value::<WorkbenchFault>(json).ok())
                .map(|mut fault| {
                    fault.request_id = request_id.clone();
                    fault
                })
                .unwrap_or_else(|| {
                    WorkbenchFault::internal(
                        request_id.clone(),
                        "저장된 실패 결과를 읽을 수 없습니다.",
                    )
                });
            ReplayDecision::ReturnStored(Err(stored))
        }
        LedgerState::Failed => ReplayDecision::Conflict(WorkbenchFault::conflict(
            request_id.clone(),
            MESSAGE_DIFFERENT_PAYLOAD,
            Outcome::NotApplied,
        )),
        LedgerState::Pending => ReplayDecision::Conflict(
            WorkbenchFault::conflict(request_id.clone(), MESSAGE_IN_FLIGHT, Outcome::Unknown)
                .with_retryable(true),
        ),
        LedgerState::Unknown => ReplayDecision::Conflict(
            WorkbenchFault::conflict(request_id.clone(), MESSAGE_UNKNOWN, Outcome::Unknown)
                .with_retryable(false),
        ),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use workbench_protocol::{FaultCode, IdempotencyKey, PrincipalKind, CONTRACT_REVISION};

    use super::*;
    use crate::ports::operation_ledger::LedgerKey;

    fn rid() -> RequestId {
        RequestId::new("r2").unwrap()
    }

    fn record(state: LedgerState, fp: &str, result: Option<serde_json::Value>) -> LedgerRecord {
        LedgerRecord {
            execution_id: "exec".into(),
            key: LedgerKey {
                principal_kind: PrincipalKind::Desktop,
                operation: OperationId::ProjectCreate,
                contract_revision: CONTRACT_REVISION,
                idempotency_key: IdempotencyKey::new("k").unwrap(),
            },
            input_fingerprint: fp.into(),
            aggregate: "projects".into(),
            reserved_resource_id: Some("project-1".into()),
            state,
            result_json: result,
            revision: Some(3),
            request_id: RequestId::new("r1").unwrap(),
            created_at: String::new(),
            updated_at: String::new(),
            expires_at: None,
        }
    }

    #[test]
    fn canonical_json_ignores_key_order() {
        let a = json!({"b": 1, "a": {"d": [1, {"z": 0, "y": 1}], "c": null}});
        let b = json!({"a": {"c": null, "d": [1, {"y": 1, "z": 0}]}, "b": 1});
        assert_eq!(canonical_json(&a), canonical_json(&b));
        assert_eq!(
            fingerprint(OperationId::ProjectCreate, &a),
            fingerprint(OperationId::ProjectCreate, &b)
        );
        assert_ne!(
            fingerprint(OperationId::ProjectCreate, &a),
            fingerprint(OperationId::ProjectList, &a)
        );
    }

    #[test]
    fn no_record_proceeds() {
        assert_eq!(decide(None, "fp", &rid()), ReplayDecision::Proceed);
    }

    #[test]
    fn applied_same_payload_returns_stored_reply() {
        let stored = record(LedgerState::Applied, "fp", Some(json!({"id": "project-1"})));
        match decide(Some(&stored), "fp", &rid()) {
            ReplayDecision::ReturnStored(Ok(reply)) => {
                assert_eq!(reply.output(), Some(&json!({"id": "project-1"})));
                assert_eq!(reply.revision(), Some(3));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn applied_different_payload_conflicts_with_applied_outcome() {
        let stored = record(LedgerState::Applied, "fp", Some(json!({})));
        match decide(Some(&stored), "other", &rid()) {
            ReplayDecision::Conflict(fault) => {
                assert_eq!(fault.code, FaultCode::Conflict);
                assert_eq!(fault.outcome, Outcome::Applied);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn failed_same_payload_returns_stored_fault_with_new_request_id() {
        let fault_json = serde_json::to_value(WorkbenchFault::unavailable(
            RequestId::new("old").unwrap(),
            "disk",
        ))
        .unwrap();
        let stored = record(LedgerState::Failed, "fp", Some(fault_json));
        match decide(Some(&stored), "fp", &rid()) {
            ReplayDecision::ReturnStored(Err(fault)) => {
                assert_eq!(fault.code, FaultCode::Unavailable);
                assert_eq!(fault.request_id.as_str(), "r2");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn failed_different_payload_conflicts_not_applied() {
        let stored = record(LedgerState::Failed, "fp", None);
        match decide(Some(&stored), "x", &rid()) {
            ReplayDecision::Conflict(fault) => assert_eq!(fault.outcome, Outcome::NotApplied),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn pending_and_unknown_conflict_with_unknown_outcome() {
        let pending = record(LedgerState::Pending, "fp", None);
        match decide(Some(&pending), "fp", &rid()) {
            ReplayDecision::Conflict(fault) => {
                assert_eq!(fault.outcome, Outcome::Unknown);
                assert!(fault.retryable);
                assert_eq!(fault.message, MESSAGE_IN_FLIGHT);
            }
            other => panic!("unexpected {other:?}"),
        }
        let unknown = record(LedgerState::Unknown, "fp", None);
        match decide(Some(&unknown), "zzz", &rid()) {
            ReplayDecision::Conflict(fault) => {
                assert_eq!(fault.outcome, Outcome::Unknown);
                assert!(!fault.retryable);
                assert_eq!(fault.message, MESSAGE_UNKNOWN);
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
