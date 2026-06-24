use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResumePolicy {
    #[default]
    Fresh,
    ResumeIfAvailable,
    ResumeRequired,
}

/// Ralph loop가 한 run에서 자동 반복할 수 있는 최대 횟수.
/// 잘못된 입력이나 폭주로 인한 무한/과도 반복을 막는 안전 상한이다.
pub const MAX_RALPH_ITERATIONS: usize = 100;

/// iteration 사이에 둘 수 있는 최대 지연(밀리초). 과도한 대기를 막는다.
pub const MAX_RALPH_DELAY_MS: u64 = 60_000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RalphLoopRequest {
    pub enabled: bool,
    pub max_iterations: usize,
    pub prompt_template: String,
    pub stop_on_error: bool,
    pub stop_on_permission: bool,
    pub delay_ms: u64,
}

impl RalphLoopRequest {
    /// 클라이언트가 보낸 loop 설정을 실행 가능한 안전 범위로 정규화한다.
    /// 반복 횟수와 지연을 상한으로 제한하고 loop prompt의 공백을 정리한다.
    pub fn sanitized(mut self) -> Self {
        self.prompt_template = self.prompt_template.trim().to_string();
        self.max_iterations = self.max_iterations.clamp(1, MAX_RALPH_ITERATIONS);
        self.delay_ms = self.delay_ms.min(MAX_RALPH_DELAY_MS);
        self
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    #[default]
    Default,
    Auto,
    ReadOnly,
    Plan,
    AcceptEdits,
    DangerouslySkipAllPermissions,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ContextSizePreset {
    #[default]
    Default,
    Medium,
    Large,
    XLarge,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunRequest {
    pub goal: String,
    pub agent_id: String,
    pub workspace_id: Option<String>,
    pub checkout_id: Option<String>,
    pub cwd: Option<String>,
    pub agent_command: Option<String>,
    pub stdio_buffer_limit_mb: Option<usize>,
    pub auto_allow: Option<bool>,
    pub permission_mode: Option<PermissionMode>,
    pub model_id: Option<String>,
    pub context_size: Option<ContextSizePreset>,
    pub run_id: Option<String>,
    pub resume_session_id: Option<String>,
    pub resume_policy: Option<ResumePolicy>,
    pub ralph_loop: Option<RalphLoopRequest>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRun {
    pub id: String,
    pub goal: String,
    pub agent_id: String,
}

impl AgentRun {
    pub fn new(goal: String, agent_id: String) -> Self {
        Self::with_id(Uuid::new_v4().to_string(), goal, agent_id)
    }

    pub fn with_id(id: String, goal: String, agent_id: String) -> Self {
        Self { id, goal, agent_id }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_RALPH_DELAY_MS, MAX_RALPH_ITERATIONS, RalphLoopRequest};

    fn request(max_iterations: usize, prompt: &str, delay_ms: u64) -> RalphLoopRequest {
        RalphLoopRequest {
            enabled: true,
            max_iterations,
            prompt_template: prompt.to_string(),
            stop_on_error: true,
            stop_on_permission: false,
            delay_ms,
        }
    }

    #[test]
    fn sanitized_clamps_iterations_into_safe_range() {
        assert_eq!(request(0, "continue", 0).sanitized().max_iterations, 1);
        assert_eq!(
            request(MAX_RALPH_ITERATIONS + 50, "continue", 0)
                .sanitized()
                .max_iterations,
            MAX_RALPH_ITERATIONS
        );
        assert_eq!(request(5, "continue", 0).sanitized().max_iterations, 5);
    }

    #[test]
    fn sanitized_caps_delay_and_trims_prompt() {
        let sanitized = request(3, "  keep going  ", MAX_RALPH_DELAY_MS + 1_000).sanitized();
        assert_eq!(sanitized.delay_ms, MAX_RALPH_DELAY_MS);
        assert_eq!(sanitized.prompt_template, "keep going");
    }
}
