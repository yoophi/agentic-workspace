mod application;
mod domain;
mod inbound;
mod infrastructure;
mod ports;

use acp_agent_core::infrastructure::agent_session_registry::AppState;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .on_window_event(|window, event| {
            if let WindowEvent::Destroyed = event {
                let owner = window.label().to_string();
                let state = window.state::<AppState>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    state.cancel_runs_owned_by(&owner).await;
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Ask Code");
}
