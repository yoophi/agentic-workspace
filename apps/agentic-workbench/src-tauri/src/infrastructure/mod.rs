/// 파일 목록 스캔(fs_worktree_file_provider)과 worktree watcher가 공유하는
/// 제외 디렉터리 목록. 화면에 표시되지 않는 디렉터리의 변경이 rescan을
/// 유발하지 않도록 단일 소스로 관리한다(specs/007 research R3).
pub const WORKSPACE_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".next",
    ".turbo",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
];

pub mod acp;
pub mod agent_catalog;
pub mod agent_session_registry;
#[cfg(debug_assertions)]
pub mod devtools;
pub mod fs_provider_session_repository;
pub mod fs_worktree_file_provider;
pub mod fs_worktree_watcher;
pub mod git_cli_branch_provider;
pub mod git_cli_remote_provider;
pub mod git_cli_worktree_change_provider;
pub mod git_cli_worktree_git_provider;
pub mod git_cli_worktree_provider;
pub mod json_acp_session_store;
pub mod json_agent_run_settings_repository;
pub mod json_goal_repository;
pub mod json_project_repository;
pub mod json_saved_prompt_repository;
pub mod json_store;
pub mod mcp;
pub mod noop_acp_session_store;
pub mod perf_log;
pub mod permission_broker;
pub mod tauri_run_event_sink;
pub mod window_manager;
