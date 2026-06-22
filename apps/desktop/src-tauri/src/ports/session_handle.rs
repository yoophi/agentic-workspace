use std::future::Future;

use anyhow::Result;

use crate::ports::event_sink::RunEventSink;

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
}
