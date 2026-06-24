use std::future::Future;

use anyhow::Result;

use crate::{domain::run::PermissionMode, ports::event_sink::RunEventSink};

/// Behavior the application layer needs from a launched agent session.
///
/// The session registry stores opaque `Arc<Session>` handles; this port
/// is what turns that handle into a prompt-capable entity so use cases
/// like `SendPromptUseCase` do not have to know about the ACP adapter.
pub trait SessionHandle: Send + Sync + 'static {
    /// Send a prompt to the session and return the stop reason reported
    /// by the agent. Streaming events are emitted through `sink` as
    /// they arrive.
    fn send_prompt<S>(&self, sink: S, text: String) -> impl Future<Output = Result<String>> + Send
    where
        S: RunEventSink;

    /// Apply a new permission mode to the already-running session so the
    /// agent's next tool/command approval policy reflects the change
    /// without restarting the run. Progress is reported through `sink`.
    fn set_permission_mode<S>(
        &self,
        sink: S,
        mode: PermissionMode,
    ) -> impl Future<Output = Result<()>> + Send
    where
        S: RunEventSink;
}
