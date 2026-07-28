//! Window-scoped Tauri orchestration event sink.

use tauri::{AppHandle, Emitter, Manager};

use crate::{
    domain::agent_orchestration::{OrchestrationError, OrchestrationErrorCode},
    ports::orchestration_event_sink::{OrchestrationEvent, OrchestrationEventSink},
};

pub const ORCHESTRATION_WORKSPACE_UPDATED_EVENT: &str = "orchestration-workspace-updated";
pub const ORCHESTRATION_WORKSPACE_UPDATED_FALLBACK_EVENT: &str =
    "orchestration-workspace-updated-fallback";
pub const ORCHESTRATION_COMMAND_UPDATED_EVENT: &str = "orchestration-command-updated";
pub const ORCHESTRATION_NOTIFICATION_UPDATED_EVENT: &str =
    "orchestration-coordinator-notification-updated";

#[derive(Clone)]
pub struct TauriOrchestrationEventSink {
    app: AppHandle,
}

impl TauriOrchestrationEventSink {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl OrchestrationEventSink for TauriOrchestrationEventSink {
    fn emit(
        &self,
        window_label: &str,
        event: OrchestrationEvent,
    ) -> Result<(), OrchestrationError> {
        let window = self.app.get_webview_window(window_label).ok_or_else(|| {
            OrchestrationError::new(
                OrchestrationErrorCode::ScopeMismatch,
                "Owner Worktree Session window is unavailable.",
            )
        })?;
        window
            .emit(ORCHESTRATION_WORKSPACE_UPDATED_EVENT, &event)
            .map_err(|error| {
                OrchestrationError::new(
                    OrchestrationErrorCode::WorkerUnavailable,
                    format!("Failed to emit orchestration event: {error}"),
                )
                .retryable()
            })?;
        let detail_event = if event.reason.contains("command") || event.reason.contains("Command") {
            Some(ORCHESTRATION_COMMAND_UPDATED_EVENT)
        } else if event.reason.contains("notification") || event.reason.contains("Notification") {
            Some(ORCHESTRATION_NOTIFICATION_UPDATED_EVENT)
        } else {
            None
        };
        if let Some(event_name) = detail_event {
            let _ = window.emit(event_name, &event);
        }
        if let Ok(payload) = serde_json::to_string(&event) {
            let _ = window.eval(&format!(
                "window.dispatchEvent(new CustomEvent('{ORCHESTRATION_WORKSPACE_UPDATED_FALLBACK_EVENT}', {{ detail: {payload} }}));"
            ));
            if let Some(event_name) = detail_event {
                let _ = window.eval(&format!(
                    "window.dispatchEvent(new CustomEvent('{event_name}-fallback', {{ detail: {payload} }}));"
                ));
            }
        }
        Ok(())
    }
}
