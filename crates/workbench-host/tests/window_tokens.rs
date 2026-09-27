//! 044 T025(Codex 설계 리뷰 C4, research R6): 창 토큰 발급과 폐기는 발급기의 같은 잠금 아래에서 직렬화되고, 폐기한 창
//! 주체는 세대 동안 tombstone으로 남는다. 폐기보다 늦게 도착한 발급 요청은 거절되고(`forbidden`), 같은 label의 새
//! incarnation은 다른 주체라 영향을 받지 않는다. 폐기 뒤에는 그 주체의 유효 토큰이 0이고, 폐기 전에 받은 이벤트 표로도
//! 구독하지 못한다. 모두 실제 HTTP 어댑터와 소유자 자격 증명으로 부른다.

use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::Arc,
    time::Duration,
};

use serde_json::{Value, json};
use workbench_core::application::workbench_runtime::RuntimeAdapters;
use workbench_host::{
    assembly::{HostAssembly, HostOptions, assemble},
    lifecycle::identity::OwnerIdentity,
};

const ORIGIN: &str = "tauri://localhost";

struct Server {
    runtime: tokio::runtime::Runtime,
    host: HostAssembly,
    base_url: String,
    owner_token: String,
    instance_id: String,
    _dir: tempfile::TempDir,
    work_dir: String,
}

impl Server {
    fn start() -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        let work_dir = std::fs::canonicalize(work)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let identity = OwnerIdentity::generate();
        let mut options = HostOptions::new(
            dir.path().join("data"),
            RuntimeAdapters::production(),
            "test",
            runtime.handle().clone(),
        );
        options.owner = Some(identity.clone());
        let host = assemble(options).expect("assembly");
        let base_url = host.http.as_ref().expect("http").base_url().to_owned();
        Self {
            runtime,
            host,
            base_url,
            owner_token: identity.token().to_owned(),
            instance_id: identity.instance_id().to_owned(),
            _dir: dir,
            work_dir,
        }
    }

    fn owner_call(&self, operation: &str, input: Value) -> (u16, Value) {
        call(&self.base_url, &self.owner_token, None, operation, input)
    }

    fn issue(&self, label: &str, incarnation: &str) -> (u16, Value) {
        self.owner_call(
            "desktop.issueWindowToken",
            json!({ "label": label, "incarnation": incarnation, "origin": ORIGIN }),
        )
    }

    fn retire(&self, label: &str, incarnation: &str, close_bench: bool) -> (u16, Value) {
        self.owner_call(
            "desktop.retireWindow",
            json!({ "label": label, "incarnation": incarnation, "closeBench": close_bench }),
        )
    }

    fn window_call(&self, token: &str, operation: &str, input: Value) -> (u16, Value) {
        call(&self.base_url, token, Some(ORIGIN), operation, input)
    }

    fn stop(self) {
        self.runtime.block_on(self.host.shutdown());
    }
}

fn raw(base_url: &str, request: String) -> (u16, Value) {
    let authority = base_url.trim_start_matches("http://");
    let mut stream = TcpStream::connect(authority).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    let text = String::from_utf8_lossy(&response).into_owned();
    let status = text
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_default();
    let body = if text
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        dechunk(&body)
    } else {
        body
    };
    (status, serde_json::from_str(&body).unwrap_or(Value::Null))
}

fn dechunk(body: &str) -> String {
    let mut out = String::new();
    let mut rest = body;
    while let Some((size, tail)) = rest.split_once("\r\n") {
        let size = usize::from_str_radix(size.trim(), 16).unwrap_or(0);
        if size == 0 {
            break;
        }
        out.push_str(&tail[..size]);
        rest = &tail[size + 2..];
    }
    out
}

fn call(
    base_url: &str,
    bearer: &str,
    origin: Option<&str>,
    operation: &str,
    input: Value,
) -> (u16, Value) {
    let body = json!({
        "protocolVersion": 1,
        "operation": operation,
        "requestId": format!("req_{}", uuid_like()),
        "idempotencyKey": format!("key_{}", uuid_like()),
        "input": input,
    });
    let body = if matches!(operation, "bench.list" | "project.list" | "server.status") {
        let mut body = body;
        body.as_object_mut().unwrap().remove("idempotencyKey");
        body
    } else {
        body
    };
    let payload = body.to_string();
    let authority = base_url.trim_start_matches("http://");
    let origin_header = origin
        .map(|origin| format!("origin: {origin}\r\n"))
        .unwrap_or_default();
    raw(
        base_url,
        format!(
            "POST /v1/calls HTTP/1.1\r\nhost: {authority}\r\nconnection: close\r\ncontent-type: application/json\r\nauthorization: Bearer {bearer}\r\n{origin_header}content-length: {}\r\n\r\n{payload}",
            payload.len()
        ),
    )
}

