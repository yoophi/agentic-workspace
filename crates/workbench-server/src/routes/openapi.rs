//! `GET /openapi.json`(인증 필요) — 커밋된 계약 문서와 같은 문자열(drift 검사가 보장).

use std::{sync::Arc, time::Instant};

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::Response,
};
use workbench_protocol::RequestId;

use super::{authenticate, record, unauthenticated};
use crate::AppState;

pub async fn document(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let started = Instant::now();
    let principal = authenticate(&state, &headers);
    let response = match &principal {
        Some(_) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(workbench_protocol::openapi::render_openapi()))
            .expect("response"),
        None => unauthenticated(&RequestId::random()),
    };
    record(
        &state,
        started,
        None,
        "openapi",
        principal.as_ref(),
        &response,
    );
    response
}
