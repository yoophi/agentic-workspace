//! saved prompt 도메인. AW `domain/saved_prompt.rs`에서 이동(038 US1). wire 형식은 `SavedPromptDto`가 미러한다.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedPrompt {
    pub id: String,
    pub label: String,
    pub prompt: String,
}

#[derive(Debug, Clone)]
pub struct SavedPromptDraft {
    pub label: String,
    pub prompt: String,
}
