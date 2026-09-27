//! 상태 확인(research R2). live는 인증 없이 아무 정보도 드러내지 않는다.

use std::{sync::Arc, time::Instant};

use axum::{extract::State, http::HeaderMap, response::Response};
use workbench_protocol::RequestId;

use super::{authenticate, json_response, record, unauthenticated};
use crate::AppState;

pub async fn live() -> Response {
    json_response(&serde_json::json!({ "status": "live" }))
}

/// router는 런타임 조립(기동 복구 포함)이 끝난 뒤에만 bind되므로 받는 순간 ready다.
pub async fn ready(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let started = Instant::now();
    let principal = authenticate(&state, &headers);
    let response = match &principal {
        Some(_) => json_response(&serde_json::json!({
            "ready": true,
            "serverEpoch": state.config.server_info.server_epoch(),
        })),
        None => unauthenticated(&RequestId::random()),
    };
    record(
        &state,
        started,
        None,
        "ready",
        principal.as_ref(),
        &response,
    );
    response
}
