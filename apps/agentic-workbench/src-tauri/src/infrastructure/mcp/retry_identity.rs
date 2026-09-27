//! 042 T027: MCP 도구 호출의 재시도 식별. 멱등성 키 출처의 우선순위:
//!
//! 1. 도구 인자의 `requestId` — agent가 새 JSON-RPC id로 다시 보내도 같은 요청이다.
//! 2. 유효한 JSON-RPC 요청 id(숫자·문자열, 종류 구분) — 같은 wire 요청의 재전송(응답 유실 뒤 같은 id). 인증된 run·
//!    operation·인자 전체를 함께 넣어 다른 도구·다른 인자의 같은 id와 섞이지 않는다.
//! 3. 둘 다 없으면 호출마다 새 키(새 요청).
//!
//! 같은 run·operation·requestId(또는 id+인자)는 같은 키가 되어, 진행 중이면 원 호출을 기다리고 끝났으면 저장된
//! 결과를 받는다(세대 범위). 한계: MCP 클라이언트가 다시 연결해 id를 처음부터 다시 쓰고 **인자까지 같으면** 재전송으로
//! 본다 — 세대 멱등 기록이 남아 있는 동안.

use serde_json::Value;
use sha2::{Digest, Sha256};
use workbench_protocol::{IdempotencyKey, OperationId};

pub fn tool_idempotency_key(
    run_id: &str,
    operation: OperationId,
    arguments: Option<&Value>,
    rpc_id: Option<&Value>,
) -> IdempotencyKey {
    let request_id = arguments
        .and_then(|arguments| arguments.get("requestId"))
        .and_then(Value::as_str)
        // 도메인은 requestId를 그대로 비교한다 — 키도 원래 값으로 만들고, 공백뿐인 값만 식별자에서 뺀다.
        .filter(|request_id| !request_id.trim().is_empty());
    if let Some(request_id) = request_id {
        return derived(&["request", run_id, operation.as_str(), request_id]);
    }
    let rpc = match rpc_id {
        Some(Value::Number(number)) => Some(("rpc-number", number.to_string())),
        Some(Value::String(text)) if !text.is_empty() => Some(("rpc-string", text.clone())),
        _ => None,
    };
    match rpc {
        Some((kind, id)) => {
            let arguments = arguments.map(Value::to_string).unwrap_or_default();
            derived(&[kind, run_id, operation.as_str(), &id, &arguments])
        }
        None => IdempotencyKey::random(),
    }
}

fn derived(parts: &[&str]) -> IdempotencyKey {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    IdempotencyKey::new(format!("mcp-{digest}")).expect("hex key within identifier limits")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn same_run_operation_and_request_id_share_a_key() {
        let args = json!({ "requestId": "r1" });
        let key = tool_idempotency_key(
            "run",
            OperationId::OrchestrationReportResult,
            Some(&args),
            None,
        );
        assert_eq!(
            key,
            tool_idempotency_key(
                "run",
                OperationId::OrchestrationReportResult,
                Some(&args),
                None
            )
        );
        assert_ne!(
            key,
            tool_idempotency_key(
                "other",
                OperationId::OrchestrationReportResult,
                Some(&args),
                None
            )
        );
        assert_ne!(
            key,
            tool_idempotency_key(
                "run",
                OperationId::OrchestrationReportProgress,
                Some(&args),
                None
            )
        );
        assert_ne!(
            key,
            tool_idempotency_key(
                "run",
                OperationId::OrchestrationReportResult,
                Some(&json!({ "requestId": "r2" })),
                None
            )
        );
    }

    #[test]
    fn request_ids_are_compared_verbatim() {
        let op = OperationId::OrchestrationReportResult;
        assert_ne!(
            tool_idempotency_key("run", op, Some(&json!({ "requestId": "r1" })), None),
            tool_idempotency_key("run", op, Some(&json!({ "requestId": " r1" })), None),
        );
    }

    #[test]
    fn explicit_request_ids_win_over_rpc_ids() {
        let args = json!({ "requestId": "r1" });
        let op = OperationId::OrchestrationReportResult;
        assert_eq!(
            tool_idempotency_key("run", op, Some(&args), Some(&json!(1))),
            tool_idempotency_key("run", op, Some(&args), Some(&json!(2))),
        );
    }

    #[test]
    fn rpc_ids_identify_the_same_wire_request_only() {
        let op = OperationId::OrchestrationCollectChildResults;
        let args = json!({});
        let key = tool_idempotency_key("run", op, Some(&args), Some(&json!(7)));
        assert_eq!(
            key,
            tool_idempotency_key("run", op, Some(&args), Some(&json!(7)))
        );
        assert_ne!(
            key,
            tool_idempotency_key("run", op, Some(&args), Some(&json!("7"))),
            "number vs string"
        );
        assert_ne!(
            key,
            tool_idempotency_key("run", op, Some(&args), Some(&json!(8)))
        );
        assert_ne!(
            key,
            tool_idempotency_key("other", op, Some(&args), Some(&json!(7)))
        );
        assert_ne!(
            key,
            tool_idempotency_key(
                "run",
                OperationId::OrchestrationAssignChildTask,
                Some(&args),
                Some(&json!(7))
            )
        );
        assert_ne!(
            key,
            tool_idempotency_key(
                "run",
                op,
                Some(&json!({ "taskIds": ["t"] })),
                Some(&json!(7))
            ),
            "same id, other arguments"
        );
        for invalid in [json!(null), json!(""), json!(true), json!({})] {
            let a = tool_idempotency_key("run", op, Some(&args), Some(&invalid));
            let b = tool_idempotency_key("run", op, Some(&args), Some(&invalid));
            assert_ne!(a, b, "{invalid} is not an identity");
        }
    }

    #[test]
    fn missing_or_blank_request_ids_are_not_retry_identities() {
        for args in [None, Some(json!({})), Some(json!({ "requestId": "  " }))] {
            let a = tool_idempotency_key(
                "run",
                OperationId::OrchestrationCollectChildResults,
                args.as_ref(),
                None,
            );
            let b = tool_idempotency_key(
                "run",
                OperationId::OrchestrationCollectChildResults,
                args.as_ref(),
                None,
            );
            assert_ne!(a, b);
        }
    }
}
