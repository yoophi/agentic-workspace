//! `run.*` handler(040). `run.start`만 변경 기록(intent-first)을 통과하고, 나머지 제어는 세대 범위 멱등성을 쓴다
//! (ADR core 0005). 소유 검사·오류 문구는 `run_service`.

use std::sync::Arc;

use async_trait::async_trait;
use workbench_protocol::{
    operations::run::{
        AgentRunDto, AgentToolCandidateResponseDto, RunCancelInput, RunListToolCandidatesInput,
        RunPromptInput, RunRespondPermissionInput, RunSetPermissionModeInput, RunStartInput,
    },
    CallReply, FaultCode, OperationId, WorkbenchFault,
};

use crate::{
    application::{
        bench_service::BenchServices,
        handlers::epoch::{async_query_handler, epoch_handler, to_json, Scope},
        intent_first::{Applied, IntentFirst, MutationSpec, Reservation},
        orchestration::runtime::OrchestrationRuntime,
        registry::{decode_input, CallContext, OperationHandler, Registry},
        run_dto::convert,
        run_service::{self, engine_fault, PromptKind},
    },
    domain::agent_orchestration::MAIN_AGENT_NODE_ID,
    infrastructure::storage_coordinator::StorageCoordinator,
    ports::desktop_bridge::LaunchContext,
};

/// 모든 `run.start`가 공유하는 aggregate. 예약·spawn까지만 lock 안이라 짧다.
pub const RUNS_AGGREGATE: &str = "runs";

struct StartHandler {
    runner: Arc<IntentFirst>,
    coordinator: Arc<StorageCoordinator>,
    services: Arc<BenchServices>,
    orchestration: Arc<OrchestrationRuntime>,
}

#[async_trait]
impl OperationHandler for StartHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let typed: RunStartInput = decode_input(&ctx.request_id, &input)?;
        // 작업대 아래 run을 등록하므로 입장권을 잡는다(닫기와 직렬화, research R1). 입장권은 `apply` closure가
        // 소유한다: apply는 blocking task에서 돌아 호출자 future가 취소돼도 끝까지 가므로, 호출자 쪽에 두면 엔진이
        // run을 등록하는 도중 닫기가 입장 경계를 지나 닫힌 작업대에 살아 있는 run이 남을 수 있다.
        let admission =
            self.services
                .admit(&ctx.request_id, Some(&ctx.principal), &typed.bench_id)?;
        let bench_id = admission.bench.id.clone();
        let mut request = run_service::normalize_run_request(convert(&typed.request));
        {
            let coordinator = Arc::clone(&self.coordinator);
            let resolved = tokio::task::spawn_blocking(move || {
                run_service::resolve_agent_command(&coordinator, &mut request).map(|()| request)
            })
            .await
            .map_err(|error| WorkbenchFault::internal(ctx.request_id.clone(), error.to_string()))?;
            request = resolved.map_err(|message| {
                WorkbenchFault::new(FaultCode::InvalidArgument, ctx.request_id.clone(), message)
            })?;
        }
        let reservation = match request.run_id.clone() {
            Some(run_id) => Reservation::CallerProvided(run_id),
            None => Reservation::ServerGenerated(Box::new(|| Ok(uuid::Uuid::new_v4().to_string()))),
        };
        let services = Arc::clone(&self.services);
        let panel_id = typed.panel_id.clone();
        let orchestration = Arc::clone(&self.orchestration);
        let runtime = tokio::runtime::Handle::current();
        let spec = MutationSpec {
            operation: OperationId::RunStart,
            aggregate: RUNS_AGGREGATE.to_owned(),
            normalized_input: input,
            reservation,
            tracks_revision: false,
            apply: Box::new(move |apply_ctx| {
                let _admission = &admission;
                let run_id = apply_ctx
                    .reserved_id
                    .expect("run.start always reserves a run id")
                    .to_owned();
                let mut request = request.clone();
                request.run_id = Some(run_id.clone());
                // research R18: 끝난 run의 id는 다시 쓰지 않는다. 같은 id가 다시 살아나면 그 id를 기록한 작업 영역(복구
                // 가능 포함)과 hub journal이 다른 run을 가리키게 된다. 멱등 재시도는 apply 전에 저장된 결과로 끝나므로
                // 여기(실제 실행)에서만 검사한다. 살아 있는 run의 중복은 엔진이 오늘 문구로 거절한다.
                if services.hub.has_run_history(&run_id) || orchestration.references_run(&run_id) {
                    return Ok(Applied::Rejected(WorkbenchFault::new(
                        FaultCode::Conflict,
                        apply_ctx.request_id.clone(),
                        format!("duplicate run id: {run_id}"),
                    )));
                }
                // 041: Main 패널 run은 작업대에 묶인 작업 영역의 활성 세대와 맞아야 한다(오늘 AW decorator의 문구·fault).
                let orchestration_role = match panel_id.as_deref() {
                    Some(MAIN_AGENT_NODE_ID) => {
                        match orchestration.coordinator_launch_role(&bench_id, &run_id) {
                            Ok(role) => Some(role),
                            Err(message) => {
                                return Ok(Applied::Rejected(WorkbenchFault::new(
                                    FaultCode::PreconditionFailed,
                                    apply_ctx.request_id.clone(),
                                    message,
                                )))
                            }
                        }
                    }
                    _ => None,
                };
                if let Some(decorator) = &services.launch_decorator {
                    let context = LaunchContext {
                        bench_id: bench_id.clone(),
                        panel_id: panel_id.clone(),
                        run_id: run_id.clone(),
                        orchestration: orchestration_role,
                    };
                    if let Err(message) = decorator.decorate(&mut request, &context) {
                        return Ok(Applied::Rejected(WorkbenchFault::new(
                            FaultCode::PreconditionFailed,
                            apply_ctx.request_id.clone(),
                            message,
                        )));
                    }
                }
                let sink = services.run_sink(&bench_id);
                // 소유 등록(research R17): 발행 전 run을 기다리는 구독도 소유 작업대로 판단한다.
                services.hub.claim_run(&run_id, &bench_id);
                match runtime.block_on(services.engine.start(request, &bench_id, sink)) {
                    Ok(run) => Ok(Applied::Ok(convert::<_, AgentRunDto>(&run))),
                    Err(error) => {
                        services.hub.release_run_claim(&run_id, &bench_id);
                        Ok(Applied::Rejected(engine_fault(apply_ctx.request_id, error)))
                    }
                }
            }),
            is_store_corrupt: |_: &WorkbenchFault| false,
            recover: Box::new(|| Ok(())),
            fault: Box::new(|_, fault| fault),
        };
        self.runner.run(ctx, spec).await
    }
}

