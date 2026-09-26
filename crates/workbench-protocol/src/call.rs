//! `Workbench.call`의 요청·응답 wire 타입.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// generic call/event wire 호환성 축. 037은 1만 지원한다.
pub const PROTOCOL_VERSION: u16 = 1;
/// operation 계약 revision. idempotency key namespace의 일부다.
pub const CONTRACT_REVISION: u32 = 1;

const MAX_IDENTIFIER_LEN: usize = 128;

/// 계약이 정의한 operation. wire에는 `project.list` 같은 문자열로 실린다.
///
/// `CallRequest.operation`은 알 수 없는 이름을 `notFound`로 거절해야 하므로 raw `String`으로 받고,
/// runtime이 이 enum으로 해석한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum OperationId {
    #[serde(rename = "project.list")]
    ProjectList,
    #[serde(rename = "project.create")]
    ProjectCreate,
    #[serde(rename = "system.describe")]
    SystemDescribe,
}

impl OperationId {
    pub const ALL: [OperationId; 3] = [
        OperationId::ProjectList,
        OperationId::ProjectCreate,
        OperationId::SystemDescribe,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            OperationId::ProjectList => "project.list",
            OperationId::ProjectCreate => "project.create",
            OperationId::SystemDescribe => "system.describe",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.as_str() == value)
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for OperationId {
    type Err = InvalidIdentifier;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).ok_or_else(|| InvalidIdentifier::UnknownOperation(value.to_owned()))
    }
}

/// 식별자 검증 실패. 계약 위반이므로 `invalidArgument`로 매핑된다.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidIdentifier {
    #[error("{field} must be 1..={MAX_IDENTIFIER_LEN} characters")]
    Length { field: &'static str },
    #[error("unknown operation: {0}")]
    UnknownOperation(String),
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), InvalidIdentifier> {
    let len = value.chars().count();
    if len == 0 || len > MAX_IDENTIFIER_LEN {
        return Err(InvalidIdentifier::Length { field });
    }
    Ok(())
}

/// 1..=128자 문자열 스키마. `RequestId`·`IdempotencyKey`가 공유한다.
fn identifier_schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
    use utoipa::openapi::schema::{ObjectBuilder, Schema, Type};
    utoipa::openapi::RefOr::T(Schema::Object(
        ObjectBuilder::new()
            .schema_type(Type::String)
            .min_length(Some(1))
            .max_length(Some(MAX_IDENTIFIER_LEN))
            .build(),
    ))
}

/// 시도별 추적 ID. 재시도마다 새 값을 만든다. 멱등성 키와 수명이 다르다.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RequestId(String);

impl utoipa::PartialSchema for RequestId {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        identifier_schema()
    }
}

impl ToSchema for RequestId {}

