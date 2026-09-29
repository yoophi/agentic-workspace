#![allow(dead_code)]
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};
use workbench_client::{
    application::admission::CallerProfile,
    infrastructure::locator::{read_descriptor, LocatedEndpoint},
};
pub const TOKEN: &str = "fixture-private-token";
pub enum Action {
    Reply(u16, Value),
    ReplyGate {
        status: u16,
        body: Value,
        gate: Arc<tokio::sync::Notify>,
    },
    Raw(u16, Vec<u8>),
    Fault(workbench_protocol::FaultCode, workbench_protocol::Outcome),
    Close,
    Pause,
    SlowBody,
    WebSocket(Vec<tokio_tungstenite::tungstenite::Message>),
    WebSocketConcurrent(Vec<tokio_tungstenite::tungstenite::Message>),
    WebSocketGateConcurrent {
        before: Vec<tokio_tungstenite::tungstenite::Message>,
        gate: Arc<tokio::sync::Notify>,
        after: Vec<tokio_tungstenite::tungstenite::Message>,
    },
    WebSocketGate {
        before: Vec<tokio_tungstenite::tungstenite::Message>,
        gate: Arc<tokio::sync::Notify>,
        after: Vec<tokio_tungstenite::tungstenite::Message>,
    },
    WebSocketRaw(Vec<u8>),
}
pub struct Peer {
    pub endpoint: Arc<LocatedEndpoint>,
    pub descriptor: std::path::PathBuf,
    pub requests: Arc<Mutex<Vec<(String, bool, Value)>>>,
    pub received: Arc<tokio::sync::Notify>,
    pub effects: Arc<Mutex<std::collections::HashMap<String, Value>>>,
    task: JoinHandle<()>,
    _dir: tempfile::TempDir,
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Peer {
    pub async fn spawn(actions: Vec<Action>, valid_identity: bool) -> Self {
        Self::spawn_at(actions, valid_identity, "").await
    }
    pub async fn settled(&mut self) {
        tokio::time::timeout(std::time::Duration::from_secs(1), &mut self.task)
            .await
            .expect("peer socket did not settle")
            .expect("peer failed");
    }
    pub async fn stop(&mut self) {
        self.task.abort();
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), &mut self.task)
            .await
            .expect("peer stop did not settle");
        assert!(result.is_ok() || result.unwrap_err().is_cancelled());
    }
    pub async fn spawn_at(actions: Vec<Action>, valid_identity: bool, halt_at: &str) -> Self {
        Self::spawn_policy(actions, valid_identity, halt_at, false).await
    }
    pub async fn spawn_multi(actions: Vec<Action>) -> Self {
        Self::spawn_policy(actions, true, "", true).await
    }
    pub async fn spawn_multi_at(actions: Vec<Action>, halt_at: &str) -> Self {
        Self::spawn_policy(actions, true, halt_at, true).await
    }
    async fn spawn_policy(
        actions: Vec<Action>,
        valid_identity: bool,
        halt_at: &str,
        multi: bool,
    ) -> Self {
        let halt_at = halt_at.to_owned();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().canonicalize().unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        let path = parent.join("server.json");
        fs::write(&path,json!({"formatVersion":1,"mode":"server","instanceId":"i","serverEpoch":"e","baseUrl":format!("http://{address}"),"protocolVersions":[1],"storageSchemaVersion":2,"ownerToken":TOKEN}).to_string()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let endpoint = Arc::new(read_descriptor(&path, CallerProfile::Owner).unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&requests);
        let effects = Arc::new(Mutex::new(std::collections::HashMap::<String, Value>::new()));
        let recorded = effects.clone();
        let received = Arc::new(tokio::sync::Notify::new());
        let signal = received.clone();
        let task = tokio::spawn(async move {
            let mut steps = actions.into_iter();
            let mut connections = 0;
            let mut upgraded = tokio::task::JoinSet::new();
            loop {
                if steps.len() == 0 && !upgraded.is_empty() {
                    while let Some(result) = upgraded.join_next().await {
                        result.unwrap();
                    }
                    return;
                }
                let (mut socket, _) = listener.accept().await.unwrap();
                connections += 1;
                'connection: loop {
                    let mut head = Vec::new();
                    let mut byte = [0];
                    loop {
                        if socket.read_exact(&mut byte).await.is_err() {
                            break 'connection;
                        }
                        head.push(byte[0]);
                        if head.ends_with(b"\r\n\r\n") {
                            break;
                        }
                        assert!(head.len() < 16384);
                    }
                    let head = String::from_utf8(head).unwrap();
                    let path = head
                        .lines()
                        .next()
                        .unwrap()
                        .split_whitespace()
                        .nth(1)
                        .unwrap()
                        .to_owned();
                    let auth = head.to_lowercase().contains("authorization:");
                    let len = head
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|s| s.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    let mut body = vec![0; len];
                    if socket.read_exact(&mut body).await.is_err() {
                        break 'connection;
                    }
                    let body: Value = if body.is_empty() {
                        Value::Null
                    } else {
                        serde_json::from_slice(&body).unwrap()
                    };
                    seen.lock()
                        .unwrap()
                        .push((path.clone(), auth, body.clone()));
                    signal.notify_one();
                    if path == "/v1/calls" {
                        if let Some(key) = body["idempotencyKey"].as_str() {
                            let mut effects = recorded.lock().unwrap();
                            if let Some(previous) = effects.get(key) {
                                assert_eq!(previous, &body["input"]);
                            } else {
                                effects.insert(key.to_owned(), body["input"].clone());
                            }
                        }
                    }
                    let action = if path == halt_at {
                        Action::Pause
                    } else if path == "/v1/system/identify" {
                        let mut mac =
                            Hmac::<Sha256>::new_from_slice(&Sha256::digest(TOKEN.as_bytes()))
                                .unwrap();
                        mac.update(format!("{}\ni", body["nonce"].as_str().unwrap()).as_bytes());
                        let proof: String = mac
                            .finalize()
                            .into_bytes()
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect();
                        Action::Reply(
                            200,
                            json!({"instanceId":"i","proof":if valid_identity && !(halt_at=="wrong-ws-identity" && connections==2) {proof}else {"0".repeat(64)}}),
                        )
                    } else if path == "/v1/system/handshake" {
                        Action::Reply(
                            200,
                            json!({"instanceId":"i","serverEpoch":if halt_at=="wrong-handshake" {"wrong"} else {"e"},"selectedProtocolVersion":1,"storageSchemaVersion":2,"apiMajor":1,"state":"serving"}),
                        )
                    } else {
                        steps.next().unwrap_or(Action::Close)
                    };
                    let (status, body) = match action {
                        Action::WebSocketRaw(bytes) => {
                            use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
                            assert!(path.starts_with("/v1/events?ticket="));
                            assert!(!auth);
                            let key = head
                                .lines()
                                .find_map(|line| {
                                    line.split_once(':')
                                        .filter(|(name, _)| {
                                            name.eq_ignore_ascii_case("sec-websocket-key")
                                        })
                                        .map(|(_, value)| value.trim())
                                })
                                .unwrap();
                            let accept = derive_accept_key(key.as_bytes());
                            socket.write_all(format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n").as_bytes()).await.unwrap();
                            socket.write_all(&bytes).await.unwrap();
                            let mut discard = [0; 1024];
                            while socket.read(&mut discard).await.unwrap_or(0) != 0 {}
                            if !multi || steps.len() == 0 {
                                return;
                            } else {
                                break 'connection;
                            }
                        }
                        action @ (Action::WebSocket(_)
                        | Action::WebSocketConcurrent(_)
                        | Action::WebSocketGate { .. }
                        | Action::WebSocketGateConcurrent { .. }) => {
                            let concurrent = matches!(
                                &action,
                                Action::WebSocketConcurrent(_)
                                    | Action::WebSocketGateConcurrent { .. }
                            );
                            let (frames, gate, after) = match action {
                                Action::WebSocket(frames) | Action::WebSocketConcurrent(frames) => {
                                    (frames, None, Vec::new())
                                }
                                Action::WebSocketGate {
                                    before,
                                    gate,
                                    after,
                                }
                                | Action::WebSocketGateConcurrent {
                                    before,
                                    gate,
                                    after,
                                } => (before, Some(gate), after),
                                _ => unreachable!(),
                            };
                            use futures_util::{SinkExt, StreamExt};
                            use tokio_tungstenite::{
                                tungstenite::{handshake::derive_accept_key, protocol::Role},
                                WebSocketStream,
                            };
                            assert!(path.starts_with("/v1/events?ticket="));
                            assert!(!auth);
                            let key = head
                                .lines()
                                .find_map(|line| {
                                    line.split_once(':')
                                        .filter(|(name, _)| {
                                            name.eq_ignore_ascii_case("sec-websocket-key")
                                        })
                                        .map(|(_, value)| value.trim())
                                })
                                .unwrap();
                            let accept = derive_accept_key(key.as_bytes());
                            socket.write_all(format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n").as_bytes()).await.unwrap();
                            let mut ws =
                                WebSocketStream::from_raw_socket(socket, Role::Server, None).await;
                            for frame in frames {
                                ws.send(frame).await.unwrap();
                            }
                            if concurrent {
                                upgraded.spawn(async move {
                                    if let Some(gate) = gate {
                                        gate.notified().await;
                                    }
                                    for frame in after {
                                        ws.send(frame).await.unwrap();
                                    }
                                    while let Some(Ok(frame)) = ws.next().await {
                                        if frame.is_close() {
                                            let _ = ws.flush().await;
                                            break;
                                        }
                                    }
                                });
                                break 'connection;
                            }
                            if let Some(gate) = gate {
                                gate.notified().await;
                            }
                            for frame in after {
                                ws.send(frame).await.unwrap();
                            }
                            while let Some(Ok(frame)) = ws.next().await {
                                if frame.is_close() {
                                    let _ = ws.flush().await;
                                    break;
                                }
                            }
                            if !multi || steps.len() == 0 {
                                return;
                            } else {
                                break 'connection;
                            }
                        }
                        Action::ReplyGate { status, body, gate } => {
                            gate.notified().await;
                            (status, body.to_string().into_bytes())
                        }
                        Action::Reply(status, value) => (status, value.to_string().into_bytes()),
                        Action::Raw(status, body) => (status, body),
                        Action::Fault(code, outcome) => {
                            let fault = workbench_protocol::WorkbenchFault::new(
                                code,
                                serde_json::from_value(body["requestId"].clone()).unwrap(),
                                "private-sentinel",
                            )
                            .with_outcome(outcome)
                            .with_details(json!({"token": TOKEN, "input": "private-sentinel"}));
                            {
                                let status = code.http_status();
                                let mut value = serde_json::to_value(&fault).unwrap();
                                value["status"] = json!(status);
                                (status, serde_json::to_vec(&value).unwrap())
                            }
                        }
                        Action::Close => break 'connection,
                        Action::SlowBody => {
                            socket
                                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\n\r\n{")
                                .await
                                .unwrap();
                            let mut buf = [0u8; 1024];
                            while socket.read(&mut buf).await.unwrap_or(0) != 0 {}
                            break 'connection;
                        }
                        Action::Pause => {
                            let mut buf = [0u8; 1024];
                            while socket.read(&mut buf).await.unwrap_or(0) != 0 {}
                            break 'connection;
                        }
                    };
                    let response=format!("HTTP/1.1 {status} response\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",body.len());
                    if socket.write_all(response.as_bytes()).await.is_err() {
                        break 'connection;
                    }
                    if socket.write_all(&body).await.is_err() {
                        break 'connection;
                    }
                    if path == "/v1/system/identify" && halt_at == "close-after-identify" {
                        break 'connection;
                    }
                }
                if !multi {
                    return;
                }
            }
        });
        Self {
            endpoint,
            descriptor: path,
            requests,
            effects,
            received,
            task,
            _dir: dir,
        }
    }
}
