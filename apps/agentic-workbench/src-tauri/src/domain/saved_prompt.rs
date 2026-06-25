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
