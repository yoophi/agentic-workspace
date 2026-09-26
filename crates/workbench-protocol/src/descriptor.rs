//! `system.describe`가 돌려주는 operation 계약 설명.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::principal::Scope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum OperationKind {
    Query,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Effect {
    Read,
    Modify,
}

/// operation 하나의 계약. `inputSchema`/`outputSchema`는 OpenAPI 3.1 호환 JSON Schema다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OperationDescriptor {
    /// 예: `project.list`
    pub id: String,
    pub kind: OperationKind,
    pub effect: Effect,
    /// command면 멱등성 키가 필수라는 뜻. query는 항상 false.
    pub idempotent: bool,
    pub required_scopes: Vec<Scope>,
    #[schema(value_type = Object)]
    pub input_schema: serde_json::Value,
    #[schema(value_type = Object)]
    pub output_schema: serde_json::Value,
    /// CLI command projection. 037은 항상 None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli_exposure: Option<String>,
    /// MCP tool 노출 opt-in. 037은 항상 false.
    #[serde(default)]
    pub mcp_exposure: bool,
}

/// `system.describe` 출력. 호출자에게 허용된 operation만 담는다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DescribeOutput {
    pub protocol_version: u16,
    pub operations: Vec<OperationDescriptor>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn descriptor_omits_cli_exposure_when_absent() {
        let descriptor = OperationDescriptor {
            id: "project.list".into(),
            kind: OperationKind::Query,
            effect: Effect::Read,
            idempotent: false,
            required_scopes: vec![Scope::ProjectRead],
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "array"}),
            cli_exposure: None,
            mcp_exposure: false,
        };
        let value = serde_json::to_value(&descriptor).unwrap();
        assert_eq!(value["kind"], "query");
        assert_eq!(value["effect"], "read");
        assert_eq!(value["requiredScopes"], json!(["project:read"]));
        assert!(value.get("cliExposure").is_none());
        assert_eq!(value["mcpExposure"], false);
    }
}
