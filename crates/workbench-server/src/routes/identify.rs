//! `POST /v1/system/identify`(044 research R5, 설계 리뷰 D1). **인증 없는** 신원 증명: 클라이언트가 안내 파일의
//! 소유자 자격 증명을 보내기 전에, 이 끝점이 그 안내 파일의 서버 인스턴스인지 확인한다. 서버는 `nonce`와 자기
//! 인스턴스 식별자에 대한 증명(소유자 자격 증명을 아는 서버만 만들 수 있는 값)을 돌려준다. 자격 증명 자체는 오가지
//! 않는다. 증명 수단이 없는 조립(embedded·시험)은 `notFound`.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    response::Response,
};
use workbench_protocol::{FaultCode, Outcome, RequestId, WorkbenchFault};

use super::{json_response, problem, read_body};
use crate::AppState;

pub const MESSAGE_IDENTIFY_UNAVAILABLE: &str = "identity proof is not available.";
pub const MESSAGE_BAD_NONCE: &str = "nonce must be 16 to 256 characters.";

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IdentifyRequest {
    nonce: String,
}

pub async fn identify(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let request_id = RequestId::random();
    let body = match read_body(&state, request, &request_id).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let Ok(parsed) = serde_json::from_slice::<IdentifyRequest>(&body) else {
        return problem(&WorkbenchFault::invalid_argument(
            request_id,
            MESSAGE_BAD_NONCE,
            Some("/nonce"),
        ));
    };
    if !(16..=256).contains(&parsed.nonce.len()) {
        return problem(&WorkbenchFault::invalid_argument(
            request_id,
            MESSAGE_BAD_NONCE,
            Some("/nonce"),
        ));
    }
    match state
        .config
        .server_info
        .identity_proof(&parsed.nonce, &state.instance_id)
    {
        Some(proof) => json_response(&serde_json::json!({
            "instanceId": state.instance_id,
            "proof": proof,
        })),
        None => problem(
            &WorkbenchFault::new(
                FaultCode::NotFound,
                request_id,
                MESSAGE_IDENTIFY_UNAVAILABLE,
            )
            .with_outcome(Outcome::NotApplied),
        ),
    }
}
