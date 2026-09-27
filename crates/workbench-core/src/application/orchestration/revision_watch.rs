//! 작업 영역 revision 알림(041 research R12). 저장소 commit이 작업 영역을 바꿀 때마다 그 작업 영역의 watch 값을
//! 새 revision으로 바꾼다. `waitChildTasks`는 **먼저 구독하고 그다음 상태를 읽어** 완료가 아니면 변경을 기다린다 —
//! 구독 뒤의 변경은 반드시 깨우므로(읽기와 기다리기 사이의 보고도) 알림을 놓치지 않는다. lock을 쥐고 기다리지 않는다.

use std::{collections::HashMap, sync::Mutex};

use tokio::sync::watch;

#[derive(Debug, Default)]
pub struct RevisionWatch {
    senders: Mutex<HashMap<String, watch::Sender<u64>>>,
}

impl RevisionWatch {
    pub fn subscribe(&self, workspace_id: &str) -> watch::Receiver<u64> {
        let mut senders = self.lock();
        senders
            .entry(workspace_id.to_owned())
            .or_insert_with(|| watch::channel(0).0)
            .subscribe()
    }

    /// 작업 영역이 바뀌었다. 구독자가 없으면 채널을 정리한다.
    pub fn notify(&self, workspace_id: &str, revision: u64) {
        let mut senders = self.lock();
        if let Some(sender) = senders.get(workspace_id) {
            if sender.receiver_count() == 0 {
                senders.remove(workspace_id);
            } else {
                sender.send_replace(revision);
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, watch::Sender<u64>>> {
        self.senders
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_change_after_subscribing_always_wakes_the_waiter() {
        let watch = RevisionWatch::default();
        let mut receiver = watch.subscribe("w1");
        watch.notify("w1", 3);
        tokio::time::timeout(std::time::Duration::from_millis(100), receiver.changed())
            .await
            .expect("woken")
            .unwrap();
        assert_eq!(*receiver.borrow(), 3);
        drop(receiver);
        watch.notify("w1", 4);
        assert!(
            watch.lock().is_empty(),
            "channel without subscribers is dropped"
        );
    }
}
