mod support;
use serde_json::json;
use support::{Action, Peer};
use tokio_tungstenite::tungstenite::Message;
use workbench_client::{
    domain::limits::Limits, infrastructure::websocket::WebSocketConnection, ports::ClientError,
};
use workbench_protocol::{events::EventFrame, workbench::StreamCursor};
fn cursors() -> Vec<StreamCursor> {
    vec![StreamCursor {
        stream_id: "orchestration:binding".into(),
        epoch: "e".into(),
        after_sequence: 0,
    }]
}
fn hello(epoch: &str) -> Message {
    Message::Text(json!({"type":"hello","protocolVersion":1,"epoch":epoch}).to_string())
}
fn ticket() -> Action {
    Action::Reply(
        200,
        json!({"ticket":"private-ticket-sentinel","expiresAt":"2026-09-29T00:00:30Z"}),
    )
}
#[tokio::test]
async fn separate_socket_proof_precedes_ticket_upgrade_and_hello() {
    let mut p=Peer::spawn_multi(vec![ticket(),Action::WebSocket(vec![hello("e"),Message::Text(json!({"type":"gap","streamId":"orchestration:binding","epoch":"e","reason":"retentionExceeded","firstSequence":3,"lastSequence":4}).to_string())])]).await;
    let mut ws = WebSocketConnection::connect(p.endpoint.clone(), Limits::default(), cursors())
        .await
        .unwrap();
    assert!(
        matches!(ws.next().await.unwrap(),Some(EventFrame::Gap{gap}) if gap.last_sequence==Some(4))
    );
    ws.close().await.unwrap();
    p.settled().await;
    let requests = p.requests.lock().unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|r| r.0.split('?').next().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "/v1/system/identify",
            "/v1/system/handshake",
            "/v1/event-tickets",
            "/v1/system/identify",
            "/v1/events"
        ]
    );
    assert_eq!(
        requests.iter().map(|r| r.1).collect::<Vec<_>>(),
        vec![false, true, true, false, false]
    );
    assert_ne!(requests[0].2["nonce"], requests[3].2["nonce"]);
    assert_eq!(requests[2].2, json!({"cursors":cursors()}));
}
#[tokio::test]
async fn wrong_proof_on_second_socket_never_sends_ticket() {
    let p = Peer::spawn_multi_at(vec![ticket()], "wrong-ws-identity").await;
    assert!(matches!(
        WebSocketConnection::connect(p.endpoint.clone(), Limits::default(), cursors()).await,
        Err(ClientError::Identity)
    ));
    assert_eq!(
        p.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.0.starts_with("/v1/events"))
            .count(),
        0
    );
}
#[tokio::test]
async fn wrong_hello_epoch_closes_without_accepting_event() {
    let mut p = Peer::spawn_multi(vec![ticket(), Action::WebSocket(vec![hello("other")])]).await;
    assert!(matches!(
        WebSocketConnection::connect(p.endpoint.clone(), Limits::default(), cursors()).await,
        Err(ClientError::Incompatible)
    ));
    p.settled().await;
}
#[tokio::test]
async fn event_before_hello_is_rejected_and_socket_reaped() {
    let mut p=Peer::spawn_multi(vec![ticket(),Action::WebSocket(vec![Message::Text("{\"type\":\"gap\",\"streamId\":\"orchestration:binding\",\"epoch\":\"e\",\"reason\":\"shutdown\"}".into())])]).await;
    assert!(matches!(
        WebSocketConnection::connect(p.endpoint.clone(), Limits::default(), cursors()).await,
        Err(ClientError::Protocol)
    ));
    p.settled().await;
}
fn raw_frame(opcode: u8, fin: bool, body: &[u8]) -> Vec<u8> {
    let mut bytes = vec![opcode | if fin { 128 } else { 0 }];
    if body.len() < 126 {
        bytes.push(body.len() as u8)
    } else {
        bytes.push(126);
        bytes.extend_from_slice(&(body.len() as u16).to_be_bytes())
    }
    bytes.extend_from_slice(body);
    bytes
}
fn raw_hello() -> Vec<u8> {
    let Message::Text(text) = hello("e") else {
        unreachable!()
    };
    raw_frame(1, true, text.as_bytes())
}
fn fast_limits() -> Limits {
    Limits::new(workbench_client::domain::limits::LimitConfig {
        request_timeout: std::time::Duration::from_millis(100),
        ..Default::default()
    })
    .unwrap()
}
#[tokio::test]
async fn partial_header_and_unfinished_fragment_have_deadlines_and_peer_eof() {
    for partial in [vec![0x81], raw_frame(1, false, b"{")] {
        let mut bytes = raw_hello();
        bytes.extend(partial);
        let mut p = Peer::spawn_multi(vec![ticket(), Action::WebSocketRaw(bytes)]).await;
        let mut ws = WebSocketConnection::connect(p.endpoint.clone(), fast_limits(), cursors())
            .await
            .unwrap();
        assert!(matches!(ws.next().await, Err(ClientError::Deadline)));
        assert!(matches!(ws.next().await, Err(ClientError::Unavailable)));
        p.settled().await;
    }
}
#[tokio::test]
async fn frame_header_quota_and_fragmented_message_quota_are_distinct() {
    use workbench_client::domain::limits::{LimitConfig, LimitError, Resource};
    let limits = Limits::new(LimitConfig {
        frame_bytes: 128,
        message_bytes: 128,
        ..Default::default()
    })
    .unwrap();
    for (payload, resource) in [
        (vec![0x81, 126, 0, 129], Resource::Frame),
        (
            {
                let mut raw = raw_frame(1, false, &[b' '; 70]);
                raw.extend(raw_frame(0, true, &[b' '; 70]));
                raw
            },
            Resource::Message,
        ),
    ] {
        let mut bytes = raw_hello();
        bytes.extend(payload);
        let mut p = Peer::spawn_multi(vec![ticket(), Action::WebSocketRaw(bytes)]).await;
        let mut ws = WebSocketConnection::connect(p.endpoint.clone(), limits.clone(), cursors())
            .await
            .unwrap();
        assert!(
            matches!(ws.next().await,Err(ClientError::Limit(LimitError::Exceeded(r))) if r==resource)
        );
        p.settled().await;
    }
}
#[tokio::test]
async fn malformed_binary_unknown_frame_schema_and_binding_close_without_raw_diagnostics() {
    let frames=vec![Message::Binary(b"private-sentinel".to_vec()),Message::Text("private-sentinel".into()),Message::Text("{\"type\":\"future\",\"secret\":\"private-sentinel\"}".into()),Message::Text(json!({"type":"gap","streamId":"orchestration:other","epoch":"e","reason":"shutdown"}).to_string()),Message::Text(json!({"type":"event","event":{"eventId":"id","streamId":"orchestration:binding","epoch":"e","sequence":1,"schema":"future.v1","occurredAt":"now","body":{"private":"private-sentinel"}}}).to_string())];
    for frame in frames {
        let mut p =
            Peer::spawn_multi(vec![ticket(), Action::WebSocket(vec![hello("e"), frame])]).await;
        let mut ws = WebSocketConnection::connect(p.endpoint.clone(), Limits::default(), cursors())
            .await
            .unwrap();
        let result = ws.next().await;
        assert!(matches!(result, Err(ClientError::Protocol)));
        if let Err(error) = result {
            assert!(!format!("{error:?}").contains("private-sentinel"));
        }
        p.settled().await;
    }
}
#[tokio::test]
async fn dropping_pending_read_future_retires_owned_socket_and_has_no_implicit_reconnect() {
    let mut p = Peer::spawn_multi(vec![ticket(), Action::WebSocket(vec![hello("e")])]).await;
    let mut ws = WebSocketConnection::connect(p.endpoint.clone(), fast_limits(), cursors())
        .await
        .unwrap();
    // Borrowing a JoinHandle permits the idle wait to stay alive past the request budget.
    let mut wait = tokio::spawn(async move {
        let result = ws.next().await;
        (ws, result)
    });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(200), &mut wait)
            .await
            .is_err()
    );
    wait.abort();
    tokio::time::timeout(std::time::Duration::from_secs(1), wait)
        .await
        .unwrap()
        .unwrap_err();
    p.settled().await;
    assert_eq!(p.requests.lock().unwrap().len(), 5);
}
#[tokio::test]
async fn stream_admission_and_cursor_identity_fail_before_network() {
    let p = Peer::spawn(vec![], true).await;
    for stream in ["run:r", "worktree:/private/root", "exchange:x"] {
        let mut requested = cursors();
        requested[0].stream_id = stream.into();
        assert!(matches!(
            WebSocketConnection::connect(p.endpoint.clone(), Limits::default(), requested).await,
            Err(ClientError::PrerequisiteUnavailable)
        ));
    }
    let mut requested = cursors();
    requested[0].epoch = "other".into();
    assert!(matches!(
        WebSocketConnection::connect(p.endpoint.clone(), Limits::default(), requested).await,
        Err(ClientError::InvalidInput)
    ));
    let mut requested = cursors();
    requested.push(requested[0].clone());
    assert!(matches!(
        WebSocketConnection::connect(p.endpoint.clone(), Limits::default(), requested).await,
        Err(ClientError::InvalidInput)
    ));
    assert!(p.requests.lock().unwrap().is_empty());
}
#[tokio::test]
async fn dropping_only_pending_read_retires_socket_even_when_connection_object_is_retained() {
    use std::future::Future;
    let mut p = Peer::spawn_multi(vec![ticket(), Action::WebSocket(vec![hello("e")])]).await;
    let mut ws = WebSocketConnection::connect(p.endpoint.clone(), Limits::default(), cursors())
        .await
        .unwrap();
    let mut read = Box::pin(ws.next());
    std::future::poll_fn(|cx| match read.as_mut().poll(cx) {
        std::task::Poll::Pending => std::task::Poll::Ready(()),
        std::task::Poll::Ready(_) => panic!("read must be pending"),
    })
    .await;
    drop(read);
    assert!(matches!(ws.next().await, Err(ClientError::Unavailable)));
    p.settled().await;
    assert_eq!(p.requests.lock().unwrap().len(), 5);
}
#[tokio::test]
async fn ping_during_unfinished_fragment_does_not_reset_partial_message_deadline() {
    let mut bytes = raw_hello();
    bytes.extend(raw_frame(1, false, b"{"));
    bytes.extend(raw_frame(9, true, b"ping"));
    let mut p = Peer::spawn_multi(vec![ticket(), Action::WebSocketRaw(bytes)]).await;
    let mut ws = WebSocketConnection::connect(p.endpoint.clone(), fast_limits(), cursors())
        .await
        .unwrap();
    assert!(matches!(ws.next().await, Err(ClientError::Deadline)));
    p.settled().await;
}

