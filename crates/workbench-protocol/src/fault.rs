//! `Workbench` 오류 계약. HTTP Adapter는 이를 RFC 9457 Problem Details로 변환한다.

use std::fmt;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::call::RequestId;

/// 안정적 오류 코드. 정본 `client-server-architecture-research.md` §Errors 표와 1:1이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum FaultCode {
    InvalidArgument,
    Unauthenticated,
    Forbidden,
    NotFound,
    Conflict,
    InteractionRequired,
    PreconditionFailed,
    UnsupportedProtocol,
    UnsupportedSchema,
    RateLimited,
    Draining,
    Unavailable,
    DeadlineExceeded,
    Internal,
}

impl FaultCode {
    pub const ALL: [FaultCode; 14] = [
        FaultCode::InvalidArgument,
        FaultCode::Unauthenticated,
        FaultCode::Forbidden,
        FaultCode::NotFound,
        FaultCode::Conflict,
        FaultCode::InteractionRequired,
        FaultCode::PreconditionFailed,
        FaultCode::UnsupportedProtocol,
        FaultCode::UnsupportedSchema,
        FaultCode::RateLimited,
        FaultCode::Draining,
        FaultCode::Unavailable,
        FaultCode::DeadlineExceeded,
        FaultCode::Internal,
    ];

    /// HTTP Adapter가 쓰는 status. domain error를 status만으로 구분하지 않으므로 겹치는 값이 있다.
    pub fn http_status(self) -> u16 {
        match self {
            FaultCode::InvalidArgument => 400,
            FaultCode::Unauthenticated => 401,
            FaultCode::Forbidden => 403,
            FaultCode::NotFound => 404,
            FaultCode::Conflict
            | FaultCode::InteractionRequired
            | FaultCode::UnsupportedProtocol => 409,
            FaultCode::PreconditionFailed => 412,
            FaultCode::UnsupportedSchema => 422,
            FaultCode::RateLimited => 429,
            FaultCode::Draining | FaultCode::Unavailable => 503,
            FaultCode::DeadlineExceeded => 504,
            FaultCode::Internal => 500,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            FaultCode::InvalidArgument => "invalidArgument",
            FaultCode::Unauthenticated => "unauthenticated",
            FaultCode::Forbidden => "forbidden",
            FaultCode::NotFound => "notFound",
            FaultCode::Conflict => "conflict",
            FaultCode::InteractionRequired => "interactionRequired",
            FaultCode::PreconditionFailed => "preconditionFailed",
            FaultCode::UnsupportedProtocol => "unsupportedProtocol",
            FaultCode::UnsupportedSchema => "unsupportedSchema",
            FaultCode::RateLimited => "rateLimited",
            FaultCode::Draining => "draining",
            FaultCode::Unavailable => "unavailable",
            FaultCode::DeadlineExceeded => "deadlineExceeded",
            FaultCode::Internal => "internal",
        }
    }
}

/// mutation의 적용 여부. timeout·단절 뒤 같은 멱등성 키로 재확인할지 판단하는 근거다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    NotApplied,
    Applied,
    Unknown,
}

/// 호출 실패. `message`는 사람이 읽는 한 문장이며 코드나 스택을 담지 않는다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchFault {
    pub code: FaultCode,
    pub message: String,
    pub retryable: bool,
    pub outcome: Outcome,
    pub request_id: RequestId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub details: Option<serde_json::Value>,
}

impl WorkbenchFault {
    pub fn new(code: FaultCode, request_id: RequestId, message: impl Into<String>) -> Self {
        let retryable = matches!(
            code,
            FaultCode::RateLimited
                | FaultCode::Draining
                | FaultCode::Unavailable
                | FaultCode::DeadlineExceeded
        );
        Self {
            code,
            message: message.into(),
            retryable,
            outcome: Outcome::NotApplied,
            request_id,
            details: None,
        }
    }

    pub fn with_outcome(mut self, outcome: Outcome) -> Self {
        self.outcome = outcome;
        self
    }

    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn invalid_argument(
        request_id: RequestId,
        message: impl Into<String>,
        path: Option<&str>,
    ) -> Self {
        let fault = Self::new(FaultCode::InvalidArgument, request_id, message);
        match path {
            Some(path) => fault.with_details(serde_json::json!({ "path": path })),
            None => fault,
        }
    }

