//! 창 수명(043). 창을 만들 때 incarnation을 등록하고([`window_principals::register`]), 그 창에만 붙인 `Destroyed` 처리기가
//! 자기 incarnation을 붙잡아 정리한다: 주체 거둬들이기 → 그 주체의 토큰·이벤트 표 폐기 → 네트워크 전달 선언 해제 →
//! (session 창) 작업대 닫기. 정리는 전역 창 이벤트가 아니라 창별 처리기가 하므로, 같은 label로 다시 만든 창이 옛 창의
//! 늦은 정리에 휘말리지 않는다(`window_principals` 참조).

use std::sync::Arc;

use tauri::{AppHandle, Manager, WebviewWindow, WindowEvent};
use workbench_core::application::workbench_runtime::WorkbenchRuntime;
use workbench_protocol::AuthenticatedPrincipal;

use crate::infrastructure::{
    desktop_benches, tauri_desktop_bridge, window_principals, workbench_http::WorkbenchHttp,
};

/// 등록 → 창 만들기 → 창별 정리 처리기. 만들기가 실패하면 방금 발급한 incarnation을 거둬들인다.
pub fn build_tracked(
    app: &AppHandle,
    label: &str,
    build: impl FnOnce() -> Result<WebviewWindow, String>,
) -> Result<WebviewWindow, String> {
    let incarnation = window_principals::register(label);
    match build() {
        Ok(window) => {
            track(app, &window, incarnation);
            Ok(window)
        }
        Err(error) => {
            window_principals::retire(label, &incarnation);
            Err(error)
        }
    }
}

/// 설정 파일로 이미 만들어진 창(`main`): setup에서 등록하고 처리기를 붙인다. 이벤트 루프가 돌기 전이라 그 창의
/// 페이지는 아직 호출을 보내지 않았다.
pub fn adopt(app: &AppHandle, window: &WebviewWindow) {
    let incarnation = window_principals::register(window.label());
    track(app, window, incarnation);
}

fn track(app: &AppHandle, window: &WebviewWindow, incarnation: String) {
    let app = app.clone();
    let label = window.label().to_owned();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            let principal = window_principals::retire(&label, &incarnation);
            on_destroyed(&app, &label, &incarnation, principal);
        }
    });
}

/// 창 정리의 부작용(시험은 기록용 구현을 넣는다).
pub trait Teardown {
    fn revoke_credentials(&self, principal: &AuthenticatedPrincipal);
    fn forget_network_delivery(&self, label: &str, incarnation: &str);
    fn close_bench(&self, label: &str, principal: AuthenticatedPrincipal);
}

/// 창 `Destroyed`의 정리 순서(043 T050): 주체 거둬들이기(호출자가 이미 함) → 토큰·표 폐기(닫힌 창 자격 증명으로 더는
/// 호출·구독 못 함) → 네트워크 전달 선언 해제 → (session 창) 작업대 닫기(연 창 주체로, 소유 run 취소).
pub fn teardown(
    label: &str,
    incarnation: &str,
    principal: AuthenticatedPrincipal,
    ops: &dyn Teardown,
) {
    ops.revoke_credentials(&principal);
    ops.forget_network_delivery(label, incarnation);
    if label.starts_with("session-") {
        ops.close_bench(label, principal);
    }
}

struct AppTeardown<'a> {
    app: &'a AppHandle,
}

impl Teardown for AppTeardown<'_> {
    fn revoke_credentials(&self, principal: &AuthenticatedPrincipal) {
        if let Some(http) = self.app.try_state::<WorkbenchHttp>() {
            if let Some(state) = http.state.as_ref() {
                state.revoke_window(&principal.subject);
            }
        }
    }

    fn forget_network_delivery(&self, label: &str, incarnation: &str) {
        tauri_desktop_bridge::forget_network_delivery(label, incarnation);
    }

    fn close_bench(&self, label: &str, principal: AuthenticatedPrincipal) {
        let runtime = self.app.state::<Arc<WorkbenchRuntime>>().inner().clone();
        let label = label.to_owned();
        tauri::async_runtime::spawn(async move {
            // 040: 창 닫힘 = 작업대 명시적 닫기(소유 run 취소·교환 작업 영역 삭제, ADR 0005). 작업대를 연 창 주체로 닫는다.
            // 041: 묶인 orchestration 작업 영역은 작업대 닫기 hook이 복구 가능으로 바꾼다(core).
            desktop_benches::close(&runtime, &label, principal).await;
        });
    }
}

fn on_destroyed(
    app: &AppHandle,
    label: &str,
    incarnation: &str,
    principal: AuthenticatedPrincipal,
) {
    teardown(label, incarnation, principal, &AppTeardown { app });
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct Recording(Mutex<Vec<String>>);

    impl Teardown for Recording {
        fn revoke_credentials(&self, principal: &AuthenticatedPrincipal) {
            self.0
                .lock()
                .unwrap()
                .push(format!("revoke {}", principal.subject));
        }
        fn forget_network_delivery(&self, label: &str, incarnation: &str) {
            self.0
                .lock()
                .unwrap()
                .push(format!("forget {label}:{incarnation}"));
        }
        fn close_bench(&self, label: &str, principal: AuthenticatedPrincipal) {
            self.0
                .lock()
                .unwrap()
                .push(format!("close {label} as {}", principal.subject));
        }
    }

    #[test]
    fn a_session_window_revokes_then_forgets_delivery_then_closes_its_bench_as_its_principal() {
        let ops = Recording::default();
        let principal = AuthenticatedPrincipal::desktop_window("session-1", "inc-1");
        teardown("session-1", "inc-1", principal, &ops);
        assert_eq!(
            *ops.0.lock().unwrap(),
            vec![
                "revoke desktop:window:session-1:inc-1".to_owned(),
                "forget session-1:inc-1".to_owned(),
                "close session-1 as desktop:window:session-1:inc-1".to_owned(),
            ]
        );
    }

    #[test]
    fn other_windows_revoke_and_forget_but_have_no_bench() {
        let ops = Recording::default();
        teardown(
            "settings",
            "inc-9",
            AuthenticatedPrincipal::desktop_window("settings", "inc-9"),
            &ops,
        );
        assert_eq!(
            *ops.0.lock().unwrap(),
            vec![
                "revoke desktop:window:settings:inc-9".to_owned(),
                "forget settings:inc-9".to_owned()
            ]
        );
    }
}
