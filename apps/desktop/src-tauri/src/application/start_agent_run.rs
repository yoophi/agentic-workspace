use crate::{
    application::agent_run_errors::StartAgentRunError,
    domain::{
        events::RunEvent,
        run::{AgentRun, AgentRunRequest},
    },
    ports::{
        event_sink::RunEventSink,
        session_launcher::{LaunchedSession, SessionLauncher},
        session_registry::SessionRegistry,
    },
};

/// Start a new agent run.
///
/// The use case owns the registry bookkeeping and the error/cleanup
/// flow around the launcher, so Tauri command handlers stay thin. All
/// adapter-level setup (spawning the ACP subprocess, opening the
/// session) is hidden behind the `SessionLauncher` port.
pub struct StartAgentRunUseCase<R>
where
    R: SessionRegistry,
{
    registry: R,
}

impl<R> StartAgentRunUseCase<R>
where
    R: SessionRegistry,
{
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    pub async fn execute<L, S>(
        self,
        launcher: L,
        sink: S,
        request: AgentRunRequest,
        owner_window_label: Option<String>,
    ) -> Result<AgentRun, StartAgentRunError>
    where
        L: SessionLauncher<Session = R::Session>,
        S: RunEventSink,
    {
        let run = build_run(&request);
        self.registry
            .reserve_run(run.id.clone(), owner_window_label)
            .await
            .map_err(StartAgentRunError::ReserveRun)?;

        let registry = self.registry.clone();
        let run_id = run.id.clone();
        let sink_for_task = sink.clone();

        let handle = tokio::spawn(async move {
            let launched = match launcher
                .launch(request, run_id.clone(), sink_for_task.clone())
                .await
            {
                Ok(launched) => launched,
                Err(err) => {
                    sink_for_task.emit(
                        &run_id,
                        RunEvent::Error {
                            message: err.to_string(),
                        },
                    );
                    registry.finish_run(&run_id).await;
                    return;
                }
            };
            let LaunchedSession { session, commander } = launched;

            if let Err(err) = registry.attach_session(&run_id, session).await {
                sink_for_task.emit(
                    &run_id,
                    RunEvent::Diagnostic {
                        message: err.to_string(),
                    },
                );
                commander.abort().await;
                registry.finish_run(&run_id).await;
                return;
            }

            commander.run_to_completion().await;
            registry.finish_run(&run_id).await;
        });

        self.registry
            .attach_run_handle(&run.id, handle)
            .await
            .map_err(|err| StartAgentRunError::AttachRunHandle(err.to_string()))?;

        Ok(run)
    }
}

