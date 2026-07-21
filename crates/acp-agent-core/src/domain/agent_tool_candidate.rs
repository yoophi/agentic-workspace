use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCandidateScope {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentToolCandidateSource {
    SessionTool,
    AppCommand,
    Extension,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCandidate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub insert_text: String,
    pub source: AgentToolCandidateSource,
    pub scope: AgentToolCandidateScope,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentToolCandidateStatus {
    Loading,
    Ready,
    Empty,
    Error,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCandidateQuery {
    pub run_id: Option<String>,
    pub agent_id: String,
    pub working_directory: String,
    pub session_mode: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCandidateResponse {
    pub status: AgentToolCandidateStatus,
    pub candidates: Vec<AgentToolCandidate>,
}

pub fn normalize_candidates(candidates: Vec<AgentToolCandidate>) -> Vec<AgentToolCandidate> {
    let mut normalized = Vec::new();
    for mut candidate in candidates {
        candidate.id = candidate.id.trim().to_string();
        candidate.name = candidate.name.trim().to_string();
        candidate.insert_text = candidate.insert_text.trim().to_string();
        candidate.description = candidate.description.and_then(|value| {
            let trimmed = value.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        });
        if candidate.name.is_empty() || candidate.insert_text.is_empty() {
            continue;
        }
        let duplicate = normalized.iter().any(|existing: &AgentToolCandidate| {
            existing.id == candidate.id
                || (existing.source == candidate.source
                    && existing.name == candidate.name
                    && existing.insert_text == candidate.insert_text)
        });
        if !duplicate {
            normalized.push(candidate);
        }
    }
    normalized
}

pub fn response_for_candidates(candidates: Vec<AgentToolCandidate>) -> AgentToolCandidateResponse {
    let candidates = normalize_candidates(candidates);
    AgentToolCandidateResponse {
        status: if candidates.is_empty() {
            AgentToolCandidateStatus::Empty
        } else {
            AgentToolCandidateStatus::Ready
        },
        candidates,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentToolCandidate, AgentToolCandidateScope, AgentToolCandidateSource,
        AgentToolCandidateStatus, normalize_candidates, response_for_candidates,
    };

    fn candidate(id: &str, name: &str, insert_text: &str) -> AgentToolCandidate {
        AgentToolCandidate {
            id: id.into(),
            name: name.into(),
            description: Some("  description  ".into()),
            insert_text: insert_text.into(),
            source: AgentToolCandidateSource::SessionTool,
            scope: AgentToolCandidateScope {
                run_id: Some("run-1".into()),
                agent_id: Some("codex".into()),
                working_directory: Some("/repo".into()),
            },
        }
    }

    #[test]
    fn normalizes_candidate_text_and_removes_invalid_entries() {
        let normalized = normalize_candidates(vec![
            candidate(" tool ", " set_window_title ", " $set_window_title "),
            candidate("blank-name", " ", "$blank"),
            candidate("blank-insert", "blank", " "),
        ]);

        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].id, "tool");
        assert_eq!(normalized[0].name, "set_window_title");
        assert_eq!(normalized[0].insert_text, "$set_window_title");
        assert_eq!(normalized[0].description.as_deref(), Some("description"));
    }

    #[test]
    fn removes_duplicate_candidates() {
        let normalized = normalize_candidates(vec![
            candidate("tool", "set_window_title", "$set_window_title"),
            candidate("tool", "set_window_title", "$set_window_title"),
        ]);
        assert_eq!(normalized.len(), 1);
    }

    #[test]
    fn empty_candidate_response_uses_empty_status() {
        let response = response_for_candidates(Vec::new());
        assert_eq!(response.status, AgentToolCandidateStatus::Empty);
        assert!(response.candidates.is_empty());
    }
}
