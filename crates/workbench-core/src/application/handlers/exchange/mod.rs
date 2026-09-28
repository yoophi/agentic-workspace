//! `exchange.*` handler(040 US2). 작업 영역을 바꾸는 동작(동기화·전송)은 작업대 입장권을 잡는다(닫기와 직렬화).
//! agent 전용 operation은 principal 주체의 run이 입력 `runId`와 같아야 한다(ADR 0006).

use std::sync::Arc;

use workbench_protocol::{
    operations::exchange::{
        AgentExchangeDto, AgentExchangeStatusDto, AgentPanelEndpointDto, AgentPeersDto,
        AgentWorkspaceSyncResponseDto, ExchangeAcknowledgeInput, ExchangeDiscardDeliveryInput,
        ExchangeGetForRunInput, ExchangeListInput, ExchangeListPeersInput,
        ExchangeSendFromRunInput, ExchangeSendInput, ExchangeSyncWorkspaceInput,
    },
    AuthenticatedPrincipal, FaultCode, OperationId, RequestId, WorkbenchFault,
};

use crate::{
    application::{
        bench_service::{BenchServices, MESSAGE_NOT_DIRECTORY},
        handlers::epoch::{async_query_handler, epoch_handler, to_json, Scope},
        registry::Registry,
        run_dto::convert,
        run_service::MESSAGE_EXCHANGE_NOT_FOUND,
    },
    domain::agent_exchange::{
        AgentExchange, AgentExchangeAckRequest, AgentExchangeError, AgentWorkspaceSyncRequest,
    },
};

pub const MESSAGE_RUN_MISMATCH: &str =
    "The requested run does not match the authenticated capability.";
pub const MESSAGE_EXCHANGE_DELIVERY_STARTED: &str =
    "The exchange delivery already started and can no longer be rejected.";

/// 도메인 오류 → fault. `message`는 도메인 문구, 코드는 `details.exchangeCode`(compat이 오늘 JSON을 다시 만든다).
pub fn exchange_fault(request_id: &RequestId, error: AgentExchangeError) -> WorkbenchFault {
    let code = match error.code.as_str() {
        "unknownSource" | "unknownTarget" | "unknownWorkspace" | "unknownExchange" => {
            FaultCode::NotFound
        }
        "staleSourceRun" | "staleTargetRun" | "targetClosing" => FaultCode::PreconditionFailed,
        "duplicateConflict" | "invalidTransition" => FaultCode::Conflict,
        "windowUnavailable" | "deliveryFailed" => FaultCode::Unavailable,
        _ => FaultCode::InvalidArgument,
    };
    WorkbenchFault::new(code, request_id.clone(), error.message)
        .with_details(serde_json::json!({ "exchangeCode": error.code }))
}

fn exchange_json(exchange: &AgentExchange) -> serde_json::Value {
    to_json(convert::<_, AgentExchangeDto>(exchange))
}

fn ensure_agent_run(
    request_id: &RequestId,
    principal: &AuthenticatedPrincipal,
    run_id: &str,
) -> Result<(), WorkbenchFault> {
    if principal.agent_run_id() == Some(run_id) {
        Ok(())
    } else {
        Err(WorkbenchFault::new(
            FaultCode::Forbidden,
            request_id.clone(),
            MESSAGE_RUN_MISMATCH,
        ))
    }
}

/// 오늘 `sync_agent_workspace` command가 하던 경로 정규화(문구 동일, 교환 오류 JSON이 아닌 평문).
fn canonical_worktree(request_id: &RequestId, path: &str) -> Result<String, WorkbenchFault> {
    let canonical = std::fs::canonicalize(path).map_err(|error| {
        WorkbenchFault::invalid_argument(
            request_id.clone(),
            format!("Failed to resolve workspace path: {error}"),
            Some("/request/worktreePath"),
        )
    })?;
    if !canonical.is_dir() {
        return Err(WorkbenchFault::invalid_argument(
            request_id.clone(),
            MESSAGE_NOT_DIRECTORY,
            Some("/request/worktreePath"),
        ));
    }
    Ok(canonical.to_string_lossy().into_owned())
}

