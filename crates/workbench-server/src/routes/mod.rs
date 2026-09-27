//! 경로 공통: 인증 추출, problem 응답(오늘 테스트 harness와 같은 형식), 접근 기록.

pub mod calls;
pub mod events;
pub mod handshake;
pub mod health;
pub mod openapi;

use std::time::Instant;

use axum::{
    body::Body,
    http::{header, HeaderMap, StatusCode},
    response::Response,
};
use workbench_protocol::{AuthenticatedPrincipal, FaultCode, RequestId, WorkbenchFault};

use crate::{access_log::AccessEntry, AppState};

/// `application/problem+json`: `WorkbenchFault` + `type`·`title`·`status`.
pub fn problem(fault: &WorkbenchFault) -> Response {
    let status = fault.code.http_status();
    let mut body = serde_json::to_value(fault).expect("fault json");
    body["type"] = serde_json::Value::String(format!("urn:aw:fault:{}", fault.code.as_str()));
    body["title"] = serde_json::Value::String(fault.code.as_str().to_owned());
    body["status"] = serde_json::Value::from(status);
    Response::builder()
        .status(StatusCode::from_u16(status).expect("valid status"))
        .header(header::CONTENT_TYPE, "application/problem+json")
        .body(Body::from(serde_json::to_vec(&body).expect("body")))
        .expect("response")
}

pub fn forbidden(message: &str) -> Response {
    problem(&WorkbenchFault::new(
        FaultCode::Forbidden,
        RequestId::random(),
        message,
    ))
}

pub fn json_response(value: &impl serde::Serialize) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(serde_json::to_vec(value).expect("json")))
        .expect("response")
}

pub fn origin(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
}

/// bearer → principal. 실패하면 호출자가 [`unauthenticated`]로 답한다(자격 증명 종류·만료 사유를 드러내지 않는다).
pub fn authenticate(state: &AppState, headers: &HeaderMap) -> Option<AuthenticatedPrincipal> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(|token| state.config.resolver.resolve(token, origin(headers)))
}

pub fn unauthenticated(request_id: &RequestId) -> Response {
    problem(&WorkbenchFault::unauthenticated(request_id.clone()))
}

pub fn record(
    state: &AppState,
    started: Instant,
    request_id: Option<&RequestId>,
    operation: &str,
    principal: Option<&AuthenticatedPrincipal>,
    response: &Response,
) {
    state.config.access_log.record(&AccessEntry {
        request_id: request_id.map(|id| id.as_str().to_owned()),
        operation: operation.to_owned(),
        principal_kind: principal.map(|principal| principal.kind.as_str()),
        status: response.status().as_u16(),
        latency_ms: started.elapsed().as_millis(),
    });
}
