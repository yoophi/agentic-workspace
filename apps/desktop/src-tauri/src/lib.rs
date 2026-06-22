mod application;
mod domain;
mod inbound;
mod infrastructure;
mod ports;

use inbound::tauri_commands::{
    cancel_agent_run, create_git_worktree, create_project, delete_git_worktree, delete_project,
    list_agents, list_git_branches, list_git_remotes, list_git_worktrees, list_projects,
    start_agent_run, update_project,
};
use infrastructure::agent_session_registry::AppState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

            Ok(())
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
            start_agent_run,
            cancel_agent_run
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
