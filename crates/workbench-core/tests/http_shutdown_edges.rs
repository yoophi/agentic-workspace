//! 042 구현 리뷰(Codex): 종료는 **받아들이지 않은** HTTP 요청(본문·헤더를 보내다 멈춘 연결, 인증 여부 무관)을
//! 기다리지 않고 끝나야 하며, 열린 WebSocket 구독은 `serve`가 반환하기 전에 소켓·EventStream·task까지 정리돼야
//! 한다. 이미 받아들인 호출은 끝까지 drain된다(`http_disconnect_retry.rs`).

use std::{sync::Arc, time::Duration};

use serde_json::json;
use support::{
    http_harness::{Harness, HarnessOptions, TOKEN_DESKTOP},
    scripted_run_engine::RunScript,
    BenchHarness, TestRuntime,
};
use tokio::io::AsyncWriteExt;
use workbench_protocol::{
    events::EventFrame, AuthenticatedPrincipal, OperationId, StreamCursor, Workbench,
};

mod support;

/// 본문 취소는 연결 유예(기본 2초)를 기다리지 않고 끝나야 한다.
const BODY_CANCEL_LIMIT: Duration = Duration::from_secs(1);
/// 헤더를 보내다 멈춘 연결은 유예 뒤 포기한다.
const SHUTDOWN_LIMIT: Duration = Duration::from_secs(3);

/// 헤더(와 본문 일부)를 보내고 멈춘 연결. 종료 뒤에도 소켓을 쥐고 있는다.
async fn stalled(harness: &Harness, head: String, partial_body: &[u8]) -> tokio::net::TcpStream {
    let mut stream = tokio::net::TcpStream::connect(harness.addr).await.unwrap();
    stream.write_all(head.as_bytes()).await.unwrap();
    stream.write_all(partial_body).await.unwrap();
    stream.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    stream
}

fn post_head(harness: &Harness, token: Option<&str>, length: usize) -> String {
    let auth = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    format!(
        "POST /v1/calls HTTP/1.1\r\nHost: {}\r\n{auth}Content-Type: application/json\r\nContent-Length: {length}\r\n\r\n",
        harness.addr
    )
}

async fn shutdown_within_limit(harness: Harness, label: &str, limit: Duration) {
    tokio::time::timeout(limit, harness.shutdown())
        .await
        .unwrap_or_else(|_| panic!("{label}: shutdown waited on an unaccepted request"));
}

/// 종료 뒤 서버가 연결을 실제로 닫았다: 클라이언트 읽기가 EOF 또는 reset(응답 바이트가 와도 끝은 닫힘).
async fn assert_closed_by_server(stream: &mut tokio::net::TcpStream, label: &str) {
    use tokio::io::AsyncReadExt;
    let mut buffer = [0u8; 1024];
    loop {
        match tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buffer)).await {
            Ok(Ok(0)) | Ok(Err(_)) => return,
            Ok(Ok(_)) => continue,
            Err(_) => panic!("{label}: the server left the connection open"),
        }
    }
}

