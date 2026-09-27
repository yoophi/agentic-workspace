/// 파일 목록 스캔과 worktree watcher가 공유하는 제외 디렉터리 목록. 038 US2에서 파일 목록 어댑터와 함께
/// `workbench-core`로 옮겼고, watcher는 이 재노출로 같은 단일 소스를 쓴다(specs/007 research R3).
pub use workbench_core::infrastructure::fs::WORKSPACE_EXCLUDED_DIRS;

#[allow(unused_imports)]
pub use acp_agent_core::infrastructure::{
    acp, agent_catalog, agent_session_registry, noop_acp_session_store, permission_broker,
};

pub mod acp_agent_launch_factory;
pub mod acp_agent_worker_adapter;
#[cfg(debug_assertions)]
pub mod devtools;
pub mod fs_worktree_watcher;
pub mod in_memory_agent_workspace_registry;
pub mod json_acp_session_store;
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
