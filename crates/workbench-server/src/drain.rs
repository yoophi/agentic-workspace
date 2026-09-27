//! 받아들인 분리 호출의 수명(042 research R17). 종료 신호가 오면 새 호출을 `503`으로 거절하고, 이미 받아들인 호출은
//! 끝날 때까지 기다린 뒤에야 `serve`가 반환한다 — 연결이 끊긴 호출이라도 실행과 멱등 결과 기록이 런타임 종료로
//! 취소되지 않게 한다.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use tokio::sync::Notify;

pub const MESSAGE_SHUTTING_DOWN: &str = "server is shutting down.";
/// drain 경고 간격 기본값. **상한이 아니다** — 이 간격마다 남은 호출 수를 경고하고 계속 기다린다(R17: 받아들인
/// 호출은 끝까지 둔다). 소유 런타임은 `serve`가 반환한 뒤에만 종료해야 한다.
pub const DEFAULT_DRAIN_WARN_AFTER: Duration = Duration::from_secs(30);

/// 받아들인 호출을 서버 소유 task에서 실행하고 그 결과를 기다린다. 호출자(연결 handler) future가 drop돼도 task는
/// 끝까지 돈다 — `/v1/calls`와 AW MCP 도구 호출이 이 함수 하나를 쓴다(R17).
pub async fn spawn_accepted<F, T>(guard: CallGuard, work: F) -> Result<T, tokio::task::JoinError>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    tokio::spawn(async move {
        let _guard = guard; // 완료·panic 때 drop → drain이 안다
        work.await
    })
    .await
}

#[derive(Debug, Default)]
pub struct DetachedCalls {
    closing: AtomicBool,
    active: AtomicUsize,
    idle: Notify,
}

/// 받아들인 호출 하나. drop(완료·panic 모두)될 때 활성 수를 줄인다.
#[derive(Debug)]
pub struct CallGuard {
    calls: Arc<DetachedCalls>,
}

impl Drop for CallGuard {
    fn drop(&mut self) {
        if self.calls.active.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.calls.idle.notify_waiters();
        }
    }
}

impl DetachedCalls {
    /// 받아들이기. 종료 중이면 `None`. 증가 뒤 다시 확인해 `close`와의 경합에서 새지 않는다.
    pub fn accept(self: &Arc<Self>) -> Option<CallGuard> {
        if self.closing.load(Ordering::Acquire) {
            return None;
        }
        self.active.fetch_add(1, Ordering::AcqRel);
        let guard = CallGuard {
            calls: Arc::clone(self),
        };
        if self.closing.load(Ordering::Acquire) {
            return None; // guard drop이 수를 되돌린다
        }
        Some(guard)
    }

    pub fn close(&self) {
        self.closing.store(true, Ordering::Release);
    }

    pub fn is_closing(&self) -> bool {
        self.closing.load(Ordering::Acquire)
    }

    pub fn active(&self) -> usize {
        self.active.load(Ordering::Acquire)
    }

    /// 활성 호출이 0이 될 때까지 기다린다. `warn_after`마다 `on_warn(남은 수)`를 부르고 계속 기다린다.
    pub async fn drain_until_idle(&self, warn_after: Duration, mut on_warn: impl FnMut(usize)) {
        while !self.wait_idle(warn_after).await {
            on_warn(self.active());
        }
    }

    /// 한 번의 기다림: 시간 안에 활성 0이면 `true`.
    async fn wait_idle(&self, timeout: Duration) -> bool {
        tokio::time::timeout(timeout, async {
            loop {
                let notified = self.idle.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                if self.active() == 0 {
                    return;
                }
                notified.await;
            }
        })
        .await
        .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn closing_rejects_new_calls_and_drain_waits_for_accepted() {
        let calls = Arc::new(DetachedCalls::default());
        let guard = calls.accept().expect("accepted before close");
        calls.close();
        assert!(calls.accept().is_none());
        assert_eq!(calls.active(), 1);
        // 요청 지연(150ms)이 경고 간격(20ms)보다 길다: drain은 경고만 내고 조기 반환하지 않는다.
        let released = Arc::new(AtomicBool::new(false));
        let releaser = {
            let released = Arc::clone(&released);
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(150)).await;
                released.store(true, Ordering::Release);
                drop(guard);
            })
        };
        let mut warnings = Vec::new();
        calls
            .drain_until_idle(Duration::from_millis(20), |left| warnings.push(left))
            .await;
        assert!(
            released.load(Ordering::Acquire),
            "drain returned before the accepted call finished"
        );
        assert!(
            warnings.len() >= 2,
            "expected repeated warnings, got {warnings:?}"
        );
        assert!(warnings.iter().all(|left| *left == 1));
        releaser.await.unwrap();
        assert_eq!(calls.active(), 0);
    }

    /// 호출자 future를 중간에 버려도(연결 단절) 받아들인 작업은 끝까지 실행되고 drain은 그 끝을 기다린다.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn accepted_work_survives_caller_cancellation() {
        let calls = Arc::new(DetachedCalls::default());
        let finished = Arc::new(AtomicBool::new(false));
        let entered = Arc::new(Notify::new());
        let caller = {
            let (calls, finished, entered) = (
                Arc::clone(&calls),
                Arc::clone(&finished),
                Arc::clone(&entered),
            );
            tokio::spawn(async move {
                let guard = calls.accept().unwrap();
                spawn_accepted(guard, async move {
                    entered.notify_one();
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    finished.store(true, Ordering::Release);
                })
                .await
            })
        };
        entered.notified().await;
        caller.abort(); // 연결 handler가 사라진다
        assert!(caller.await.unwrap_err().is_cancelled());
        assert!(!finished.load(Ordering::Acquire));
        calls.close();
        calls
            .drain_until_idle(Duration::from_millis(10), |_| {})
            .await;
        assert!(
            finished.load(Ordering::Acquire),
            "accepted work ran to completion"
        );
    }

    #[tokio::test]
    async fn drain_without_calls_returns_immediately() {
        let calls = Arc::new(DetachedCalls::default());
        calls.close();
        let mut warned = false;
        calls
            .drain_until_idle(Duration::from_millis(1), |_| warned = true)
            .await;
        assert!(!warned);
    }
}