fn uuid_like() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    format!(
        "{:x}{:x}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    )
}

fn token_of(response: &(u16, Value)) -> String {
    assert_eq!(response.0, 200, "{response:?}");
    response.1["output"]["token"]
        .as_str()
        .unwrap_or_else(|| panic!("token in {response:?}"))
        .to_owned()
}

/// 창 토큰으로 조회가 되는가(유효 토큰).
fn valid(server: &Server, token: &str) -> bool {
    server.window_call(token, "project.list", json!({})).0 == 200
}

#[test]
fn a_retired_window_gets_no_new_token_but_a_new_incarnation_does() {
    let server = Server::start();
    let first = token_of(&server.issue("session-a", "inc-1"));
    assert!(valid(&server, &first));

    let retired = server.retire("session-a", "inc-1", false);
    assert_eq!(retired.0, 200, "{retired:?}");
    assert!(retired.1["output"]["revokedTokens"].as_u64().unwrap() >= 1);
    assert!(
        !valid(&server, &first),
        "the retired window's token is revoked"
    );

    // 폐기보다 늦게 도착한 발급(같은 창 주체)은 거절된다.
    let late = server.issue("session-a", "inc-1");
    assert_eq!(late.0, 403, "{late:?}");
    assert_eq!(late.1["code"], "forbidden", "{late:?}");

    // 같은 label을 다시 연 창(새 incarnation)은 다른 주체다.
    let reopened = token_of(&server.issue("session-a", "inc-2"));
    assert!(valid(&server, &reopened));
    server.stop();
}

#[test]
fn issuance_rejects_origins_outside_the_webview_allowlist() {
    let server = Server::start();
    let response = server.owner_call(
        "desktop.issueWindowToken",
        json!({ "label": "session-a", "incarnation": "inc-1", "origin": "https://evil.example" }),
    );
    assert_eq!(response.0, 403, "{response:?}");
    server.stop();
}

#[test]
fn concurrent_issue_and_retire_leave_no_valid_token_after_the_retire() {
    let server = Arc::new(Server::start());
    let mut handles = Vec::new();
    for _ in 0..100 {
        let server = Arc::clone(&server);
        handles.push(std::thread::spawn(move || {
            server.issue("session-race", "inc-1")
        }));
    }
    let retire = {
        let server = Arc::clone(&server);
        std::thread::spawn(move || server.retire("session-race", "inc-1", false))
    };
    let retired = retire.join().unwrap();
    assert_eq!(retired.0, 200, "{retired:?}");
    let mut issued = Vec::new();
    for handle in handles {
        let response = handle.join().unwrap();
        match response.0 {
            200 => issued.push(token_of(&response)),
            403 => {}
            other => panic!("unexpected {other}: {response:?}"),
        }
    }
    // 폐기가 끝난 뒤: 발급된 어떤 토큰도 유효하지 않다(폐기 전 발급은 폐기로 지워지고, 폐기 뒤 발급은 거절됐다).
    let still_valid = issued.iter().filter(|token| valid(&server, token)).count();
    assert_eq!(
        still_valid,
        0,
        "{} issued, {still_valid} still valid",
        issued.len()
    );
    assert_eq!(server.issue("session-race", "inc-1").0, 403);
    Arc::try_unwrap(server).ok().expect("sole owner").stop();
}

fn ticket(server: &Server, token: &str, stream_id: &str, epoch: &str) -> String {
    let payload = json!({
        "cursors": [{ "streamId": stream_id, "epoch": epoch, "afterSequence": 0 }]
    })
    .to_string();
    let authority = server.base_url.trim_start_matches("http://");
    let (status, body) = raw(
        &server.base_url,
        format!(
            "POST /v1/event-tickets HTTP/1.1\r\nhost: {authority}\r\nconnection: close\r\ncontent-type: application/json\r\nauthorization: Bearer {token}\r\norigin: {ORIGIN}\r\ncontent-length: {}\r\n\r\n{payload}",
            payload.len()
        ),
    );
    assert_eq!(status, 200, "{body}");
    body["ticket"].as_str().unwrap().to_owned()
}

