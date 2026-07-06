use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const JSON_RPC_VERSION: &str = "2.0";
pub const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: Option<String>,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn result(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: JSON_RPC_VERSION,
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSON_RPC_VERSION,
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

pub fn parse_request(value: Value) -> Result<JsonRpcRequest, JsonRpcResponse> {
    serde_json::from_value::<JsonRpcRequest>(value).map_err(|error| {
        JsonRpcResponse::error(None, -32600, format!("Invalid JSON-RPC request: {error}"))
    })
}

pub fn initialize_result() -> Value {
    json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "agentic-workbench-session-control",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{MCP_PROTOCOL_VERSION, initialize_result, parse_request};
    use serde_json::json;

    #[test]
    fn parses_json_rpc_request() {
        let request = parse_request(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        }))
        .unwrap();

        assert_eq!(request.method, "tools/list");
        assert_eq!(request.id, Some(json!(1)));
    }

    #[test]
    fn rejects_invalid_json_rpc_request() {
        let response = parse_request(json!({ "jsonrpc": "2.0" })).unwrap_err();
        assert!(
            response
                .error
                .unwrap()
                .message
                .contains("Invalid JSON-RPC request")
        );
    }

    #[test]
    fn initialize_result_advertises_tools_capability() {
        let result = initialize_result();
        assert_eq!(result["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert!(result["capabilities"]["tools"].is_object());
    }
}
