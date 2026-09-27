//! 042 US2: 1회용 구독 표의 규칙(contracts §3·§4) — 재사용·만료 `401`, Origin 불일치 `403`(upgrade 전), 발급 형식
//! 상한 `400`, 판정(cursor 0개·권한)은 연결 뒤 `fault` 프레임, 표는 발급 주체의 권한만, 재연결은 새 표 + 마지막 cursor.

use std::{sync::Arc, time::Duration};

use serde_json::json;
use support::{
    http_harness::{Harness, HarnessOptions, TOKEN_DESKTOP, TOKEN_NOSCOPE},
    scripted_run_engine::RunScript,
    BenchHarness,
};
use workbench_protocol::{
    events::{EventFrame, StreamKind, RUN_EVENT_V1},
    AuthenticatedPrincipal, FaultCode, OperationId, StreamCursor, Workbench,
};
use workbench_server::tickets::{MESSAGE_TOO_MANY_CURSORS, TICKET_MAX_CURSORS};

mod support;

const APP: &str = "http://localhost:1420";
const RELEASE: &str = "tauri://localhost";

async fn prepared() -> (BenchHarness, String) {
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
    (h, epoch)
}

fn cursor(epoch: &str, after: u64) -> Vec<StreamCursor> {
    vec![StreamCursor {
        stream_id: "run:r1".into(),
        epoch: epoch.to_owned(),
        after_sequence: after,
    }]
}

fn publish(h: &BenchHarness, count: usize) {
    for _ in 0..count {
        h.rt.runtime.events_hub().publish_state(
            StreamKind::Run,
            "r1",
            RUN_EVENT_V1,
            json!({}),
            false,
            &mut |_| {},
        );
    }
}