#[tokio::test]
async fn ws_proof_replacement_is_listening_before_release_and_receives_no_ticket() {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    use std::time::Duration;
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
        async fn read(socket: &mut tokio::net::TcpStream) -> (String, serde_json::Value) {
            let mut bytes = Vec::new();
            let mut byte = [0];
            while !bytes.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).await.unwrap();
                bytes.push(byte[0]);
                assert!(bytes.len() < 16384);
            }
            let head = String::from_utf8(bytes).unwrap();
            let len = head
                .lines()
                .find_map(|line| {
                    line.split_once(':')
                        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .and_then(|(_, s)| s.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            let mut body = vec![0; len];
            socket.read_exact(&mut body).await.unwrap();
            (head, serde_json::from_slice(&body).unwrap())
        }
        fn proof(nonce: &str) -> String {
            let mut mac =
                Hmac::<Sha256>::new_from_slice(&Sha256::digest(support::TOKEN.as_bytes())).unwrap();
            mac.update(format!("{nonce}\ni").as_bytes());
            mac.finalize()
                .into_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        }
        let (mut ticket_socket, _) = old_listener.accept().await.unwrap();
        for step in 0..3 {
            let (head, body) = read(&mut ticket_socket).await;
            assert_eq!(head.to_lowercase().contains("authorization:"), step > 0);
            let body=match step {
                0=>json!({"instanceId":"i","proof":proof(body["nonce"].as_str().unwrap())}),
                1=>json!({"instanceId":"i","serverEpoch":"e","selectedProtocolVersion":1,"storageSchemaVersion":2,"apiMajor":1,"state":"serving"}),
                _=>json!({"ticket":"private-ticket-sentinel","expiresAt":"2026-09-29T00:00:30Z"}),
            }.to_string();
            ticket_socket.write_all(format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
        let mut eof = [0];
        assert_eq!(ticket_socket.read(&mut eof).await.unwrap(), 0);
        drop(ticket_socket);
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
            let mut tickets = 0;
            loop {
                // Accept takes priority so queued replacement connections cannot hide behind teardown.
                tokio::select! {biased;
                    accepted=replacement.accept()=>{
                        let (mut socket,_)=accepted.unwrap();connections+=1;let mut bytes=vec![0;16384];
                        if let Ok(Ok(n))=tokio::time::timeout(Duration::from_millis(100),socket.read(&mut bytes)).await {let head=String::from_utf8_lossy(&bytes[..n]).to_lowercase();if head.contains("authorization:"){credentials+=1;}
                        if head.contains("ticket="){tickets+=1;}}
                    },
                    _=&mut stop_rx=>break,
                }
            }
            (connections, credentials, tickets)
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
    let connecting = tokio::spawn(WebSocketConnection::connect(
        endpoint,
        Limits::default(),
        cursors(),
    ));
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
    assert_eq!(counts, (0, 0, 0));
}
#[tokio::test]
async fn ticket_issue_and_expired_upgrade_faults_preserve_full_typed_status_without_retry() {
    use workbench_protocol::{FaultCode, RequestId, WorkbenchFault};
    for expired_upgrade in [false, true] {
        let fault = WorkbenchFault::new(
            FaultCode::Unauthenticated,
            RequestId::random(),
            "private-ticket-sentinel",
        )
        .with_details(json!({"private":"private-sentinel"}));
        let mut actions = Vec::new();
        if expired_upgrade {
            actions.push(ticket());
        }
        let mut body = serde_json::to_value(&fault).unwrap();
        body["status"] = json!(401);
        actions.push(Action::Reply(401, body));
        let p = Peer::spawn_multi(actions).await;
        let result =
            WebSocketConnection::connect(p.endpoint.clone(), Limits::default(), cursors()).await;
        assert!(
            matches!(result,Err(ClientError::Fault(ref f)) if f==&fault),
            "expired={expired_upgrade} result={result:?}"
        );
        let requests = p.requests.lock().unwrap();
        assert_eq!(requests.len(), if expired_upgrade { 5 } else { 3 });
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.0 == "/v1/event-tickets")
                .count(),
            1
        );
    }
}
#[tokio::test]
async fn ticket_query_is_encoded_and_connection_debug_redacts_all_secrets() {
    let mut p = Peer::spawn_multi(vec![
        Action::Reply(
            200,
            json!({"ticket":"private+ticket/sentinel=","expiresAt":"2026-09-29T00:00:30Z"}),
        ),
        Action::WebSocket(vec![hello("e")]),
    ])
    .await;
    let mut ws = WebSocketConnection::connect(p.endpoint.clone(), Limits::default(), cursors())
        .await
        .unwrap();
    assert_eq!(format!("{ws:?}"), "WebSocketConnection([redacted])");
    assert_eq!(
        p.requests.lock().unwrap().last().unwrap().0,
        "/v1/events?ticket=private%2Bticket%2Fsentinel%3D"
    );
    ws.close().await.unwrap();
    p.settled().await;
}
