//! 외부 Seam: `Workbench` trait. 모든 inbound Adapter(Tauri compat, HTTP, in-memory)는 이 두 동작만 안다.
//!
//! `events`의 계약(039): `specs/039-workbench-events/contracts/workbench-events.md`.

use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};

use async_trait::async_trait;
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    call::{CallReply, CallRequest, RequestId},
    fault::WorkbenchFault,
    principal::AuthenticatedPrincipal,
};

/// stream별 재연결 cursor. 호출자가 마지막으로 반영한 (스트림, 세대, 순번).
/// 알림용 스트림(`worktree:*`)은 `afterSequence`를 보지 않고 live부터 전달한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StreamCursor {
    pub stream_id: String,
    pub epoch: String,
    pub after_sequence: u64,
}

/// 이벤트 구독 요청. cursor는 1–64개.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    #[serde(default)]
    pub cursors: Vec<StreamCursor>,
}

/// 공통 이벤트 봉투. `sequence`는 스트림 안에서 1부터 1씩 증가한다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope {
    pub event_id: String,
    pub stream_id: String,
    pub epoch: String,
    pub sequence: u64,
    pub schema: String,
    pub occurred_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<RequestId>,
    #[schema(value_type = Value)]
    pub body: serde_json::Value,
}

/// cursor 다음을 이어 붙일 수 없는 이유.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum GapReason {
    /// 스트림이 없고 cursor가 0보다 크다.
    UnknownStream,
    /// 보관 한도로 run이 제거되었다(cursor 0이어도).
    Evicted,
    /// cursor의 세대가 현재 세대와 다르다(서버 재시작).
    EpochChanged,
    /// cursor가 보관 범위보다 오래되었다.
    RetentionExceeded,
    /// 구독자 대기열이 넘쳤다. 구독 전체가 닫힌다.
    SubscriberLagged,
    /// 서버가 종료 중이다.
    Shutdown,
}

/// gap 신호. 받은 쪽은 상태를 다시 조회해 동기화한다. 해당 스트림에는 그 구독에서 더 이상 이벤트가 오지 않는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GapNotice {
    pub stream_id: String,
    /// 현재 세대.
    pub epoch: String,
    pub reason: GapReason,
    /// 현재 보관 범위(있으면).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_sequence: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sequence: Option<u64>,
}

/// `EventStream`의 항목.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EventItem {
    Event { event: EventEnvelope },
    Gap { gap: GapNotice },
}

/// 이벤트 흐름. drop하면 구독이 해제된다.
pub struct EventStream {
    inner: Pin<Box<dyn Stream<Item = EventItem> + Send>>,
}

impl EventStream {
    pub fn new(inner: impl Stream<Item = EventItem> + Send + 'static) -> Self {
        Self {
            inner: Box::pin(inner),
        }
    }

    /// 다음 항목. 스트림 유틸 crate 없이 소비할 수 있게 둔다(Tauri 호환 어댑터용).
    pub async fn next_item(&mut self) -> Option<EventItem> {
        std::future::poll_fn(|cx| self.inner.as_mut().poll_next(cx)).await
    }
}

impl fmt::Debug for EventStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventStream").finish_non_exhaustive()
    }
}

impl Stream for EventStream {
    type Item = EventItem;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<EventItem>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Seam. `call`은 유한 명령·조회, `events`는 순서 있는 이벤트와 replay.
#[async_trait]
pub trait Workbench: Send + Sync {
    async fn call(
        &self,
        principal: AuthenticatedPrincipal,
        request: CallRequest,
    ) -> Result<CallReply, WorkbenchFault>;

    /// 구독 등록·기준점·replay 복사가 한 lock 안에서 끝나므로 동기 함수다. 오류(권한·입력·한도)는 즉시 돌려준다.
    fn events(
        &self,
        principal: AuthenticatedPrincipal,
        request: Subscription,
    ) -> Result<EventStream, WorkbenchFault>;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn event_item_and_gap_serialize_with_kind_tag() {
        let gap = EventItem::Gap {
            gap: GapNotice {
                stream_id: "run:r1".into(),
                epoch: "e1".into(),
                reason: GapReason::RetentionExceeded,
                first_sequence: Some(5),
                last_sequence: Some(10),
            },
        };
        assert_eq!(
            serde_json::to_value(&gap).unwrap(),
            json!({"kind": "gap", "gap": {"streamId": "run:r1", "epoch": "e1", "reason": "retentionExceeded", "firstSequence": 5, "lastSequence": 10}})
        );
        let back: EventItem = serde_json::from_value(serde_json::to_value(&gap).unwrap()).unwrap();
        assert_eq!(back, gap);
    }
}
