//! orchestration 갱신을 데스크톱에 전달하는 sink(041). 창 전달은 `DesktopBridge` 하나로만 한다(오늘의 전체 창 방송과
//! 이중 전달을 없앤다, research R10). 묶임 스트림 발행은 US3에서 이 sink에 더한다.

use std::sync::Arc;

use crate::{
    domain::agent_orchestration::OrchestrationError,
    ports::{
        desktop_bridge::{DesktopBridge, DesktopDelivery},
        orchestration_event_sink::{OrchestrationEvent, OrchestrationEventSink},
    },
};

#[derive(Clone, Default)]
pub struct DeliveryOrchestrationSink {
    desktop: Option<Arc<dyn DesktopBridge>>,
}

impl DeliveryOrchestrationSink {
    pub fn new(desktop: Option<Arc<dyn DesktopBridge>>) -> Self {
        Self { desktop }
    }
}

impl OrchestrationEventSink for DeliveryOrchestrationSink {
    fn emit(&self, bench_id: &str, event: OrchestrationEvent) -> Result<(), OrchestrationError> {
        if let Some(desktop) = &self.desktop {
            desktop.deliver(DesktopDelivery::Orchestration {
                bench_id: bench_id.to_owned(),
                payload: serde_json::to_value(&event).unwrap_or_default(),
            });
        }
        Ok(())
    }
}
