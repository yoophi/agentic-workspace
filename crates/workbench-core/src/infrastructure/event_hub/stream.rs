//! 스트림 하나의 상태와 cursor 판정(research R2 표). 판정은 순수 함수라 단위 테스트로 표의 모든 행을 고정한다.

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use tokio::sync::mpsc;
use workbench_protocol::{EventEnvelope, EventItem, GapReason};

/// 스트림에 등록된 구독자 하나.
pub(crate) struct Subscriber {
    pub id: u64,
    pub tx: mpsc::Sender<EventItem>,
    /// 대기열이 넘쳐 해제되었으면 true. 받는 쪽이 `Gap(subscriberLagged)`로 닫는다.
    pub lagged: Arc<AtomicBool>,
    /// 넘친 스트림(gap에 싣는다).
    pub lagged_stream: Arc<std::sync::Mutex<Option<String>>>,
}

impl Subscriber {
    /// 막히지 않고 보낸다. 가득 찼거나 받는 쪽이 사라졌으면 false(호출자가 목록에서 뺀다).
    pub fn offer(&self, stream_id: &str, item: EventItem) -> bool {
        match self.tx.try_send(item) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(_)) => {
                *self.lagged_stream.lock().unwrap_or_else(|p| p.into_inner()) =
                    Some(stream_id.to_owned());
                self.lagged.store(true, Ordering::SeqCst);
                false
            }
            Err(mpsc::error::TrySendError::Closed(_)) => false,
        }
    }
}

pub(crate) struct JournalEntry {
    pub envelope: EventEnvelope,
    /// 그 이벤트가 run을 끝냈는지(호환 replay의 이벤트별 `terminal`).
    pub terminal: bool,
}

pub(crate) struct StreamState {
    pub stream_id: String,
    pub sequence: u64,
    pub journal: VecDeque<JournalEntry>,
    pub terminal: bool,
    /// 정리(eviction)되어 map에서 빠졌으면 true. 이 뒤의 발행·구독은 거절한다.
    pub removed: bool,
    pub subscribers: Vec<Subscriber>,
}

impl StreamState {
    pub fn new(stream_id: String) -> Self {
        Self {
            stream_id,
            sequence: 0,
            journal: VecDeque::new(),
            terminal: false,
            removed: false,
            subscribers: Vec::new(),
        }
    }

    pub fn first_retained(&self) -> u64 {
        self.journal
            .front()
            .map(|entry| entry.envelope.sequence)
            .unwrap_or(self.sequence.saturating_add(1))
    }

    /// 보관 한도를 넘으면 앞에서 버린다.
    pub fn push_journal(&mut self, entry: JournalEntry, capacity: usize) {
        self.journal.push_back(entry);
        while self.journal.len() > capacity {
            self.journal.pop_front();
        }
    }

    /// 구독자 모두에게 보내고, 넘치거나 끊긴 구독자는 뺀다.
    pub fn fan_out(&mut self, item: &EventItem) {
        let stream_id = self.stream_id.clone();
        self.subscribers
            .retain(|subscriber| subscriber.offer(&stream_id, item.clone()));
    }

    pub fn unsubscribe(&mut self, id: u64) {
        self.subscribers.retain(|subscriber| subscriber.id != id);
    }
}

/// 상태 복원용 스트림에서 cursor를 어떻게 이어 붙일지.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CursorDecision {
    /// `after+1 ..= last`를 replay한 뒤 live.
    Replay { after: u64 },
    /// replay 없이 live.
    Live,
    Gap {
        reason: GapReason,
        first: Option<u64>,
        last: Option<u64>,
    },
    /// 같은 세대에서 cursor가 마지막 번호보다 크다 → 입력 오류.
    Ahead,
}

/// 존재하는 상태 복원용 스트림에 대한 판정. `after == 0`은 세대와 무관하게 "처음부터"다.
pub(crate) fn decide_existing(
    after: u64,
    cursor_epoch: &str,
    epoch: &str,
    first_retained: u64,
    last: u64,
) -> CursorDecision {
    if after == 0 {
        return if first_retained > 1 {
            CursorDecision::Gap {
                reason: GapReason::RetentionExceeded,
                first: Some(first_retained),
                last: Some(last),
            }
        } else {
            CursorDecision::Replay { after: 0 }
        };
    }
    if cursor_epoch != epoch {
        return CursorDecision::Gap {
            reason: GapReason::EpochChanged,
            first: None,
            last: None,
        };
    }
    if after > last {
        return CursorDecision::Ahead;
    }
    if after.saturating_add(1) < first_retained {
        return CursorDecision::Gap {
            reason: GapReason::RetentionExceeded,
            first: Some(first_retained),
            last: Some(last),
        };
    }
    CursorDecision::Replay { after }
}

/// 스트림이 없을 때(제거 표식도 없음). cursor 0이면 시작 전 run으로 보고 live를 기다린다.
pub(crate) fn decide_missing(after: u64, cursor_epoch: &str, epoch: &str) -> CursorDecision {
    if after == 0 {
        CursorDecision::Live
    } else if cursor_epoch != epoch {
        CursorDecision::Gap {
            reason: GapReason::EpochChanged,
            first: None,
            last: None,
        }
    } else {
        CursorDecision::Gap {
            reason: GapReason::UnknownStream,
            first: None,
            last: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const E: &str = "epoch-1";

    #[test]
    fn existing_stream_rows() {
        // 처음부터(세대 무관)
        assert_eq!(
            decide_existing(0, "other", E, 1, 10),
            CursorDecision::Replay { after: 0 }
        );
        // 처음부터지만 앞부분이 보관 한도로 사라짐
        assert!(matches!(
            decide_existing(0, E, E, 5, 10),
            CursorDecision::Gap {
                reason: GapReason::RetentionExceeded,
                ..
            }
        ));
        // 중간
        assert_eq!(
            decide_existing(4, E, E, 1, 10),
            CursorDecision::Replay { after: 4 }
        );
        // 끝
        assert_eq!(
            decide_existing(10, E, E, 1, 10),
            CursorDecision::Replay { after: 10 }
        );
        // 미래
        assert_eq!(decide_existing(11, E, E, 1, 10), CursorDecision::Ahead);
        // 세대 불일치
        assert!(matches!(
            decide_existing(4, "old", E, 1, 10),
            CursorDecision::Gap {
                reason: GapReason::EpochChanged,
                ..
            }
        ));
        // 보관 범위 밖(4 다음인 5가 없음: 보관 6..)
        assert_eq!(
            decide_existing(4, E, E, 6, 10),
            CursorDecision::Gap {
                reason: GapReason::RetentionExceeded,
                first: Some(6),
                last: Some(10)
            }
        );
        // 경계: 5 다음인 6이 보관 첫 번호 → 이어 붙일 수 있다
        assert_eq!(
            decide_existing(5, E, E, 6, 10),
            CursorDecision::Replay { after: 5 }
        );
    }

    #[test]
    fn missing_stream_rows() {
        assert_eq!(decide_missing(0, "any", E), CursorDecision::Live);
        assert!(matches!(
            decide_missing(3, E, E),
            CursorDecision::Gap {
                reason: GapReason::UnknownStream,
                ..
            }
        ));
        assert!(matches!(
            decide_missing(3, "old", E),
            CursorDecision::Gap {
                reason: GapReason::EpochChanged,
                ..
            }
        ));
    }
}
