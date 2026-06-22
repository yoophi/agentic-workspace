use tauri::{AppHandle, Emitter};

use crate::{
    domain::events::{RunEvent, RunEventEnvelope},
    infrastructure::agent_session_registry::AppState,
    ports::event_sink::RunEventSink,
};

pub const AGENT_RUN_EVENT: &str = "agent-run-event";

#[derive(Clone)]
pub struct TauriRunEventSink {
    app: AppHandle,
}

impl TauriRunEventSink {
    pub fn new(app: AppHandle, _state: AppState) -> Self {
        Self { app }
    }
}

impl RunEventSink for TauriRunEventSink {
    fn emit(&self, run_id: &str, event: RunEvent) {
        let envelope = RunEventEnvelope {
            run_id: run_id.to_string(),
            event,
        };
        let _ = self.app.emit(AGENT_RUN_EVENT, envelope);
    }
}
