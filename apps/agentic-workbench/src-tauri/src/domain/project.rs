use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub working_directory: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectDraft {
    pub name: String,
    pub working_directory: String,
    pub description: Option<String>,
}
