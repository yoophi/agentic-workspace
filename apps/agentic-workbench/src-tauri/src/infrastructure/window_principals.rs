//! 창별 데스크톱 주체(043, research R1). 창을 **만들 때** incarnation을 발급해 등록하고, 그 창의 모든 Workbench 호출(호환
//! 경로 command와 네트워크 경로 토큰)은 주체 `desktop:window:<label>:<incarnation>`을 쓴다. 작업대 소유가 주체로 갈리므로
//! 한 창의 자격 증명으로 다른 창의 작업대를 조작할 수 없다(서버 판정).
//!
//! 수명 규칙(호출 순서에 기대지 않는다):
//! - 등록은 창 생성 직전에만 한다([`register`]). 조회([`current`])는 만들지 않는다 — 창이 닫힌 뒤 늦게 도는 command는
//!   주체가 없어 거절되고, 새 incarnation을 만들어 내지 않는다.
//! - 거둬들이기([`retire`])는 **발급한 incarnation과 같을 때만** 지운다. 창마다 붙인 `Destroyed` 처리기가 자기
//!   incarnation을 붙잡고 부르므로, 같은 label로 새로 만든 창의 등록을 옛 창의 늦은 정리가 지우지 못한다.
//! - Tauri async command는 인자(`window` 포함)를 future 안에서 추출하고(`tauri-macros` `body_async`), `Window`에는
//!   label 말고 인스턴스 식별자가 없다. 그래서 같은 label로 다시 만든 창이 등록된 뒤 옛 창의 command가 늦게 실행되면
//!   새 창 주체로 풀릴 수 있다. session 창 label은 매번 새 id라 재사용되지 않는다(`window_manager::session_label`) —
//!   작업대에 묶인 호출은 session 창에서만 나오므로 새 창의 작업대로 승격되지 않는다. label을 다시 쓰는 창(`settings`,
//!   `main`)의 호출은 작업대와 무관하다(인벤토리 참조).

use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard, OnceLock},
};

use workbench_protocol::AuthenticatedPrincipal;

pub const MESSAGE_WINDOW_NOT_REGISTERED: &str = "Window is no longer available.";

fn table() -> MutexGuard<'static, HashMap<String, String>> {
    static TABLE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    TABLE
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 창 생성 직전: 새 incarnation을 발급해 등록한다. 같은 label의 이전 등록이 남아 있으면(옛 창 정리 전) 덮어쓴다 —
/// Tauri는 살아 있는 창끼리 label을 겹치지 않으므로 남은 것은 닫힌 창의 것이다.
pub fn register(label: &str) -> String {
    let incarnation = uuid::Uuid::new_v4().to_string();
    table().insert(label.to_owned(), incarnation.clone());
    incarnation
}

/// 창의 현재 incarnation. 등록이 없으면(닫힌 창) `None`.
pub fn incarnation(label: &str) -> Option<String> {
    table().get(label).cloned()
}

/// 창의 현재 주체. 등록이 없으면 `None`.
pub fn current(label: &str) -> Option<AuthenticatedPrincipal> {
    incarnation(label)
        .map(|incarnation| AuthenticatedPrincipal::desktop_window(label, &incarnation))
}

/// 창 `Destroyed`: 그 창이 받은 `incarnation`이 아직 현재 값일 때만 지운다. 폐기할 주체는 등록 여부와 상관없이 그
/// incarnation의 주체다(토큰·표·작업대 정리는 이 주체로 한다).
pub fn retire(label: &str, incarnation: &str) -> AuthenticatedPrincipal {
    let mut table = table();
    if table
        .get(label)
        .is_some_and(|current| current == incarnation)
    {
        table.remove(label);
    }
    AuthenticatedPrincipal::desktop_window(label, incarnation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label() -> String {
        format!("test-window-{}", uuid::Uuid::new_v4())
    }

    #[test]
    fn lookups_never_create_an_incarnation() {
        let label = label();
        assert_eq!(current(&label), None);
        assert_eq!(
            current(&label),
            None,
            "a late call must not register the window"
        );
    }

    #[test]
    fn a_registered_window_keeps_its_principal_until_its_own_retire() {
        let label = label();
        let inc = register(&label);
        let principal = current(&label).unwrap();
        assert_eq!(
            principal.subject.as_str(),
            format!("desktop:window:{label}:{inc}")
        );
        assert_eq!(current(&label), Some(principal.clone()));
        assert_eq!(retire(&label, &inc), principal);
        assert_eq!(
            current(&label),
            None,
            "calls after Destroyed have no principal"
        );
    }

    /// 같은 label로 새 창이 먼저 등록된 뒤 옛 창의 `Destroyed` 정리가 늦게 돌아도, 새 창의 등록은 남는다.
    #[test]
    fn a_late_retire_of_the_old_incarnation_does_not_remove_the_reopened_window() {
        let label = label();
        let old = register(&label);
        let new = register(&label);
        assert_ne!(old, new);
        let retired = retire(&label, &old);
        assert_eq!(
            retired,
            AuthenticatedPrincipal::desktop_window(&label, &old),
            "cleanup still targets the old principal"
        );
        assert_eq!(
            current(&label),
            Some(AuthenticatedPrincipal::desktop_window(&label, &new)),
            "the reopened window keeps its own registration"
        );
        retire(&label, &new);
        assert_eq!(current(&label), None);
    }
}
