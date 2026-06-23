use std::hash::{Hash, Hasher};

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// macOS 네이티브 탭 그룹 식별자. 같은 식별자의 세션 창끼리 탭으로 묶인다.
const TABBING_IDENTIFIER: &str = "acp-session";

/// worktree 세션 창의 결정적 레이블. project_id + worktree_path를 해시한다.
/// 윈도우 레이블은 `[a-zA-Z0-9-/:_]`만 허용하므로 경로를 직접 쓰지 않고 해시하며,
/// 결정적이므로 같은 worktree는 항상 같은 레이블 → 중복 창 생성을 막는다.
pub fn session_label(project_id: &str, worktree_path: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    project_id.hash(&mut hasher);
    worktree_path.hash(&mut hasher);
    format!("session-{}", hasher.finish())
}

/// 해당 레이블의 창이 살아 있으면 포커스하고 `true`를 반환한다.
fn focus_if_open(app: &AppHandle, label: &str) -> bool {
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.set_focus();
        return true;
    }
    false
}

/// 세션 창이 로드할 URL. HashRouter이므로 `#` 뒤가 라우트가 된다.
/// worktree_path는 `decodeURIComponent`로 복원되도록 퍼센트 인코딩한다.
fn session_url(project_id: &str, worktree_path: &str) -> WebviewUrl {
    let encoded_path = utf8_percent_encode(worktree_path, NON_ALPHANUMERIC).to_string();
    WebviewUrl::App(format!("index.html#/session/{project_id}/{encoded_path}").into())
}

/// worktree 세션을 새 창 또는 새 탭으로 연다. 이미 열려 있으면 그 창을 포커스한다.
/// `mode`가 "tab"이면 macOS 네이티브 탭으로 합치고(비-macOS는 새 창으로 fallback),
/// 그 외에는 새 창으로 연다.
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
        open_as_tab(
            app,
            label,
            project_id.to_string(),
            worktree_path.to_string(),
        );
        return Ok(());
    }

    // 비-macOS에서 "tab"은 새 창과 동일하게 동작한다.
    let _ = mode;
    build_window(app, &label, project_id, worktree_path).map(|_| ())
}

/// 세션 창을 빌드한다. macOS에서는 탭 그룹 식별자를 부여한다.
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

    let window = builder.build().map_err(|err| err.to_string())?;

    #[cfg(debug_assertions)]
    window.open_devtools();

    Ok(window)
}

/// macOS: 새 세션 창을 만든 뒤 이미 열려 있는 세션 창에 `addTabbedWindow`로 합친다.
/// 드롭다운은 항상 메인 창에서 눌리므로 keyWindow가 아니라 기존 세션 창을 기준으로 삼는다.
/// 기존 세션 창이 없으면(첫 탭) 그냥 새 창으로 열려 탭 그룹의 시작점이 된다.
/// 탭이 2개 이상이면 탭바는 macOS가 자동으로 표시한다. NSWindow API는 메인 스레드 전용.
#[cfg(target_os = "macos")]
fn open_as_tab(app: &AppHandle, label: String, project_id: String, worktree_path: String) {
    use objc2_app_kit::{NSWindow, NSWindowOrderingMode};

    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        // 새 창을 만들기 전에 기준이 될 기존 세션 창을 먼저 찾는다.
        let base_window = app
            .webview_windows()
            .into_iter()
            .find(|(existing_label, _)| existing_label.starts_with("session-"))
            .map(|(_, window)| window);

        let new_win = match build_window(&app, &label, &project_id, &worktree_path) {
            Ok(window) => window,
            Err(err) => {
                eprintln!("[session-tab] failed to create window: {err}");
                return;
            }
        };

        // 기존 세션 창이 있으면 그 창에 탭으로 합친다. 없으면 새 창 그대로(첫 탭 그룹).
        if let Some(base) = base_window {
            if let (Ok(base_ptr), Ok(new_ptr)) = (base.ns_window(), new_win.ns_window()) {
                let base_ns: &NSWindow = unsafe { &*base_ptr.cast::<NSWindow>() };
                let new_ns: &NSWindow = unsafe { &*new_ptr.cast::<NSWindow>() };
                base_ns.addTabbedWindow_ordered(new_ns, NSWindowOrderingMode::Above);
            }
        }
    });
}
