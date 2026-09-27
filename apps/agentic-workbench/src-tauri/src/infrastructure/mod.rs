#[allow(unused_imports)]
pub use acp_agent_core::infrastructure::{
    acp, agent_catalog, agent_session_registry, noop_acp_session_store, permission_broker,
};

pub mod acp_agent_launch_factory;
pub mod acp_agent_worker_adapter;
#[cfg(debug_assertions)]
pub mod devtools;
pub mod in_memory_agent_workspace_registry;
pub mod json_appearance_preferences_repository;
pub mod json_orchestration_repository;
pub mod json_session_window_state_repository;
pub mod json_store;
pub mod json_worktree_workspace_layout_repository;
pub mod mcp;
pub mod native_window_menu;
pub mod perf_log;
pub mod tauri_orchestration_event_sink;
pub mod tauri_run_event_sink;
pub mod window_manager;

/// 040 과도기: 세션 저장소는 core로 옮겼다(`workbench_core::infrastructure::fs::acp_session_store`). US1에서 런타임의
/// 단일 인스턴스를 쓰도록 바뀐다.
pub fn acp_session_store_for(
    app: &tauri::AppHandle,
) -> Result<workbench_core::infrastructure::fs::acp_session_store::JsonAcpSessionStore, String> {
    use tauri::Manager;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;
    Ok(
        workbench_core::infrastructure::fs::acp_session_store::JsonAcpSessionStore::new(
            dir.join("acp-sessions.json"),
        ),
    )
}