/// 표로 WebSocket 업그레이드를 시도하고 상태 코드를 돌려준다(101 = 구독 수락).
fn upgrade_status(server: &Server, ticket: &str) -> u16 {
    let authority = server.base_url.trim_start_matches("http://");
    let mut stream = TcpStream::connect(authority).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    write!(
        stream,
        "GET /v1/events?ticket={ticket} HTTP/1.1\r\nhost: {authority}\r\norigin: {ORIGIN}\r\nconnection: Upgrade\r\nupgrade: websocket\r\nsec-websocket-version: 13\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n"
    )
    .unwrap();
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while stream.read(&mut byte).unwrap_or(0) == 1 {
        head.push(byte[0]);
        if head.ends_with(b"\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&head)
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0)
}

#[test]
fn a_ticket_issued_before_the_retire_cannot_subscribe_after_it() {
    let server = Server::start();
    let token = token_of(&server.issue("session-t", "inc-1"));
    let opened = server.window_call(
        &token,
        "bench.open",
        json!({ "workingDirectory": server.work_dir }),
    );
    assert_eq!(opened.0, 200, "{opened:?}");
    let bench = opened.1["output"]["benchId"].as_str().unwrap().to_owned();
    let epoch = server.host.runtime.epoch().to_owned();
    let stream = format!("bench:{bench}");

    let control = ticket(&server, &token, &stream, &epoch);
    assert_eq!(
        upgrade_status(&server, &control),
        101,
        "a live window subscribes"
    );

    let pending = ticket(&server, &token, &stream, &epoch);
    assert_eq!(server.retire("session-t", "inc-1", false).0, 200);
    assert_eq!(
        upgrade_status(&server, &pending),
        401,
        "a ticket issued before the retire is refused"
    );
    server.stop();
}

#[test]
fn retiring_with_close_bench_closes_the_benches_that_window_opened() {
    let server = Server::start();
    let token = token_of(&server.issue("session-c", "inc-1"));
    let opened = server.window_call(
        &token,
        "bench.open",
        json!({ "workingDirectory": server.work_dir }),
    );
    assert_eq!(opened.0, 200, "{opened:?}");
    let bench = opened.1["output"]["benchId"].as_str().unwrap().to_owned();
    let other = token_of(&server.issue("session-d", "inc-1"));
    let untouched = server.window_call(
        &other,
        "bench.open",
        json!({ "workingDirectory": server.work_dir }),
    );
    let untouched = untouched.1["output"]["benchId"]
        .as_str()
        .unwrap()
        .to_owned();

    let retired = server.retire("session-c", "inc-1", true);
    assert_eq!(retired.0, 200, "{retired:?}");
    assert_eq!(retired.1["output"]["closedBenches"], json!([bench]));
    let listed = server.owner_call("bench.list", json!({}));
    let ids: Vec<&str> = listed.1["output"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["benchId"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec![untouched.as_str()],
        "only the retired window's bench closed"
    );
    server.stop();
}

/// T026: 조립된 host의 `server.status`는 소유자 신원의 인스턴스 식별자를 싣고, 아직 파생하지 않는 수는 `null`로 둔다.
/// 창 주체는 소유자 전용 조회를 부를 수 없다.
#[test]
fn server_status_carries_the_instance_id_and_marks_underived_fields() {
    let server = Server::start();
    let status = server.owner_call("server.status", json!({}));
    assert_eq!(status.0, 200, "{status:?}");
    let output = &status.1["output"];
    assert_eq!(output["instanceId"], server.instance_id.as_str());
    assert_eq!(output["state"], "serving");
    assert!(
        output["activeWork"]["pendingExchanges"].is_null(),
        "{output}"
    );
    assert!(output["unresolvedOperations"].is_null(), "{output}");
    assert!(
        output["notYetDerived"]
            .as_array()
            .unwrap()
            .contains(&json!("activeWork.pendingExchanges")),
        "{output}"
    );

    let window = token_of(&server.issue("session-a", "inc-1"));
    let refused = server.window_call(&window, "server.status", json!({}));
    assert_eq!(refused.0, 403, "{refused:?}");
    server.stop();
}