fn build_run(request: &AgentRunRequest) -> AgentRun {
    match request.run_id.clone() {
        Some(id) if !id.trim().is_empty() => {
            AgentRun::with_id(id, request.goal.clone(), request.agent_id.clone())
        }
        _ => AgentRun::new(request.goal.clone(), request.agent_id.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::agent_run_errors::StartAgentRunError;
    use crate::domain::events::{LifecycleStatus, RunEvent};
    use crate::ports::session_launcher::{AbortFuture, DriverFuture, RunCommander};
    use crate::ports::session_registry::{ReserveRunError, SessionRegistry};
    use anyhow::{Result, anyhow};
    use std::{
        collections::HashMap,
        sync::{
            Arc, Mutex as StdMutex,
            atomic::{AtomicUsize, Ordering},
        },
    };
    use tokio::sync::{Mutex, Notify};
    use tokio::task::JoinHandle;

    struct FakeSession;

    #[derive(Clone, Default)]
    struct FakeRegistry {
        inner: Arc<Mutex<FakeRegistryState>>,
    }

    #[derive(Default)]
    struct FakeRegistryState {
        reserved: Vec<String>,
        owners: HashMap<String, Option<String>>,
        finished: Vec<String>,
        sessions: HashMap<String, Arc<FakeSession>>,
        handles: HashMap<String, JoinHandle<()>>,
        reserve_run_error: Option<ReserveRunError>,
        fail_attach_run_handle: bool,
        fail_attach_session: bool,
    }

    impl FakeRegistry {
        async fn with_failing_reserve_run(error: ReserveRunError) -> Self {
            let reg = Self::default();
            reg.inner.lock().await.reserve_run_error = Some(error);
            reg
        }

        async fn with_failing_attach_run_handle() -> Self {
            let reg = Self::default();
            reg.inner.lock().await.fail_attach_run_handle = true;
            reg
        }

        async fn with_failing_attach() -> Self {
            let reg = Self::default();
            reg.inner.lock().await.fail_attach_session = true;
            reg
        }
    }

    impl SessionRegistry for FakeRegistry {
        type Session = FakeSession;

        async fn reserve_run(
            &self,
            run_id: String,
            owner_window_label: Option<String>,
        ) -> Result<(), ReserveRunError> {
            let mut state = self.inner.lock().await;
            if let Some(error) = state.reserve_run_error.clone() {
                return Err(error);
            }
            state.owners.insert(run_id.clone(), owner_window_label);
            state.reserved.push(run_id);
            Ok(())
        }

        async fn attach_run_handle(&self, run_id: &str, handle: JoinHandle<()>) -> Result<()> {
            let mut state = self.inner.lock().await;
            if state.fail_attach_run_handle {
                handle.abort();
                return Err(anyhow!("simulated attach_run_handle failure"));
            }
            state.handles.insert(run_id.to_string(), handle);
            Ok(())
        }

        async fn attach_session(&self, run_id: &str, session: Arc<FakeSession>) -> Result<()> {
            let mut state = self.inner.lock().await;
            if state.fail_attach_session {
                return Err(anyhow!("simulated attach_session failure"));
            }
            state.sessions.insert(run_id.to_string(), session);
            Ok(())
        }

        async fn active_session(&self, run_id: &str) -> Option<Arc<FakeSession>> {
            self.inner.lock().await.sessions.get(run_id).cloned()
        }

        async fn finish_run(&self, run_id: &str) {
            self.inner.lock().await.finished.push(run_id.to_string());
        }

        async fn cancel_run(&self, _run_id: &str) -> bool {
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

    struct FakeCommander {
        aborted: Arc<AtomicUsize>,
        completed: Arc<AtomicUsize>,
        done: Arc<Notify>,
    }

    impl RunCommander for FakeCommander {
        fn run_to_completion(self: Box<Self>) -> DriverFuture {
            Box::pin(async move {
                self.completed.fetch_add(1, Ordering::SeqCst);
                self.done.notify_one();
            })
        }

        fn abort(self: Box<Self>) -> AbortFuture {
            Box::pin(async move {
                self.aborted.fetch_add(1, Ordering::SeqCst);
                self.done.notify_one();
            })
        }
    }

    struct FakeLauncher {
        aborted: Arc<AtomicUsize>,
        completed: Arc<AtomicUsize>,
        done: Arc<Notify>,
        fail_launch: bool,
    }

    impl FakeLauncher {
        fn success(counters: &(Arc<AtomicUsize>, Arc<AtomicUsize>, Arc<Notify>)) -> Self {
            Self {
                aborted: counters.0.clone(),
                completed: counters.1.clone(),
                done: counters.2.clone(),
                fail_launch: false,
            }
        }
        fn failing(done: Arc<Notify>) -> Self {
            Self {
                aborted: Arc::new(AtomicUsize::new(0)),
                completed: Arc::new(AtomicUsize::new(0)),
                done,
                fail_launch: true,
            }
        }
    }

    impl SessionLauncher for FakeLauncher {
        type Session = FakeSession;

        async fn launch<S>(
            self,
            _request: AgentRunRequest,
            _run_id: String,
            _sink: S,
        ) -> Result<LaunchedSession<FakeSession>>
        where
            S: RunEventSink,
        {
            if self.fail_launch {
                self.done.notify_one();
                return Err(anyhow!("launch failed"));
            }
            Ok(LaunchedSession {
                session: Arc::new(FakeSession),
                commander: Box::new(FakeCommander {
                    aborted: self.aborted,
                    completed: self.completed,
                    done: self.done,
                }),
            })
        }
    }

    fn counters() -> (Arc<AtomicUsize>, Arc<AtomicUsize>, Arc<Notify>) {
        (
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(Notify::new()),
        )
    }

    fn make_request() -> AgentRunRequest {
        AgentRunRequest {
            goal: "hello".into(),
            agent_id: "agent".into(),
            workspace_id: None,
            checkout_id: None,
            cwd: None,
            agent_command: None,
            stdio_buffer_limit_mb: None,
            auto_allow: None,
            permission_mode: None,
            run_id: Some("run-1".into()),
            resume_session_id: None,
            resume_policy: None,
            ralph_loop: None,
        }
    }

    #[tokio::test]
    async fn driver_runs_when_launch_and_attach_succeed() {
        let registry = FakeRegistry::default();
        let sink = CollectingSink::default();
        let c = counters();
        let launcher = FakeLauncher::success(&c);

        let run = StartAgentRunUseCase::new(registry.clone())
            .execute(
                launcher,
                sink.clone(),
                make_request(),
                Some("workbench-a".into()),
            )
            .await
            .expect("start should succeed");

        c.2.notified().await;
        let mut state = registry.inner.lock().await;
        assert_eq!(
            state.owners.get(&run.id).and_then(|value| value.as_deref()),
            Some("workbench-a")
        );
        let handle = state.handles.remove(&run.id).expect("handle stored");
        drop(state);
        handle.await.expect("task should finish");

        assert_eq!(c.1.load(Ordering::SeqCst), 1);
        assert_eq!(c.0.load(Ordering::SeqCst), 0);
        let state = registry.inner.lock().await;
        assert_eq!(state.reserved, vec!["run-1".to_string()]);
        assert_eq!(state.finished, vec!["run-1".to_string()]);
    }

    #[tokio::test]
    async fn returns_typed_error_when_reserve_run_fails() {
        let registry = FakeRegistry::with_failing_reserve_run(ReserveRunError::DuplicateRunId {
            run_id: "run-1".into(),
        })
        .await;
        let sink = CollectingSink::default();
        let c = counters();
        let launcher = FakeLauncher::success(&c);

        let result = StartAgentRunUseCase::new(registry.clone())
            .execute(launcher, sink, make_request(), None)
            .await;

        assert!(matches!(
            result,
            Err(StartAgentRunError::ReserveRun(ReserveRunError::DuplicateRunId { run_id }))
                if run_id == "run-1"
        ));
        let state = registry.inner.lock().await;
        assert!(state.reserved.is_empty());
        assert!(state.handles.is_empty());
    }

    #[tokio::test]
    async fn returns_typed_error_when_reserve_run_hits_concurrent_limit() {
        let registry =
            FakeRegistry::with_failing_reserve_run(ReserveRunError::ConcurrentLimit { limit: 1 })
                .await;
        let sink = CollectingSink::default();
        let c = counters();
        let launcher = FakeLauncher::success(&c);

        let result = StartAgentRunUseCase::new(registry.clone())
            .execute(launcher, sink, make_request(), None)
            .await;

        assert!(matches!(
            result,
            Err(StartAgentRunError::ReserveRun(
                ReserveRunError::ConcurrentLimit { limit: 1 }
            ))
        ));
        let state = registry.inner.lock().await;
        assert!(state.reserved.is_empty());
        assert!(state.handles.is_empty());
    }

    #[tokio::test]
    async fn returns_typed_error_when_attach_run_handle_fails() {
        let registry = FakeRegistry::with_failing_attach_run_handle().await;
        let sink = CollectingSink::default();
        let c = counters();
        let launcher = FakeLauncher::success(&c);

        let result = StartAgentRunUseCase::new(registry.clone())
            .execute(launcher, sink, make_request(), None)
            .await;

        assert!(matches!(
            result,
            Err(StartAgentRunError::AttachRunHandle(message))
                if message == "simulated attach_run_handle failure"
        ));
        let state = registry.inner.lock().await;
        assert_eq!(state.reserved, vec!["run-1".to_string()]);
        assert!(state.handles.is_empty());
    }

    #[tokio::test]
    async fn aborter_runs_when_attach_session_fails() {
        let registry = FakeRegistry::with_failing_attach().await;
        let sink = CollectingSink::default();
        let c = counters();
        let launcher = FakeLauncher::success(&c);

        let run = StartAgentRunUseCase::new(registry.clone())
            .execute(launcher, sink.clone(), make_request(), None)
            .await
            .expect("start call itself should succeed");

        c.2.notified().await;
        let handle = registry
            .inner
            .lock()
            .await
            .handles
            .remove(&run.id)
            .expect("handle stored");
        handle.await.expect("task should finish");

        assert_eq!(c.0.load(Ordering::SeqCst), 1);
        assert_eq!(c.1.load(Ordering::SeqCst), 0);
        let events = sink.events.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|(_, event)| matches!(event, RunEvent::Diagnostic { .. }))
        );
    }

    #[tokio::test]
    async fn error_during_launch_emits_run_error_and_finishes_run() {
        let registry = FakeRegistry::default();
        let sink = CollectingSink::default();
        let done = Arc::new(Notify::new());
        let launcher = FakeLauncher::failing(done.clone());

        let run = StartAgentRunUseCase::new(registry.clone())
            .execute(launcher, sink.clone(), make_request(), None)
            .await
            .expect("start call itself should succeed");

        done.notified().await;
        let handle = registry
            .inner
            .lock()
            .await
            .handles
            .remove(&run.id)
            .expect("handle stored");
        handle.await.expect("task should finish");

        let events = sink.events.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|(_, event)| matches!(event, RunEvent::Error { .. }))
        );
        assert!(!events.iter().any(|(_, event)| matches!(
            event,
            RunEvent::Lifecycle {
                status: LifecycleStatus::Completed,
                ..
            }
        )));
        let state = registry.inner.lock().await;
        assert_eq!(state.finished, vec!["run-1".to_string()]);
    }
}
