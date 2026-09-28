//! 044 Codex 구현 리뷰(medium): 창 폐기(`desktop.retireWindow`)와 그 창 주체의 늦은 호출.
//!
//! 한계(증거 범위): 이 파일의 (a)–(d)는 런타임·서비스를 **순차로** 부른다. "폐기 뒤 그 주체로 부른 `Workbench::call`"은 전송
//! 계층을 거치지 않으므로 실제 HTTP 경로(헤더 인증 → 본문 대기 → 폐기 → 본문 도착)의 증거가 아니다. 그 경로는 host 시험
//! `crates/workbench-host/tests/window_retire_http.rs`(실제 HTTP, `Expect: 100-continue`로 인증 통과를 관찰)가 본다.
//! 입구를 이미 지난 호출과 폐기의 실제 동시 실행은 (e)가 등록 직전 문(`open_probe`)으로 붙잡아 본다.

#![allow(clippy::result_large_err)]

mod support;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use serde_json::json;
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_core::ports::server_host::{ServerHost, WindowToken, WindowTokenError};
use workbench_protocol::{
    AuthenticatedPrincipal, FaultCode, OperationId, PrincipalSubject, Workbench,
};

/// 토큰 저장이 없는 시험용 host(폐기 수만 센다). 폐기의 작업대 쪽 효과만 본다.
struct CountingHost;

static ISSUED_TOKENS: AtomicUsize = AtomicUsize::new(0);