pub fn register(
    registry: &mut Registry,
    runner: &Arc<IntentFirst>,
    coordinator: &Arc<StorageCoordinator>,
    services: &Arc<BenchServices>,
    orchestration: &Arc<OrchestrationRuntime>,
) {
    registry.register(
        OperationId::RunListToolCandidates,
        async_query_handler(
            services,
            |services, ctx, input: RunListToolCandidatesInput| async move {
                let response = run_service::list_tool_candidates(
                    &services,
                    &ctx.request_id,
                    &ctx.principal,
                    &input.bench_id,
                    convert(&input.query),
                )
                .await?;
                Ok(to_json(convert::<_, AgentToolCandidateResponseDto>(
                    &response,
                )))
            },
        ),
    );
    registry.register(
        OperationId::RunStart,
        Arc::new(StartHandler {
            runner: Arc::clone(runner),
            coordinator: Arc::clone(coordinator),
            services: Arc::clone(services),
            orchestration: Arc::clone(orchestration),
        }),
    );
    for (operation, kind) in [
        (OperationId::RunSendPrompt, PromptKind::Send),
        (OperationId::RunSteer, PromptKind::Steer),
        (OperationId::RunCancelAndSend, PromptKind::CancelAndSend),
    ] {
        registry.register(
            operation,
            epoch_handler(
                operation,
                services,
                |input: &RunPromptInput| Scope::Bench(input.bench_id.clone()),
                move |services, ctx, input: RunPromptInput| async move {
                    run_service::prompt(
                        services,
                        &ctx.request_id,
                        &ctx.principal,
                        &input.bench_id,
                        &input.run_id,
                        input.prompt,
                        kind,
                    )
                    .await
                    .map(|()| serde_json::Value::Null)
                },
            ),
        );
    }
    registry.register(
        OperationId::RunSetPermissionMode,
        epoch_handler(
            OperationId::RunSetPermissionMode,
            services,
            |input: &RunSetPermissionModeInput| Scope::Bench(input.bench_id.clone()),
            |services, ctx, input: RunSetPermissionModeInput| async move {
                run_service::set_permission_mode(
                    services,
                    &ctx.request_id,
                    &ctx.principal,
                    &input.bench_id,
                    &input.run_id,
                    convert(&input.mode),
                )
                .await
                .map(|()| serde_json::Value::Null)
            },
        ),
    );
    registry.register(
        OperationId::RunCancel,
        epoch_handler(
            OperationId::RunCancel,
            services,
            |input: &RunCancelInput| Scope::Bench(input.bench_id.clone()),
            |services, ctx, input: RunCancelInput| async move {
                run_service::cancel(
                    services,
                    &ctx.request_id,
                    &ctx.principal,
                    &input.bench_id,
                    &input.run_id,
                )
                .await
                .map(|()| serde_json::Value::Null)
            },
        ),
    );
    registry.register(
        OperationId::RunRespondPermission,
        epoch_handler(
            OperationId::RunRespondPermission,
            services,
            |input: &RunRespondPermissionInput| Scope::Bench(input.bench_id.clone()),
            |services, ctx, input: RunRespondPermissionInput| async move {
                run_service::respond_permission(
                    services,
                    &ctx.request_id,
                    &ctx.principal,
                    &input.bench_id,
                    &input.run_id,
                    &input.permission_id,
                    &input.option_id,
                )
                .await
                .map(|()| serde_json::Value::Null)
            },
        ),
    );
}
