//! Codex r10(crates-rest medium): lifecycle 요청의 응답 틀(`content-length`·chunked)은 **신뢰하지 않는 입력**이다 — 신원
//! 확인(`identify`) 전에, 남은 안내 파일의 포트를 차지한 아무 프로세스가 보낼 수 있다. 악성·잘못된 길이는 panic이 아니라
//! 오류로 끝나고, 응답 크기 상한을 넘는 선언은 본문을 기다리지 않고 곧바로 거절된다.
//!
//! 요청은 별도 스레드에서 부르고 시험 상한 안에 끝을 기다린 뒤 그 스레드를 join한다: panic은 "panic 없이 Err" 단정의 실패로
//! 드러나고, 끝나지 않는 읽기는 상한 초과로 실패한다(멈추지 않는다).

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use serde_json::json;
use workbench_host::lifecycle::client::{MAX_RESPONSE_BYTES, REQUEST_TIMEOUT, request};

/// 시험 대기 상한.
const TEST_BOUND: Duration = Duration::from_secs(30);
/// "곧바로" 거절: 요청 deadline(5초)의 절반보다 먼저 끝난다(deadline까지 기다린 시간 초과가 아니다).
const PROMPT: Duration = Duration::from_millis(2500);

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

/// 연결마다 `response`를 한 번 보내고 연결은 연 채 둔다(클라이언트가 끊을 때까지).
fn fake_endpoint(response: &'static [u8]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            thread::spawn(move || {
                read_request(&mut stream);
                if stream.write_all(response).is_err() {
                    return;
                }
                let mut buffer = [0u8; 64];
                let _ = stream.set_read_timeout(Some(TEST_BOUND));
                let _ = stream.read(&mut buffer);
            });
        }
    });
    format!("http://{address}")
}

/// `request`를 별도 스레드에서 부른다. 시험 상한 안에 끝나야 하고, panic하지 않아야 한다(panic이면 그 payload로 실패).
fn request_without_panic(
    response: &'static [u8],
) -> (Result<(u16, serde_json::Value), String>, Duration) {
    let base_url = fake_endpoint(response);
    let (tx, rx) = mpsc::channel();
    let caller = thread::spawn(move || {
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
    match rx.recv_timeout(TEST_BOUND) {
        Ok(outcome) => {
            caller.join().expect("the request thread ends");
            outcome
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            let payload = caller
                .join()
                .expect_err("the request thread ended without a result");
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_default();
            panic!("the request panicked instead of returning an error: {message}");
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("the request did not finish within the test bound")
        }
    }
}

fn assert_prompt_error(response: &'static [u8], expect: &str) {
    let (result, elapsed) = request_without_panic(response);
    let error = result.expect_err("untrusted framing is an error");
    assert!(
        error.contains(expect),
        "expected an error mentioning {expect:?}: {error}"
    );
    assert!(
        elapsed < PROMPT,
        "refused without waiting for the request deadline ({REQUEST_TIMEOUT:?}): took {elapsed:?} ({error})"
    );
}

#[test]
fn a_chunk_size_of_usize_max_is_an_error_not_a_panic() {
    assert_prompt_error(
        b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\nffffffffffffffff\r\n\r\n",
        "too large",
    );
}

#[test]
fn a_chunk_size_of_usize_max_minus_one_is_an_error_not_a_panic() {
    assert_prompt_error(
        b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\nfffffffffffffffe\r\n\r\n",
        "too large",
    );
}

#[test]
fn a_chunk_size_that_does_not_fit_in_usize_is_an_error() {
    assert_prompt_error(
        b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n1ffffffffffffffff\r\n\r\n",
        "malformed chunk",
    );
}

#[test]
fn a_chunk_declared_over_the_response_cap_is_refused_before_its_body() {
    // 0x1000001 = 16MiB + 1: 선언만으로 상한을 넘는다(본문은 몇 바이트뿐, 연결은 열려 있다).
    const { assert!(0x100_0001 > MAX_RESPONSE_BYTES) };
    assert_prompt_error(
        b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n1000001\r\nabc",
        "too large",
    );
}

#[test]
fn a_content_length_over_the_response_cap_is_refused_before_its_body() {
    assert_prompt_error(
        b"HTTP/1.1 200 OK\r\ncontent-length: 18446744073709551615\r\n\r\n{}",
        "too large",
    );
    assert_prompt_error(
        b"HTTP/1.1 200 OK\r\ncontent-length: 16777217\r\n\r\n{}",
        "too large",
    );
}

#[test]
fn a_malformed_content_length_is_an_error() {
    assert_prompt_error(
        b"HTTP/1.1 200 OK\r\ncontent-length: -1\r\n\r\n{}",
        "content-length",
    );
    assert_prompt_error(
        b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\ncontent-length: 3\r\n\r\n{}",
        "content-length",
    );
}

#[test]
fn a_non_hex_chunk_size_is_an_error() {
    assert_prompt_error(
        b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\nzz\r\n\r\n",
        "malformed chunk",
    );
}

#[test]
fn a_chunk_without_its_line_end_is_an_error() {
    // 선언한 3바이트 뒤에 CRLF가 아닌 바이트가 온다.
    assert_prompt_error(
        b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n3\r\nabcXY0\r\n\r\n",
        "malformed chunk",
    );
}

#[test]
fn chunk_extensions_and_a_well_formed_body_still_parse() {
    let (result, elapsed) = request_without_panic(
        b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n4;ext=1\r\n{\"ok\r\n7\r\n\":true}\r\n0\r\n\r\n",
    );
    assert_eq!(result.expect("parsed"), (200, json!({ "ok": true })));
    assert!(elapsed < PROMPT, "ends at the last chunk: {elapsed:?}");
}

#[test]
fn a_non_utf8_head_is_an_error_not_a_panic() {
    let (result, _) = request_without_panic(b"HTTP/1.1 2\xff\xfe OK\r\ncontent-length: 0\r\n\r\n");
    result.expect_err("a malformed status line is an error");
}
