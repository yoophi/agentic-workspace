use crate::domain::agent_tool_candidate::{
    AgentToolCandidate, AgentToolCandidateQuery, AgentToolCandidateResponse,
    AgentToolCandidateStatus, response_for_candidates,
};

pub trait AgentToolCandidateRegistry {
    fn active_owner_for_run(
        &self,
        run_id: &str,
    ) -> impl std::future::Future<Output = Option<String>> + Send;
}

impl AgentToolCandidateRegistry for crate::infrastructure::agent_session_registry::AppState {
    async fn active_owner_for_run(&self, run_id: &str) -> Option<String> {
        self.active_owner_of(run_id).await
    }
}

pub struct AgentToolCandidateService<R>
where
    R: AgentToolCandidateRegistry,
{
    registry: R,
}

impl<R> AgentToolCandidateService<R>
where
    R: AgentToolCandidateRegistry,
{
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    pub async fn list_candidates(
        &self,
        owner_window_label: &str,
        query: AgentToolCandidateQuery,
        candidates: Vec<AgentToolCandidate>,
    ) -> Result<AgentToolCandidateResponse, String> {
        let run_id = query
            .run_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty());
        if let Some(run_id) = run_id {
            let Some(owner) = self.registry.active_owner_for_run(run_id).await else {
                return Ok(AgentToolCandidateResponse {
                    status: AgentToolCandidateStatus::Empty,
                    candidates: Vec::new(),
                });
            };
            if owner != owner_window_label {
                return Err(
                    "tool command candidates were requested from a non-owner window".into(),
                );
            }
        }

        Ok(response_for_candidates(candidates))
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentToolCandidateRegistry, AgentToolCandidateService};
    use crate::domain::agent_tool_candidate::{
        AgentToolCandidate, AgentToolCandidateQuery, AgentToolCandidateScope,
        AgentToolCandidateSource, AgentToolCandidateStatus,
    };
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::Mutex;

    #[derive(Clone, Default)]
    struct FakeRegistry {
        owners: Arc<Mutex<HashMap<String, String>>>,
    }

    impl FakeRegistry {
        async fn with_owner(run_id: &str, owner: &str) -> Self {
            let registry = Self::default();
            registry
                .owners
                .lock()
                .await
                .insert(run_id.to_string(), owner.to_string());
            registry
        }
    }

    impl AgentToolCandidateRegistry for FakeRegistry {
        async fn active_owner_for_run(&self, run_id: &str) -> Option<String> {
            self.owners.lock().await.get(run_id).cloned()
        }
    }

    fn query(run_id: Option<&str>) -> AgentToolCandidateQuery {
        AgentToolCandidateQuery {
            run_id: run_id.map(str::to_string),
            agent_id: "codex".into(),
            working_directory: "/repo".into(),
            session_mode: "reuse".into(),
        }
    }

    fn candidate() -> AgentToolCandidate {
        AgentToolCandidate {
            id: "session:set_window_title".into(),
            name: "set_window_title".into(),
            description: Some("Change title".into()),
            insert_text: "$set_window_title".into(),
            source: AgentToolCandidateSource::SessionTool,
            scope: AgentToolCandidateScope {
                run_id: Some("run-1".into()),
                agent_id: Some("codex".into()),
                working_directory: Some("/repo".into()),
            },
        }
    }

    #[tokio::test]
    async fn returns_candidates_for_owner_window() {
        let service =
            AgentToolCandidateService::new(FakeRegistry::with_owner("run-1", "session-a").await);
        let response = service
            .list_candidates("session-a", query(Some("run-1")), vec![candidate()])
            .await
            .unwrap();

        assert_eq!(response.status, AgentToolCandidateStatus::Ready);
        assert_eq!(response.candidates.len(), 1);
    }

    #[tokio::test]
    async fn rejects_owner_mismatch() {
        let service =
            AgentToolCandidateService::new(FakeRegistry::with_owner("run-1", "session-a").await);
        let err = service
            .list_candidates("session-b", query(Some("run-1")), vec![candidate()])
            .await
            .unwrap_err();

        assert!(err.contains("non-owner"));
    }

    #[tokio::test]
    async fn missing_active_session_degrades_to_empty() {
        let service = AgentToolCandidateService::new(FakeRegistry::default());
        let response = service
            .list_candidates("session-a", query(Some("missing")), vec![candidate()])
            .await
            .unwrap();

        assert_eq!(response.status, AgentToolCandidateStatus::Empty);
        assert!(response.candidates.is_empty());
    }

    #[tokio::test]
    async fn no_run_query_can_return_static_candidates() {
        let service = AgentToolCandidateService::new(FakeRegistry::default());
        let response = service
            .list_candidates("session-a", query(None), vec![candidate()])
            .await
            .unwrap();

        assert_eq!(response.status, AgentToolCandidateStatus::Ready);
        assert_eq!(response.candidates[0].name, "set_window_title");
    }
}
