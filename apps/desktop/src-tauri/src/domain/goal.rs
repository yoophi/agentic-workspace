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
