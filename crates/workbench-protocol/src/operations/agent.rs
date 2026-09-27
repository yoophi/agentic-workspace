//! `agent.*` operation의 input/output wire 타입. acp-agent-core `domain::agent`와 core
//! `domain::provider_session`의 미러(research R1). serde 속성은 원본과 같다 — `runtimeVersion`은 없으면 생략된다.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentOptionDescriptorDto {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentDescriptorDto {
    pub id: String,
    pub label: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_version: Option<String>,
    #[serde(default)]
    pub models: Vec<AgentOptionDescriptorDto>,
    #[serde(default)]
    pub efforts: Vec<AgentOptionDescriptorDto>,
    #[serde(default)]
    pub context_sizes: Vec<AgentOptionDescriptorDto>,
}

/// provider가 로컬에 남긴 네이티브 세션 하나의 요약. 시각은 RFC3339.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSessionDto {
    pub agent_id: String,
    pub id: String,
    pub cwd: Option<String>,
    pub title: Option<String>,
    pub file: String,
    pub message_count: u64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub model: Option<String>,
    pub branch: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentListInput {}

/// `cwd`가 없거나 공백이면 전체 범위. 결과는 최신순 최대 50개. 네이티브 세션 조회를 지원하지 않는 agent는 빈 목록.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentListProviderSessionsInput {
    pub agent_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn runtime_version_is_omitted_and_option_lists_default_to_empty() {
        let descriptor: AgentDescriptorDto =
            serde_json::from_value(json!({"id": "codex", "label": "Codex", "command": "codex"}))
                .unwrap();
        assert!(descriptor.models.is_empty());
        assert_eq!(
            serde_json::to_value(&descriptor).unwrap(),
            json!({"id": "codex", "label": "Codex", "command": "codex", "models": [], "efforts": [], "contextSizes": []})
        );
        assert!(serde_json::from_value::<AgentListInput>(json!({"x": 1})).is_err());
    }
}
