//! 도메인 오류 enum. `Display`는 AW가 `Result<_, String>`으로 돌려주던 문구와 바이트 단위로 같다
//! (`specs/038-workbench-domains/contracts/workbench-operations.md` §3 골든). FaultCode 매핑은 research R7.

pub mod agent_run_settings_error;
pub mod goal_error;
pub mod saved_prompt_error;

pub use agent_run_settings_error::AgentRunSettingsError;
pub use goal_error::GoalError;
pub use saved_prompt_error::SavedPromptError;
