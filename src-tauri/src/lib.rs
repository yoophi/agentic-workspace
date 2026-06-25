mod adapters;
mod application;
mod domain;

use adapters::inbound::tauri_commands::{
    app_info, create_repository, get_commit_detail, get_file_diff, list_branches, list_history,
    list_repositories, list_worktrees,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            app_info,
            list_repositories,
            create_repository,
            list_worktrees,
            list_branches,
            list_history,
            get_commit_detail,
            get_file_diff
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
