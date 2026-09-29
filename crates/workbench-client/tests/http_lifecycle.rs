mod support;
use serde_json::json;
use std::time::Duration;
use support::{Action, Peer};
use workbench_client::{
    domain::limits::{LimitConfig, Limits},
    infrastructure::http::HttpConnection,
    ports::{CallTransport, ClientError},
};
use workbench_protocol::{CallRequest, OperationId};
fn limits() -> Limits {
    Limits::new(LimitConfig {
        request_timeout: Duration::from_millis(100),
        connect_timeout: Duration::from_millis(100),
        ..Default::default()
    })
    .unwrap()
}
#[tokio::test]
async fn slow_proof_and_handshake_have_whole_deadline_and_socket_settles() {
    for path in ["/v1/system/identify", "/v1/system/handshake"] {
        let mut peer = Peer::spawn_at(vec![], true, path).await;
        assert!(matches!(
            HttpConnection::connect(peer.endpoint.clone(), limits()).await,
            Err(ClientError::Deadline)
        ));
        peer.settled().await;
        let seen = peer.requests.lock().unwrap();
        assert_eq!(seen.len(), if path.ends_with("identify") { 1 } else { 2 });
        assert!(!seen[0].1);
    }
}
#[tokio::test]
async fn slow_headers_and_body_poison_connection_without_cancel_or_retry() {
    for action in [Action::Pause, Action::SlowBody] {
        let mut peer = Peer::spawn(vec![action], true).await;
        let mut connection = HttpConnection::connect(peer.endpoint.clone(), limits())
            .await
            .unwrap();
        let request = CallRequest::command(
            OperationId::ProjectCreate,
            json!({"name":"test","workingDirectory":"/private/tmp"}),
        );
        assert!(matches!(
            connection.call(&request).await,
            Err(ClientError::Deadline)
        ));
        assert!(matches!(
            connection.call(&request).await,
            Err(ClientError::Unavailable)
        ));
        connection.close().await.unwrap();
        peer.settled().await;
        assert_eq!(peer.requests.lock().unwrap().len(), 3);
    }
}
#[tokio::test]
async fn dropped_caller_aborts_owned_driver_and_peer_sees_eof() {
    let mut peer = Peer::spawn(vec![], true).await;
    let connection = HttpConnection::connect(peer.endpoint.clone(), limits())
        .await
        .unwrap();
    drop(connection);
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 2);
}
#[tokio::test]
async fn cancelling_connect_future_closes_proof_socket() {
    let mut peer = Peer::spawn_at(vec![], true, "/v1/system/identify").await;
    let future = HttpConnection::connect(peer.endpoint.clone(), Limits::default());
    assert!(tokio::time::timeout(Duration::from_millis(50), future)
        .await
        .is_err());
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn dropping_only_call_future_retires_retained_connection() {
    let mut peer = Peer::spawn(vec![Action::Pause], true).await;
    let mut connection = HttpConnection::connect(peer.endpoint.clone(), Limits::default())
        .await
        .unwrap();
    let request = CallRequest::query(OperationId::ProjectList, json!({}));
    let mut future = Box::pin(connection.call(&request));
    loop {
        if peer.requests.lock().unwrap().len() == 3 {
            break;
        }
        tokio::select! {result=&mut future=>panic!("pending response completed: {result:?}"),()=peer.received.notified()=>{}}
    }
    drop(future);
    assert!(matches!(
        connection.call(&request).await,
        Err(ClientError::Unavailable)
    ));
    connection.close().await.unwrap();
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 3);
}
#[tokio::test]
async fn proof_socket_close_does_not_reconnect_to_replacement_or_send_its_credentials() {
    let mut peer = Peer::spawn_at(vec![], true, "close-after-identify").await;
    assert!(HttpConnection::connect(peer.endpoint.clone(), limits())
        .await
        .is_err());
    peer.settled().await;
    let socket = tokio::net::TcpSocket::new_v4().unwrap();
    socket.set_reuseaddr(true).unwrap();
    socket.bind(peer.endpoint.address()).unwrap();
    let listener = socket.listen(1).unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
    let seen = peer.requests.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert!(!seen[0].1);
}

#[tokio::test]
async fn mismatched_handshake_closes_without_sending_a_call() {
    let mut peer = Peer::spawn_at(vec![], true, "wrong-handshake").await;
    assert!(matches!(
        HttpConnection::connect(peer.endpoint.clone(), limits()).await,
        Err(ClientError::Incompatible)
    ));
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 2);
}
