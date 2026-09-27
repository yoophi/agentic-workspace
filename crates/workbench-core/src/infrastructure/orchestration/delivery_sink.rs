//! orchestration 갱신 sink(041 research R10). 작업 영역의 **현재 묶임** 스트림(`orchestration:<bindingId>`)에 발행하고,
//! 같은 스트림 lock 안에서 데스크톱 창에 한 번 전달한다(오늘의 전체 창 방송과 이중 전달을 없앤다). 발행은 저장
//! 성공 뒤·binding mutex 밖에서 일어나므로, 그 사이 묶임이 바뀌었으면(다른 작업대이거나 풀림) 버린다 — 새 묶임의
//! 구독자는 `orchestration.get`으로 따라잡는다.

use std::sync::Arc;

use workbench_protocol::events::{StreamKind, ORCHESTRATION_WORKSPACE_UPDATED_V1};

use crate::{
    application::orchestration::binding::OrchestrationBindings,
    domain::agent_orchestration::OrchestrationError,
    infrastructure::event_hub::EventHub,
    ports::{
        desktop_bridge::{DesktopBridge, DesktopDelivery},
        orchestration_event_sink::{OrchestrationEvent, OrchestrationEventSink},
    },
};

#[derive(Clone)]
pub struct DeliveryOrchestrationSink {
    hub: Arc<EventHub>,
    bindings: Arc<OrchestrationBindings>,
    desktop: Option<Arc<dyn DesktopBridge>>,
}

impl DeliveryOrchestrationSink {
    pub fn new(
        hub: Arc<EventHub>,
        bindings: Arc<OrchestrationBindings>,
        desktop: Option<Arc<dyn DesktopBridge>>,
    ) -> Self {
        Self {
            hub,
            bindings,
            desktop,
        }
    }
}

impl OrchestrationEventSink for DeliveryOrchestrationSink {
    fn emit(&self, bench_id: &str, event: OrchestrationEvent) -> Result<(), OrchestrationError> {
        let Some(binding) = self.bindings.binding_of(&event.workspace_id) else {
            return Ok(());
        };
        if binding.bench_id != bench_id {
            return Ok(());
        }
        let payload = serde_json::to_value(&event).unwrap_or_default();
        let desktop = &self.desktop;
        self.hub.publish_state(
            StreamKind::Orchestration,
            &binding.binding_id,
            ORCHESTRATION_WORKSPACE_UPDATED_V1,
            payload.clone(),
            false,
            &mut |envelope| {
                if let Some(desktop) = desktop {
                    // 창 payload: 오늘 `OrchestrationEvent` + 스트림 위치(run 전달과 같은 필드).
                    let mut delivered = payload.clone();
                    if let Some(object) = delivered.as_object_mut() {
                        object.insert("sequence".into(), envelope.sequence.into());
                        object.insert("epoch".into(), envelope.epoch.clone().into());
                        object.insert("streamId".into(), envelope.stream_id.clone().into());
                        object.insert("eventId".into(), envelope.event_id.clone().into());
                    }
                    desktop.deliver(DesktopDelivery::Orchestration {
                        bench_id: bench_id.to_owned(),
                        payload: delivered,
                    });
                }
            },
        );
        Ok(())
    }
}
