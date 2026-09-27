//! 교환 이벤트를 작업대의 교환 스트림(`exchange:<benchId>`, 상태 복원용)에 발행하고, 같은 스트림 lock 안에서
//! 작업대의 창으로 넘긴다(040 Q5, ADR 0003·0004를 교환에 적용). 요청과 상태가 한 스트림에서 순번을 받는다.

use std::sync::Arc;

use serde_json::Value;
use workbench_protocol::{
    events::{StreamKind, EXCHANGE_REQUESTED_V1, EXCHANGE_STATUS_V1},
    EventEnvelope,
};

use crate::{
    domain::agent_exchange::{AgentExchange, AgentExchangeError, AgentExchangeRequestedEvent},
    infrastructure::event_hub::EventHub,
    ports::{
        agent_workspace_registry::AgentExchangeEventSink,
        desktop_bridge::{DesktopBridge, DesktopDelivery},
    },
};

#[derive(Clone)]
pub struct HubExchangeEventSink {
    hub: Arc<EventHub>,
    desktop: Option<Arc<dyn DesktopBridge>>,
}

/// 창 payload: 오늘 이벤트 본문 + 순번 필드(공유 봉투의 상위 집합, run과 같은 방식).
pub fn delivered_payload(body: &Value, envelope: &EventEnvelope) -> Value {
    let mut payload = body.clone();
    if let Some(object) = payload.as_object_mut() {
        object.insert("sequence".into(), envelope.sequence.into());
        object.insert("epoch".into(), envelope.epoch.clone().into());
        object.insert("streamId".into(), envelope.stream_id.clone().into());
        object.insert("eventId".into(), envelope.event_id.clone().into());
    }
    payload
}

impl HubExchangeEventSink {
    pub fn new(hub: Arc<EventHub>, desktop: Option<Arc<dyn DesktopBridge>>) -> Self {
        Self { hub, desktop }
    }

    fn publish(&self, bench_id: &str, schema: &str, body: Value, requested: bool) {
        let desktop = &self.desktop;
        self.hub.publish_state(
            StreamKind::Exchange,
            bench_id,
            schema,
            body.clone(),
            false,
            &mut |envelope| {
                if let Some(desktop) = desktop {
                    let payload = delivered_payload(&body, envelope);
                    let bench_id = bench_id.to_owned();
                    desktop.deliver(if requested {
                        DesktopDelivery::ExchangeRequested { bench_id, payload }
                    } else {
                        DesktopDelivery::ExchangeStatus { bench_id, payload }
                    });
                }
            },
        );
    }
}

impl AgentExchangeEventSink for HubExchangeEventSink {
    fn emit_requested(&self, exchange: &AgentExchange) -> Result<(), AgentExchangeError> {
        let body = serde_json::to_value(AgentExchangeRequestedEvent::from(exchange))
            .expect("requested event serializes");
        self.publish(&exchange.bench_id, EXCHANGE_REQUESTED_V1, body, true);
        Ok(())
    }

    fn emit_status(&self, exchange: &AgentExchange) -> Result<(), AgentExchangeError> {
        let body = serde_json::to_value(exchange).expect("exchange serializes");
        self.publish(&exchange.bench_id, EXCHANGE_STATUS_V1, body, false);
        Ok(())
    }
}
