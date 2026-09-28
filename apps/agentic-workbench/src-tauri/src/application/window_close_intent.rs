//! 창 닫기 의도 판정(044 T030, research R8 "설계 확정(T003)"). 순수 상태 기계다 — Tauri 이벤트 연결은
//! `infrastructure::window_lifecycle`이 한다.
//!
//! 관측(R8-spike, macOS): 사용자가 창을 닫으면(빨간 버튼·Close Window 메뉴·마지막 창) 그 창의 `CloseRequested`가 먼저
//! 오고 `Destroyed`가 뒤따른다. 앱 종료(Cmd+Q·Dock Quit·AppleScript `quit`)는 `RunEvent::Exit`만 오고 그 전에 창 이벤트가
//! 없다. `SIGTERM`은 이벤트가 없다. 그래서 작업대 닫기는 "종료 의도가 서기 전에 온 그 창의 `CloseRequested`"가 있을
//! 때만 한다. 종료 의도 뒤의 `CloseRequested`·`Destroyed`는 작업대를 남긴다.

use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseDecision {
    /// 사용자가 이 창을 닫았다: 그 창이 연 작업대를 닫는다.
    CloseBench,
    /// 앱 종료가 창을 걷어 냈다(또는 의도 신호가 없다): 작업대·run을 남기고 토큰만 폐기한다.
    KeepBench,
}

#[derive(Debug, Default)]
pub struct WindowCloseIntent {
    intents: HashSet<(String, String)>,
    quitting: bool,
}

impl WindowCloseIntent {
    pub fn new() -> Self {
        Self::default()
    }

    /// 창의 `CloseRequested`. 종료 의도가 선 뒤라면 의도로 세지 않는다.
    pub fn close_requested(&mut self, label: &str, incarnation: &str) {
        if !self.quitting {
            self.intents
                .insert((label.to_owned(), incarnation.to_owned()));
        }
    }

    /// 앱 종료 의도(`ExitRequested`·`RunEvent::Exit`).
    pub fn quitting(&mut self) {
        self.quitting = true;
    }

    pub fn is_quitting(&self) -> bool {
        self.quitting
    }

    /// 창의 `Destroyed`: 그 incarnation의 의도를 꺼내 판정한다.
    pub fn destroyed(&mut self, label: &str, incarnation: &str) -> CloseDecision {
        if self
            .intents
            .remove(&(label.to_owned(), incarnation.to_owned()))
        {
            CloseDecision::CloseBench
        } else {
            CloseDecision::KeepBench
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (a) 빨간 버튼·(b1) Close Window 메뉴: `CloseRequested` → `Destroyed`.
    #[test]
    fn a_user_closed_window_closes_its_bench() {
        let mut intent = WindowCloseIntent::new();
        intent.close_requested("session-1", "i1");
        assert_eq!(
            intent.destroyed("session-1", "i1"),
            CloseDecision::CloseBench
        );
    }

    /// (f) 마지막 창 닫기: `CloseRequested` → `Destroyed` → `ExitRequested` → `Exit`. 창 닫기가 먼저 판정된다.
    #[test]
    fn closing_the_last_window_closes_its_bench_before_the_app_exits() {
        let mut intent = WindowCloseIntent::new();
        intent.close_requested("main", "i1");
        let decision = intent.destroyed("main", "i1");
        intent.quitting();
        assert_eq!(decision, CloseDecision::CloseBench);
    }

    /// (c) Cmd+Q·(d) Dock Quit·(e) AppleScript quit: `Exit`만 오고, 그 뒤에 `Destroyed`가 와도 작업대를 남긴다.
    #[test]
    fn an_app_quit_that_tears_windows_down_keeps_their_benches() {
        let mut intent = WindowCloseIntent::new();
        intent.quitting();
        assert_eq!(
            intent.destroyed("session-1", "i1"),
            CloseDecision::KeepBench
        );
        assert_eq!(intent.destroyed("main", "i2"), CloseDecision::KeepBench);
    }

    /// 종료 의도 뒤의 `CloseRequested`는 의도가 아니다.
    #[test]
    fn a_close_request_after_quitting_is_not_an_intent() {
        let mut intent = WindowCloseIntent::new();
        intent.quitting();
        intent.close_requested("session-1", "i1");
        assert_eq!(
            intent.destroyed("session-1", "i1"),
            CloseDecision::KeepBench
        );
    }

    /// 의도는 incarnation에 묶인다: 같은 label로 다시 연 창의 `Destroyed`가 옛 창의 의도를 쓰지 않는다.
    #[test]
    fn an_intent_belongs_to_one_incarnation() {
        let mut intent = WindowCloseIntent::new();
        intent.close_requested("settings", "old");
        assert_eq!(
            intent.destroyed("settings", "new"),
            CloseDecision::KeepBench
        );
        assert_eq!(
            intent.destroyed("settings", "old"),
            CloseDecision::CloseBench
        );
    }

    /// 의도 없는 `Destroyed`(`SIGTERM`은 이벤트 자체가 없다 — 이것은 신호 없는 파괴의 기본값).
    #[test]
    fn a_destroy_without_any_close_request_keeps_the_bench() {
        let mut intent = WindowCloseIntent::new();
        assert_eq!(
            intent.destroyed("session-1", "i1"),
            CloseDecision::KeepBench
        );
    }
}
