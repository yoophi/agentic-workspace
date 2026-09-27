//! 043 T006 (Codex 설계 리뷰 H1 근거 고정): 실제 event hub에서 보관 한도를 넘긴 상태 스트림(run·교환·orchestration)은
//! `after = 0` 재구독도 `RetentionExceeded` gap이고 live 수신자를 등록하지 않는다(구독이 곧 끝난다). gap이 알려 준
//! `lastSequence`로 다시 연 구독은 live를 등록해 그 뒤 이벤트를 빠짐없이 받는다. 클라이언트의 복구(live 먼저 → 버퍼 →
//! 스냅샷 병합)가 기대는 서버 동작이다.

use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use serde_json::json;
use workbench_core::infrastructure::event_hub::{EventHub, EventHubLimits};
use workbench_protocol::{
    events::StreamKind, AuthenticatedPrincipal, EventItem, EventStream, GapReason, StreamCursor,
    Subscription,
};

const CAPACITY: usize = 4;
const PUBLISHED: u64 = 11;

fn hub() -> Arc<EventHub> {
    EventHub::new(
        "epoch-1",
        EventHubLimits {
            run_journal_capacity: CAPACITY,
            exchange_journal_capacity: CAPACITY,
            orchestration_journal_capacity: CAPACITY,
            ..EventHubLimits::default()
        },
    )
}

fn publish(hub: &EventHub, kind: StreamKind, key: &str, n: u64) {
    hub.publish_state(kind, key, "test.v1", json!({ "n": n }), false, &mut |_| {})
        .expect("published");
}

fn subscribe(hub: &Arc<EventHub>, stream_id: &str, after: u64) -> EventStream {
    hub.subscribe(
        &AuthenticatedPrincipal::desktop(),
        Subscription {
            cursors: vec![StreamCursor {
                stream_id: stream_id.into(),
                epoch: "epoch-1".into(),
                after_sequence: after,
            }],
        },
    )
    .expect("subscribe")
}

async fn next(stream: &mut EventStream) -> Option<EventItem> {
    // 안전 상한일 뿐 동기화 수단이 아니다: 기대 항목은 이미 준비돼 있거나(대기열) 즉시 끝난다(송신자 없음).
    tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("stream must yield or end without waiting")
}

#[tokio::test]
async fn after_zero_resubscription_is_gap_only_and_last_sequence_resubscription_goes_live() {
    for (kind, key) in [
        (StreamKind::Run, "r1"),
        (StreamKind::Exchange, "bench-1"),
        (StreamKind::Orchestration, "binding-1"),
    ] {
        let hub = hub();
        let stream_id = kind.stream_id(key);
        for n in 1..=PUBLISHED {
            publish(&hub, kind, key, n);
        }

        // (a) after = 0: 보관 범위가 1부터가 아니므로 gap, 그리고 live 미등록 → 구독은 끝난다.
        let mut zero = subscribe(&hub, &stream_id, 0);
        let last = match next(&mut zero).await {
            Some(EventItem::Gap { gap }) => {
                assert_eq!(gap.reason, GapReason::RetentionExceeded, "{stream_id}");
                assert_eq!(gap.last_sequence, Some(PUBLISHED), "{stream_id}");
                assert_eq!(
                    gap.first_sequence,
                    Some(PUBLISHED - CAPACITY as u64 + 1),
                    "{stream_id}"
                );
                gap.last_sequence.unwrap()
            }
            other => panic!("{stream_id}: expected retention gap, got {other:?}"),
        };
        publish(&hub, kind, key, PUBLISHED + 1);
        assert!(
            next(&mut zero).await.is_none(),
            "{stream_id}: after-0 resubscription must not register a live receiver"
        );

        // (b) gap의 lastSequence로 연 구독: 이미 발행된 L+1부터(대기열 replay) 그리고 이후 live까지 연속으로.
        let mut live = subscribe(&hub, &stream_id, last);
        publish(&hub, kind, key, PUBLISHED + 2);
        for expected in [PUBLISHED + 1, PUBLISHED + 2] {
            match next(&mut live).await {
                Some(EventItem::Event { event }) => {
                    assert_eq!(event.stream_id, stream_id);
                    assert_eq!(event.sequence, expected, "{stream_id}");
                }
                other => panic!("{stream_id}: expected event {expected}, got {other:?}"),
            }
        }
    }
}
