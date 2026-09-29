mod support;
use serde_json::json;
use support::{Action, Peer};
use workbench_client::{
    domain::limits::Limits,
    infrastructure::http::HttpConnection,
    ports::{CallTransport, ClientError},
};
use workbench_protocol::{CallRequest, FaultCode, OperationId, Outcome, WorkbenchFault};
#[tokio::test]
async fn same_socket_identity_handshake_call_preserves_complete_reply() {
    let peer = Peer::spawn(
        vec![Action::Reply(
            200,
            json!({"kind":"complete","output":[1],"revision":7,"replayed":true}),
        )],
        true,
    )
    .await;
    let mut connection = HttpConnection::connect(peer.endpoint.clone(), Limits::default())
        .await
        .unwrap();
    let result = connection
        .call(&CallRequest::query(OperationId::ProjectList, json!({})))
        .await
        .unwrap();
    assert_eq!(result.revision(), Some(7));
    assert_eq!(serde_json::to_value(result).unwrap()["replayed"], true);
    connection.close().await.unwrap();
    let seen = peer.requests.lock().unwrap();
    assert_eq!(seen.len(), 3);
    assert!(!seen[0].1);
    assert!(seen[1].1 && seen[2].1);
}
#[tokio::test]
async fn failed_proof_never_sends_credentials() {
    let peer = Peer::spawn(vec![], false).await;
    assert!(matches!(
        HttpConnection::connect(peer.endpoint.clone(), Limits::default()).await,
        Err(ClientError::Identity)
    ));
    let seen = peer.requests.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert!(!seen[0].1);
}
#[tokio::test]
async fn malformed_and_status_mismatched_replies_never_succeed() {
    for action in [
        Action::Raw(200, b"not-json-private-sentinel".to_vec()),
        Action::Reply(200, json!({})),
        Action::Reply(302, json!({"kind":"complete","output":null})),
        Action::Reply(200, json!({"kind":"complete"})),
        Action::Reply(200, json!({"kind":"accepted","executionId":""})),
    ] {
        let peer = Peer::spawn(vec![action], true).await;
        let mut connection = HttpConnection::connect(peer.endpoint.clone(), Limits::default())
            .await
            .unwrap();
        assert!(matches!(
            connection
                .call(&CallRequest::query(OperationId::ProjectList, json!({})))
                .await,
            Err(ClientError::Protocol)
        ));
        connection.close().await.unwrap();
    }
}
#[tokio::test]
async fn full_fault_is_preserved_with_exact_request_and_http_identity() {
    let request = CallRequest::command(
        OperationId::ProjectCreate,
        json!({"name":"test","workingDirectory":"/private/tmp"}),
    );
    let fault = WorkbenchFault::new(
        FaultCode::Conflict,
        request.request_id.clone(),
        "private-sentinel",
    )
    .with_outcome(Outcome::Unknown)
    .with_details(json!({"private":"sentinel"}));
    let mut value = serde_json::to_value(&fault).unwrap();
    value["status"] = json!(409);
    let peer = Peer::spawn(vec![Action::Reply(409, value)], true).await;
    let mut connection = HttpConnection::connect(peer.endpoint.clone(), Limits::default())
        .await
        .unwrap();
    match connection.call(&request).await {
        Err(ClientError::Fault(result)) => assert_eq!(*result, fault),
        other => panic!("{other:?}"),
    };
    connection.close().await.unwrap();
}
#[tokio::test]
async fn transport_loss_is_unknown_without_resubmission() {
    let peer = Peer::spawn(vec![Action::Close], true).await;
    let mut connection = HttpConnection::connect(peer.endpoint.clone(), Limits::default())
        .await
        .unwrap();
    assert!(matches!(
        connection
            .call(&CallRequest::command(
                OperationId::ProjectCreate,
                json!({"name":"test","workingDirectory":"/private/tmp"})
            ))
            .await,
        Err(ClientError::TransportUnknown)
    ));
    connection.close().await.unwrap();
    assert_eq!(peer.requests.lock().unwrap().len(), 3);
}

#[test]
fn fault_request_status_and_body_identity_must_all_match() {
    use workbench_client::infrastructure::http::decode_reply;
    let request = CallRequest::query(OperationId::ProjectList, json!({}));
    let mut fault = serde_json::to_value(WorkbenchFault::new(
        FaultCode::Forbidden,
        request.request_id.clone(),
        "secret",
    ))
    .unwrap();
    fault["status"] = json!(403);
    assert!(matches!(
        decode_reply(403, fault.clone(), &request),
        Err(ClientError::Fault(_))
    ));
    assert!(matches!(
        decode_reply(401, fault.clone(), &request),
        Err(ClientError::Protocol)
    ));
    fault["requestId"] = json!("wrong");
    assert!(matches!(
        decode_reply(403, fault, &request),
        Err(ClientError::Protocol)
    ));
    for value in [
        json!({"kind":"future","output":null}),
        json!({"kind":"complete","output":null,"private":"echo"}),
        json!({"kind":"complete","output":null,"replayed":"true"}),
    ] {
        assert!(matches!(
            decode_reply(200, value, &request),
            Err(ClientError::Protocol)
        ));
    }
}
#[tokio::test]
async fn body_limit_is_enforced_before_retention_and_closes_socket() {
    use workbench_client::domain::limits::{LimitConfig, Resource};
    let peer = Peer::spawn(vec![Action::Raw(200, vec![b' '; 2049])], true).await;
    let limits = Limits::new(LimitConfig {
        body_bytes: 2048,
        ..Default::default()
    })
    .unwrap();
    let mut connection = HttpConnection::connect(peer.endpoint.clone(), limits)
        .await
        .unwrap();
    assert!(matches!(
        connection
            .call(&CallRequest::query(OperationId::ProjectList, json!({})))
            .await,
        Err(ClientError::Limit(
            workbench_client::domain::limits::LimitError::Exceeded(Resource::Body)
        ))
    ));
    connection.close().await.unwrap();
}
