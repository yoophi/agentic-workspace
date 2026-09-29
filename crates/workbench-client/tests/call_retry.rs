use async_trait::async_trait;
use serde_json::json;
use workbench_client::{
    application::call::{execute, validate_request},
    domain::attempt::{Attempt, EndpointIdentity},
    ports::{CallTransport, ClientError},
};
use workbench_protocol::{CallReply, CallRequest, OperationId, Outcome};
struct Consumer {
    identity: EndpointIdentity,
    calls: usize,
    lost: bool,
}
#[async_trait]
impl CallTransport for Consumer {
    fn identity(&self) -> &EndpointIdentity {
        &self.identity
    }
    async fn call(&mut self, _request: &CallRequest) -> Result<CallReply, ClientError> {
        self.calls += 1;
        if self.lost {
            self.lost = false;
            Err(ClientError::TransportUnknown)
        } else {
            Ok(CallReply::complete(json!({"effectCount":1}), Some(1)).mark_replayed())
        }
    }
    async fn close(&mut self) -> Result<(), ClientError> {
        Ok(())
    }
}
#[tokio::test]
async fn explicit_retry_retains_identity_and_new_epoch_transmits_zero() {
    let identity = EndpointIdentity::new("i", "e").unwrap();
    let request = CallRequest::command(
        OperationId::ProjectCreate,
        json!({"name":"test","workingDirectory":"/private/tmp"}),
    );
    let mut attempt = Attempt::new(request.clone(), identity.clone()).unwrap();
    let mut transport = Consumer {
        identity,
        calls: 0,
        lost: true,
    };
    assert!(execute(&mut transport, &mut attempt).await.is_err());
    assert_eq!(attempt.outcome(), Outcome::Unknown);
    transport.identity = EndpointIdentity::new("i", "new").unwrap();
    assert!(execute(&mut transport, &mut attempt).await.is_err());
    assert_eq!(transport.calls, 1);
    transport.identity = EndpointIdentity::new("i", "e").unwrap();
    execute(&mut transport, &mut attempt).await.unwrap();
    assert_eq!(attempt.request(), &request);
    assert_eq!(transport.calls, 2);
    assert_eq!(attempt.outcome(), Outcome::Applied);
}
#[test]
fn input_schema_and_gate_are_checked_before_transport() {
    for request in [
        CallRequest::query(OperationId::ProjectList, json!({"private":"sentinel"})),
        CallRequest::command(OperationId::ProjectCreate, json!({})),
        CallRequest::command(OperationId::OrchestrationRecover, json!({"benchId":"b"})),
    ] {
        assert!(validate_request(&request).is_err());
    }
    assert!(validate_request(&CallRequest::query(OperationId::ProjectList, json!({}))).is_ok());
}

struct PendingConsumer {
    identity: EndpointIdentity,
    entered: std::sync::Arc<tokio::sync::Notify>,
}
#[async_trait]
impl CallTransport for PendingConsumer {
    fn identity(&self) -> &EndpointIdentity {
        &self.identity
    }
    async fn call(&mut self, _request: &CallRequest) -> Result<CallReply, ClientError> {
        self.entered.notify_one();
        std::future::pending().await
    }
    async fn close(&mut self) -> Result<(), ClientError> {
        Ok(())
    }
}
#[tokio::test]
async fn dropped_execute_becomes_unknown_and_explicit_retry_rejects_old_completion() {
    let identity = EndpointIdentity::new("i", "e").unwrap();
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let request = CallRequest::command(
        OperationId::ProjectCreate,
        json!({"name":"test","workingDirectory":"/private/tmp"}),
    );
    let mut attempt = Attempt::new(request.clone(), identity.clone()).unwrap();
    let mut pending = PendingConsumer {
        identity: identity.clone(),
        entered: entered.clone(),
    };
    let mut future = Box::pin(execute(&mut pending, &mut attempt));
    tokio::select! {result=&mut future=>panic!("pending transport completed: {result:?}"),()=entered.notified()=>{}}
    drop(future);
    assert_eq!(attempt.outcome(), Outcome::Unknown);
    assert!(matches!(
        attempt.state(),
        workbench_client::domain::attempt::AttemptState::Unknown
    ));
    let old_generation = attempt.generation();
    let mut retry = Consumer {
        identity,
        calls: 0,
        lost: false,
    };
    execute(&mut retry, &mut attempt).await.unwrap();
    assert_eq!(attempt.request(), &request);
    assert_eq!(attempt.outcome(), Outcome::Applied);
    assert_eq!(retry.calls, 1);
    assert_eq!(
        attempt.complete(
            old_generation,
            CallReply::complete(json!({"old":true}), None)
        ),
        Err(workbench_client::domain::attempt::AttemptError::StaleGeneration)
    );
    assert_eq!(attempt.outcome(), Outcome::Applied);
}

mod support;
#[tokio::test]
async fn wire_response_loss_reconnects_with_fresh_proof_and_same_key_effect_once() {
    use support::{Action, Peer};
    use workbench_client::{domain::limits::Limits, infrastructure::http::HttpConnection};
    let peer = Peer::spawn_multi(vec![
        Action::Close,
        Action::Reply(
            200,
            json!({"kind":"complete","output":{"effectCount":1},"revision":1,"replayed":true}),
        ),
    ])
    .await;
    let request = CallRequest::command(
        OperationId::ProjectCreate,
        json!({"name":"test","workingDirectory":"/private/tmp"}),
    );
    let mut attempt = Attempt::new(request.clone(), peer.endpoint.identity().clone()).unwrap();
    let mut first = HttpConnection::connect(peer.endpoint.clone(), Limits::default())
        .await
        .unwrap();
    assert!(matches!(
        execute(&mut first, &mut attempt).await,
        Err(ClientError::TransportUnknown)
    ));
    first.close().await.unwrap();
    assert_eq!(attempt.outcome(), Outcome::Unknown);
    let mut second = HttpConnection::connect(peer.endpoint.clone(), Limits::default())
        .await
        .unwrap();
    execute(&mut second, &mut attempt).await.unwrap();
    second.close().await.unwrap();
    assert_eq!(peer.effects.lock().unwrap().len(), 1);
    let seen = peer.requests.lock().unwrap();
    assert_eq!(seen.len(), 6);
    assert!(!seen[0].1 && !seen[3].1);
    assert_eq!(seen[2].2, seen[5].2);
    assert_eq!(attempt.outcome(), Outcome::Applied);
}
