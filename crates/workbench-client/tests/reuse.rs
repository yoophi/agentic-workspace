use async_trait::async_trait;
use serde_json::json;
use workbench_client::{
    domain::attempt::EndpointIdentity,
    ports::{
        CallTransport, ClientError, Credential, CredentialProvider, EventConsumer, Snapshot,
        SnapshotPort,
    },
};
use workbench_protocol::{
    workbench::{EventEnvelope, StreamCursor},
    CallReply, CallRequest, OperationId,
};

struct IndependentCaller {
    identity: EndpointIdentity,
    secret: Credential,
}
impl CredentialProvider for IndependentCaller {
    fn credential(&self) -> &Credential {
        &self.secret
    }
}
#[async_trait]
impl CallTransport for IndependentCaller {
    fn identity(&self) -> &EndpointIdentity {
        &self.identity
    }
    async fn call(&mut self, request: &CallRequest) -> Result<CallReply, ClientError> {
        Ok(CallReply::complete(
            json!({"operation":request.operation}),
            Some(9),
        ))
    }
    async fn close(&mut self) -> Result<(), ClientError> {
        Ok(())
    }
}
#[async_trait]
impl EventConsumer for IndependentCaller {
    async fn consume(&mut self, _event: &EventEnvelope) -> Result<(), ClientError> {
        Ok(())
    }
    async fn reset(&mut self, _snapshot: &Snapshot) -> Result<(), ClientError> {
        Ok(())
    }
}
#[async_trait]
impl SnapshotPort for IndependentCaller {
    async fn snapshot(&mut self, cursor: &StreamCursor) -> Result<Snapshot, ClientError> {
        Ok(Snapshot {
            cursor: cursor.clone(),
            value: json!({"revision":9}),
        })
    }
}

#[tokio::test]
async fn independent_consumer_uses_ports_without_server_or_app_runtime() {
    let mut caller = IndependentCaller {
        identity: EndpointIdentity::new("i", "e").unwrap(),
        secret: Credential::new("private-sentinel".into()).unwrap(),
    };
    let request = CallRequest::query(OperationId::ProjectList, json!({}));
    let output = caller.call(&request).await.unwrap();
    assert_eq!(output.revision(), Some(9));
    let cursor = StreamCursor {
        stream_id: "run:r".into(),
        epoch: "e".into(),
        after_sequence: 0,
    };
    let snapshot = caller.snapshot(&cursor).await.unwrap();
    caller.reset(&snapshot).await.unwrap();
    caller.close().await.unwrap();
    assert!(!format!("{:?}", caller.credential()).contains("private-sentinel"));
}

#[test]
fn error_debug_hides_server_message_and_arbitrary_details() {
    let fault = workbench_protocol::WorkbenchFault::new(
        workbench_protocol::FaultCode::Internal,
        workbench_protocol::RequestId::random(),
        "private-sentinel",
    )
    .with_details(json!({"token":"private-sentinel"}));
    let error = ClientError::Fault(fault.clone());
    assert!(!format!("{error:?} {error}").contains("private-sentinel"));
    if let ClientError::Fault(preserved) = error {
        assert_eq!(preserved, fault);
    } else {
        panic!("fault lost");
    }
}
