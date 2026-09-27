//! 044 T028: 데스크톱·소유자 클라이언트가 쓰는 호출 도우미(`lifecycle::calls`). 실제 HTTP 어댑터에 `/v1/calls`
//! 봉투로 보내고, 성공은 출력으로, 문제 응답은 fault(코드·메시지)로, 닿지 못함은 전송 오류로 돌려준다. 창 토큰은
//! WebView 출처를 함께 실어야 받아들여진다.

use serde_json::json;
use workbench_core::application::workbench_runtime::RuntimeAdapters;
use workbench_host::{
    assembly::{HostOptions, assemble},
    lifecycle::{
        calls::{CallError, call},
        identity::OwnerIdentity,
    },
};

const ORIGIN: &str = "tauri://localhost";

#[test]
fn owner_and_window_calls_round_trip_through_the_http_adapter() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let identity = OwnerIdentity::generate();
    let mut options = HostOptions::new(
        dir.path().join("data"),
        RuntimeAdapters::production(),
        "test",
        runtime.handle().clone(),
    );
    options.owner = Some(identity.clone());
    let host = assemble(options).expect("assembly");
    let base = host.http.as_ref().unwrap().base_url().to_owned();

    let lease = call(
        &base,
        identity.token(),
        None,
        "lease.acquire",
        json!({ "clientKind": "desktop", "clientId": "app-1" }),
        true,
    )
    .expect("owner lease");
    assert!(lease["leaseId"].as_str().is_some_and(|id| !id.is_empty()), "{lease}");

    let issued = call(
        &base,
        identity.token(),
        None,
        "desktop.issueWindowToken",
        json!({ "label": "session-1", "incarnation": "i1", "origin": ORIGIN }),
        true,
    )
    .expect("window token");
    let token = issued["token"].as_str().unwrap().to_owned();
    let bench = call(
        &base,
        &token,
        Some(ORIGIN),
        "bench.open",
        json!({ "workingDirectory": work.to_string_lossy() }),
        true,
    )
    .expect("window bench.open");
    assert!(bench["benchId"].as_str().is_some(), "{bench}");

    // 창 토큰은 출처 없이 받아들여지지 않는다(인증 실패는 fault로 온다).
    match call(&base, &token, None, "bench.list", json!({}), false) {
        Err(CallError::Fault { status, .. }) => assert_eq!(status, 401),
        other => panic!("a window token without its origin must fail: {other:?}"),
    }
    // 소유자 전용 op를 창 토큰으로 부르면 forbidden fault.
    match call(
        &base,
        &token,
        Some(ORIGIN),
        "lease.acquire",
        json!({ "clientKind": "desktop", "clientId": "x" }),
        true,
    ) {
        Err(CallError::Fault { code, .. }) => assert_eq!(code, "forbidden"),
        other => panic!("owner-only op must be forbidden for a window: {other:?}"),
    }

    runtime.block_on(host.shutdown());
    // 닫힌 끝점: 전송 오류.
    match call(&base, identity.token(), None, "bench.list", json!({}), false) {
        Err(CallError::Transport(_)) => {}
        other => panic!("a closed endpoint is a transport error: {other:?}"),
    }
}
