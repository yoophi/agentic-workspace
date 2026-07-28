use tauri::{AppHandle, Emitter, Manager};

use crate::{
    application::orchestration_service::OrchestrationService,
    domain::events::{LifecycleStatus, RunEvent, RunEventEnvelope},
    infrastructure::{
        acp_agent_worker_adapter::{take_worktree_guard, verify_worktree_unchanged},
        agent_session_registry::AppState,
        in_memory_runtime_event_journal::InMemoryRuntimeEventJournal,
        json_orchestration_repository::JsonOrchestrationRepository,
        tauri_orchestration_event_sink::TauriOrchestrationEventSink,
    },
    ports::{event_sink::RunEventSink, runtime_event_journal::RuntimeEventJournal},
};

pub const AGENT_RUN_EVENT: &str = "agent-run-event";

#[derive(Clone)]
pub struct TauriRunEventSink {
    app: AppHandle,
    /// 이벤트를 전달할 대상 창 레이블. `Some`이면 그 창에만, `None`이면 전체 창에 emit.
    target_label: Option<String>,
}

impl TauriRunEventSink {
    /// 특정 창(세션 창)에만 이벤트를 전달하는 sink. 멀티 윈도우에서 창 간
    /// 이벤트가 섞이지 않도록 `emit_to(label, ...)`로 격리한다.
    pub fn with_target(app: AppHandle, _state: AppState, target_label: String) -> Self {
        Self {
            app,
            target_label: Some(target_label),
        }
    }
}

impl RunEventSink for TauriRunEventSink {
    fn emit(&self, run_id: &str, event: RunEvent) {
        let terminal = matches!(
            &event,
            RunEvent::Lifecycle {
                status: LifecycleStatus::Completed | LifecycleStatus::Cancelled,
                ..
            }
        );
        if let Some(journal) = self.app.try_state::<InMemoryRuntimeEventJournal>()
            && let Ok(value) = serde_json::to_value(&event)
        {
            journal.append(run_id, value, terminal);
        }
        if terminal
            && let Some(guard) = take_worktree_guard(run_id)
            && let Err(violation) = verify_worktree_unchanged(&guard.worktree_path, &guard.baseline)
            && let Ok(repository) = JsonOrchestrationRepository::from_app(&self.app)
        {
            let service = OrchestrationService::new(
                repository,
                TauriOrchestrationEventSink::new(self.app.clone()),
            );
            let _ = service.fail_task_for_runtime(
                &guard.window_label,
                &guard.task_id,
                &guard.node_id,
                violation.code,
                &violation.message,
            );
        }
        let envelope = RunEventEnvelope {
            run_id: run_id.to_string(),
            event,
        };
        let fallback_script = serde_json::to_string(&envelope).ok().map(|payload| {
            format!(
                "window.dispatchEvent(new CustomEvent('agent-run-event-fallback', {{ detail: {payload} }}));"
            )
        });
        match &self.target_label {
            Some(label) => {
                if let Some(window) = self.app.get_webview_window(label) {
                    let _ = window.emit(AGENT_RUN_EVENT, envelope);
                    if let Some(script) = fallback_script {
                        let _ = window.eval(script);
                    }
                } else {
                    let _ = self.app.emit_to(label.as_str(), AGENT_RUN_EVENT, envelope);
                }
            }
            None => {
                let _ = self.app.emit(AGENT_RUN_EVENT, envelope);
            }
        }
    }
}
