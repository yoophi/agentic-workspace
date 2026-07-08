use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use std::{sync::mpsc, time::Duration};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use uuid::Uuid;

#[cfg(debug_assertions)]
use crate::infrastructure::devtools;

/// macOS 네이티브 탭 그룹 식별자. 같은 식별자의 세션 창끼리 탭으로 묶인다.
const TABBING_IDENTIFIER: &str = "acp-session";
pub const SETTINGS_WINDOW_LABEL: &str = "settings";
const SETTINGS_WINDOW_TITLE: &str = "Settings";
const SETTINGS_WINDOW_ROUTE: &str = "/settings-window";

pub fn session_label(session_id: &str) -> String {
    format!("session-{session_id}")
}

fn new_session_id() -> String {
    Uuid::new_v4().simple().to_string()
}

/// 세션 창이 로드할 URL. HashRouter이므로 `#` 뒤가 라우트가 된다.
/// worktree path는 route segment가 아니라 query string에 넣어 `/`, `#`, `%` 같은
/// 경로 문자가 router matching에 영향을 주지 않도록 한다.
fn session_url(project_id: &str, worktree_path: &str) -> WebviewUrl {
    WebviewUrl::App(format!("index.html#{}", session_route(project_id, worktree_path)).into())
}

fn settings_url() -> WebviewUrl {
    WebviewUrl::App(format!("index.html#{SETTINGS_WINDOW_ROUTE}").into())
}

fn session_route(project_id: &str, worktree_path: &str) -> String {
    let encoded_path = utf8_percent_encode(worktree_path, NON_ALPHANUMERIC).to_string();
    format!("/session/{project_id}?worktreePath={encoded_path}")
}

pub fn open_session_window(
    app: &AppHandle,
    project_id: &str,
    project_name: &str,
    worktree_path: &str,
    mode: &str,
) -> Result<(), String> {
    let label = session_label(&new_session_id());
    let title = session_title(project_name, worktree_path);

    #[cfg(target_os = "macos")]
    if mode == "tab" {
        return open_as_tab(
            app,
            label,
            project_id.to_string(),
            title,
            worktree_path.to_string(),
        );
    }

    let _ = mode;
    build_window(app, &label, project_id, worktree_path, &title).map(|_| ())
}

pub fn open_settings_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        if window.is_minimized().map_err(|error| error.to_string())? {
            window.unminimize().map_err(|error| error.to_string())?;
        }
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(app, SETTINGS_WINDOW_LABEL, settings_url())
        .title(SETTINGS_WINDOW_TITLE)
        .inner_size(920.0, 760.0)
        .min_inner_size(760.0, 560.0)
        .build()
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn build_window(
    app: &AppHandle,
    label: &str,
    project_id: &str,
    worktree_path: &str,
    title: &str,
) -> Result<WebviewWindow, String> {
    #[allow(unused_mut)]
    let mut builder = WebviewWindowBuilder::new(app, label, session_url(project_id, worktree_path))
        .title(title)
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
    title: String,
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

                let new_win = build_window(&app, &label, &project_id, &worktree_path, &title)?;

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

fn session_title(project_name: &str, worktree_path: &str) -> String {
    format!(
        "{} / {}",
        non_empty_or(project_name, "Project"),
        worktree_name(worktree_path)
    )
}

fn worktree_name(worktree_path: &str) -> String {
    worktree_path
        .trim()
        .replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty())
        .next_back()
        .unwrap_or("worktree")
        .to_string()
}

fn non_empty_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{SETTINGS_WINDOW_LABEL, session_label, session_route, session_title, settings_url};
    use tauri::WebviewUrl;

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
        let first = session_label("session-a");
        let second = session_label("session-a");

        assert_eq!(first, second);
        assert!(first.starts_with("session-"));
        assert!(
            first
                .chars()
                .all(|char| char.is_ascii_alphanumeric() || char == '-')
        );
    }

    #[test]
    fn different_session_ids_allow_multiple_windows_for_same_worktree() {
        assert_ne!(session_label("session-a"), session_label("session-b"));
    }

    #[test]
    fn settings_window_label_is_fixed_and_not_a_session_label() {
        assert_eq!(SETTINGS_WINDOW_LABEL, "settings");
        assert!(!SETTINGS_WINDOW_LABEL.starts_with("session-"));
    }

    #[test]
    fn settings_url_uses_dedicated_hash_route() {
        match settings_url() {
            WebviewUrl::App(path) => assert_eq!(path, Path::new("index.html#/settings-window")),
            WebviewUrl::External(_) => panic!("settings window must use an app URL"),
            _ => panic!("settings window must use an app URL"),
        }
    }

    #[test]
    fn session_title_uses_project_name_and_worktree_basename() {
        assert_eq!(
            session_title(
                "Agentic Workbench",
                "/Users/yoophi/project/worktrees/agentic-workbench/feature-login"
            ),
            "Agentic Workbench / feature-login"
        );
    }
}
