//! 작업대 단위 run 이벤트 sink(040, research R4). run 스트림에 발행하고, 같은 스트림 lock 안에서 발행 결과를
//! 작업대의 창으로 넘긴다(039 ADR 0003·0004를 작업대 단위로). 종료 이벤트 뒤 후처리는 lock 밖에서 한다.

use std::sync::Arc;

use acp_agent_core::{domain::events::RunEvent, ports::event_sink::RunEventSink};
use serde_json::{json, Value};
use workbench_protocol::{
    events::{StreamKind, RUN_EVENT_V1},
    EventEnvelope,
};

use crate::{
    application::workbench_runtime::is_terminal_run_event,
    infrastructure::event_hub::EventHub,
    ports::desktop_bridge::{DesktopBridge, DesktopDelivery, RunTerminalHook},
};

#[derive(Clone)]
pub struct WorkbenchRunSink {
    bench_id: String,
    hub: Arc<EventHub>,
    desktop: Option<Arc<dyn DesktopBridge>>,
    terminal_hook: Option<Arc<dyn RunTerminalHook>>,
}

impl WorkbenchRunSink {
    pub fn new(
        bench_id: impl Into<String>,
        hub: Arc<EventHub>,
        desktop: Option<Arc<dyn DesktopBridge>>,
        terminal_hook: Option<Arc<dyn RunTerminalHook>>,
    ) -> Self {
        Self {
            bench_id: bench_id.into(),
            hub,
            desktop,
            terminal_hook,
        }
    }

    pub fn bench_id(&self) -> &str {
        &self.bench_id
    }
}

/// 창이 받는 run payload: 공유 봉투 `{runId, event}`의 상위 집합(039 research R6과 같은 모양).
pub fn delivered_payload(run_id: &str, envelope: &EventEnvelope) -> Value {
    json!({
        "runId": run_id,
        "event": envelope.body,
        "sequence": envelope.sequence,
        "epoch": envelope.epoch,
        "streamId": envelope.stream_id,
        "eventId": envelope.event_id,
    })
}

impl RunEventSink for WorkbenchRunSink {
    fn emit(&self, run_id: &str, event: RunEvent) {
        let terminal = is_terminal_run_event(&event);
        let body = serde_json::to_value(&event).expect("run event serializes");
        let bench_id = &self.bench_id;
        let desktop = &self.desktop;
        // 소유 확정(research R17): 이 sink의 작업대가 엔진이 준 소유자다. 기동 경로가 claim하지 않은 run도 여기서 남는다.
        self.hub.assign_run_owner(run_id, bench_id);
        self.hub.publish_state(
            StreamKind::Run,
            run_id,
            RUN_EVENT_V1,
            body,
            terminal,
            &mut |envelope| {
                if let Some(desktop) = desktop {
                    desktop.deliver(DesktopDelivery::Run {
                        bench_id: bench_id.clone(),
                        payload: delivered_payload(run_id, envelope),
                    });
                }
            },
        );
        if terminal {
            if let Some(hook) = &self.terminal_hook {
                hook.on_terminal(run_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::infrastructure::event_hub::EventHubLimits;
    use acp_agent_core::domain::events::LifecycleStatus;

    #[derive(Default)]
    struct Recorder {
        deliveries: Mutex<Vec<DesktopDelivery>>,
        terminals: Mutex<Vec<String>>,
    }

    impl DesktopBridge for Recorder {
        fn deliver(&self, delivery: DesktopDelivery) {
            self.deliveries.lock().unwrap().push(delivery);
        }
    }

    impl RunTerminalHook for Recorder {
        fn on_terminal(&self, run_id: &str) {
            self.terminals.lock().unwrap().push(run_id.to_owned());
        }
    }

    #[test]
    fn emits_to_the_run_stream_and_delivers_to_the_bench_with_sequence() {
        let hub = EventHub::new("e1", EventHubLimits::default());
        let recorder = Arc::new(Recorder::default());
        let sink = WorkbenchRunSink::new(
            "b1",
            Arc::clone(&hub),
            Some(recorder.clone() as Arc<dyn DesktopBridge>),
            Some(recorder.clone() as Arc<dyn RunTerminalHook>),
        );
        sink.emit("r1", RunEvent::AgentMessage { text: "hi".into() });
        sink.emit(
            "r1",
            RunEvent::Lifecycle {
                status: LifecycleStatus::Completed,
                message: "done".into(),
            },
        );
        let deliveries = recorder.deliveries.lock().unwrap();
        assert_eq!(deliveries.len(), 2);
        let DesktopDelivery::Run { bench_id, payload } = &deliveries[0] else {
            panic!("run delivery expected");
        };
        assert_eq!(bench_id, "b1");
        assert_eq!(payload["runId"], "r1");
        assert_eq!(payload["sequence"], 1);
        assert_eq!(
            payload["event"],
            json!({"type": "agentMessage", "text": "hi"})
        );
        assert_eq!(payload["streamId"], "run:r1");
        assert_eq!(
            recorder.terminals.lock().unwrap().as_slice(),
            ["r1".to_owned()]
        );
        assert_eq!(hub.replay_run("r1", 0).last_sequence, 2);
    }
}
