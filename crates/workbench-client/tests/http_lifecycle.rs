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
async fn mismatched_handshake_closes_without_sending_a_call() {
    let mut peer = Peer::spawn_at(vec![], true, "wrong-handshake").await;
    assert!(matches!(
        HttpConnection::connect(peer.endpoint.clone(), limits()).await,
        Err(ClientError::Incompatible)
    ));
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn proof_socket_replacement_is_listening_before_identify_response_release() {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    use std::{fs, os::unix::fs::PermissionsExt, sync::Arc};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpSocket},
        sync::oneshot,
    };
    use workbench_client::{
        application::admission::CallerProfile, infrastructure::locator::read_descriptor,
    };
    let old_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = old_listener.local_addr().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().canonicalize().unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    let path = parent.join("server.json");
    fs::write(&path,json!({"formatVersion":1,"mode":"server","instanceId":"i","serverEpoch":"e","baseUrl":format!("http://{address}"),"protocolVersions":[1],"storageSchemaVersion":2,"ownerToken":support::TOKEN}).to_string()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let endpoint = Arc::new(read_descriptor(&path, CallerProfile::Owner).unwrap());
    let (ready_tx, ready_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let (stop_tx, mut stop_rx) = oneshot::channel();
    let peers = tokio::spawn(async move {
        let (mut original, _) = old_listener.accept().await.unwrap();
        drop(old_listener);
        let mut head = Vec::new();
        let mut byte = [0u8];
        while !head.ends_with(b"\r\n\r\n") {
            original.read_exact(&mut byte).await.unwrap();
            head.push(byte[0]);
            assert!(head.len() < 16384);
        }
        let head = String::from_utf8(head).unwrap();
        assert!(!head.to_lowercase().contains("authorization:"));
        let len = head
            .lines()
            .find_map(|line| {
                line.to_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|s| s.trim().parse::<usize>().ok())
            })
            .unwrap();
        let mut body = vec![0; len];
        original.read_exact(&mut body).await.unwrap();
        let identity: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let replacement = TcpSocket::new_v4().unwrap();
        replacement.set_reuseaddr(true).unwrap();
        replacement.bind(address).unwrap();
        let replacement = replacement.listen(8).unwrap();
        let monitor = tokio::spawn(async move {
            let mut connections = 0;
            let mut credentials = 0;
            loop {
                // Accept takes priority so queued replacement connections cannot hide behind teardown.
                tokio::select! {biased;
                    accepted=replacement.accept()=>{
                        let (mut socket,_)=accepted.unwrap();connections+=1;let mut bytes=vec![0;16384];
                        if let Ok(Ok(n))=tokio::time::timeout(Duration::from_millis(100),socket.read(&mut bytes)).await {if String::from_utf8_lossy(&bytes[..n]).to_lowercase().contains("authorization:"){credentials+=1;}}
                    },
                    _=&mut stop_rx=>break,
                }
            }
            (connections, credentials)
        });
        ready_tx.send(()).unwrap();
        release_rx.await.unwrap();
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&Sha256::digest(support::TOKEN.as_bytes())).unwrap();
        mac.update(format!("{}\ni", identity["nonce"].as_str().unwrap()).as_bytes());
        let proof: String = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let body = json!({"instanceId":"i","proof":proof}).to_string();
        let response=format!("HTTP/1.1 200 OK\r\nconnection: close\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",body.len());
        original.write_all(response.as_bytes()).await.unwrap();
        drop(original);
        monitor.await.unwrap()
    });
    let connecting = tokio::spawn(HttpConnection::connect(endpoint, Limits::default()));
    tokio::time::timeout(Duration::from_secs(1), ready_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(
        !connecting.is_finished(),
        "client completed before proof release"
    );
    release_tx.send(()).unwrap();
    assert!(tokio::time::timeout(Duration::from_secs(1), connecting)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    stop_tx.send(()).unwrap();
    let counts = tokio::time::timeout(Duration::from_secs(1), peers)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(counts, (0, 0));
}
