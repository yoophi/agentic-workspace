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

    /// Attempt to inject a steer prompt into the currently active turn.
    /// Adapters that cannot provide cancel-free steering should return an
    /// error instead of cancelling or restarting the run.
    fn steer_prompt<S>(&self, _sink: S, _text: String) -> impl Future<Output = Result<()>> + Send
    where
        S: RunEventSink,
    {
        async { anyhow::bail!("active-turn steer is not supported by this session") }
    }

    /// Ask the currently active prompt request to cancel, then send a
    /// replacement prompt on the same session after the adapter is ready.
    fn cancel_current_prompt_and_send<S>(
        &self,
        _sink: S,
        _text: String,
    ) -> impl Future<Output = Result<String>> + Send
    where
        S: RunEventSink,
    {
        async { anyhow::bail!("current prompt cancellation is not supported by this session") }
    }

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
