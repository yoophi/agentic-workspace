//! `savedPrompt.*` operation의 input/output wire 타입. core `domain::saved_prompt::SavedPrompt`의 미러(research R1).

use serde::{Deserialize, Serialize};
use utoipa::{
    openapi::{RefOr, Schema},
    PartialSchema, ToSchema,
};

use super::common::array_schema;

/// 프론트 `entities/saved-prompt`의 `SavedPrompt`와 필드·표기가 같다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavedPromptDto {
    pub id: String,
    pub label: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SavedPromptListInput {}

pub fn saved_prompt_list_output_schema() -> RefOr<Schema> {
    array_schema(SavedPromptDto::schema())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedPromptCreateInput {
    pub label: String,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedPromptUpdateInput {
    pub id: String,
    pub label: String,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedPromptDeleteInput {
    pub id: String,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn dto_matches_frontend_shape_and_inputs_reject_unknown_fields() {
        let dto = SavedPromptDto {
            id: "saved-prompt-1".into(),
            label: "Continue".into(),
            prompt: "keep going".into(),
        };
        assert_eq!(
            serde_json::to_value(&dto).unwrap(),
            json!({"id": "saved-prompt-1", "label": "Continue", "prompt": "keep going"})
        );
        assert!(serde_json::from_value::<SavedPromptCreateInput>(
            json!({"label": "a", "prompt": "b", "extra": 1})
        )
        .is_err());
        assert!(serde_json::from_value::<SavedPromptDeleteInput>(json!({"id": "x"})).is_ok());
    }
}
