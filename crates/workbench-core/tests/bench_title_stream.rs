//! 040 US3: agent의 제목 요청은 그 run을 소유한 작업대의 알림 스트림(`bench:<id>`)에만 발행되고, 데스크톱
//! 전달도 그 작업대로 한 번만 간다(다른 작업대에는 0). 구독자가 없어도 전달은 일어난다.

mod support;

use std::time::Duration;

use futures_util::StreamExt;
use serde_json::json;
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_core::ports::desktop_bridge::DesktopDelivery;
use workbench_protocol::{
    events::BENCH_TITLE_REQUESTED_V1, AuthenticatedPrincipal, EventItem, OperationId,
    StreamCursor, Subscription, Workbench,
};

fn titles(h: &BenchHarness, bench: &str) -> Vec<String> {
    h.desktop
        .deliveries_for(bench)
        .into_iter()
        .filter_map(|delivery| match delivery {
            DesktopDelivery::TitleRequested { title, .. } => Some(title),
            _ => None,
        })
        .collect()
}

fn subscribe(h: &BenchHarness, bench: &str) -> workbench_protocol::EventStream {
    h.rt.runtime
        .events(
            AuthenticatedPrincipal::desktop(),
            Subscription {
                cursors: vec![StreamCursor {
                    stream_id: format!("bench:{bench}"),
                    epoch: h.rt.runtime.epoch().into(),
                    after_sequence: 0,
                }],
            },
        )
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn title_request_reaches_only_the_owning_bench() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    let b = h.open().await;
    h.start(&a, "ra").await.unwrap();
    let mut on_a = subscribe(&h, &a);
    let mut on_b = subscribe(&h, &b);

    let reply = h
        .keyed(
            OperationId::BenchRequestTitle,
            &support::uuid_key(),
            json!({"runId": "ra", "title": " Review PR "}),
        )
        .await;
    // `keyed`는 데스크톱 principal이다: agent 전용이므로 거부된다.
    assert_eq!(
        reply.unwrap_err().code,
        workbench_protocol::FaultCode::Forbidden
    );

    let applied = h
        .call(
            &AuthenticatedPrincipal::agent("ra"),
            OperationId::BenchRequestTitle,
            json!({"runId": "ra", "title": " Review PR "}),
        )
        .await
        .unwrap();
    assert_eq!(applied, json!({"ok": true, "appliedTitle": "Review PR"}));

    match tokio::time::timeout(Duration::from_secs(2), on_a.next()).await {
        Ok(Some(EventItem::Event { event })) => {
            assert_eq!(event.schema, BENCH_TITLE_REQUESTED_V1);
            assert_eq!(event.body, json!({"title": "Review PR"}));
        }
        other => panic!("expected title event, got {other:?}"),
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(200), on_b.next())
            .await
            .is_err(),
        "other bench stream must stay silent"
    );
    assert_eq!(titles(&h, &a), vec!["Review PR".to_owned()]);
    assert!(titles(&h, &b).is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn title_is_delivered_without_subscribers() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    h.start(&a, "ra").await.unwrap();
    h.call(
        &AuthenticatedPrincipal::agent("ra"),
        OperationId::BenchRequestTitle,
        json!({"runId": "ra", "title": "t"}),
    )
    .await
    .unwrap();
    assert_eq!(titles(&h, &a), vec!["t".to_owned()]);
}
