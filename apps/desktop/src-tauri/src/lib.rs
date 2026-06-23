mod application;
mod domain;
mod inbound;
mod infrastructure;
mod ports;

use inbound::tauri_commands::{
    cancel_agent_run, create_git_worktree, create_project, delete_git_worktree, delete_project,
    list_agents, list_git_branches, list_git_remotes, list_git_worktrees, list_projects,
    open_worktree_window, respond_agent_permission, send_prompt_to_run, start_agent_run,
    update_project,
};
use infrastructure::agent_session_registry::AppState;
use tauri::{Manager, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|_app| {
            #[cfg(debug_assertions)]
            {
                if infrastructure::devtools::should_open_devtools()
                    && let Some(window) = _app.get_webview_window("main")
                {
                    window.open_devtools();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // 세션 창이 닫히면 그 창이 소유한 진행 중 run을 모두 취소한다.
            if let WindowEvent::Destroyed = event {
                let label = window.label().to_string();
                if label.starts_with("session-") {
                    let state = window.state::<AppState>().inner().clone();
                    tauri::async_runtime::spawn(async move {
                        state.cancel_runs_owned_by(&label).await;
                    });
                }
            }
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            list_projects,
            create_project,
            update_project,
            delete_project,
            list_git_remotes,
            list_git_branches,
            list_git_worktrees,
            create_git_worktree,
            delete_git_worktree,
            list_agents,
            open_worktree_window,
            start_agent_run,
            send_prompt_to_run,
            cancel_agent_run,
            respond_agent_permission
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
