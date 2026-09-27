//! 044 T021(research R5, 설계 리뷰 D1): 클라이언트는 안내 파일의 끝점에 소유자 자격 증명을 보내기 **전에** 인증 없는
//! 신원 증명(`/v1/system/identify`, HMAC)으로 그 끝점이 안내 파일의 서버 인스턴스인지 확인한다. 남은 안내 파일의
//! 포트를 다른 프로세스가 차지했다면 자격 증명이 새어 나가지 않는다.

use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};

use workbench_core::application::workbench_runtime::RuntimeAdapters;
use workbench_host::{
    assembly::{HostOptions, assemble},
    lifecycle::{
        client::{VerifyError, verify},
        descriptor::Descriptor,
        identity::OwnerIdentity,
    },
};

/// 요청 한 개(헤더 + content-length 본문)를 끝까지 읽는다.
fn read_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
    let mut data = Vec::new();
    let mut buffer = [0u8; 4096];
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
    data
}

/// 모든 요청 바이트를 기록하고, identify에는 틀린 증명을, 그 밖에는 200을 돌려주는 가짜 서버.
fn impostor(instance_id: String) -> (String, Arc<Mutex<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorded = seen.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let request = read_request(&mut stream);
            recorded.lock().unwrap().extend_from_slice(&request);
            let body = format!(r#"{{"instanceId":"{instance_id}","proof":"00"}}"#);
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (format!("http://{address}"), seen)
}

#[test]
fn an_impostor_on_the_descriptor_port_never_receives_the_owner_token() {
    let identity = OwnerIdentity::generate();
    let (base_url, seen) = impostor(identity.instance_id().to_owned());
    let descriptor = Descriptor::for_test(&identity, &base_url);

    let error = verify(&descriptor).expect_err("the impostor cannot prove the instance");
    assert!(matches!(error, VerifyError::Identity(_)), "{error:?}");

    let bytes = seen.lock().unwrap().clone();
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        !text.is_empty(),
        "the identify request reached the impostor"
    );
    assert!(
        !text.contains(identity.token()),
        "the owner token must not be sent before the identity proof: {text}"
    );
    assert!(
        !text.to_ascii_lowercase().contains("authorization"),
        "{text}"
    );
}

#[test]
fn the_real_server_proves_its_identity_and_accepts_the_owner_token() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let identity = OwnerIdentity::generate();
    let mut options = HostOptions::new(
        dir.path().to_path_buf(),
        RuntimeAdapters::production(),
        "test",
        runtime.handle().clone(),
    );
    options.owner = Some(identity.clone());
    let host = assemble(options).expect("assembly");
    let http = host.http.clone().expect("http started");
    let descriptor = Descriptor::for_test(&identity, http.base_url());

    let verified = verify(&descriptor).expect("the real server proves its identity");
    assert_eq!(verified.instance_id, identity.instance_id());
    runtime.block_on(host.shutdown());
}
