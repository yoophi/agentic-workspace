//! 042 T027: MCP 도구 호출의 재시도 식별. agent는 응답을 못 받으면 같은 도구를 같은 인자로 다시 부른다(JSON-RPC
//! id는 새로 붙는다). 그래서 멱등성 키는 도구 인자의 `requestId`에서 만든다 — 같은 run·operation·requestId는 같은
//! 키가 되어, 진행 중이면 원 호출을 기다리고 끝났으면 저장된 결과를 받는다. `requestId`가 없는 호출은 재시도를
//! 식별할 수 없으므로 호출마다 새 키다(제목 변경처럼 결과가 같은 상태로 수렴하는 도구, `requestId`를 선택으로 받는
//! 도구에서 생략한 경우).

use serde_json::Value;
use sha2::{Digest, Sha256};
use workbench_protocol::{IdempotencyKey, OperationId};

pub fn tool_idempotency_key(
    run_id: &str,
    operation: OperationId,
    arguments: Option<&Value>,
) -> IdempotencyKey {
    let request_id = arguments
        .and_then(|arguments| arguments.get("requestId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|request_id| !request_id.is_empty());
    let Some(request_id) = request_id else {
        return IdempotencyKey::random();
    };
    let mut hasher = Sha256::new();
    for part in [run_id, operation.as_str(), request_id] {
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
        let key = tool_idempotency_key("run", OperationId::OrchestrationReportResult, Some(&args));
        assert_eq!(
            key,
            tool_idempotency_key("run", OperationId::OrchestrationReportResult, Some(&args))
        );
        assert_ne!(
            key,
            tool_idempotency_key("other", OperationId::OrchestrationReportResult, Some(&args))
        );
        assert_ne!(
            key,
            tool_idempotency_key("run", OperationId::OrchestrationReportProgress, Some(&args))
        );
        assert_ne!(
            key,
            tool_idempotency_key(
                "run",
                OperationId::OrchestrationReportResult,
                Some(&json!({ "requestId": "r2" }))
            )
        );
    }

    #[test]
    fn missing_or_blank_request_ids_are_not_retry_identities() {
        for args in [None, Some(json!({})), Some(json!({ "requestId": "  " }))] {
            let a = tool_idempotency_key(
                "run",
                OperationId::OrchestrationCollectChildResults,
                args.as_ref(),
            );
            let b = tool_idempotency_key(
                "run",
                OperationId::OrchestrationCollectChildResults,
                args.as_ref(),
            );
            assert_ne!(a, b);
        }
    }
}