    pub fn unauthenticated(request_id: RequestId) -> Self {
        Self::new(
            FaultCode::Unauthenticated,
            request_id,
            "인증 정보가 없거나 만료되었습니다.",
        )
    }

    pub fn forbidden(request_id: RequestId, operation: &str) -> Self {
        Self::new(
            FaultCode::Forbidden,
            request_id,
            format!("현재 호출자에게 허용되지 않은 operation입니다: {operation}"),
        )
    }

    pub fn not_found(request_id: RequestId, operation: &str) -> Self {
        Self::new(
            FaultCode::NotFound,
            request_id,
            format!("알 수 없는 operation입니다: {operation}"),
        )
    }

    pub fn unsupported_protocol(request_id: RequestId, requested: u16, supported: u16) -> Self {
        Self::new(
            FaultCode::UnsupportedProtocol,
            request_id,
            format!("지원하지 않는 protocolVersion {requested}입니다. 지원: {supported}"),
        )
        .with_details(serde_json::json!({
            "requested": requested,
            "supportedProtocolRevisions": [supported]
        }))
    }

    pub fn conflict(request_id: RequestId, message: impl Into<String>, outcome: Outcome) -> Self {
        Self::new(FaultCode::Conflict, request_id, message).with_outcome(outcome)
    }

    pub fn precondition_failed(request_id: RequestId, expected: u64, current: u64) -> Self {
        Self::new(
            FaultCode::PreconditionFailed,
            request_id,
            format!("기대한 revision {expected}과 현재 revision {current}이 다릅니다."),
        )
        .with_details(serde_json::json!({
            "expectedRevision": expected,
            "currentRevision": current
        }))
    }

    pub fn unavailable(request_id: RequestId, message: impl Into<String>) -> Self {
        Self::new(FaultCode::Unavailable, request_id, message)
    }

    pub fn internal(request_id: RequestId, message: impl Into<String>) -> Self {
        Self::new(FaultCode::Internal, request_id, message)
    }

    pub fn unsupported_schema(request_id: RequestId, message: impl Into<String>) -> Self {
        Self::new(FaultCode::UnsupportedSchema, request_id, message)
    }
}

impl fmt::Display for WorkbenchFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for WorkbenchFault {}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn rid() -> RequestId {
        RequestId::new("r1").unwrap()
    }

    #[test]
    fn http_status_matches_design_table() {
        let expected = [
            (FaultCode::InvalidArgument, 400),
            (FaultCode::Unauthenticated, 401),
            (FaultCode::Forbidden, 403),
            (FaultCode::NotFound, 404),
            (FaultCode::Conflict, 409),
            (FaultCode::InteractionRequired, 409),
            (FaultCode::PreconditionFailed, 412),
            (FaultCode::UnsupportedProtocol, 409),
            (FaultCode::UnsupportedSchema, 422),
            (FaultCode::RateLimited, 429),
            (FaultCode::Draining, 503),
            (FaultCode::Unavailable, 503),
            (FaultCode::DeadlineExceeded, 504),
            (FaultCode::Internal, 500),
        ];
        assert_eq!(expected.len(), FaultCode::ALL.len());
        for (code, status) in expected {
            assert_eq!(code.http_status(), status, "{code:?}");
            assert_eq!(serde_json::to_value(code).unwrap(), json!(code.as_str()));
        }
    }

    #[test]
    fn fault_serializes_camel_case_and_default_outcome() {
        let fault =
            WorkbenchFault::invalid_argument(rid(), "Project name is required.", Some("/name"));
        assert_eq!(
            serde_json::to_value(&fault).unwrap(),
            json!({
                "code": "invalidArgument",
                "message": "Project name is required.",
                "retryable": false,
                "outcome": "notApplied",
                "requestId": "r1",
                "details": {"path": "/name"}
            })
        );
    }

    #[test]
    fn transient_codes_are_retryable_by_default() {
        assert!(WorkbenchFault::unavailable(rid(), "x").retryable);
        assert!(!WorkbenchFault::not_found(rid(), "y").retryable);
        assert!(!WorkbenchFault::internal(rid(), "z").retryable);
    }

    #[test]
    fn precondition_failed_carries_current_revision() {
        let fault = WorkbenchFault::precondition_failed(rid(), 3, 4);
        assert_eq!(fault.code, FaultCode::PreconditionFailed);
        assert_eq!(fault.details.unwrap()["currentRevision"], json!(4));
    }
}
