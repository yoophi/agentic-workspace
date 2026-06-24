use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use sha2::{Digest, Sha256};
use std::{sync::mpsc, time::Duration};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

#[cfg(debug_assertions)]
use crate::infrastructure::devtools;

/// macOS 네이티브 탭 그룹 식별자. 같은 식별자의 세션 창끼리 탭으로 묶인다.
const TABBING_IDENTIFIER: &str = "acp-session";

pub fn session_label(project_id: &str, worktree_path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_id.as_bytes());
    hasher.update([0]);
    hasher.update(worktree_path.as_bytes());
    let digest = hasher.finalize();
    let hash_prefix = digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("session-{hash_prefix}")
}

fn focus_if_open(app: &AppHandle, label: &str) -> bool {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.set_focus();
        return true;
    }
    false
}

/// 세션 창이 로드할 URL. HashRouter이므로 `#` 뒤가 라우트가 된다.
/// worktree path는 route segment가 아니라 query string에 넣어 `/`, `#`, `%` 같은
/// 경로 문자가 router matching에 영향을 주지 않도록 한다.
fn session_url(project_id: &str, worktree_path: &str) -> WebviewUrl {
    WebviewUrl::App(format!("index.html#{}", session_route(project_id, worktree_path)).into())
}

fn session_route(project_id: &str, worktree_path: &str) -> String {
    let encoded_path = utf8_percent_encode(worktree_path, NON_ALPHANUMERIC).to_string();
    format!("/session/{project_id}?worktreePath={encoded_path}")
}

pub fn open_session_window(
    app: &AppHandle,
    project_id: &str,
    worktree_path: &str,
    mode: &str,
) -> Result<(), String> {
    let label = session_label(project_id, worktree_path);
    if focus_if_open(app, &label) {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    if mode == "tab" {
        return open_as_tab(
            app,
            label,
            project_id.to_string(),
            worktree_path.to_string(),
        );
    }

    let _ = mode;
    build_window(app, &label, project_id, worktree_path).map(|_| ())
}

fn build_window(
    app: &AppHandle,
    label: &str,
    project_id: &str,
    worktree_path: &str,
) -> Result<WebviewWindow, String> {
    #[allow(unused_mut)]
    let mut builder = WebviewWindowBuilder::new(app, label, session_url(project_id, worktree_path))
        .title("ACP Worktree Session")
        .inner_size(1100.0, 820.0)
        .min_inner_size(980.0, 680.0);

    #[cfg(target_os = "macos")]
    {
        builder = builder.tabbing_identifier(TABBING_IDENTIFIER);
    }

    let window = builder.build().map_err(|error| error.to_string())?;

    #[cfg(debug_assertions)]
    if devtools::should_open_devtools() {
        window.open_devtools();
    }

    Ok(window)
}

#[cfg(target_os = "macos")]
fn open_as_tab(
    app: &AppHandle,
    label: String,
    project_id: String,
    worktree_path: String,
) -> Result<(), String> {
    use objc2_app_kit::{NSWindow, NSWindowOrderingMode};

    let app = app.clone();
    let (sender, receiver) = mpsc::channel();
    app.clone()
        .run_on_main_thread(move || {
            let result = (|| {
                // 새 창을 만들기 전에 기준이 될 기존 세션 창을 먼저 찾는다.
                let base_window = app
                    .webview_windows()
                    .into_iter()
                    .find(|(existing_label, _)| existing_label.starts_with("session-"))
                    .map(|(_, window)| window);

                let new_win = build_window(&app, &label, &project_id, &worktree_path)?;

                // 기존 세션 창이 있으면 그 창에 탭으로 합친다. 없으면 새 창 그대로(첫 탭 그룹).
                if let Some(base) = base_window {
                    if let (Ok(base_ptr), Ok(new_ptr)) = (base.ns_window(), new_win.ns_window()) {
                        let base_ns: &NSWindow = unsafe { &*base_ptr.cast::<NSWindow>() };
                        let new_ns: &NSWindow = unsafe { &*new_ptr.cast::<NSWindow>() };
                        base_ns.addTabbedWindow_ordered(new_ns, NSWindowOrderingMode::Above);
                    }
                }
                Ok(())
            })();
            let _ = sender.send(result);
        })
        .map_err(|err| err.to_string())?;

    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "timed out while creating session tab".to_string())?
}

#[cfg(test)]
mod tests {
    use super::{session_label, session_route};

    #[test]
    fn session_route_keeps_worktree_path_out_of_route_segments() {
        let route = session_route("project-1", "/tmp/작업 tree/a#b%c");

        assert_eq!(
            route,
            "/session/project-1?worktreePath=%2Ftmp%2F%EC%9E%91%EC%97%85%20tree%2Fa%23b%25c"
        );
    }

    #[test]
    fn session_label_is_stable_and_route_safe() {
        let first = session_label("project-1", "/tmp/worktree");
        let second = session_label("project-1", "/tmp/worktree");

        assert_eq!(first, second);
        assert!(first.starts_with("session-"));
        assert!(
            first
                .chars()
                .all(|char| char.is_ascii_alphanumeric() || char == '-')
        );
    }
}
