pub use acp_agent_core::domain::{acp_session, agent, agent_tool_candidate, events, run};

// 038 US1: saved prompt·goal·agent 실행 설정 도메인은 `workbench-core`로 이동했다. 기존 import 경로를 유지하기 위해 재노출한다.
pub use workbench_core::domain::{agent_run_settings, goal, saved_prompt};
// 038 US2: Git·worktree·파일 도메인도 `workbench-core`로 이동했다. 조회 결과 타입(git-core 재노출 두 모듈)은 그대로 둔다.
pub use workbench_core::domain::{
    git_branch, git_remote, git_worktree, worktree_change, worktree_file,
};
pub mod appearance_preferences;
pub mod git_worktree_changes;
pub mod mcp_title_control;
// 프로젝트 도메인은 037에서 `workbench-core`로 이동했다. 기존 import 경로를 유지하기 위해 재노출한다.
pub use workbench_core::domain::project;
// 038 US3: provider 세션 도메인도 `workbench-core`로 이동했다.
pub use workbench_core::domain::provider_session;
pub mod session_window_state;
pub mod session_window_state_repository;
pub mod window_menu;
pub mod worktree_git;
pub mod worktree_workspace_layout;
pub mod worktree_workspace_layout_repository;
