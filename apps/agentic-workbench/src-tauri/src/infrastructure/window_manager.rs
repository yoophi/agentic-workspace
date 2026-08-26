use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock, mpsc},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window};
use uuid::Uuid;

use crate::{
    application::session_window_state_service,
    domain::session_window_state::{VisibleArea, WindowBounds},
    infrastructure::json_session_window_state_repository::JsonSessionWindowStateRepository,
};

#[cfg(debug_assertions)]
use crate::infrastructure::devtools;

/// macOS 네이티브 탭 그룹 식별자. 같은 식별자의 세션 창끼리 탭으로 묶인다.
const TABBING_IDENTIFIER: &str = "acp-session";
pub const SETTINGS_WINDOW_LABEL: &str = "settings";
const SETTINGS_WINDOW_TITLE: &str = "Settings";
const SETTINGS_WINDOW_ROUTE: &str = "/settings-window";

const SESSION_WINDOW_DEFAULT_WIDTH: u32 = 1100;
const SESSION_WINDOW_DEFAULT_HEIGHT: u32 = 820;
const SESSION_WINDOW_MINIMUM_WIDTH: u32 = 980;
const SESSION_WINDOW_MINIMUM_HEIGHT: u32 = 680;
/// 이동·리사이즈 이벤트는 드래그 중 연속으로 들어온다. 매 이벤트마다 JSON 저장소를 다시 쓰면
/// 메인 스레드에서 디스크 쓰기가 폭주하므로, 최소 간격을 두고 저장한다. 창을 닫을 때는
/// 간격과 무관하게 마지막 값을 반드시 기록한다.
const BOUNDS_SAVE_INTERVAL: Duration = Duration::from_millis(700);

pub fn session_label(session_id: &str) -> String {
    format!("session-{session_id}")
}

/// 세션 창 label과 Worktree 경로의 대응. 창 이벤트에는 Worktree 경로가 없고, 세션 URL은
/// HashRouter라서 `worktreePath`가 fragment 안에 들어가 `Url::query_pairs()`로는 읽히지 않는다.
/// 그래서 창을 만든 window manager가 경로를 직접 기억한다.
fn session_worktree_paths() -> &'static Mutex<HashMap<String, String>> {
    static PATHS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    PATHS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn last_bounds_save() -> &'static Mutex<HashMap<String, Instant>> {
    static SAVED_AT: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
    SAVED_AT.get_or_init(|| Mutex::new(HashMap::new()))
}

fn remember_session_worktree_path(label: &str, worktree_path: &str) {
    if let Ok(mut paths) = session_worktree_paths().lock() {
        paths.insert(label.to_string(), worktree_path.to_string());
    }
}

fn session_worktree_path(label: &str) -> Option<String> {
    session_worktree_paths()
        .lock()
        .ok()
        .and_then(|paths| paths.get(label).cloned())
}

pub fn forget_session_window(label: &str) {
    if let Ok(mut paths) = session_worktree_paths().lock() {
        paths.remove(label);
    }
    if let Ok(mut saved_at) = last_bounds_save().lock() {
        saved_at.remove(label);
    }
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
    build_window(app, &label, project_id, worktree_path, &title).map(|_| {
        let _ = crate::infrastructure::native_window_menu::sync_window_menu(app);
    })
}

pub fn open_settings_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        if window.is_minimized().map_err(|error| error.to_string())? {
            window.unminimize().map_err(|error| error.to_string())?;
        }
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        let _ = crate::infrastructure::native_window_menu::sync_window_menu(app);
        return Ok(());
    }

    WebviewWindowBuilder::new(app, SETTINGS_WINDOW_LABEL, settings_url())
        .title(SETTINGS_WINDOW_TITLE)
        .inner_size(920.0, 760.0)
        .min_inner_size(760.0, 560.0)
        .build()
        .map_err(|error| error.to_string())?;

    let _ = crate::infrastructure::native_window_menu::sync_window_menu(app);

    Ok(())
}