impl ServerHost for CountingHost {
    fn instance_id(&self) -> Option<String> {
        None
    }
    fn issue_window_token(
        &self,
        _principal: AuthenticatedPrincipal,
        _origin: &str,
    ) -> Result<WindowToken, WindowTokenError> {
        let n = ISSUED_TOKENS.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(WindowToken {
            token: format!("token-{n}"),
            expires_at: "2099-01-01T00:00:00Z".into(),
        })
    }
    fn retire_window(&self, _subject: &PrincipalSubject) -> u64 {
        0
    }
    fn accepted_calls(&self) -> u64 {
        0
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_lost_window_token_response_replays_without_issuing_another_bearer() {
    use support::command_request;

    let h = harness();
    let before = ISSUED_TOKENS.load(Ordering::SeqCst);
    let call = || {
        h.rt.runtime.call(
            AuthenticatedPrincipal::owner(),
            command_request(
                OperationId::DesktopIssueWindowToken,
                "same-window-token-command",
                json!({ "label": "retry", "incarnation": "i1", "origin": "tauri://localhost" }),
            ),
        )
    };
    let first = call().await.unwrap().output().cloned().unwrap();
    let replay = call().await.unwrap().output().cloned().unwrap();
    assert_eq!(replay, first);
    assert_eq!(ISSUED_TOKENS.load(Ordering::SeqCst), before + 1);
}

fn harness() -> BenchHarness {
    let h = BenchHarness::new(RunScript::default());
    assert!(h.rt.runtime.attach_server_host(Arc::new(CountingHost)));
    h
}

async fn retire(h: &BenchHarness, label: &str, incarnation: &str, close_bench: bool) {
    h.call(
        &AuthenticatedPrincipal::owner(),
        OperationId::DesktopRetireWindow,
        json!({ "label": label, "incarnation": incarnation, "closeBench": close_bench }),
    )
    .await
    .expect("desktop.retireWindow");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_call_from_a_retired_window_is_refused_and_opens_no_bench() {
    let h = harness();
    let window = AuthenticatedPrincipal::desktop_window("session-a", "i1");
    // (b) 폐기 전에 연 작업대는 closeBench:true 폐기가 닫는다.
    let before = h
        .call(
            &window,
            OperationId::BenchOpen,
            json!({ "workingDirectory": h.dir }),
        )
        .await
        .expect("bench.open before the retirement");
    retire(&h, "session-a", "i1", true).await;
    let open = h.rt.runtime.benches().registry.open_benches();
    assert!(
        !open
            .iter()
            .any(|(id, _)| Some(id.as_str()) == before["benchId"].as_str()),
        "the bench opened before the retirement is closed: {open:?}"
    );
    // (a) 폐기 전에 인증된 늦은 요청: 폐기 뒤 그 주체로 도착한 bench.open은 거절되고 작업대가 생기지 않는다.
    let late = h
        .call(
            &window,
            OperationId::BenchOpen,
            json!({ "workingDirectory": h.dir }),
        )
        .await;
    assert_eq!(
        late.expect_err("a retired window cannot open a bench").code,
        FaultCode::Unauthenticated
    );
    assert!(
        h.rt.runtime.benches().registry.open_benches().is_empty(),
        "no bench was created for the retired window"
    );
    // 작업대와 무관한 호출도 거절된다(폐기된 토큰과 같은 뜻).
    let other = h.call(&window, OperationId::ProjectList, json!({})).await;
    assert_eq!(other.expect_err("refused").code, FaultCode::Unauthenticated);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retiring_without_closing_the_bench_still_refuses_later_calls() {
    let h = harness();
    let window = AuthenticatedPrincipal::desktop_window("session-q", "i1");
    // 앱 종료 경로(closeBench:false): 작업대는 남고(run 지속), 그 창 주체의 늦은 호출은 거절된다.
    let opened = h
        .call(
            &window,
            OperationId::BenchOpen,
            json!({ "workingDirectory": h.dir }),
        )
        .await
        .expect("bench.open");
    retire(&h, "session-q", "i1", false).await;
    let bench = opened["benchId"].as_str().unwrap();
    assert!(
        h.rt.runtime
            .benches()
            .registry
            .open_benches()
            .iter()
            .any(|(id, _)| id == bench),
        "closeBench:false keeps the bench"
    );
    let late = h
        .call(
            &window,
            OperationId::BenchOpen,
            json!({ "workingDirectory": h.dir }),
        )
        .await;
    assert_eq!(late.expect_err("refused").code, FaultCode::Unauthenticated);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn another_incarnation_of_the_same_label_is_unaffected() {
    let h = harness();
    retire(&h, "settings", "old", true).await;
    // (c) 같은 label의 새 incarnation은 다른 주체다.
    let fresh = AuthenticatedPrincipal::desktop_window("settings", "new");
    h.call(
        &fresh,
        OperationId::BenchOpen,
        json!({ "workingDirectory": h.dir }),
    )
    .await
    .expect("a new incarnation opens a bench");
    h.call(&fresh, OperationId::ProjectList, json!({}))
        .await
        .expect("a new incarnation calls normally");
}

/// (d) 입구 판정을 폐기 **전에** 지난 호출: 그 뒤 폐기 표시가 서면 작업대 등록(검사·삽입, 표시와 같은 잠금)이 거절한다.
/// 입구 판정만 있으면 이 순서의 호출이 폐기된 주체의 작업대를 만든다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bench_open_past_the_entry_before_the_retirement_is_refused_at_registration() {
    let h = harness();
    let window = AuthenticatedPrincipal::desktop_window("session-d", "i1");
    retire(&h, "session-d", "i1", true).await;
    // 입구를 이미 지난 호출이 handler에서 부르는 것과 같은 서비스 호출.
    let late =
        h.rt.runtime
            .benches()
            .open(&workbench_protocol::RequestId::random(), &window, &h.dir);
    assert_eq!(
        late.expect_err("registration refuses a retired subject")
            .code,
        FaultCode::Unauthenticated
    );
    assert!(h.rt.runtime.benches().registry.open_benches().is_empty());
}

/// (e) 등록 경합: 입구를 지난 `bench.open`을 등록 **직전**에 붙잡은 채 폐기가 끝나고(표시 + 연 작업대 닫기), 그 뒤 풀면
/// 등록이 거절된다. 폐기된 주체의 작업대는 남지 않는다. 등록의 폐기 확인을 지우면 이 순서에서 작업대가 남는다(닫기는 이미
/// 끝났다).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_bench_open_held_before_registration_while_the_window_is_retired_leaves_no_bench() {
    let h = harness();
    let window = AuthenticatedPrincipal::desktop_window("session-e", "i1");
    let (reached_tx, reached_rx) = std::sync::mpsc::channel::<()>();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let release_rx = std::sync::Mutex::new(release_rx);
    *h.rt.runtime.benches().open_probe.lock().unwrap() = Some(Arc::new(move || {
        let _ = reached_tx.send(());
        let _ = release_rx
            .lock()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(10));
    }));
    let runtime = Arc::clone(&h.rt.runtime);
    let open = tokio::spawn({
        let window = window.clone();
        let dir = h.dir.clone();
        async move {
            runtime
                .call(
                    window,
                    support::command_request(
                        OperationId::BenchOpen,
                        "open-e",
                        json!({ "workingDirectory": dir }),
                    ),
                )
                .await
        }
    });
    // 입구를 지나 등록 직전에 섰다.
    tokio::task::spawn_blocking(move || {
        reached_rx.recv_timeout(std::time::Duration::from_secs(10))
    })
    .await
    .unwrap()
    .expect("the bench.open reached the registration point");
    retire(&h, "session-e", "i1", true).await;
    release_tx.send(()).unwrap();
    let result = open.await.unwrap();
    assert_eq!(
        result
            .expect_err("registration refuses the retired subject")
            .code,
        FaultCode::Unauthenticated
    );
    let left: Vec<_> =
        h.rt.runtime
            .benches()
            .registry
            .open_benches()
            .into_iter()
            .filter(|(_, owner)| owner == &window.subject)
            .collect();
    assert!(
        left.is_empty(),
        "no bench of the retired window remains: {left:?}"
    );
    *h.rt.runtime.benches().open_probe.lock().unwrap() = None;
}
