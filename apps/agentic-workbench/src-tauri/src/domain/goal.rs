use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GoalStatus {
    Active,
    Paused,
    Blocked,
    UsageLimited,
    BudgetLimited,
    Complete,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadGoal {
    pub working_directory: String,
    pub objective: String,
    pub status: GoalStatus,
    pub token_budget: Option<usize>,
    pub tokens_used: usize,
    pub time_used_seconds: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl ThreadGoal {
    /// agent run에서 관측한 사용량을 누적한다.
    /// token budget이 설정되어 있고 누적 사용량이 budget에 도달하면,
    /// goal이 아직 Active인 경우에 한해 BudgetLimited로 전환한다.
    /// (이미 Paused/Complete 등인 goal의 상태는 건드리지 않는다.)
    pub fn apply_run_usage(&mut self, tokens_used_delta: usize, time_used_seconds_delta: u64) {
        self.tokens_used = self.tokens_used.saturating_add(tokens_used_delta);
        self.time_used_seconds = self
            .time_used_seconds
            .saturating_add(time_used_seconds_delta);

        if self.status == GoalStatus::Active
            && let Some(budget) = self.token_budget
            && budget > 0
            && self.tokens_used >= budget
        {
            self.status = GoalStatus::BudgetLimited;
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoalDraft {
    pub working_directory: String,
    pub objective: String,
    pub token_budget: Option<usize>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GoalUpdate {
    pub objective: Option<String>,
    pub status: Option<GoalStatus>,
    pub token_budget: Option<Option<usize>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GoalProgressUpdate {
    pub tokens_used: usize,
    pub time_used_seconds: u64,
}
