use std::sync::Arc;

use serde_json::json;
use tauri::{AppHandle, Manager};
use workbench_core::{
    application::workbench_runtime::{WorkbenchRuntime, is_terminal_run_event},
    ports::event_publisher::RunEventPublisher,
};
use workbench_protocol::EventEnvelope;

use crate::{
    application::orchestration_service::OrchestrationService,
    domain::events::RunEvent,
    infrastructure::{
        acp_agent_worker_adapter::{take_worktree_guard, verify_worktree_unchanged},
        agent_session_registry::AppState,
        json_orchestration_repository::JsonOrchestrationRepository,
        tauri_orchestration_event_sink::TauriOrchestrationEventSink,
    },
    ports::event_sink::RunEventSink,
};

/// 화면이 듣는 창 삽입 이벤트 이름(ADR 0004: run 이벤트는 이 경로 하나만 남긴다).
pub const AGENT_RUN_EVENT_FALLBACK: &str = "agent-run-event-fallback";

#[derive(Clone)]
pub struct TauriRunEventSink {
    app: AppHandle,
    /// 이벤트를 전달할 대상 창 레이블(sink를 만든 세션 창). 창 격리는 창 삽입(`eval`)이 보장한다 —
    /// Tauri 2의 `WebviewWindow::emit`은 전체 방송이므로 쓰지 않는다.
    target_label: String,
}

impl TauriRunEventSink {
    pub fn with_target(app: AppHandle, _state: AppState, target_label: String) -> Self {
        Self { app, target_label }
    }
}

/// 창 삽입 payload: 공유 타입 `RunEventEnvelope {runId, event}`의 상위집합(research R6).
/// 화면은 `sequence`로 순번을 쓰고(추정 없음), 패널은 추가 필드를 무시한다.
pub fn delivered_payload(run_id: &str, envelope: &EventEnvelope) -> serde_json::Value {
    json!({
        "runId": run_id,
        "event": envelope.body,
        "sequence": envelope.sequence,
        "epoch": envelope.epoch,
        "streamId": envelope.stream_id,
        "eventId": envelope.event_id,
    })
}

pub fn delivery_script(payload: &serde_json::Value) -> String {
    format!(
        "window.dispatchEvent(new CustomEvent('{AGENT_RUN_EVENT_FALLBACK}', {{ detail: {payload} }}));"
    )
}

impl RunEventSink for TauriRunEventSink {
    fn emit(&self, run_id: &str, event: RunEvent) {
        let terminal = is_terminal_run_event(&event);
        if let Some(runtime) = self.app.try_state::<Arc<WorkbenchRuntime>>() {
            let window = self.app.get_webview_window(&self.target_label);
            // 순번 부여와 창 전달을 같은 스트림 lock 안에서 한다: 여러 발행자가 동시에 발행해도 창 도착 순서 = 순번 순서.
            // `eval`은 webview 대기열에 넣고 바로 돌아온다(막히지 않음). 창이 없으면 전달하지 않는다.
            runtime.publish_run(run_id, &event, terminal, &mut |envelope| {
                if let Some(window) = &window {
                    let _ = window.eval(delivery_script(&delivered_payload(run_id, envelope)));
                }
            });
        }
        // worktree 변경 검사는 발행 lock 밖에서(오늘과 같은 동작).
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
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use workbench_protocol::EventEnvelope;

    use super::*;
    use crate::domain::events::{RunEvent, RunEventEnvelope};

    #[test]
    fn delivered_payload_is_a_superset_of_the_shared_envelope() {
        let event = RunEvent::AgentMessage { text: "hi".into() };
        let envelope = EventEnvelope {
            event_id: "evt-1".into(),
            stream_id: "run:r1".into(),
            epoch: "e1".into(),
            sequence: 7,
            schema: "run.event.v1".into(),
            occurred_at: "2026-09-27T00:00:00Z".into(),
            correlation_id: None,
            body: serde_json::to_value(&event).unwrap(),
        };
        let payload = delivered_payload("r1", &envelope);
        assert_eq!(
            payload,
            json!({"runId": "r1", "event": {"type": "agentMessage", "text": "hi"}, "sequence": 7, "epoch": "e1", "streamId": "run:r1", "eventId": "evt-1"})
        );
        // 이전 payload(`RunEventEnvelope`)의 필드는 그대로다.
        let legacy = serde_json::to_value(RunEventEnvelope {
            run_id: "r1".into(),
            event,
        })
        .unwrap();
        for (key, value) in legacy.as_object().unwrap() {
            assert_eq!(&payload[key], value, "{key}");
        }
        let script = delivery_script(&payload);
        assert!(script.starts_with(
            "window.dispatchEvent(new CustomEvent('agent-run-event-fallback', { detail: {"
        ));
    }
}
