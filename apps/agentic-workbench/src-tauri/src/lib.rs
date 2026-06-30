mod application;
mod domain;
mod inbound;
mod infrastructure;
mod ports;

use inbound::tauri_commands::{
    WorktreeWatcherState, cancel_agent_run, clear_goal, create_git_worktree, create_goal,
    create_project, create_saved_prompt, delete_git_worktree, delete_project, delete_saved_prompt,
    get_agent_run_settings, get_goal, get_worktree_changes, get_worktree_commit_detail,
    get_worktree_commit_file_diff, get_worktree_file_diff, get_worktree_git_graph, list_agents,
    list_git_branches, list_git_remotes, list_git_worktrees, list_projects, list_provider_sessions,
    list_saved_prompts, list_worktree_changes, list_worktree_files, list_worktree_git_history,
    open_external_url, open_worktree_window, read_worktree_text_file, record_goal_progress,
    respond_agent_permission, save_agent_run_settings, send_prompt_to_run, set_run_permission_mode,
    start_agent_run, start_worktree_watcher, stop_worktree_watcher, update_goal, update_project,
    update_saved_prompt,
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
                    let watcher_state = window.state::<WorktreeWatcherState>();
                    let _ = watcher_state.stop_for_window(&label);
                    tauri::async_runtime::spawn(async move {
                        state.cancel_runs_owned_by(&label).await;
                    });
                }
            }
        })
        .manage(AppState::default())
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
            list_provider_sessions,
            open_external_url,
            open_worktree_window,
            start_agent_run,
            cancel_agent_run,
            send_prompt_to_run,
            set_run_permission_mode,
            respond_agent_permission
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