async fn spawn(h: &BenchHarness, options: HarnessOptions) -> Harness {
    Harness::spawn_with(h.rt.runtime.clone() as Arc<dyn Workbench>, options).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tickets_are_single_use_and_expire() {
    let (h, epoch) = prepared().await;
    let harness = spawn(&h, HarnessOptions::default()).await;
    let ticket = harness
        .issue_ticket(TOKEN_DESKTOP, None, &cursor(&epoch, 0))
        .await
        .unwrap();
    assert!(harness.connect_ticket(&ticket, None).await.is_ok());
    assert_eq!(
        harness.connect_ticket(&ticket, None).await.err(),
        Some(401),
        "reuse"
    );
    assert_eq!(
        harness.connect_ticket("not-a-ticket", None).await.err(),
        Some(401)
    );

    let short = spawn(
        &h,
        HarnessOptions {
            ticket_ttl: Duration::from_millis(20),
            ..HarnessOptions::default()
        },
    )
    .await;
    let ticket = short
        .issue_ticket(TOKEN_DESKTOP, None, &cursor(&epoch, 0))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(60)).await;
    assert_eq!(
        short.connect_ticket(&ticket, None).await.err(),
        Some(401),
        "expired"
    );
    // 표는 기록되지 않는다(URI query 비기록).
    assert!(short
        .access_log
        .lines()
        .iter()
        .all(|line| !line.contains(&ticket)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tickets_are_bound_to_the_issuing_origin() {
    let (h, epoch) = prepared().await;
    let harness = spawn(
        &h,
        HarnessOptions {
            origins: vec![APP.into(), RELEASE.into()],
            ..HarnessOptions::default()
        },
    )
    .await;
    let ticket = harness
        .issue_ticket(TOKEN_DESKTOP, Some(APP), &cursor(&epoch, 0))
        .await
        .unwrap();
    assert_eq!(
        harness.connect_ticket(&ticket, Some(RELEASE)).await.err(),
        Some(403)
    );
    assert_eq!(
        harness.connect_ticket(&ticket, Some(APP)).await.err(),
        Some(401),
        "a mismatched attempt consumed the ticket"
    );
    let ticket = harness
        .issue_ticket(TOKEN_DESKTOP, Some(APP), &cursor(&epoch, 0))
        .await
        .unwrap();
    assert_eq!(
        harness.connect_ticket(&ticket, None).await.err(),
        Some(403),
        "origin dropped"
    );
    let ticket = harness
        .issue_ticket(TOKEN_DESKTOP, Some(APP), &cursor(&epoch, 0))
        .await
        .unwrap();
    assert!(harness.connect_ticket(&ticket, Some(APP)).await.is_ok());
    // 허용되지 않은 Origin은 발급부터 거절.
    let rejected = harness
        .issue_ticket(
            TOKEN_DESKTOP,
            Some("https://evil.example"),
            &cursor(&epoch, 0),
        )
        .await
        .unwrap_err();
    assert_eq!(rejected.code, FaultCode::Forbidden);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn issuance_checks_shape_and_connect_judges_the_subscription() {
    let (h, epoch) = prepared().await;
    let harness = spawn(&h, HarnessOptions::default()).await;
    let too_many = harness
        .issue_ticket(
            TOKEN_DESKTOP,
            None,
            &vec![cursor(&epoch, 0)[0].clone(); TICKET_MAX_CURSORS + 1],
        )
        .await
        .unwrap_err();
    assert_eq!(too_many.code, FaultCode::InvalidArgument);
    assert_eq!(too_many.message, MESSAGE_TOO_MANY_CURSORS);

    // cursor 0개는 발급되고, 판정은 연결 뒤 fault 프레임(오늘 `events` 판정과 같은 문구).
    let mut ws = harness.subscribe(TOKEN_DESKTOP, Vec::new()).await;
    assert!(matches!(ws.hello, EventFrame::Hello { .. }));
    match ws.next_frame(Duration::from_secs(5)).await {
        Some(EventFrame::Fault { fault }) => assert_eq!(fault.code, FaultCode::InvalidArgument),
        other => panic!("expected fault, got {other:?}"),
    }

    // 다른 주체의 표는 그 주체의 권한만 — scope 없는 주체는 run 스트림을 볼 수 없다.
    let mut ws = harness.subscribe(TOKEN_NOSCOPE, cursor(&epoch, 0)).await;
    match ws.next_frame(Duration::from_secs(5)).await {
        Some(EventFrame::Fault { fault }) => assert_eq!(fault.code, FaultCode::Forbidden),
        other => panic!("expected fault, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconnecting_with_a_new_ticket_resumes_after_the_last_cursor() {
    let (h, epoch) = prepared().await;
    let harness = spawn(&h, HarnessOptions::default()).await;
    let start =
        h.rt.runtime
            .events_hub()
            .replay_run("r1", u64::MAX)
            .last_sequence;
    let mut ws = harness
        .subscribe(TOKEN_DESKTOP, cursor(&epoch, start))
        .await;
    publish(&h, 5);
    let mut last = start;
    for _ in 0..5 {
        match ws.next_frame(Duration::from_secs(5)).await {
            Some(EventFrame::Event { event }) => {
                assert_eq!(event.sequence, last + 1);
                last = event.sequence;
            }
            other => panic!("expected event, got {other:?}"),
        }
    }
    drop(ws);
    publish(&h, 5);
    let mut ws = harness.subscribe(TOKEN_DESKTOP, cursor(&epoch, last)).await;
    for _ in 0..5 {
        match ws.next_frame(Duration::from_secs(5)).await {
            Some(EventFrame::Event { event }) => {
                assert_eq!(
                    event.sequence,
                    last + 1,
                    "no gap or duplicate across reconnect"
                );
                last = event.sequence;
            }
            other => panic!("expected event, got {other:?}"),
        }
    }
}

/// hello는 구독 준비 완료 신호다: hello를 받은 직후의 변경은 재생 없는 알림 스트림(worktree)에서도 전달된다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hello_means_the_subscription_is_ready() {
    let (h, epoch) = prepared().await;
    let harness = spawn(&h, HarnessOptions::default()).await;
    let dir = tempfile::tempdir().unwrap();
    let path = std::fs::canonicalize(dir.path()).unwrap();
    let mut ws = harness
        .subscribe(
            TOKEN_DESKTOP,
            vec![StreamCursor {
                stream_id: format!("worktree:{}", path.display()),
                epoch,
                after_sequence: 0,
            }],
        )
        .await;
    assert!(matches!(ws.hello, EventFrame::Hello { .. }));
    std::fs::write(path.join("after-hello.txt"), "x").unwrap();
    match ws.next_frame(Duration::from_secs(5)).await {
        Some(EventFrame::Event { event }) => {
            assert_eq!(event.schema, "worktree.changed.v1");
        }
        other => panic!("the change right after hello was lost: {other:?}"),
    }
}
