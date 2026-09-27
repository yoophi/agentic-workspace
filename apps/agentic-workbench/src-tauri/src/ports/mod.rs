pub use acp_agent_core::ports::{
    acp_session_store, agent_catalog, event_sink, permission, session_handle, session_launcher,
    session_registry,
};

pub mod appearance_preferences_repository;
// 041: core로 이동. compat 전환(US1–US4) 동안 기존 경로를 유지하는 재노출이다.
pub use workbench_core::ports::{
    agent_worker, coordinator_notification, orchestration_event_sink, orchestration_repository,
};
