//! Codex r9(crates-rest medium): 루프백 lifecycle 요청(`client::request*` — `verify`·`require_serving`·`calls::call`이 모두
//! 거친다)은 **요청 전체** 시간 상한과 응답 크기 상한을 가진다. 남은 안내 파일의 포트를 다른 프로세스가 차지하고 끝없이·
//! 조금씩 보내도 요청은 제한 시간 안에 끝나고, `ensure`는 `startup.lock`을 쥔 채 멈추지 않는다(전체 deadline은 최초 검증
//! 전에 시작한다). 응답은 HTTP 메시지 길이(`content-length`·chunked 끝)에서 읽기를 끝낸다.
//!
//! 시험 자체의 대기에는 상한이 있다(`recv_timeout`): 고치기 전 동작(무한 읽기)은 멈춤이 아니라 상한 초과로 실패한다.

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use serde_json::json;
use workbench_host::lifecycle::{
    client::{MAX_RESPONSE_BYTES, REQUEST_TIMEOUT, request},
    descriptor::{Descriptor, write_descriptor},
    ensure::{EnsureError, EnsureOptions, ensure},
    identity::OwnerIdentity,
    lock::{ensure_server_dir, startup_lock, try_owner_lock},
};

/// 시험 대기 상한: 고치기 전 동작이 끝나지 않아도 시험은 이 안에 실패한다.
const TEST_BOUND: Duration = Duration::from_secs(30);

/// 요청 헤더(+content-length 본문)를 읽는다.
fn read_request(stream: &mut TcpStream) {
    let mut data = Vec::new();
    let mut buffer = [0u8; 4096];
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    while let Ok(read) = stream.read(&mut buffer) {
        if read == 0 {
            break;
        }
        data.extend_from_slice(&buffer[..read]);
        if let Some(end) = data.windows(4).position(|window| window == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&data[..end]).to_ascii_lowercase();
            let length = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if data.len() >= end + 4 + length {
                break;
            }
        }
    }
}

/// 연결마다 `respond`를 부르는 가짜 끝점. 쓰기가 실패하면(클라이언트가 끊음) 그 연결을 끝낸다.
fn fake_endpoint(respond: fn(&mut TcpStream)) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            thread::spawn(move || {
                read_request(&mut stream);
                respond(&mut stream);
            });
        }
    });
    format!("http://{address}")
}

/// 상태 줄만 보내고 100ms마다 한 바이트씩 끝없이 보낸다(각 읽기 대기는 5초보다 짧다).
fn drip(stream: &mut TcpStream) {
    if stream.write_all(b"HTTP/1.1 200 OK\r\n").is_err() {
        return;
    }
    loop {
        thread::sleep(Duration::from_millis(100));
        if stream.write_all(b"x").is_err() {
            return;
        }
    }
}

/// 연결을 닫지 않고 기다린다(클라이언트가 끊을 때까지).
fn hold_open(stream: &mut TcpStream) {
    let mut buffer = [0u8; 64];
    let _ = stream.set_read_timeout(Some(TEST_BOUND));
    let _ = stream.read(&mut buffer);
}

/// 헤더는 맞지만 크기 상한보다 1MiB 큰 본문을 보낸 뒤 연결을 연 채 둔다(메모리를 끝없이 키우지 않게 보낼 양은 유한).
fn oversized_body(stream: &mut TcpStream) {
    let total = MAX_RESPONSE_BYTES + (1 << 20);
    let head = format!("HTTP/1.1 200 OK\r\ncontent-length: {total}\r\n\r\n");
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    let chunk = vec![b'x'; 64 * 1024];
    let mut sent = 0;
    while sent < total {
        if stream.write_all(&chunk).is_err() {
            return;
        }
        sent += chunk.len();
    }
    hold_open(stream);
}

/// 헤더 끝(`\r\n\r\n`) 없이 상한보다 1MiB 많은 바이트를 보낸 뒤 연결을 연 채 둔다.
fn no_header_end(stream: &mut TcpStream) {
    let total = MAX_RESPONSE_BYTES + (1 << 20);
    let chunk = vec![b'x'; 64 * 1024];
    let mut sent = 0;
    while sent < total {
        if stream.write_all(&chunk).is_err() {
            return;
        }
        sent += chunk.len();
    }
    hold_open(stream);
}

/// 완결된 응답(`content-length`)을 보내고 연결은 닫지 않는다.
fn complete_but_kept_open(stream: &mut TcpStream) {
    let body = br#"{"ok":true}"#;
    let head = format!("HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n", body.len());
    if stream.write_all(head.as_bytes()).is_err() || stream.write_all(body).is_err() {
        return;
    }
    hold_open(stream);
}