pub fn register(registry: &mut Registry, services: &Arc<BenchServices>) {
    registry.register(
        OperationId::ExchangeSyncWorkspace,
        epoch_handler(
            OperationId::ExchangeSyncWorkspace,
            services,
            |input: &ExchangeSyncWorkspaceInput| Scope::Bench(input.bench_id.clone()),
            |services, ctx, input: ExchangeSyncWorkspaceInput| async move {
                let admission =
                    services.admit(&ctx.request_id, Some(&ctx.principal), &input.bench_id)?;
                let mut request: AgentWorkspaceSyncRequest = convert(&input.request);
                request.worktree_path =
                    canonical_worktree(&ctx.request_id, &request.worktree_path)?;
                let response = services
                    .exchange_service()
                    .sync_workspace(admission.bench.id.clone(), request)
                    .await
                    .map_err(|error| exchange_fault(&ctx.request_id, error))?;
                drop(admission);
                Ok(to_json(convert::<_, AgentWorkspaceSyncResponseDto>(
                    &response,
                )))
            },
        ),
    );
    registry.register(
        OperationId::ExchangeSend,
        epoch_handler(
            OperationId::ExchangeSend,
            services,
            |input: &ExchangeSendInput| Scope::Bench(input.bench_id.clone()),
            |services, ctx, input: ExchangeSendInput| async move {
                let admission =
                    services.admit(&ctx.request_id, Some(&ctx.principal), &input.bench_id)?;
                let exchange = services
                    .exchange_service()
                    .send_user_exchange(&admission.bench.id, convert(&input.request))
                    .await
                    .map_err(|error| exchange_fault(&ctx.request_id, error))?;
                drop(admission);
                Ok(exchange_json(&exchange))
            },
        ),
    );
    registry.register(
        OperationId::ExchangeAcknowledge,
        epoch_handler(
            OperationId::ExchangeAcknowledge,
            services,
            |input: &ExchangeAcknowledgeInput| Scope::Bench(input.bench_id.clone()),
            |services, ctx, input: ExchangeAcknowledgeInput| async move {
                let bench = services.resolve(&ctx.request_id, &ctx.principal, &input.bench_id)?;
                // Rejected/failed/cancelled 확인과 전달 소비는 같은 work-gate 잠금에서 승자를 정한다. delivery가 먼저 소비했으면
                // 뒤늦은 거절이 Accepted를 Rejected로 바꿔 이미 시작한 prompt와 모순되지 않게 거절한다.
                if input.request.outcome != AgentExchangeStatusDto::Delivered {
                    let current = services
                        .exchange_service()
                        .list_exchanges(&bench.id)
                        .await
                        .into_iter()
                        .find(|exchange| exchange.request_id == input.request.request_id)
                        .ok_or_else(|| {
                            WorkbenchFault::new(
                                FaultCode::NotFound,
                                ctx.request_id.clone(),
                                MESSAGE_EXCHANGE_NOT_FOUND,
                            )
                        })?;
                    let requested: AgentExchangeAckRequest = convert(&input.request);
                    let requested = requested.outcome;
                    if current.status != requested {
                        if current.target.panel_id != input.request.target_panel_id {
                            return Err(WorkbenchFault::new(
                                FaultCode::NotFound,
                                ctx.request_id.clone(),
                                "Acknowledgement target does not match the exchange.",
                            ));
                        }
                        let rejection_claim = match services.work_gate() {
                            Some(gate) => Some(
                                gate.begin_exchange_rejection(&bench.id, &input.request.request_id)
                                    .ok_or_else(|| {
                                        WorkbenchFault::conflict(
                                            ctx.request_id.clone(),
                                            MESSAGE_EXCHANGE_DELIVERY_STARTED,
                                            workbench_protocol::Outcome::NotApplied,
                                        )
                                    })?,
                            ),
                            None => None,
                        };
                        #[cfg(feature = "test-hooks")]
                        {
                            let probe = services
                                .exchange_rejection_store_probe
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner())
                                .clone();
                            if let Some(probe) = probe {
                                probe().map_err(|message| {
                                    WorkbenchFault::internal(ctx.request_id.clone(), message)
                                })?;
                            }
                        }
                        let exchange = services
                            .exchange_service()
                            .acknowledge(&bench.id, convert(&input.request))
                            .await
                            .map_err(|error| exchange_fault(&ctx.request_id, error))?;
                        if let Some(rejection_claim) = rejection_claim {
                            rejection_claim.commit();
                        }
                        return Ok(exchange_json(&exchange));
                    }
                }
                let exchange = services
                    .exchange_service()
                    .acknowledge(&bench.id, convert(&input.request))
                    .await
                    .map_err(|error| exchange_fault(&ctx.request_id, error))?;
                Ok(exchange_json(&exchange))
            },
        ),
    );
    registry.register(
        OperationId::ExchangeDiscardDelivery,
        epoch_handler(
            OperationId::ExchangeDiscardDelivery,
            services,
            |input: &ExchangeDiscardDeliveryInput| Scope::Bench(input.bench_id.clone()),
            |services, ctx, input: ExchangeDiscardDeliveryInput| async move {
                // 화면 대기열에서 지운 교환(Codex r7): 확인했지만 run에 보내지 않은 교환의 전달을 포기한다. 이 작업대의
                // 교환이어야 한다(요청 id는 작업대마다 겹칠 수 있다).
                let bench = services.resolve(&ctx.request_id, &ctx.principal, &input.bench_id)?;
                let known = services
                    .exchange_service()
                    .list_exchanges(&bench.id)
                    .await
                    .into_iter()
                    .any(|exchange| exchange.request_id == input.request_id);
                if !known {
                    return Err(WorkbenchFault::new(
                        FaultCode::NotFound,
                        ctx.request_id.clone(),
                        MESSAGE_EXCHANGE_NOT_FOUND,
                    ));
                }
                // 이미 전달·포기된 교환이면 효과 없이 성공한다(멱등). 관문이 없는 조립(embedded)에는 셀 활동이 없다.
                if let Some(gate) = services.work_gate() {
                    gate.discard_exchange(&bench.id, &input.request_id);
                }
                Ok(serde_json::Value::Null)
            },
        ),
    );
    registry.register(
        OperationId::ExchangeList,
        async_query_handler(
            services,
            |services, ctx, input: ExchangeListInput| async move {
                let bench = services.resolve(&ctx.request_id, &ctx.principal, &input.bench_id)?;
                let exchanges = services.exchange_service().list_exchanges(&bench.id).await;
                Ok(serde_json::Value::Array(
                    exchanges.iter().map(exchange_json).collect(),
                ))
            },
        ),
    );
    registry.register(
        OperationId::ExchangeListPeers,
        async_query_handler(
            services,
            |services, ctx, input: ExchangeListPeersInput| async move {
                ensure_agent_run(&ctx.request_id, &ctx.principal, &input.run_id)?;
                let peers = services
                    .exchange_service()
                    .list_peers_for_run(&input.run_id)
                    .await
                    .map_err(|error| exchange_fault(&ctx.request_id, error))?;
                Ok(to_json(AgentPeersDto {
                    peers: peers
                        .iter()
                        .map(convert::<_, AgentPanelEndpointDto>)
                        .collect(),
                }))
            },
        ),
    );
    registry.register(
        OperationId::ExchangeSendFromRun,
        epoch_handler(
            OperationId::ExchangeSendFromRun,
            services,
            |input: &ExchangeSendFromRunInput| Scope::RunOwner(input.run_id.clone()),
            |services, ctx, input: ExchangeSendFromRunInput| async move {
                ensure_agent_run(&ctx.request_id, &ctx.principal, &input.run_id)?;
                // 출발 run의 작업대에 쓰므로 그 작업대 입장권을 전송이 끝날 때까지 잡는다(주체 검사 없음 — agent는
                // 작업대를 열지 않는다). 닫히는 중이면 입장 실패를 그대로 돌려준다: 닫기의 입장 경계 밖에서 쓰면 정리가
                // 끝난 작업대에 교환이 남을 수 있다. 소유 작업대가 없으면 서비스가 오늘 오류(작업 영역 미등록)를 낸다.
                let admission = match services.engine.active_owner_of(&input.run_id).await {
                    Some(bench) => Some(services.admit(&ctx.request_id, None, &bench)?),
                    None => None,
                };
                let exchange = services
                    .exchange_service()
                    .send_agent_exchange(&input.run_id, convert(&input.request))
                    .await
                    .map_err(|error| exchange_fault(&ctx.request_id, error))?;
                drop(admission);
                Ok(exchange_json(&exchange))
            },
        ),
    );
    registry.register(
        OperationId::ExchangeGetForRun,
        async_query_handler(
            services,
            |services, ctx, input: ExchangeGetForRunInput| async move {
                ensure_agent_run(&ctx.request_id, &ctx.principal, &input.run_id)?;
                let exchange = services
                    .exchange_service()
                    .exchange_for_source_run(&input.run_id, &input.request_id)
                    .await
                    .map_err(|error| exchange_fault(&ctx.request_id, error))?;
                Ok(exchange_json(&exchange))
            },
        ),
    );
}
