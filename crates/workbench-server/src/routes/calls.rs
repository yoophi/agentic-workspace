//! `POST /v1/calls`(contracts §3). 받아들인 호출은 **서버 소유 분리 task**에서 끝까지 실행한다(research R17): 연결이
//! 끊겨 이 handler future가 drop돼도 실행과 멱등 결과 기록은 끝나고, 같은 키 재시도는 저장된 결과를 받는다.

use std::{sync::Arc, time::Instant};

use axum::{body::Bytes, extract::State, http::HeaderMap, response::Response};
use workbench_protocol::{CallRequest, FaultCode, OperationId, RequestId, WorkbenchFault};

use super::{authenticate, json_response, problem, record, unauthenticated};
use crate::{drain::MESSAGE_SHUTTING_DOWN, AppState, MESSAGE_NOT_EXPOSED};

pub const MESSAGE_BAD_BODY: &str = "invalid request body.";

pub async fn call(State(state): State<Arc<AppState>>, headers: HeaderMap, body: Bytes) -> Response {
    let started = Instant::now();
    let request: CallRequest = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(_) => {
            let response = problem(&WorkbenchFault::new(
                FaultCode::InvalidArgument,
                RequestId::random(),
                MESSAGE_BAD_BODY,
            ));
            record(&state, started, None, "calls", None, &response);
            return response;
        }
    };
    let request_id = request.request_id.clone();
    let operation = request.operation.clone();
    let principal = match authenticate(&state, &headers) {
        Some(principal) => principal,
        None => {
            let response = unauthenticated(&request_id);
            record(
                &state,
                started,
                Some(&request_id),
                &operation,
                None,
                &response,
            );
            return response;
        }
    };
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
    let executed = tokio::spawn(async move {
        let _guard = guard; // 완료·panic 때 drop → drain이 안다
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