/// 완결된 chunked 응답을 보내고 연결은 닫지 않는다.
fn chunked_but_kept_open(stream: &mut TcpStream) {
    let response =
        b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\nb\r\n{\"ok\":true}\r\n0\r\n\r\n";
    if stream.write_all(response).is_err() {
        return;
    }
    hold_open(stream);
}

/// `request`를 별도 스레드에서 부르고 시험 상한 안에 결과를 받는다(받지 못하면 그 자체가 실패).
fn bounded_request(base_url: String) -> (Result<(u16, serde_json::Value), String>, Duration) {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let started = Instant::now();
        let result = request(
            &base_url,
            "POST",
            "/v1/system/identify",
            Some(&json!({ "nonce": "n" })),
            None,
        );
        let _ = tx.send((result, started.elapsed()));
    });
    rx.recv_timeout(TEST_BOUND)
        .expect("the request did not finish within the test bound (unbounded read)")
}

#[test]
fn a_slowly_dripping_endpoint_fails_the_request_within_the_request_deadline() {
    let (result, elapsed) = bounded_request(fake_endpoint(drip));
    let error = result.expect_err("a response that never completes is an error");
    assert!(
        elapsed <= REQUEST_TIMEOUT + Duration::from_secs(2),
        "the whole request is bounded by its deadline, not per read: took {elapsed:?} ({error})"
    );
}

#[test]
fn an_oversized_body_is_cut_at_the_response_size_cap() {
    let (result, _) = bounded_request(fake_endpoint(oversized_body));
    let error = result.expect_err("a body over the cap is refused");
    assert!(
        error.contains("too large"),
        "the size cap refuses it: {error}"
    );
}

#[test]
fn a_response_without_a_header_end_is_cut_at_the_response_size_cap() {
    let (result, _) = bounded_request(fake_endpoint(no_header_end));
    let error = result.expect_err("bytes without a header end are refused");
    assert!(
        error.contains("too large"),
        "the size cap refuses it: {error}"
    );
}

#[test]
fn a_complete_response_on_a_kept_open_connection_returns_at_its_message_length() {
    let (result, elapsed) = bounded_request(fake_endpoint(complete_but_kept_open));
    assert_eq!(
        result.expect("the complete message is read"),
        (200, json!({ "ok": true }))
    );
    assert!(
        elapsed < REQUEST_TIMEOUT,
        "reading ends at content-length, not at EOF or the deadline: {elapsed:?}"
    );
    let (result, elapsed) = bounded_request(fake_endpoint(chunked_but_kept_open));
    assert_eq!(
        result.expect("the complete chunked message is read"),
        (200, json!({ "ok": true }))
    );
    assert!(
        elapsed < REQUEST_TIMEOUT,
        "reading ends at the last chunk: {elapsed:?}"
    );
}

/// `ensure`: 안내 파일의 포트를 끝없이 조금씩 보내는 프로세스가 차지했고 소유 잠금은 살아 있는 서버가 쥔 것처럼 잡혀
/// 있다(띄우지 않고 기다리는 경로). 전체 deadline(`ready_timeout`)은 최초 검증 전에 시작하고 각 요청도 그 안에서 끝나므로
/// `ensure`는 `ready_timeout` 안팎에서 끝나고 `startup.lock`을 놓는다.
#[test]
fn ensure_against_a_dripping_endpoint_times_out_within_its_deadline_and_releases_the_startup_lock()
{
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let server_dir = ensure_server_dir(&data).unwrap();
    let identity = OwnerIdentity::generate();
    write_descriptor(
        &server_dir,
        &Descriptor::for_test(&identity, &fake_endpoint(drip)),
    )
    .unwrap();
    let _owner = try_owner_lock(&data).unwrap().expect("owner lock");
    let ready_timeout = Duration::from_secs(2);
    let (tx, rx) = mpsc::channel();
    let ensure_data = data.clone();
    thread::spawn(move || {
        let started = Instant::now();
        let result = ensure(
            &ensure_data,
            std::path::Path::new("/nonexistent/agentic-workbench-server"),
            &EnsureOptions {
                startup_lock_timeout: Duration::from_secs(5),
                ready_timeout,
            },
        );
        let _ = tx.send((result.map(|_| ()), started.elapsed()));
    });
    let (result, elapsed) = rx
        .recv_timeout(TEST_BOUND)
        .expect("ensure did not finish within the test bound (unbounded verification)");
    assert!(matches!(result, Err(EnsureError::Timeout(_))), "{result:?}");
    assert!(
        elapsed <= ready_timeout + Duration::from_secs(2),
        "the deadline covers the first verification and every request: took {elapsed:?}"
    );
    let released = startup_lock(&data, Duration::from_secs(1));
    assert!(
        released.is_ok(),
        "ensure released the startup lock: {:?}",
        released.err()
    );
}