pub fn focus_window_by_label<R: Runtime>(app: &AppHandle<R>, label: &str) -> Result<(), String> {
    let Some(window) = app.get_webview_window(label) else {
        return Ok(());
    };

    if window.is_minimized().map_err(|error| error.to_string())? {
        window.unminimize().map_err(|error| error.to_string())?;
    }
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;

    Ok(())
}

fn build_window(
    app: &AppHandle,
    label: &str,
    project_id: &str,
    worktree_path: &str,
    title: &str,
) -> Result<WebviewWindow, String> {
    // 창 이벤트에서 Worktree 경로를 되찾을 수 있도록 생성 시점에 기억한다.
    remember_session_worktree_path(label, worktree_path);

    let restored = restore_session_bounds(app, worktree_path);
    let (width, height) = restored
        .map(|bounds| (bounds.width, bounds.height))
        .unwrap_or((SESSION_WINDOW_DEFAULT_WIDTH, SESSION_WINDOW_DEFAULT_HEIGHT));

    #[allow(unused_mut)]
    let mut builder = WebviewWindowBuilder::new(app, label, session_url(project_id, worktree_path))
        .title(title)
        .inner_size(width as f64, height as f64)
        .min_inner_size(
            SESSION_WINDOW_MINIMUM_WIDTH as f64,
            SESSION_WINDOW_MINIMUM_HEIGHT as f64,
        );
    if let Some(bounds) = restored {
        builder = builder.position(bounds.x as f64, bounds.y as f64);
    }

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

/// 저장된 창 상태를 현재 모니터 구성에 맞게 보정해 돌려준다. 저장 값이 없거나 화면 정보를
/// 읽을 수 없으면 기본 크기로 열도록 `None`을 준다.
fn restore_session_bounds(app: &AppHandle, worktree_path: &str) -> Option<WindowBounds> {
    let repository = JsonSessionWindowStateRepository::from_app(app)
        .inspect_err(|error| eprintln!("session window state store unavailable: {error}"))
        .ok()?;
    let saved = session_window_state_service::get_bounds(&repository, worktree_path)
        .inspect_err(|error| eprintln!("failed to read session window state: {error}"))
        .ok()??;

    let areas = visible_areas(app);
    if areas.is_empty() {
        return Some(saved);
    }
    session_window_state_service::fit_bounds_to_visible_areas(
        saved,
        &areas,
        SESSION_WINDOW_MINIMUM_WIDTH,
        SESSION_WINDOW_MINIMUM_HEIGHT,
    )
}

/// 모니터 정보는 물리 픽셀로 오므로 각 모니터의 scale factor로 논리 좌표로 바꾼다.
/// 저장 값과 최소 크기 상수가 모두 논리 단위이므로 같은 단위로 비교해야 한다.
fn visible_areas(app: &AppHandle) -> Vec<VisibleArea> {
    app.available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|monitor| {
            let scale = monitor.scale_factor();
            let position = monitor.position().to_logical::<i32>(scale);
            let size = monitor.size().to_logical::<u32>(scale);
            VisibleArea {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
            }
        })
        .collect()
}

