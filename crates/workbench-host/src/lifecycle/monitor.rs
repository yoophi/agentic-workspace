//! 서버 감시 루프(044 T041, research R9·R10, contracts/server-lifecycle.md §5): 주기적으로 core의 생명주기 판정
//! (`ServerControl::tick`)을 부른다 — 유휴 시계, 유휴 비우기, wait 비우기의 정지 판정. `stopping`에 들어가면(판정·
//! `server.stop`·강제 정지 어느 쪽이든) 끝난다. 정지 뒤 정리(받아들인 호출 drain·쉬는 세션 취소)는 호출자(`serve`·시험)가
//! `WorkbenchHost::shutdown`으로 한다.

use std::{sync::Arc, time::Duration};

use workbench_core::application::server_control::ServerControl;

/// 기본 유휴 시간(R9).
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(600);
/// 판정 주기. 정지 지연의 상한이다(판정 자체는 G 아래에서 원자적이다).
pub const TICK: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy)]
pub struct MonitorOptions {
    pub idle_timeout: Duration,
    pub tick: Duration,
}

impl Default for MonitorOptions {
    fn default() -> Self {
        Self {
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            tick: TICK,
        }
    }
}

/// `stopping`에 들어갈 때까지 판정을 돈다.
pub async fn run_until_stopped(control: Arc<ServerControl>, options: MonitorOptions) {
    let mut stopped = control.stopped();
    let mut interval = tokio::time::interval(options.tick);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        if *stopped.borrow_and_update() {
            return;
        }
        tokio::select! {
            _ = interval.tick() => {
                if control.tick(options.idle_timeout).await {
                    return;
                }
            }
            changed = stopped.changed() => {
                if changed.is_err() {
                    return;
                }
            }
        }
    }
}
