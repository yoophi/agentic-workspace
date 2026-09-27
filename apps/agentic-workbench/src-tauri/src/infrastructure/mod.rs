#[allow(unused_imports)]
pub use acp_agent_core::infrastructure::{
    acp, agent_catalog, agent_session_registry, noop_acp_session_store, permission_broker,
};

pub mod acp_agent_launch_factory;
pub mod acp_agent_worker_adapter;
pub mod desktop_benches;
#[cfg(debug_assertions)]
pub mod devtools;
pub mod json_appearance_preferences_repository;
pub mod json_session_window_state_repository;
pub mod json_store;
pub mod json_worktree_workspace_layout_repository;
pub mod mcp;
pub mod native_window_menu;
pub mod perf_log;
pub mod tauri_desktop_bridge;
pub mod tauri_orchestration_event_sink;
pub mod window_manager;
// 041: core로 이동. compat 전환(US1–US4) 동안 기존 경로를 유지하는 재노출이다.
pub use workbench_core::infrastructure::fs::orchestration_store as json_orchestration_repository;
