use crate::{
    application::agent_run_errors::SetPermissionModeError,
    domain::run::PermissionMode,
    ports::{
        event_sink::RunEventSink, session_handle::SessionHandle, session_registry::SessionRegistry,
    },
};

/// Change the permission mode of an in-flight agent run.
///
/// The agent keeps running; the new mode is applied to the live session so
/// the next tool/command approval policy reflects the change without
/// restarting the run. Errors are surfaced to the caller so the UI can keep
/// the previously applied mode and explain what happened.
pub struct SetPermissionModeUseCase<R>
where
    R: SessionRegistry,
    R::Session: SessionHandle,
{
    registry: R,
}

impl<R> SetPermissionModeUseCase<R>
where
    R: SessionRegistry,
    R::Session: SessionHandle,
{
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    pub async fn execute<S>(
        self,
        sink: S,
        run_id: String,
        mode: PermissionMode,
    ) -> Result<(), SetPermissionModeError>
    where
        S: RunEventSink,
    {
        let session = self
            .registry
            .active_session(&run_id)
            .await
            .ok_or(SetPermissionModeError::RunNotActive)?;

        session
            .set_permission_mode(sink, mode)
            .await
            .map_err(|err| SetPermissionModeError::Apply(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::RunEvent;
    use anyhow::{Result, anyhow};
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex as StdMutex},
    };
    use tokio::sync::Mutex;
    use tokio::task::JoinHandle;

    enum FakeBehavior {
        Ok,
        Err,
    }

    struct FakeSession {
        behavior: FakeBehavior,
        applied: Arc<Mutex<Vec<PermissionMode>>>,
    }

    impl SessionHandle for FakeSession {
        async fn send_prompt<S>(&self, _sink: S, _text: String) -> Result<String>
        where
            S: RunEventSink,
        {
            Ok("end_turn".into())
        }

        async fn set_permission_mode<S>(&self, _sink: S, mode: PermissionMode) -> Result<()>
        where
            S: RunEventSink,
        {
            self.applied.lock().await.push(mode);
            match self.behavior {
                FakeBehavior::Ok => Ok(()),
                FakeBehavior::Err => Err(anyhow!("agent rejected the mode change")),
            }
        }
    }

    #[derive(Clone, Default)]
    struct FakeRegistry {
        sessions: Arc<Mutex<HashMap<String, Arc<FakeSession>>>>,
    }

    impl FakeRegistry {
        async fn with_session(
            run_id: &str,
            behavior: FakeBehavior,
        ) -> (Self, Arc<Mutex<Vec<PermissionMode>>>) {
            let registry = Self::default();
            let applied = Arc::new(Mutex::new(Vec::new()));
            registry.sessions.lock().await.insert(
                run_id.to_string(),
                Arc::new(FakeSession {
                    behavior,
                    applied: applied.clone(),
                }),
            );
            (registry, applied)
        }
    }

    impl SessionRegistry for FakeRegistry {
        type Session = FakeSession;

        async fn reserve_run(
            &self,
            _: String,
            _: Option<String>,
        ) -> Result<(), crate::ports::session_registry::ReserveRunError> {
            Ok(())
        }

        async fn attach_run_handle(&self, _: &str, handle: JoinHandle<()>) -> Result<()> {
            handle.abort();
            Ok(())
        }

        async fn attach_session(&self, _: &str, _: Arc<FakeSession>) -> Result<()> {
            Ok(())
        }

        async fn active_session(&self, run_id: &str) -> Option<Arc<FakeSession>> {
            self.sessions.lock().await.get(run_id).cloned()
        }

        async fn finish_run(&self, _: &str) {}

        async fn cancel_run(&self, _: &str) -> bool {
            false
        }
    }

    #[derive(Clone, Default)]
    struct CollectingSink {
        events: Arc<StdMutex<Vec<(String, RunEvent)>>>,
    }

    impl RunEventSink for CollectingSink {
        fn emit(&self, run_id: &str, event: RunEvent) {
            self.events
                .lock()
                .unwrap()
                .push((run_id.to_string(), event));
        }
    }

    #[tokio::test]
    async fn rejects_when_run_is_not_active() {
        let result = SetPermissionModeUseCase::new(FakeRegistry::default())
            .execute(
                CollectingSink::default(),
                "missing".into(),
                PermissionMode::Plan,
            )
            .await;

        assert_eq!(result, Err(SetPermissionModeError::RunNotActive));
    }

    #[tokio::test]
    async fn applies_mode_to_active_session() {
        let (registry, applied) = FakeRegistry::with_session("run-a", FakeBehavior::Ok).await;

        SetPermissionModeUseCase::new(registry)
            .execute(
                CollectingSink::default(),
                "run-a".into(),
                PermissionMode::AcceptEdits,
            )
            .await
            .expect("mode change should succeed");

        assert_eq!(
            applied.lock().await.as_slice(),
            [PermissionMode::AcceptEdits]
        );
    }

    #[tokio::test]
    async fn surfaces_apply_error_from_session() {
        let (registry, _applied) = FakeRegistry::with_session("run-a", FakeBehavior::Err).await;

        let result = SetPermissionModeUseCase::new(registry)
            .execute(
                CollectingSink::default(),
                "run-a".into(),
                PermissionMode::ReadOnly,
            )
            .await;

        assert!(matches!(
            result,
            Err(SetPermissionModeError::Apply(message))
                if message == "agent rejected the mode change"
        ));
    }
}
