//! 044 Codex 구현 리뷰(medium): 창 폐기와 **실제 HTTP 경로**의 늦은 요청. `POST /v1/calls`는 헤더로 인증한 뒤 본문을
//! 기다린다(`workbench-server` `routes/calls.rs`: `authenticate` → `read_body`). 그래서 폐기 **전에** 헤더 인증을 통과한 요청이
//! 폐기 **뒤에** 본문을 보내면, 토큰 tombstone만으로는 막히지 않는다.
//!
//! 관찰 가능한 동기화: 요청에 `Expect: 100-continue`를 싣는다. hyper는 handler가 본문을 처음 poll할 때(= 인증을 통과해
//! `read_body`에 들어간 뒤) `100 Continue`를 보낸다. 시험은 그 중간 응답을 받은 뒤에야 폐기를 부른다 — 고정 sleep이 없다.
//! 인증이 실패했다면 `100` 대신 최종 응답(401)이 온다.
//!
//! 흐름: 창 토큰 발급 → 늦은 `bench.open`·`savedPrompt.create` 두 요청의 헤더 전송 → 둘 다 `100 Continue` 수신 → 소유자
//! `desktop.retireWindow{closeBench:true}` 완료 → 본문 전송 → 둘 다 401 → 그 주체의 작업대 0, 저장된 prompt 없음.
//!
//! `savedPrompt.create`(작업대와 무관한 변경)는 런타임 입구의 폐기 확인만 막는다. `bench.open`은 입구 확인과 작업대 등록의
//! 확인이 모두 막는다. 따라서 입구 확인을 지우는 변이는 `savedPrompt.create` 쪽에서 실패하고, 작업대 등록 확인만 지우는 변이는
//! 이 시험으로 잡히지 않는다. 그 순서(입구를 이미 지난 호출)는 core `window_retire.rs`의 등록 경합 시험이 본다.

use std::{
    io::{Read, Write},
    net::TcpStream,
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
            _dir: dir,
            work_dir,
        }
    }

    fn owner_call(&self, operation: &str, input: Value) -> (u16, Value) {
        let (stream, payload) = open_call(
            &self.base_url,
            &self.owner_token,
            None,
            operation,
            input,
            false,
        );
        finish(stream, &payload)
    }

    fn stop(self) {
        self.runtime.block_on(self.host.shutdown());
    }
}

fn envelope(operation: &str, input: Value) -> String {
    let mut body = json!({
        "protocolVersion": 1,
        "operation": operation,
        "requestId": format!("req_{}", unique()),
        "idempotencyKey": format!("key_{}", unique()),
        "input": input,
    });
    if matches!(operation, "bench.list" | "savedPrompt.list") {
        body.as_object_mut().unwrap().remove("idempotencyKey");
    }
    body.to_string()
}

/// 헤더만 보낸 요청. `expect`면 `Expect: 100-continue`를 싣는다.
fn open_call(
    base_url: &str,
    bearer: &str,
    origin: Option<&str>,
    operation: &str,
    input: Value,
    expect: bool,
) -> (TcpStream, String) {
    let payload = envelope(operation, input);
    let authority = base_url.trim_start_matches("http://");
    let mut stream = TcpStream::connect(authority).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let origin_header = origin
        .map(|origin| format!("origin: {origin}\r\n"))
        .unwrap_or_default();
    let expect_header = if expect {
        "expect: 100-continue\r\n"
    } else {
        ""
    };
    stream
        .write_all(
            format!(
                "POST /v1/calls HTTP/1.1\r\nhost: {authority}\r\nconnection: close\r\ncontent-type: application/json\r\nauthorization: Bearer {bearer}\r\n{origin_header}{expect_header}content-length: {}\r\n\r\n",
                payload.len()
            )
            .as_bytes(),
        )
        .unwrap();
    (stream, payload)
}

/// 서버가 헤더 인증을 지나 본문을 읽기 시작했다는 중간 응답(`100 Continue`)을 받는다.
fn await_continue(stream: &mut TcpStream) {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        let read = stream.read(&mut byte).expect("interim response");
        assert_eq!(
            read,
            1,
            "connection closed before the interim response: {:?}",
            String::from_utf8_lossy(&head)
        );
        head.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&head);
    assert!(
        head.starts_with("HTTP/1.1 100"),
        "the headers must pass authentication first (got: {head})"
    );
}

/// 본문을 보내고 최종 응답을 읽는다.
fn finish(mut stream: TcpStream, payload: &str) -> (u16, Value) {
    stream.write_all(payload.as_bytes()).unwrap();
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

fn unique() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

#[test]
fn a_request_authenticated_before_the_retirement_is_refused_when_its_body_arrives_after_it() {
    let server = Server::start();
    let (status, issued) = server.owner_call(
        "desktop.issueWindowToken",
        json!({ "label": "session-late", "incarnation": "i1", "origin": ORIGIN }),
    );
    assert_eq!(status, 200, "{issued}");
    let token = issued["output"]["token"].as_str().unwrap().to_owned();
    let label = format!("late-{}", unique());

    // 두 늦은 요청: 헤더만 보내고, 서버가 인증을 통과해 본문을 기다리는 것을 `100 Continue`로 확인한다.
    let (mut open, open_payload) = open_call(
        &server.base_url,
        &token,
        Some(ORIGIN),
        "bench.open",
        json!({ "workingDirectory": server.work_dir }),
        true,
    );
    let (mut create, create_payload) = open_call(
        &server.base_url,
        &token,
        Some(ORIGIN),
        "savedPrompt.create",
        json!({ "label": label, "prompt": "p" }),
        true,
    );
    await_continue(&mut open);
    await_continue(&mut create);

    // 폐기가 끝난 **뒤에** 본문을 보낸다.
    let (status, retired) = server.owner_call(
        "desktop.retireWindow",
        json!({ "label": "session-late", "incarnation": "i1", "closeBench": true }),
    );
    assert_eq!(status, 200, "{retired}");
    let (open_status, open_body) = finish(open, &open_payload);
    let (create_status, create_body) = finish(create, &create_payload);
    assert_eq!(
        open_status, 401,
        "the late bench.open is refused: {open_body}"
    );
    assert_eq!(
        create_status, 401,
        "the late savedPrompt.create is refused: {create_body}"
    );

    // 그 주체의 작업대가 없고, 저장된 prompt도 없다.
    let (benches_status, benches) = server.owner_call("bench.list", json!({}));
    assert_eq!(benches_status, 200, "bench.list answers: {benches}");
    let owned: Vec<&Value> = benches["output"]
        .as_array()
        .expect("bench.list is an array")
        .iter()
        .filter(|bench| {
            bench["owner"]
                .as_str()
                .is_some_and(|by| by.contains("session-late"))
        })
        .collect();
    assert!(
        owned.is_empty(),
        "no bench of the retired window: {benches}"
    );
    let (prompts_status, prompts) = server.owner_call("savedPrompt.list", json!({}));
    // 호출이 실패해 `output`이 비면 "없음" 단정이 헛되이 통과한다 — 성공 응답과 배열을 먼저 확인한다(OCR 2차).
    assert_eq!(prompts_status, 200, "savedPrompt.list answers: {prompts}");
    assert!(
        prompts["output"].is_array(),
        "savedPrompt.list output is an array: {prompts}"
    );
    assert!(
        !prompts["output"].to_string().contains(&label),
        "the late savedPrompt.create had no effect: {prompts}"
    );
    server.stop();
}
