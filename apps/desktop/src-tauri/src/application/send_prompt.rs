use crate::{
    application::agent_run_errors::SendPromptError,
    domain::events::RunEvent,
    ports::{
        event_sink::RunEventSink, session_handle::SessionHandle, session_registry::SessionRegistry,
    },
};

pub struct SendPromptUseCase<R>
where
    R: SessionRegistry,
    R::Session: SessionHandle,
{
    registry: R,
}

impl<R> SendPromptUseCase<R>
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
    ) -> Result<(), SendPromptError>
    where
        S: RunEventSink,
    {
        let trimmed = prompt.trim().to_string();
        if trimmed.is_empty() {
            return Err(SendPromptError::EmptyPrompt);
        }

        let session = self
            .registry
            .active_session(&run_id)
            .await
            .ok_or(SendPromptError::RunNotActive)?;

        let sink_for_task = sink.clone();
        tokio::spawn(async move {
            if let Err(err) = session.send_prompt(sink_for_task.clone(), trimmed).await {
                sink_for_task.emit(
                    &run_id,
                    RunEvent::Error {
                        message: err.to_string(),
                    },
                );
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Result, anyhow};
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex as StdMutex},
    };
    use tokio::sync::{Mutex, Notify};
    use tokio::task::JoinHandle;

    #[allow(dead_code)]
    enum FakeBehavior {
        Ok,
        Err,
    }

    struct FakeSession {
        behavior: FakeBehavior,
        done: Arc<Notify>,
    }

    impl SessionHandle for FakeSession {
        async fn send_prompt<S>(&self, _sink: S, _text: String) -> Result<String>
        where
            S: RunEventSink,
        {
            self.done.notify_one();
            match self.behavior {
                FakeBehavior::Ok => Ok("end_turn".into()),
                FakeBehavior::Err => Err(anyhow!("dispatch exploded")),
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
    }

    impl FakeRegistry {
        async fn with_session(run_id: &str, behavior: FakeBehavior) -> (Self, Arc<Notify>) {
            let registry = Self::default();
            let done = Arc::new(Notify::new());
            registry.sessions.lock().await.insert(
                run_id.to_string(),
                Arc::new(FakeSession {
                    behavior,
                    done: done.clone(),
                }),
            );
            (registry, done)
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
    async fn rejects_empty_prompt_without_dispatch() {
        let result = SendPromptUseCase::new(FakeRegistry::default())
            .execute(CollectingSink::default(), "run-a".into(), "   ".into())
            .await;

        assert_eq!(result, Err(SendPromptError::EmptyPrompt));
    }

    #[tokio::test]
    async fn rejects_when_run_is_not_active() {
        let result = SendPromptUseCase::new(FakeRegistry::default())
            .execute(CollectingSink::default(), "missing".into(), "hi".into())
            .await;

        assert_eq!(result, Err(SendPromptError::RunNotActive));
    }

    #[tokio::test]
    async fn emits_error_event_when_prompt_dispatch_fails() {
        let (registry, done) = FakeRegistry::with_session("run-a", FakeBehavior::Err).await;
        let sink = CollectingSink::default();

        SendPromptUseCase::new(registry)
            .execute(sink.clone(), "run-a".into(), "hi".into())
            .await
            .unwrap();

        done.notified().await;
        tokio::task::yield_now().await;

        assert!(
            sink.events
                .lock()
                .unwrap()
                .iter()
                .any(|(_, event)| matches!(event, RunEvent::Error { .. }))
        );
    }
}
