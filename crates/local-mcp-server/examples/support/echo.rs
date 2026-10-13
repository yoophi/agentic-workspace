use local_mcp_server::{principal, rmcp};
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ListToolsResult,
        PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerConfig, Tool,
    },
    service::RequestContext,
};
use serde_json::json;
use std::{borrow::Cow, sync::Arc};

#[derive(Clone)]
pub struct Echo(pub &'static str);

impl Echo {
    fn tool(&self) -> Tool {
        let mut tool = Tool::new(
            self.0,
            "Echo a value and the authenticated opaque principal",
            Arc::new(
                json!({
                    "type":"object", "properties":{"value":{"type":"string"}},
                    "required":["value"], "additionalProperties":false
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        );
        tool.output_schema = Some(Arc::new(
            json!({"type":"object"}).as_object().unwrap().clone(),
        ));
        tool
    }
}

impl ServerHandler for Echo {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2025_11_25)
            .with_server_info(Implementation::new("local-mcp-example", "0.1.0"))
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(&[ProtocolVersion::V_2025_11_25])
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        (name == self.0).then(|| self.tool())
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(vec![self.tool()]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if request.name != self.0 {
            return Err(ErrorData::new(
                rmcp::model::ErrorCode::METHOD_NOT_FOUND,
                "Unknown tool",
                None,
            ));
        }
        let Some(scope) = principal::<String>(&context) else {
            return Ok(CallToolResult::structured_error(
                json!({"ok":false,"error":{"code":"missingPrincipal"}}),
            )
            .into());
        };
        let parts = context.extensions.get::<http::request::Parts>().unwrap();
        assert!(!parts.headers.contains_key(http::header::AUTHORIZATION));
        let arguments = request.arguments.as_ref();
        let value = request
            .arguments
            .as_ref()
            .and_then(|args| args.get("value"))
            .and_then(|v| v.as_str());
        if value.is_none() || value == Some("fail") || arguments.is_none_or(|args| args.len() != 1)
        {
            return Ok(CallToolResult::structured_error(
                json!({"ok":false,"error":{"code":"invalidValue"}}),
            )
            .into());
        }
        Ok(
            CallToolResult::structured(json!({"ok":true,"data":{"value":value,"principal":scope}}))
                .into(),
        )
    }
}
