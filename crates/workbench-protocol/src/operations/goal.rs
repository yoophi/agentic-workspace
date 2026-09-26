//! `goal.*` operation의 input/output wire 타입. core `domain::goal::ThreadGoal`의 미러(research R1).

use serde::{Deserialize, Serialize};
use utoipa::{
    openapi::{RefOr, Schema},
    PartialSchema, ToSchema,
};

use super::common::nullable_schema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum GoalStatus {
    Active,
    Paused,
    Blocked,
    UsageLimited,
    BudgetLimited,
    Complete,
}

/// 프론트 `entities/agent-run/model`의 `ThreadGoal`과 필드·표기가 같다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GoalDto {
    pub working_directory: String,
    pub objective: String,
    pub status: GoalStatus,
    pub token_budget: Option<u64>,
    pub tokens_used: u64,
    pub time_used_seconds: u64,
    pub created_at: String,
    pub updated_at: String,
}

/// `goal.get`·`agentRunSettings.get`처럼 없을 수 있는 단일 항목의 output schema.
pub fn goal_get_output_schema() -> RefOr<Schema> {
    nullable_schema(GoalDto::schema())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GoalGetInput {
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GoalCreateInput {
    pub working_directory: String,
    pub objective: String,
    #[serde(default)]
    pub token_budget: Option<u64>,
}

/// `goal.update` input. `tokenBudget`는 AW `GoalUpdateInput`과 같은 serde 규칙(`Option<Option<_>>`)을 유지한다:
/// 필드 생략과 `null` 모두 "변경 없음"이다. (오늘의 wire로는 예산을 `null`로 지울 수 없고, 038은 동작을 바꾸지 않는다.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GoalUpdateInput {
    pub working_directory: String,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub status: Option<GoalStatus>,
    #[serde(default)]
    #[schema(value_type = Option<u64>)]
    pub token_budget: Option<Option<u64>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GoalClearInput {
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GoalRecordProgressInput {
    pub working_directory: String,
    pub tokens_used: u64,
    pub time_used_seconds: u64,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn update_input_keeps_legacy_token_budget_semantics() {
        let missing: GoalUpdateInput =
            serde_json::from_value(json!({"workingDirectory": "/w"})).unwrap();
        assert_eq!(missing.token_budget, None);
        let null: GoalUpdateInput =
            serde_json::from_value(json!({"workingDirectory": "/w", "tokenBudget": null})).unwrap();
        assert_eq!(null.token_budget, None, "null도 '변경 없음'(AW와 동일)");
        let value: GoalUpdateInput =
            serde_json::from_value(json!({"workingDirectory": "/w", "tokenBudget": 5})).unwrap();
        assert_eq!(value.token_budget, Some(Some(5)));
    }

    #[test]
    fn status_and_dto_use_camel_case() {
        assert_eq!(
            serde_json::to_value(GoalStatus::BudgetLimited).unwrap(),
            "budgetLimited"
        );
        let schema = serde_json::to_value(goal_get_output_schema()).unwrap();
        assert!(schema["oneOf"].is_array());
    }
}