impl RequestId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidIdentifier> {
        let value = value.into();
        validate_identifier("requestId", &value)?;
        Ok(Self(value))
    }

    pub fn random() -> Self {
        Self(format!("req_{}", uuid::Uuid::new_v4().simple()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RequestId {
    type Error = InvalidIdentifier;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<RequestId> for String {
    fn from(value: RequestId) -> Self {
        value.0
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 부작용 중복 제거 키. mutation 재시도에만 같은 값을 재사용한다.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IdempotencyKey(String);

impl utoipa::PartialSchema for IdempotencyKey {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        identifier_schema()
    }
}

impl ToSchema for IdempotencyKey {}

impl IdempotencyKey {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidIdentifier> {
        let value = value.into();
        validate_identifier("idempotencyKey", &value)?;
        Ok(Self(value))
    }

    pub fn random() -> Self {
        Self(format!("idem_{}", uuid::Uuid::new_v4().simple()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IdempotencyKey {
    type Error = InvalidIdentifier;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<IdempotencyKey> for String {
    fn from(value: IdempotencyKey) -> Self {
        value.0
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 단일 호출 요청. `input`은 operation descriptor의 스키마로 검증된 뒤 typed handler로 넘어간다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CallRequest {
    pub protocol_version: u16,
    /// 예: `project.list`. 알 수 없는 값은 `notFound`.
    pub operation: String,
    pub request_id: RequestId,
    /// operation별 typed input. generic 봉투에서는 임의 JSON이며, 실제 스키마는 `CallRequest` oneOf variant에 있다.
    #[serde(default)]
    #[schema(value_type = Value)]
    pub input: serde_json::Value,
    /// command(mutation)에는 필수, query에는 무시된다.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    /// command에서만 의미가 있다. aggregate revision과 다르면 `preconditionFailed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<u64>,
    /// 서버가 상한을 적용하는 상대 시간(ms). 037은 검증만 한다.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl CallRequest {
    /// query 호출용 최소 요청.
    pub fn query(operation: OperationId, input: serde_json::Value) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            operation: operation.as_str().to_owned(),
            request_id: RequestId::random(),
            input,
            idempotency_key: None,
            expected_revision: None,
            timeout_ms: None,
        }
    }

    /// command 호출용 최소 요청. 멱등성 키를 새로 만든다.
    pub fn command(operation: OperationId, input: serde_json::Value) -> Self {
        Self {
            idempotency_key: Some(IdempotencyKey::random()),
            ..Self::query(operation, input)
        }
    }
}

/// 호출 응답. 037의 모든 operation은 동기 완료라 `Complete`만 발생한다.
///
/// `output`은 generic 봉투에서는 임의 JSON이다(`project.list`는 배열을 돌려준다). operation별 typed 결과는
/// `CallReplyByOperation`(openapi.rs)에 있다. 필드 표기는 variant 단위 `rename_all`을 쓴다 — serde의
/// `rename_all_fields`는 utoipa derive가 인식하지 않아 스키마가 `execution_id`로 새기 때문이다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CallReply {
    #[serde(rename_all = "camelCase")]
    Complete {
        #[schema(value_type = Value)]
        output: serde_json::Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        revision: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    Accepted {
        execution_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        revision: Option<u64>,
    },
}

impl CallReply {
    pub fn complete(output: serde_json::Value, revision: Option<u64>) -> Self {
        CallReply::Complete { output, revision }
    }

    pub fn output(&self) -> Option<&serde_json::Value> {
        match self {
            CallReply::Complete { output, .. } => Some(output),
            CallReply::Accepted { .. } => None,
        }
    }

    pub fn revision(&self) -> Option<u64> {
        match self {
            CallReply::Complete { revision, .. } | CallReply::Accepted { revision, .. } => {
                *revision
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn operation_id_round_trips_through_wire_names() {
        for id in OperationId::ALL {
            let wire = serde_json::to_value(id).unwrap();
            assert_eq!(wire, json!(id.as_str()));
            assert_eq!(id.as_str().parse::<OperationId>().unwrap(), id);
        }
        assert!(OperationId::parse("project.rename").is_none());
    }

    #[test]
    fn request_serializes_camel_case_and_omits_absent_optionals() {
        let request = CallRequest {
            protocol_version: 1,
            operation: "project.list".into(),
            request_id: RequestId::new("r1").unwrap(),
            input: json!({}),
            idempotency_key: None,
            expected_revision: None,
            timeout_ms: None,
        };
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(
            value,
            json!({"protocolVersion": 1, "operation": "project.list", "requestId": "r1", "input": {}})
        );
        let back: CallRequest = serde_json::from_value(value).unwrap();
        assert_eq!(back, request);
    }

    #[test]
    fn request_input_defaults_to_null_when_missing() {
        let request: CallRequest = serde_json::from_value(json!({
            "protocolVersion": 1, "operation": "system.describe", "requestId": "r2"
        }))
        .unwrap();
        assert!(request.input.is_null());
    }

    #[test]
    fn identifiers_reject_empty_and_overlong_values() {
        assert!(RequestId::new("").is_err());
        assert!(IdempotencyKey::new("x".repeat(129)).is_err());
        assert!(IdempotencyKey::new("x".repeat(128)).is_ok());
        let err = serde_json::from_value::<RequestId>(json!("")).unwrap_err();
        assert!(err.to_string().contains("requestId"));
    }

    #[test]
    fn reply_is_tagged_by_kind_with_camel_case_fields() {
        let complete = CallReply::complete(json!([1]), Some(7));
        assert_eq!(
            serde_json::to_value(&complete).unwrap(),
            json!({"kind": "complete", "output": [1], "revision": 7})
        );
        let accepted = CallReply::Accepted {
            execution_id: "exec_1".into(),
            revision: None,
        };
        assert_eq!(
            serde_json::to_value(&accepted).unwrap(),
            json!({"kind": "accepted", "executionId": "exec_1"})
        );
    }

    #[test]
    fn command_constructor_attaches_fresh_idempotency_key() {
        let a = CallRequest::command(OperationId::ProjectCreate, json!({}));
        let b = CallRequest::command(OperationId::ProjectCreate, json!({}));
        assert!(a.idempotency_key.is_some());
        assert_ne!(a.idempotency_key, b.idempotency_key);
        assert_ne!(a.request_id, b.request_id);
        assert!(CallRequest::query(OperationId::ProjectList, json!({}))
            .idempotency_key
            .is_none());
    }
}
