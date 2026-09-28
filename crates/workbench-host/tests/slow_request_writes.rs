//! Codex r11(crates-rest medium): lifecycle 요청의 **쓰기** 단계도 요청 전체 deadline 안에서 끝난다. 소켓 버퍼보다 큰 요청을
//! 끝점이 아주 천천히 읽으면(backpressure) 부분 쓰기가 끝없이 이어진다 — 쓰기마다 남은 시간으로 대기 상한을 다시 잡지
//! 않으면 호출자 deadline과 `REQUEST_TIMEOUT`을 넘어서도 호출 스레드가 묶인다. 공개 호출 경로(`calls::call_by`)로 큰
//! operation 입력을 보내고, 짧은 호출자 deadline 안에 오류로 끝나는지 본다.
//!
//! 시험 자체의 대기에는 상한이 있다(`recv_timeout`): 고치기 전 동작(끝없는 부분 쓰기)은 멈춤이 아니라 상한 초과로 실패한다.

use std::{
    io::Read,
    net::TcpListener,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use serde_json::json;
use workbench_host::lifecycle::calls::{CallError, call_by};

/// 시험 대기 상한: 고치기 전 동작이 끝나지 않아도 시험은 이 안에 실패한다.
const TEST_BOUND: Duration = Duration::from_secs(30);
/// 호출자 deadline(요청 자체 상한 5초보다 짧다).
const CALLER_DEADLINE: Duration = Duration::from_secs(1);
/// deadline 뒤 허용 여유(스레드 깨움·연결 정리).
const SLACK: Duration = Duration::from_millis(1500);
/// 소켓 송·수신 버퍼(루프백 기본 수백 KiB)를 훨씬 넘는 요청 본문.
const LARGE_INPUT_BYTES: usize = 8 * 1024 * 1024;

/// 연결을 받고 요청을 천천히 읽는다(10ms마다 최대 4KiB, 약 400KiB/s): 쓰기마다 조금씩은 진행되므로(부분 쓰기) 쓰기 하나의
/// 대기 상한에는 걸리지 않지만, 8MiB 본문 전체는 수십 초가 걸린다. 응답은 보내지 않는다. 클라이언트가 끊으면(읽기 0·오류) 그
/// 연결을 끝낸다.
fn slow_reader() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            thread::spawn(move || {
                let mut buffer = [0u8; 4096];
                let started = Instant::now();
                while started.elapsed() < TEST_BOUND {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => return,
                        Ok(_) => thread::sleep(Duration::from_millis(10)),
                    }
                }
            });
        }
    });
    format!("http://{address}")
}

#[test]
fn a_large_request_to_a_slowly_reading_endpoint_fails_within_the_caller_deadline() {
    let base_url = slow_reader();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let started = Instant::now();
        let result = call_by(
            &base_url,
            "owner-token",
            None,
            "server.status",
            json!({ "padding": "x".repeat(LARGE_INPUT_BYTES) }),
            false,
            Some(started + CALLER_DEADLINE),
        );
        let _ = sender.send((result, started.elapsed()));
    });
    let (result, elapsed) = receiver
        .recv_timeout(TEST_BOUND)
        .expect("the request never returned: the write phase is not bounded by the deadline");
    match result {
        Err(CallError::Transport(message)) => {
            assert!(
                message.contains("timed out"),
                "a write-phase timeout, not another error: {message}"
            )
        }
        other => panic!("expected a transport timeout, got {other:?}"),
    }
    assert!(
        elapsed <= CALLER_DEADLINE + SLACK,
        "the write phase must end at the caller deadline: took {elapsed:?}"
    );
}
