//! `POST /v1/system/handshake`(research R6).

use std::{sync::Arc, time::Instant};

use axum::{
    extract::{Request, State},
    response::Response,
};
use workbench_protocol::{FaultCode, Outcome, RequestId, WorkbenchFault};

use super::{
    authenticate, calls::MESSAGE_BAD_BODY, json_response, problem, read_body, record,
    unauthenticated,
};
use crate::{
    handshake::{
        respond, HandshakeRequest, MESSAGE_PROTOCOL_UNSUPPORTED, SUPPORTED_PROTOCOL_VERSIONS,
    },
    AppState,
};

pub async fn handshake(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let started = Instant::now();
    let request_id = RequestId::random();
    let principal = match authenticate(&state, request.headers()) {
        Some(principal) => principal,
        None => {
            let response = unauthenticated(&request_id);
            record(&state, started, None, "handshake", None, &response);
            return response;
        }
    };
    let body = match read_body(&state, request, &request_id).await {
        Ok(body) => body,
        Err(response) => {
            record(
                &state,
                started,
                None,
                "handshake",
                Some(&principal),
                &response,
            );
            return response;
        }
    };
    let response = match serde_json::from_slice::<HandshakeRequest>(&body) {
        Err(_) => problem(&WorkbenchFault::new(
            FaultCode::InvalidArgument,
            request_id,
            MESSAGE_BAD_BODY,
        )),
        Ok(request) => match respond(
            &request,
            state.config.server_info.as_ref(),
            &state.instance_id,
        ) {
            Some(result) => json_response(&result),
            None => problem(
                &WorkbenchFault::conflict(
                    request_id,
                    MESSAGE_PROTOCOL_UNSUPPORTED,
                    Outcome::NotApplied,
                )
                .with_details(serde_json::json!({
                    "supportedProtocolVersions": SUPPORTED_PROTOCOL_VERSIONS,
                })),
            ),
        },
    };
    record(
        &state,
        started,
        None,
        "handshake",
        Some(&principal),
        &response,
    );
    response
}