/// 세션 창의 현재 위치·내부 크기를 Worktree별로 저장한다.
///
/// `flush`가 false면 최소 간격 안에 들어온 연속 이벤트를 건너뛴다. 창을 닫을 때는 true로
/// 호출해 마지막 값을 반드시 남긴다.
pub fn save_session_window_bounds(window: &Window, flush: bool) {
    let label = window.label().to_string();
    if !label.starts_with("session-") {
        return;
    }
    let Some(worktree_path) = session_worktree_path(&label) else {
        return;
    };
    if !flush && !should_save_now(&label) {
        return;
    }

    let (Ok(position), Ok(size), Ok(scale)) = (
        window.outer_position(),
        window.inner_size(),
        window.scale_factor(),
    ) else {
        return;
    };
    // 최소화된 창은 위치·크기가 실제 사용 값이 아니므로 저장하지 않는다.
    if window.is_minimized().unwrap_or(false) || size.width == 0 || size.height == 0 {
        return;
    }

    let Ok(repository) = JsonSessionWindowStateRepository::from_app(window.app_handle()) else {
        return;
    };
    // `outer_position`·`inner_size`는 물리 픽셀을 주지만 창을 만들 때 쓰는
    // `WebviewWindowBuilder::position`·`inner_size`는 논리 단위를 받는다. 그대로 저장하면
    // Retina(2x) 화면에서 다음 실행 때 창이 두 배 크기·위치로 열린다. 논리 단위로 바꿔 저장한다.
    let position = position.to_logical::<i32>(scale);
    let size = size.to_logical::<u32>(scale);
    let bounds = WindowBounds {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    };
    if let Err(error) =
        session_window_state_service::save_bounds(&repository, &worktree_path, bounds)
    {
        eprintln!("failed to save session window state: {error}");
        return;
    }
    if let Ok(mut saved_at) = last_bounds_save().lock() {
        saved_at.insert(label, Instant::now());
    }
}

fn should_save_now(label: &str) -> bool {
    let Ok(saved_at) = last_bounds_save().lock() else {
        return false;
    };
    saved_at
        .get(label)
        .is_none_or(|last| last.elapsed() >= BOUNDS_SAVE_INTERVAL)
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
                if let Some(base) = base_window
                    && let (Ok(base_ptr), Ok(new_ptr)) = (base.ns_window(), new_win.ns_window())
                {
                    let base_ns: &NSWindow = unsafe { &*base_ptr.cast::<NSWindow>() };
                    let new_ns: &NSWindow = unsafe { &*new_ptr.cast::<NSWindow>() };
                    base_ns.addTabbedWindow_ordered(new_ns, NSWindowOrderingMode::Above);
                }
                let _ = crate::infrastructure::native_window_menu::sync_window_menu(&app);
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
        .rfind(|part| !part.is_empty())
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

    use super::{
        SETTINGS_WINDOW_LABEL, forget_session_window, remember_session_worktree_path,
        session_label, session_route, session_title, session_url, session_worktree_path,
        settings_url,
    };
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

    /// 세션 URL은 HashRouter라서 `worktreePath`가 fragment 안에 들어간다. 따라서 창 이벤트에서
    /// `Url::query_pairs()`로 Worktree 경로를 되찾으려 하면 항상 빈 결과가 나온다.
    /// 창 상태 저장이 이 방식으로 되돌아가지 않도록 고정한다.
    #[test]
    fn worktree_path_is_not_readable_from_the_url_query() {
        let WebviewUrl::App(path) = session_url("project-1", "/repo/tree-a") else {
            panic!("session window must use an app URL");
        };
        let raw = format!("http://tauri.localhost/{}", path.to_string_lossy());
        let url = tauri::Url::parse(&raw).expect("parse session url");

        assert_eq!(url.query(), None);
        assert!(url.fragment().expect("fragment").contains("worktreePath="));
        assert!(
            url.query_pairs()
                .find(|(key, _)| key == "worktreePath")
                .is_none()
        );
    }

    #[test]
    fn window_manager_remembers_worktree_path_per_session_label() {
        let label = "session-test-remember";
        remember_session_worktree_path(label, "/repo/tree-a");

        assert_eq!(
            session_worktree_path(label).as_deref(),
            Some("/repo/tree-a")
        );

        forget_session_window(label);
        assert_eq!(session_worktree_path(label), None);
    }

    #[test]
    fn separate_session_windows_keep_separate_worktree_paths() {
        remember_session_worktree_path("session-test-a", "/repo/tree-a");
        remember_session_worktree_path("session-test-b", "/repo/tree-b");

        assert_eq!(
            session_worktree_path("session-test-a").as_deref(),
            Some("/repo/tree-a")
        );
        assert_eq!(
            session_worktree_path("session-test-b").as_deref(),
            Some("/repo/tree-b")
        );

        forget_session_window("session-test-a");
        forget_session_window("session-test-b");
    }
}
