use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::Result;

use crate::{domain::run::AgentRunRequest, ports::event_sink::RunEventSink};

/// Future that drives a launched session to its natural completion
/// (initial prompt submission, process wait, stream cleanup, terminal
/// lifecycle event).
pub type DriverFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Future that tears a launched session down without running it to
/// completion (used when the orchestrator decides to stop the run
/// before the driver is awaited).
pub type AbortFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Adapter-facing controller over a freshly launched agent session.
///
/// Exactly one of `run_to_completion` or `abort` is awaited per run
/// and both consume the commander by value.
pub trait RunCommander: Send + 'static {
    fn run_to_completion(self: Box<Self>) -> DriverFuture;
    fn abort(self: Box<Self>) -> AbortFuture;
}

/// Outcome of a successful launcher call. The session handle is shared
/// with the registry; the commander owns the process/tasks that back it.
pub struct LaunchedSession<Session>
where
    Session: Send + Sync + 'static,
{
    pub session: Arc<Session>,
    pub commander: Box<dyn RunCommander>,
}

/// Bring a session online so the application layer can attach it to
/// a `SessionRegistry` and drive it to completion.
///
/// Adapter implementors (currently only `AcpAgentRunner`) are
/// responsible for all external setup — spawning the agent subprocess,
/// initializing the ACP session, and producing a `RunCommander` that
/// can either drive the session or tear it down.
pub trait SessionLauncher: Send + Sync + 'static {
    type Session: Send + Sync + 'static;

    fn launch<S>(
        self,
        request: AgentRunRequest,
        run_id: String,
        sink: S,
    ) -> impl Future<Output = Result<LaunchedSession<Self::Session>>> + Send
    where
        S: RunEventSink;
}
