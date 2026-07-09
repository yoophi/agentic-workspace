use crate::{
    application::agent_run_errors::SteerPromptError,
    domain::events::{LifecycleStatus, RunEvent},
    ports::{
        event_sink::RunEventSink, session_handle::SessionHandle, session_registry::SessionRegistry,
    },
};

pub struct SteerPromptUseCase<R>
where
    R: SessionRegistry,
    R::Session: SessionHandle,
{
    registry: R,
}

impl<R> SteerPromptUseCase<R>
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
        prompt: String,
    ) -> Result<(), SteerPromptError>
    where
        S: RunEventSink,
    {
        let trimmed = prompt.trim().to_string();
        if trimmed.is_empty() {
            return Err(SteerPromptError::EmptyPrompt);
        }

        let session = self
            .registry
            .active_session(&run_id)
            .await
            .ok_or(SteerPromptError::RunNotActive)?;

        sink.emit(
            &run_id,
            RunEvent::Lifecycle {
                status: LifecycleStatus::SteerPending,
                message: "steer submitted".into(),
            },
        );

        match session.steer_prompt(sink.clone(), trimmed).await {
            Ok(()) => {
                sink.emit(
                    &run_id,
                    RunEvent::Lifecycle {
                        status: LifecycleStatus::SteerAccepted,
                        message: "steer accepted".into(),
                    },
                );
                Ok(())
            }
            Err(err) if is_unsupported_steer(&err) => {
                let message = err.to_string();
                sink.emit(
                    &run_id,
                    RunEvent::Lifecycle {
                        status: LifecycleStatus::SteerRejected,
                        message: message.clone(),
                    },
                );
                Err(SteerPromptError::Unsupported(message))
            }
            Err(err) => {
                let message = err.to_string();
                sink.emit(
                    &run_id,
                    RunEvent::Lifecycle {
                        status: LifecycleStatus::SteerRejected,
                        message: message.clone(),
                    },
                );
                Err(SteerPromptError::DispatchFailed(message))
            }
        }
    }
}

fn is_unsupported_steer(err: &anyhow::Error) -> bool {
    err.to_string().to_lowercase().contains("not supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Result, anyhow};
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex as StdMutex},
    };
    use tokio::sync::Mutex;
    use tokio::task::JoinHandle;

    enum FakeBehavior {
        Accepted,
        Unsupported,
        DispatchFailed,
    }

    struct FakeSession {
        behavior: FakeBehavior,
    }

    impl SessionHandle for FakeSession {
        async fn send_prompt<S>(&self, _sink: S, _text: String) -> Result<String>
        where
            S: RunEventSink,
        {
            Ok("end_turn".into())
        }

        async fn steer_prompt<S>(&self, _sink: S, _text: String) -> Result<()>
        where
            S: RunEventSink,
        {
            match self.behavior {
                FakeBehavior::Accepted => Ok(()),
                FakeBehavior::Unsupported => Err(anyhow!(
                    "active-turn steer is not supported by this session"
                )),
                FakeBehavior::DispatchFailed => Err(anyhow!("transport failed")),
            }
        }

        async fn set_permission_mode<S>(
            &self,
            _sink: S,
            _mode: crate::domain::run::PermissionMode,
        ) -> Result<()>
        where
            S: RunEventSink,
        {
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct FakeRegistry {
        sessions: Arc<Mutex<HashMap<String, Arc<FakeSession>>>>,
        cancelled: Arc<Mutex<Vec<String>>>,
    }

    impl FakeRegistry {
        async fn with_session(run_id: &str, behavior: FakeBehavior) -> Self {
            let registry = Self::default();
            registry
                .sessions
                .lock()
                .await
                .insert(run_id.to_string(), Arc::new(FakeSession { behavior }));
            registry
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

        async fn cancel_run(&self, run_id: &str) -> bool {
            self.cancelled.lock().await.push(run_id.to_string());
            true
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
    async fn rejects_empty_prompt_without_dispatch() {
        let result = SteerPromptUseCase::new(FakeRegistry::default())
            .execute(CollectingSink::default(), "run-a".into(), "   ".into())
            .await;

        assert_eq!(result, Err(SteerPromptError::EmptyPrompt));
    }

    #[tokio::test]
    async fn rejects_when_run_is_not_active() {
        let result = SteerPromptUseCase::new(FakeRegistry::default())
            .execute(CollectingSink::default(), "missing".into(), "hi".into())
            .await;

        assert_eq!(result, Err(SteerPromptError::RunNotActive));
    }

    #[tokio::test]
    async fn emits_accepted_lifecycle_without_cancelling_run() {
        let registry = FakeRegistry::with_session("run-a", FakeBehavior::Accepted).await;
        let sink = CollectingSink::default();

        SteerPromptUseCase::new(registry.clone())
            .execute(sink.clone(), "run-a".into(), "steer".into())
            .await
            .unwrap();

        assert!(registry.cancelled.lock().await.is_empty());
        assert!(
            sink.events
                .lock()
                .unwrap()
                .iter()
                .any(|(_, event)| matches!(
                    event,
                    RunEvent::Lifecycle {
                        status: LifecycleStatus::SteerAccepted,
                        ..
                    }
                ))
        );
    }

    #[tokio::test]
    async fn maps_unsupported_capability_to_rejected_lifecycle() {
        let registry = FakeRegistry::with_session("run-a", FakeBehavior::Unsupported).await;
        let sink = CollectingSink::default();

        let result = SteerPromptUseCase::new(registry)
            .execute(sink.clone(), "run-a".into(), "steer".into())
            .await;

        assert!(matches!(result, Err(SteerPromptError::Unsupported(_))));
        assert!(
            sink.events
                .lock()
                .unwrap()
                .iter()
                .any(|(_, event)| matches!(
                    event,
                    RunEvent::Lifecycle {
                        status: LifecycleStatus::SteerRejected,
                        ..
                    }
                ))
        );
    }

    #[tokio::test]
    async fn maps_other_failures_to_dispatch_failed() {
        let registry = FakeRegistry::with_session("run-a", FakeBehavior::DispatchFailed).await;

        let result = SteerPromptUseCase::new(registry)
            .execute(CollectingSink::default(), "run-a".into(), "steer".into())
            .await;

        assert!(matches!(result, Err(SteerPromptError::DispatchFailed(_))));
    }
}
