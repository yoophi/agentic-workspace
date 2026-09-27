//! 구독 하나의 `Stream` 구현. replay 목록 → 대기열(스트림별 high-water 초과만) 순으로 내고, 대기열이 넘쳤으면
//! `Gap(subscriberLagged)`를 내고 끝난다. drop하면 등록된 스트림들에서 해제된다.

use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
};

use futures_core::Stream;
use tokio::sync::mpsc;
use workbench_protocol::{EventItem, GapNotice, GapReason};

use super::{stream::StreamState, EventHub};

pub(crate) struct HubSubscription {
    pub(super) id: u64,
    pub(super) hub: Arc<EventHub>,
    pub(super) epoch: String,
    /// 구독 시점에 복사한 replay와 즉시 판정된 gap.
    pub(super) pending: VecDeque<EventItem>,
    pub(super) rx: mpsc::Receiver<EventItem>,
    /// 스트림별 기준점. 대기열에서 이 이하 순번은 버린다(방어적 중복 제거).
    pub(super) high_water: HashMap<String, u64>,
    pub(super) registered: Vec<Arc<Mutex<StreamState>>>,
    /// 알림용 스트림으로 참조 수를 올린 worktree 실제 경로(US3).
    pub(super) watched: Vec<std::path::PathBuf>,
    pub(super) lagged: Arc<AtomicBool>,
    pub(super) lagged_stream: Arc<Mutex<Option<String>>>,
    pub(super) finished: bool,
}

impl HubSubscription {
    fn lag_gap(&self) -> EventItem {
        let stream_id = self
            .lagged_stream
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
            .unwrap_or_default();
        EventItem::Gap {
            gap: GapNotice {
                stream_id,
                epoch: self.epoch.clone(),
                reason: GapReason::SubscriberLagged,
                first_sequence: None,
                last_sequence: None,
            },
        }
    }

    fn keep(&self, item: &EventItem) -> bool {
        match item {
            EventItem::Event { event } => self
                .high_water
                .get(&event.stream_id)
                .is_none_or(|high| event.sequence > *high),
            EventItem::Gap { .. } => true,
        }
    }
}

impl Stream for HubSubscription {
    type Item = EventItem;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<EventItem>> {
        let this = &mut *self;
        if this.finished {
            return Poll::Ready(None);
        }
        if let Some(item) = this.pending.pop_front() {
            return Poll::Ready(Some(item));
        }
        loop {
            // 넘친 뒤의 대기열은 신뢰할 수 없다(빠진 이벤트가 있다). 바로 gap으로 닫는다.
            if this.lagged.load(Ordering::SeqCst) {
                this.finished = true;
                return Poll::Ready(Some(this.lag_gap()));
            }
            match this.rx.poll_recv(cx) {
                Poll::Ready(Some(item)) => {
                    if this.keep(&item) {
                        return Poll::Ready(Some(item));
                    }
                }
                Poll::Ready(None) => {
                    this.finished = true;
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl Drop for HubSubscription {
    fn drop(&mut self) {
        for stream in &self.registered {
            self.hub.unsubscribe(stream, self.id);
        }
        for path in &self.watched {
            self.hub.release_watch(path);
        }
        self.hub.subscription_closed();
    }
}
