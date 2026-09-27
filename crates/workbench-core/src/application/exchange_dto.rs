//! 교환 DTO wire parity(040 US2). core 도메인 타입을 JSON으로 옮겨 protocol DTO로 읽고 다시 쓰면 같은 JSON이어야
//! 한다(모든 variant). `AgentExchange`의 작업대 id는 wire에 없다(`windowLabel` 제거).

#[cfg(test)]
mod tests {
    use workbench_protocol::{
        events::exchange::ExchangeRequestedDto,
        operations::exchange::{
            AgentExchangeAckRequestDto, AgentExchangeDto, AgentWorkspaceSyncRequestDto,
            SendAgentExchangeRequestDto,
        },
    };

    use crate::domain::agent_exchange::{
        AgentExchange, AgentExchangeAckRequest, AgentExchangeDelivery, AgentExchangeEndpointRef,
        AgentExchangeRequestedEvent, AgentExchangeStatus, AgentPanelEndpoint, AgentPanelStatus,
        AgentWorkspaceSyncRequest, SendAgentExchangeRequest,
    };

    fn parity<D: serde::Serialize, P: serde::de::DeserializeOwned + serde::Serialize>(
        label: &str,
        domain: &D,
    ) {
        let json = serde_json::to_value(domain).unwrap();
        let dto: P = serde_json::from_value(json.clone())
            .unwrap_or_else(|error| panic!("{label}: {error}\n{json}"));
        assert_eq!(serde_json::to_value(&dto).unwrap(), json, "{label}");
    }

    fn endpoint() -> AgentExchangeEndpointRef {
        AgentExchangeEndpointRef {
            panel_id: "p".into(),
            title: "t".into(),
            run_id: Some("r".into()),
        }
    }

    #[test]
    fn exchange_wire_parity_for_every_variant() {
        for status in [
            AgentPanelStatus::Idle,
            AgentPanelStatus::Running,
            AgentPanelStatus::Closing,
        ] {
            parity::<_, AgentWorkspaceSyncRequestDto>(
                "sync",
                &AgentWorkspaceSyncRequest {
                    worktree_path: "/w".into(),
                    revision: 2,
                    focused_panel_id: "p".into(),
                    panels: vec![AgentPanelEndpoint {
                        panel_id: "p".into(),
                        title: "t".into(),
                        run_id: None,
                        status,
                    }],
                },
            );
        }
        for delivery in [
            AgentExchangeDelivery::Send,
            AgentExchangeDelivery::Queue,
            AgentExchangeDelivery::Draft,
        ] {
            parity::<_, SendAgentExchangeRequestDto>(
                "send",
                &SendAgentExchangeRequest {
                    request_id: "q".into(),
                    source_panel_id: "a".into(),
                    source_run_id: None,
                    target_panel_id: "b".into(),
                    target_run_id: Some("r".into()),
                    message: "m".into(),
                    delivery,
                },
            );
            for status in [
                AgentExchangeStatus::Pending,
                AgentExchangeStatus::Accepted,
                AgentExchangeStatus::Delivered,
                AgentExchangeStatus::Rejected,
                AgentExchangeStatus::Failed,
                AgentExchangeStatus::Cancelled,
            ] {
                let exchange = AgentExchange {
                    request_id: "q".into(),
                    bench_id: "b1".into(),
                    worktree_path: "/w".into(),
                    source: endpoint(),
                    target: endpoint(),
                    message: "m".into(),
                    delivery,
                    status,
                    failure_code: None,
                    failure_reason: Some("why".into()),
                    created_at: "c".into(),
                    updated_at: "u".into(),
                };
                let json = serde_json::to_value(&exchange).unwrap();
                assert!(json.get("benchId").is_none() && json.get("windowLabel").is_none());
                parity::<_, AgentExchangeDto>("exchange", &exchange);
                parity::<_, ExchangeRequestedDto>(
                    "requested",
                    &AgentExchangeRequestedEvent::from(&exchange),
                );
                parity::<_, AgentExchangeAckRequestDto>(
                    "ack",
                    &AgentExchangeAckRequest {
                        request_id: "q".into(),
                        target_panel_id: "b".into(),
                        outcome: status,
                        reason: None,
                    },
                );
            }
        }
    }
}