/// 연결 task·router가 해제돼 `Workbench` 참조 수가 기동 전으로 돌아왔다(독립 서버 재조립 때 누수 없음).
fn assert_released(rt: &TestRuntime, before: usize, label: &str) {
    assert_eq!(
        Arc::strong_count(&rt.runtime),
        before,
        "{label}: a connection task or router still holds the runtime"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_does_not_wait_for_an_unauthenticated_stalled_body() {
    let rt = TestRuntime::new();
    let before = Arc::strong_count(&rt.runtime);
    let harness = Harness::spawn(rt.runtime.clone() as Arc<dyn Workbench>).await;
    let head = post_head(&harness, None, 1_000);
    let mut held = stalled(&harness, head, b"{\"protocolVersion\":").await;
    assert_eq!(harness.calls.active(), 0, "nothing was accepted");
    shutdown_within_limit(harness, "unauthenticated body", BODY_CANCEL_LIMIT).await;
    assert_closed_by_server(&mut held, "unauthenticated body").await;
    assert_released(&rt, before, "unauthenticated body");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_does_not_wait_for_an_authenticated_stalled_body() {
    let rt = TestRuntime::new();
    let before = Arc::strong_count(&rt.runtime);
    let harness = Harness::spawn(rt.runtime.clone() as Arc<dyn Workbench>).await;
    let head = post_head(&harness, Some(TOKEN_DESKTOP), 1_000);
    let mut held = stalled(&harness, head, b"{\"protocolVersion\":").await;
    assert_eq!(harness.calls.active(), 0, "nothing was accepted");
    shutdown_within_limit(harness, "authenticated body", BODY_CANCEL_LIMIT).await;
    assert_closed_by_server(&mut held, "authenticated body").await;
    assert_released(&rt, before, "authenticated body");
    assert!(rt.projects().is_empty(), "no effect");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_does_not_wait_for_stalled_headers() {
    let rt = TestRuntime::new();
    let before = Arc::strong_count(&rt.runtime);
    let harness = Harness::spawn(rt.runtime.clone() as Arc<dyn Workbench>).await;
    let head = format!(
        "POST /v1/calls HTTP/1.1\r\nHost: {}\r\nContent-Ty",
        harness.addr
    );
    let mut held = stalled(&harness, head, b"").await;
    shutdown_within_limit(harness, "headers", SHUTDOWN_LIMIT).await;
    assert_closed_by_server(&mut held, "headers").await;
    assert_released(&rt, before, "headers");
}

/// 평상시에도 본문을 끝내 보내지 않는 연결은 읽기 제한 시간 뒤 `deadlineExceeded`(504)로 끝난다(느린 본문으로 연결을 붙잡기 방지).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stalled_bodies_time_out_while_serving() {
    use tokio::io::AsyncReadExt;
    let rt = TestRuntime::new();
    let harness = Harness::spawn_with(
        rt.runtime.clone() as Arc<dyn Workbench>,
        HarnessOptions {
            body_read_timeout: Duration::from_millis(200),
            ..HarnessOptions::default()
        },
    )
    .await;
    let head = post_head(&harness, Some(TOKEN_DESKTOP), 1_000);
    let mut held = stalled(&harness, head, b"{").await;
    let mut response = Vec::new();
    let mut chunk = [0u8; 1024];
    while !String::from_utf8_lossy(&response).contains("deadlineExceeded") {
        let read = tokio::time::timeout(Duration::from_secs(3), held.read(&mut chunk))
            .await
            .expect("server answered the stalled body")
            .unwrap();
        if read == 0 {
            break;
        }
        response.extend_from_slice(&chunk[..read]);
    }
    let text = String::from_utf8_lossy(&response);
    assert!(text.starts_with("HTTP/1.1 504"), "{text}");
    assert!(text.contains("deadlineExceeded"), "{text}");
}

/// 열린 구독은 `serve` 반환 전에 끝난다: 클라이언트는 close를 받고, hub의 구독 수는 원래대로 돌아와 있다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_closes_subscriptions_before_returning() {
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let epoch = h
        .call(
            &AuthenticatedPrincipal::desktop(),
            OperationId::SystemDescribe,
            json!({}),
        )
        .await
        .unwrap()["epoch"]
        .as_str()
        .unwrap()
        .to_owned();
    let hub = Arc::clone(h.rt.runtime.events_hub());
    let before = hub.subscription_count();
    let runtime_refs = Arc::strong_count(&h.rt.runtime);
    let harness = Harness::spawn(h.rt.runtime.clone() as Arc<dyn Workbench>).await;
    let mut subscriptions = Vec::new();
    for _ in 0..3 {
        let ws = harness
            .subscribe(
                TOKEN_DESKTOP,
                vec![StreamCursor {
                    stream_id: "run:r1".into(),
                    epoch: epoch.clone(),
                    after_sequence: 0,
                }],
            )
            .await;
        assert!(matches!(ws.hello, EventFrame::Hello { .. }));
        subscriptions.push(ws);
    }
    for _ in 0..200 {
        if hub.subscription_count() == before + 3 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(hub.subscription_count(), before + 3, "subscribed");

    tokio::time::timeout(SHUTDOWN_LIMIT, harness.shutdown())
        .await
        .expect("shutdown finished");
    assert_eq!(
        hub.subscription_count(),
        before,
        "every EventStream was released before serve returned"
    );
    assert_eq!(
        Arc::strong_count(&h.rt.runtime),
        runtime_refs,
        "subscription tasks, connections and the router released the runtime"
    );
    for mut ws in subscriptions {
        let mut closed = false;
        for _ in 0..20 {
            match ws.next_frame(Duration::from_millis(100)).await {
                None => {
                    closed = true;
                    break;
                }
                Some(EventFrame::Gap { .. } | EventFrame::Event { .. }) => continue,
                Some(other) => panic!("unexpected frame after shutdown: {other:?}"),
            }
        }
        assert!(closed, "the socket was closed by the server");
    }
}

/// 연결 유예(100ms)가 받아들인 호출(효과 뒤 600ms)보다 짧아도, `serve`는 그 호출이 끝나고 멱등 기록이 남은 뒤에만
/// 반환한다 — 연결은 포기해도 받아들인 호출은 추적기로 끝까지 기다린다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn accepted_calls_outlive_the_connection_grace() {
    use std::sync::atomic::Ordering;
    let h = BenchHarness::new(RunScript {
        prompt_settle_ms: 600,
        ..RunScript::default()
    });
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let mut harness = Harness::spawn_with(
        h.rt.runtime.clone() as Arc<dyn Workbench>,
        HarnessOptions {
            connection_grace: Duration::from_millis(100),
            drain_warn_after: Duration::from_millis(50),
            ..HarnessOptions::default()
        },
    )
    .await;
    let request = support::command_request(
        OperationId::RunSendPrompt,
        &support::uuid_key(),
        json!({ "benchId": bench, "runId": "r1", "prompt": "grace" }),
    );
    // 연결을 쥔 채로(끊지 않음) 효과 진입을 확인하고 종료한다 — 유예 뒤 연결은 포기된다.
    let held = {
        let body = serde_json::to_vec(&request).unwrap();
        let mut stream = tokio::net::TcpStream::connect(harness.addr).await.unwrap();
        stream
            .write_all(
                format!(
                    "POST /v1/calls HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {TOKEN_DESKTOP}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                    harness.addr,
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        stream.write_all(&body).await.unwrap();
        h.engine
            .wait_applied(|l| l == "prompt:r1:grace", Duration::from_secs(10))
            .await;
        stream
    };
    let prompts = h.engine.prompts.load(Ordering::SeqCst);
    let served = harness.begin_shutdown();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        !served.is_finished(),
        "serve returned while the accepted call was running"
    );
    served.await.unwrap().unwrap();
    assert_eq!(harness.calls.active(), 0);
    let replay =
        h.rt.runtime
            .call(AuthenticatedPrincipal::desktop(), request)
            .await;
    assert!(replay.is_ok(), "the idempotent record exists: {replay:?}");
    assert_eq!(
        h.engine.prompts.load(Ordering::SeqCst),
        prompts,
        "applied once"
    );
    drop(held);
}
