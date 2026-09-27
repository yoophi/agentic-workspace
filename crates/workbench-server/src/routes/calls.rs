//! `POST /v1/calls`(contracts §3). 받아들인 호출은 **서버 소유 분리 task**에서 끝까지 실행한다(research R17): 연결이
//! 끊겨 이 handler future가 drop돼도 실행과 멱등 결과 기록은 끝나고, 같은 키 재시도는 저장된 결과를 받는다.

use std::{sync::Arc, time::Instant};

use axum::{body::Bytes, extract::State, http::HeaderMap, response::Response};
use workbench_protocol::{CallRequest, FaultCode, OperationId, RequestId, WorkbenchFault};

use super::{authenticate, json_response, problem, record, unauthenticated};
use crate::{
    drain::{spawn_accepted, MESSAGE_SHUTTING_DOWN},
    AppState, MESSAGE_NOT_EXPOSED,
};

pub const MESSAGE_BAD_BODY: &str = "invalid request body.";

pub async fn call(State(state): State<Arc<AppState>>, headers: HeaderMap, body: Bytes) -> Response {
    let started = Instant::now();
    // 인증이 먼저다: 자격 증명 없는 요청은 본문이 어떻든 `401`. 본문이 올바르면 그 requestId를 싣는다.
    let parsed = serde_json::from_slice::<CallRequest>(&body);
    let principal = authenticate(&state, &headers);
    let Some(principal) = principal else {
        let request_id = parsed
            .as_ref()
            .map(|request| request.request_id.clone())
            .unwrap_or_else(|_| RequestId::random());
        let response = unauthenticated(&request_id);
        let operation = parsed
            .as_ref()
            .map(|request| request.operation.as_str())
            .unwrap_or("calls");
        record(
            &state,
            started,
            Some(&request_id),
            operation,
            None,
            &response,
        );
        return response;
    };
    let request = match parsed {
        Ok(request) => request,
        Err(_) => {
            let response = problem(&WorkbenchFault::new(
                FaultCode::InvalidArgument,
                RequestId::random(),
                MESSAGE_BAD_BODY,
            ));
            record(&state, started, None, "calls", Some(&principal), &response);
            return response;
        }
    };
    let request_id = request.request_id.clone();
    let operation = request.operation.clone();
    if let Some(id) = OperationId::parse(&operation) {
        if !state.config.exposure.allows(id) {
            let response = problem(&WorkbenchFault::new(
                FaultCode::Forbidden,
                request_id.clone(),
                MESSAGE_NOT_EXPOSED,
            ));
            record(
                &state,
                started,
                Some(&request_id),
                &operation,
                Some(&principal),
                &response,
            );
            return response;
        }
    }
    // 받아들임 = 추적 시작. 종료 중이면 `503`(아무 효과 없음).
    let Some(guard) = state.calls.accept() else {
        let response = problem(&WorkbenchFault::unavailable(
            request_id.clone(),
            MESSAGE_SHUTTING_DOWN,
        ));
        record(
            &state,
            started,
            Some(&request_id),
            &operation,
            Some(&principal),
            &response,
        );
        return response;
    };
    let workbench = Arc::clone(&state.workbench);
    let task_principal = principal.clone();
    let executed = spawn_accepted(guard, async move {
        workbench.call(task_principal, request).await
    })
    .await;
    let response = match executed {
        Ok(Ok(reply)) => json_response(&reply),
        Ok(Err(fault)) => problem(&fault),
        Err(_) => problem(&WorkbenchFault::internal(
            request_id.clone(),
            "call execution failed.",
        )),
    };
    record(
        &state,
        started,
        Some(&request_id),
        &operation,
        Some(&principal),
        &response,
    );
    response
}
