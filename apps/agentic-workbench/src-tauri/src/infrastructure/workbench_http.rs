//! 042: Workbench HTTP/WS 어댑터의 AW 쪽(044 T014: 조립·토큰 발급기·종료 drain은 `workbench-host::http`로 옮겼다 —
//! 이 모듈은 재노출과, Tauri에 묶인 부분만 둔다: WebView URL → 출처, 창 incarnation으로 토큰을 발급하는 앱 상태).

use std::sync::Arc;

pub use workbench_host::http::*;
use workbench_protocol::AuthenticatedPrincipal;

/// WebView URL → 출처 문자열(`scheme://host[:port]`). 사용자 정의 scheme(`tauri:`)은 표준 origin 직렬화가 `null`이라
/// 직접 만든다.
pub fn origin_of(url: &tauri::Url) -> Option<String> {
    let host = url.host_str()?;
    Some(match url.port() {
        Some(port) => format!("{}://{host}:{port}", url.scheme()),
        None => format!("{}://{host}", url.scheme()),
    })
}

/// 앱이 관리하는 어댑터 상태. 기동 실패면 `state`가 없고(FR-016: 앱은 계속 동작) 이유를 남긴다.
pub struct WorkbenchHttp {
    pub state: Option<Arc<WorkbenchHttpState>>,
    pub start_error: Option<String>,
    pub exit: ExitGate,
}

impl WorkbenchHttp {
    /// 창 `label`의 현재 incarnation 주체로 토큰을 발급한다(043). 등록이 없는 창(닫힘)은 거절한다.
    pub fn connection_for(&self, origin: &str, label: &str) -> Result<WorkbenchConnection, String> {
        let incarnation =
            crate::infrastructure::window_principals::incarnation(label).ok_or_else(|| {
                crate::infrastructure::window_principals::MESSAGE_WINDOW_NOT_REGISTERED.to_owned()
            })?;
        let principal = AuthenticatedPrincipal::desktop_window(label, &incarnation);
        match &self.state {
            Some(state) => state
                .connection_for(origin, principal)
                .map(|mut connection| {
                    connection.incarnation = Some(incarnation);
                    connection
                }),
            None => Err(format!(
                "Workbench HTTP server is not running: {}",
                self.start_error.as_deref().unwrap_or("not started")
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_start_still_answers_with_a_reason() {
        let http = WorkbenchHttp {
            state: None,
            start_error: Some("address in use".into()),
            exit: ExitGate::default(),
        };
        let label = format!("session-test-{}", uuid::Uuid::new_v4());
        let incarnation = crate::infrastructure::window_principals::register(&label);
        let error = http
            .connection_for("tauri://localhost", &label)
            .unwrap_err();
        assert!(error.contains("address in use"), "{error}");
        crate::infrastructure::window_principals::retire(&label, &incarnation);
        // 등록이 없는 창(닫힘)은 토큰을 받지 못한다.
        assert_eq!(
            http.connection_for("tauri://localhost", &label)
                .unwrap_err(),
            crate::infrastructure::window_principals::MESSAGE_WINDOW_NOT_REGISTERED
        );
    }

    #[test]
    fn window_origins_include_custom_schemes() {
        let url = tauri::Url::parse("tauri://localhost/index.html").unwrap();
        assert_eq!(origin_of(&url).as_deref(), Some("tauri://localhost"));
        let url = tauri::Url::parse("http://localhost:1420/#/session").unwrap();
        assert_eq!(origin_of(&url).as_deref(), Some("http://localhost:1420"));
        let url = tauri::Url::parse("http://tauri.localhost/").unwrap();
        assert_eq!(origin_of(&url).as_deref(), Some("http://tauri.localhost"));
    }
}
