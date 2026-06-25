use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentOptionDescriptor {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDescriptor {
    pub id: String,
    pub label: String,
    pub command: String,
    #[serde(default)]
    pub models: Vec<AgentOptionDescriptor>,
    #[serde(default)]
    pub context_sizes: Vec<AgentOptionDescriptor>,
}
