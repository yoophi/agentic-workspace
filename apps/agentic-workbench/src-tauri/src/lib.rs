mod application;
mod domain;
mod inbound;
mod infrastructure;
mod ports;

use inbound::tauri_commands::{
    WorktreeWatcherState, cancel_agent_run, cancel_current_prompt_and_send_to_run, clear_goal,
    create_git_worktree, create_goal, create_project, create_saved_prompt, delete_git_worktree,
    delete_project, delete_saved_prompt, get_agent_run_settings, get_goal, get_worktree_changes, get_worktree_workspace_layout,
    get_worktree_commit_detail, get_worktree_commit_file_diff, get_worktree_file_diff,
    get_worktree_git_graph, list_agent_tool_command_candidates, list_agents, list_git_branches,
    list_git_remotes, list_git_worktrees, list_projects, list_provider_sessions,
    list_saved_prompts, list_worktree_changes, list_worktree_files, list_worktree_git_history,
    open_external_url, open_settings_window, open_worktree_window, read_worktree_text_file,
    record_goal_progress, respond_agent_permission, save_agent_run_settings, save_worktree_workspace_layout, send_prompt_to_run,
    set_run_permission_mode, start_agent_run, start_worktree_watcher, steer_prompt_to_run,
    stop_worktree_watcher, update_goal, update_project, update_saved_prompt,
};
use infrastructure::{agent_session_registry::AppState, mcp::McpServerState};
use tauri::{
    Manager, WindowEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

const ABOUT_MENU_ID: &str = "about-agentic-workbench";
const PREFERENCES_MENU_ID: &str = "preferences-agentic-workbench";
const PREFERENCES_ACCELERATOR: &str = "CmdOrCtrl+,";
const APP_DISPLAY_NAME: &str = "Agentic Workbench";
const APP_VERSION: &str = env!("AGENTIC_WORKBENCH_PACKAGE_VERSION");
const BUILD_COMMIT_HASH: &str = env!("AGENTIC_WORKBENCH_GIT_COMMIT_HASH");
const BUILD_COMMIT_TAG: &str = env!("AGENTIC_WORKBENCH_GIT_COMMIT_TAG");
const BUILD_COMMIT_FALLBACK: &str = "unknown";

pub fn run() {
    let app_state = AppState::default();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .menu(build_native_menu)
        .on_menu_event(|app, event| {
            if event.id() == ABOUT_MENU_ID {
                show_about_dialog(app);
            } else if event.id() == PREFERENCES_MENU_ID {
                if let Err(error) = infrastructure::window_manager::open_settings_window(app) {
                    show_error_dialog(app, "Could not open Settings", &error);
                }
            } else if let Ok(true) =
                infrastructure::native_window_menu::focus_window_from_menu_event(
                    app,
                    event.id().as_ref(),
                )
            {
                // Window focus menu events are handled by the native menu adapter.
            }
        })
        .setup(|_app| {
            let mcp_state = McpServerState::start(
                _app.handle().clone(),
                _app.state::<AppState>().inner().clone(),
            )?;
            _app.manage(mcp_state);

            #[cfg(debug_assertions)]
            {
                if infrastructure::devtools::should_open_devtools()
                    && let Some(window) = _app.get_webview_window("main")
                {
                    window.open_devtools();
                }
            }

            let _ = infrastructure::native_window_menu::sync_window_menu(_app.handle());

            Ok(())
        })
        .on_window_event(|window, event| {
            // 세션 창이 닫히면 그 창이 소유한 진행 중 run을 모두 취소한다.
            if matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_) | WindowEvent::CloseRequested { .. }) {
                infrastructure::window_manager::save_session_window_bounds(window);
            }
            if let WindowEvent::Destroyed = event {
                let label = window.label().to_string();
                if label.starts_with("session-") {
                    let state = window.state::<AppState>().inner().clone();
                    let watcher_state = window.state::<WorktreeWatcherState>();
                    let _ = watcher_state.stop_for_window(&label);
                    tauri::async_runtime::spawn(async move {
                        state.cancel_runs_owned_by(&label).await;
                    });
                }
                let _ = infrastructure::native_window_menu::sync_window_menu(window.app_handle());
            }
        })
        .manage(app_state)
        .manage(WorktreeWatcherState::new())
        .invoke_handler(tauri::generate_handler![
            list_projects,
            create_project,
            update_project,
            delete_project,
            list_saved_prompts,
            create_saved_prompt,
            update_saved_prompt,
            delete_saved_prompt,
            get_goal,
            create_goal,
            update_goal,
            clear_goal,
            record_goal_progress,
            get_agent_run_settings,
            save_agent_run_settings,
            get_worktree_workspace_layout,
            save_worktree_workspace_layout,
            list_git_remotes,
            list_git_branches,
            list_git_worktrees,
            list_worktree_changes,
            create_git_worktree,
            delete_git_worktree,
            get_worktree_changes,
            get_worktree_file_diff,
            list_worktree_files,
            read_worktree_text_file,
            start_worktree_watcher,
            stop_worktree_watcher,
            list_worktree_git_history,
            get_worktree_git_graph,
            get_worktree_commit_detail,
            get_worktree_commit_file_diff,
            list_agents,
            list_agent_tool_command_candidates,
            list_provider_sessions,
            open_external_url,
            open_worktree_window,
            open_settings_window,
            start_agent_run,
            cancel_agent_run,
            send_prompt_to_run,
            steer_prompt_to_run,
            cancel_current_prompt_and_send_to_run,
            set_run_permission_mode,
            respond_agent_permission
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_native_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let about_item = MenuItem::with_id(
        app,
        ABOUT_MENU_ID,
        format!("About {APP_DISPLAY_NAME}"),
        true,
        None::<&str>,
    )?;
    let preferences_item = MenuItem::with_id(
        app,
        preferences_menu_id(),
        "Preferences...",
        true,
        Some(preferences_accelerator()),
    )?;
    let window_menu = Submenu::with_id_and_items(
        app,
        "Window",
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            #[cfg(target_os = "macos")]
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    let help_menu = Submenu::with_id_and_items(
        app,
        "Help",
        "Help",
        true,
        &[
            #[cfg(not(target_os = "macos"))]
            &about_item,
        ],
    )?;

    Menu::with_items(
        app,
        &[
            #[cfg(target_os = "macos")]
            &Submenu::with_items(
                app,
                APP_DISPLAY_NAME,
                true,
                &[
                    &about_item,
                    &PredefinedMenuItem::separator(app)?,
                    &preferences_item,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, None)?,
                    &PredefinedMenuItem::hide_others(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?,
            #[cfg(not(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            )))]
            &Submenu::with_items(
                app,
                "File",
                true,
                &[
                    &PredefinedMenuItem::close_window(app, None)?,
                    #[cfg(not(target_os = "macos"))]
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?,
            &Submenu::with_items(
                app,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(app, None)?,
                    &PredefinedMenuItem::redo(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, None)?,
                    &PredefinedMenuItem::copy(app, None)?,
                    &PredefinedMenuItem::paste(app, None)?,
                    &PredefinedMenuItem::select_all(app, None)?,
                ],
            )?,
            #[cfg(target_os = "macos")]
            &Submenu::with_items(
                app,
                "View",
                true,
                &[&PredefinedMenuItem::fullscreen(app, None)?],
            )?,
            &window_menu,
            &help_menu,
        ],
    )
}

fn show_about_dialog<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    app.dialog()
        .message(format!(
            "{APP_DISPLAY_NAME}\n\nVersion: {APP_VERSION}\nCommit: {}\nTag: {}",
            display_commit_hash(),
            display_commit_tag()
        ))
        .title(format!("About {APP_DISPLAY_NAME}"))
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::Ok)
        .show(|_| {});
}

fn show_error_dialog<R: tauri::Runtime>(app: &tauri::AppHandle<R>, title: &str, message: &str) {
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::Ok)
        .show(|_| {});
}

fn display_commit_hash() -> &'static str {
    if BUILD_COMMIT_HASH.trim().is_empty() {
        BUILD_COMMIT_FALLBACK
    } else {
        BUILD_COMMIT_HASH
    }
}

fn display_commit_tag() -> &'static str {
    if BUILD_COMMIT_TAG.trim().is_empty() {
        BUILD_COMMIT_FALLBACK
    } else {
        BUILD_COMMIT_TAG
    }
}

fn preferences_menu_id() -> &'static str {
    PREFERENCES_MENU_ID
}

fn preferences_accelerator() -> &'static str {
    PREFERENCES_ACCELERATOR
}

#[cfg(test)]
mod tests {
    use super::{preferences_accelerator, preferences_menu_id};

    #[test]
    fn preferences_menu_uses_stable_id() {
        assert_eq!(preferences_menu_id(), "preferences-agentic-workbench");
    }

    #[test]
    fn preferences_menu_uses_standard_macos_accelerator() {
        assert_eq!(preferences_accelerator(), "CmdOrCtrl+,");
    }
}
