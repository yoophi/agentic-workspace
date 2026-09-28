#[allow(unused_imports)]
pub use acp_agent_core::infrastructure::{
    acp, agent_catalog, agent_session_registry, noop_acp_session_store, permission_broker,
};

pub mod desktop_benches;
// 044 T014–T016: MCP 서버와 run 시작 MCP 주입은 `workbench-host`로 옮겼다(`workbench_host::{mcp, launch}`).
pub use workbench_host::mcp;
#[cfg(debug_assertions)]
pub mod devtools;
#[cfg(debug_assertions)]
pub mod http_probe;
pub mod json_appearance_preferences_repository;
pub mod json_session_window_state_repository;
pub mod json_store;
pub mod json_worktree_workspace_layout_repository;
pub mod native_window_menu;
pub mod perf_log;
pub mod server_client;
pub mod tauri_desktop_bridge;
pub mod window_lifecycle;
pub mod window_manager;
pub mod window_principals;
pub mod workbench_http;
pub mod workbench_mode;
