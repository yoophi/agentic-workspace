use crate::{
    domain::events::{LifecycleStatus, RunEvent},
    ports::{event_sink::RunEventSink, session_registry::SessionRegistry},
};

/// Cancel an active agent run and emit a Cancelled lifecycle event.
///
/// The lifecycle is emitted even when the run is unknown so the
/// frontend can always clear a "closing" tab on an acknowledged event
/// instead of waiting for a response it will never receive.
pub struct CancelAgentRunUseCase<R>
where
    R: SessionRegistry,
{
    registry: R,
}

impl<R> CancelAgentRunUseCase<R>
where
    R: SessionRegistry,
{
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    pub async fn execute<S>(self, sink: S, run_id: String)
    where
        S: RunEventSink,
    {
        let cancelled = self.registry.cancel_run(&run_id).await;
        sink.emit(
            &run_id,
            RunEvent::Lifecycle {
                status: LifecycleStatus::Cancelled,
                message: if cancelled {
                    "run cancelled".into()
                } else {
                    "run was already terminated".into()
                },
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::session_registry::{ReserveRunError, SessionRegistry};
    use anyhow::Result;
    use std::sync::{Arc, Mutex as StdMutex};
    use tokio::sync::Mutex;
    use tokio::task::JoinHandle;

    struct FakeSession;

    #[derive(Clone, Default)]
    struct FakeRegistry {
        cancelled: Arc<Mutex<Vec<String>>>,
        next_cancel_result: Arc<Mutex<bool>>,
    }

    impl SessionRegistry for FakeRegistry {
        type Session = FakeSession;

        async fn reserve_run(&self, _: String, _: Option<String>) -> Result<(), ReserveRunError> {
            Ok(())
        }
        async fn attach_run_handle(&self, _: &str, handle: JoinHandle<()>) -> Result<()> {
            handle.abort();
            Ok(())
        }
        async fn attach_session(&self, _: &str, _: Arc<FakeSession>) -> Result<()> {
            Ok(())
        }
        async fn active_session(&self, _: &str) -> Option<Arc<FakeSession>> {
            None
        }
        async fn finish_run(&self, _: &str) {}
        async fn cancel_run(&self, run_id: &str) -> bool {
            self.cancelled.lock().await.push(run_id.to_string());
            *self.next_cancel_result.lock().await
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
    async fn emits_cancelled_lifecycle_with_success_message_when_cancel_returns_true() {
        let registry = FakeRegistry::default();
        *registry.next_cancel_result.lock().await = true;
        let sink = CollectingSink::default();

        CancelAgentRunUseCase::new(registry)
            .execute(sink.clone(), "run-a".into())
            .await;

        let events = sink.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        match &events[0].1 {
            RunEvent::Lifecycle { status, message } => {
                assert!(matches!(status, LifecycleStatus::Cancelled));
                assert_eq!(message, "run cancelled");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn emits_cancelled_lifecycle_with_fallback_message_when_run_unknown() {
        let registry = FakeRegistry::default();
        let sink = CollectingSink::default();

        CancelAgentRunUseCase::new(registry)
            .execute(sink.clone(), "run-b".into())
            .await;

        let events = sink.events.lock().unwrap();
        match &events[0].1 {
            RunEvent::Lifecycle { message, .. } => {
                assert_eq!(message, "run was already terminated");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
