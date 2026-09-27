//! `system.describe`: principal에게 허용된 operation의 계약만 돌려준다(정본 Invariant 2).

use async_trait::async_trait;
use workbench_protocol::{
    events::{EventSchemaDescriptor, EVENT_SCHEMAS},
    operations::system::SystemDescribeInput,
    CallReply, DescribeOutput, WorkbenchFault, PROTOCOL_VERSION,
};

use crate::application::{
    authorization::visible_operations,
    registry::{decode_input, descriptor_for, CallContext, OperationHandler},
};

pub struct SystemDescribeHandler {
    epoch: String,
}

impl SystemDescribeHandler {
    pub fn new(epoch: impl Into<String>) -> Self {
        Self {
            epoch: epoch.into(),
        }
    }
}

pub fn describe(
    principal: &workbench_protocol::AuthenticatedPrincipal,
    epoch: &str,
) -> DescribeOutput {
    DescribeOutput {
        protocol_version: PROTOCOL_VERSION,
        epoch: epoch.to_owned(),
        operations: visible_operations(principal)
            .into_iter()
            .map(descriptor_for)
            .collect(),
        event_schemas: EVENT_SCHEMAS
            .iter()
            .filter(|spec| spec.stream_kind.is_subscribable())
            .filter(|spec| principal.has_scope(spec.stream_kind.required_scope()))
            .map(EventSchemaDescriptor::from)
            .collect(),
    }
}

#[async_trait]
impl OperationHandler for SystemDescribeHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        decode_input::<SystemDescribeInput>(&ctx.request_id, &input)?;
        Ok(CallReply::complete(
            serde_json::to_value(describe(&ctx.principal, &self.epoch))
                .expect("describe serializes"),
            None,
        ))
    }
}
