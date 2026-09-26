//! operation → handler 매핑과 입력 디코딩. 입력 검증은 typed 역직렬화(`deny_unknown_fields`)로 한다.
//! descriptor의 스키마는 describe·OpenAPI 노출용이며 runtime 검증기는 별도로 두지 않는다.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use workbench_protocol::{
    operations::{schema_for, spec_for},
    AuthenticatedPrincipal, CallReply, IdempotencyKey, OperationDescriptor, OperationId, RequestId,
    WorkbenchFault,
};

/// handler에 넘기는 호출 문맥. principal은 인증 계층이 만든 값이다.
#[derive(Debug, Clone)]
pub struct CallContext {
    pub principal: AuthenticatedPrincipal,
    pub request_id: RequestId,
    pub idempotency_key: Option<IdempotencyKey>,
    pub expected_revision: Option<u64>,
}

#[async_trait]
pub trait OperationHandler: Send + Sync {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault>;
}

#[derive(Default)]
pub struct Registry {
    handlers: HashMap<OperationId, Arc<dyn OperationHandler>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, id: OperationId, handler: Arc<dyn OperationHandler>) {
        self.handlers.insert(id, handler);
    }

    pub fn handler_for(&self, id: OperationId) -> Option<Arc<dyn OperationHandler>> {
        self.handlers.get(&id).cloned()
    }

    /// 정적 표 + 스키마로 descriptor를 만든다. 037은 CLI·MCP 노출이 없다.
    pub fn descriptor_for(&self, id: OperationId) -> OperationDescriptor {
        descriptor_for(id)
    }
}

/// 정적 표 + 스키마로 descriptor를 만든다. handler 등록 여부와 무관하다.
pub fn descriptor_for(id: OperationId) -> OperationDescriptor {
    let spec = spec_for(id);
    let (input_schema, output_schema) = schema_for(id);
    OperationDescriptor {
        id: id.as_str().to_owned(),
        kind: spec.kind,
        effect: spec.effect,
        idempotent: spec.idempotent,
        required_scopes: spec.required_scopes.to_vec(),
        input_schema,
        output_schema,
        cli_exposure: None,
        mcp_exposure: false,
    }
}

/// `input`을 typed 값으로 바꾼다. `null`(필드 생략)은 빈 객체로 본다. 실패는 `invalidArgument`.
pub fn decode_input<T: DeserializeOwned>(
    request_id: &RequestId,
    input: &serde_json::Value,
) -> Result<T, WorkbenchFault> {
    let value = if input.is_null() {
        serde_json::Value::Object(Default::default())
    } else {
        input.clone()
    };
    serde_json::from_value(value).map_err(|error| {
        WorkbenchFault::invalid_argument(
            request_id.clone(),
            format!("입력이 계약과 다릅니다: {error}"),
            None,
        )
        .with_details(serde_json::json!({ "reason": error.to_string() }))
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use workbench_protocol::{operations::project::ProjectListInput, FaultCode};

    use super::*;

    fn rid() -> RequestId {
        RequestId::new("r1").unwrap()
    }

    #[test]
    fn decode_accepts_empty_object_and_null_for_empty_input() {
        decode_input::<ProjectListInput>(&rid(), &json!({})).unwrap();
        decode_input::<ProjectListInput>(&rid(), &serde_json::Value::Null).unwrap();
    }

    #[test]
    fn decode_rejects_unknown_fields_and_wrong_types() {
        let fault = decode_input::<ProjectListInput>(&rid(), &json!({"foo": 1})).unwrap_err();
        assert_eq!(fault.code, FaultCode::InvalidArgument);
        assert!(fault.details.unwrap()["reason"]
            .as_str()
            .unwrap()
            .contains("foo"));
        let fault = decode_input::<ProjectListInput>(&rid(), &json!("text")).unwrap_err();
        assert_eq!(fault.code, FaultCode::InvalidArgument);
    }

    #[test]
    fn descriptor_reflects_static_spec_and_schemas() {
        let registry = Registry::new();
        let descriptor = registry.descriptor_for(OperationId::ProjectCreate);
        assert_eq!(descriptor.id, "project.create");
        assert!(descriptor.idempotent);
        assert_eq!(
            descriptor.required_scopes,
            vec![workbench_protocol::Scope::ProjectWrite]
        );
        assert!(descriptor.input_schema.is_object());
        assert!(descriptor.output_schema.is_object());
        assert!(registry.handler_for(OperationId::ProjectCreate).is_none());
    }
}
