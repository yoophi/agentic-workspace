pub use acp_agent_core::application::{
    agent_run_errors, cancel_agent_run, cancel_prompt_and_send, send_prompt, set_permission_mode,
    start_agent_run, steer_prompt,
};

pub mod appearance_preferences_service;
pub mod orchestration_event_projector;
pub mod session_window_state_service;
pub mod window_menu_service;
pub mod worktree_workspace_layout_service;
// 041: core로 이동. compat 전환(US1–US4) 동안 기존 경로를 유지하는 재노출이다.
pub use workbench_core::application::orchestration::{
    command_service as orchestration_command_service,
    notification_dispatcher as coordinator_notification_dispatcher,
    scheduler as orchestration_scheduler, service as orchestration_service,
};
