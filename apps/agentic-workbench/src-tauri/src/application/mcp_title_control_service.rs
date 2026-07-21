use crate::domain::mcp_title_control::{
    TitleChangeFailureCode, TitleChangeRequest, TitleChangeResult, ValidatedWindowTitle,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowTitleCommand {
    pub window_label: String,
    pub title: String,
}

pub trait TitleControlRegistry {
    fn active_owner_for_run(
        &self,
        run_id: &str,
    ) -> impl std::future::Future<Output = Option<String>> + Send;
}

impl TitleControlRegistry for crate::infrastructure::agent_session_registry::AppState {
    async fn active_owner_for_run(&self, run_id: &str) -> Option<String> {
        self.active_owner_of(run_id).await
    }
}

pub struct McpTitleControlService<R>
where
    R: TitleControlRegistry,
{
    registry: R,
}

impl<R> McpTitleControlService<R>
where
    R: TitleControlRegistry,
{
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    pub async fn build_command(
        &self,
        request: TitleChangeRequest,
    ) -> Result<WindowTitleCommand, TitleChangeResult> {
        let title = ValidatedWindowTitle::parse(&request.title).map_err(|err| {
            TitleChangeResult::failure(TitleChangeFailureCode::InvalidTitle, err.reason())
        })?;
        let run_id = request.run_id.trim();
        if run_id.is_empty() {
            return Err(TitleChangeResult::failure(
                TitleChangeFailureCode::UnknownRun,
                "Agent run id is required.",
            ));
        }
        let Some(window_label) = self.registry.active_owner_for_run(run_id).await else {
            return Err(TitleChangeResult::failure(
                TitleChangeFailureCode::UnknownRun,
                "Agent run is not active or is not owned by a session window.",
            ));
        };

        Ok(WindowTitleCommand {
            window_label,
            title: title.into_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{McpTitleControlService, TitleControlRegistry, WindowTitleCommand};
    use crate::domain::mcp_title_control::{
        TitleChangeFailureCode, TitleChangeRequest, TitleChangeResult,
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

    impl TitleControlRegistry for FakeRegistry {
        async fn active_owner_for_run(&self, run_id: &str) -> Option<String> {
            self.owners.lock().await.get(run_id).cloned()
        }
    }

    #[tokio::test]
    async fn builds_command_for_active_owner() {
        let service =
            McpTitleControlService::new(FakeRegistry::with_owner("run-1", "session-a").await);
        let command = service
            .build_command(TitleChangeRequest {
                run_id: "run-1".into(),
                title: "  Ship title control  ".into(),
            })
            .await
            .unwrap();

        assert_eq!(
            command,
            WindowTitleCommand {
                window_label: "session-a".into(),
                title: "Ship title control".into()
            }
        );
    }

    #[tokio::test]
    async fn rejects_unknown_run() {
        let service = McpTitleControlService::new(FakeRegistry::default());
        let result = service
            .build_command(TitleChangeRequest {
                run_id: "missing".into(),
                title: "Title".into(),
            })
            .await
            .unwrap_err();

        assert_eq!(
            result,
            TitleChangeResult::failure(
                TitleChangeFailureCode::UnknownRun,
                "Agent run is not active or is not owned by a session window.",
            )
        );
    }

    #[tokio::test]
    async fn rejects_invalid_title_before_owner_lookup() {
        let service =
            McpTitleControlService::new(FakeRegistry::with_owner("run-1", "session-a").await);
        let result = service
            .build_command(TitleChangeRequest {
                run_id: "run-1".into(),
                title: " ".into(),
            })
            .await
            .unwrap_err();

        assert_eq!(result.code, Some(TitleChangeFailureCode::InvalidTitle));
    }
}
