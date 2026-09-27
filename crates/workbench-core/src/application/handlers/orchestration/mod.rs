//! `orchestration.*` 데스크톱 handler와 `run.replay`(041 US1). 모든 operation은 `benchId`의 작업대 주체를 검사하고
//! 그 작업대에 묶인 작업 영역만 다룬다. 흐름은 `OrchestrationRuntime`(AW command에서 옮김)에 있다. agent 도구는
//! `agent` 모듈.

mod agent;

use std::sync::Arc;

use workbench_protocol::{
    events::StreamKind,
    operations::{
        orchestration::{
            OrchestrationAdoptManualChildInput, OrchestrationBenchInput,
            OrchestrationBindCoordinatorInput, OrchestrationBootstrapInput,
            OrchestrationDelegateGoalInput, OrchestrationDispatchPromptInput,
            OrchestrationHandoffCoordinatorInput, OrchestrationListRecoverableInput,
            OrchestrationListTasksInput, OrchestrationSendChildCommandInput,
            OrchestrationSetPresentationInput, OrchestrationTaskActionInput,
        },
        orchestration_dto::{
            DelegateGoalOutcomeDto, OrchestrationSessionDto, OrchestrationTaskDto,
            PromptDispatchDto, TaskCommandDto, TaskReportDto,
        },
        run::{RunReplayDto, RunReplayInput},
    },
    FaultCode, OperationId, RequestId, WorkbenchFault,
};

use crate::{
    application::{
        bench_service::BenchServices,
        handlers::epoch::{async_query_handler, epoch_handler, to_json, Scope},
        orchestration::{
            command_service::DeliverTaskCommandRequest,
            runtime::{OrchestrationFailure, OrchestrationRuntime},
        },
        registry::Registry,
        run_dto::convert,
    },
    domain::agent_orchestration::{
        OrchestrationError, OrchestrationErrorCode, OrchestrationSession, TaskCommandSource,
    },
};

/// 도메인 오류 → fault. 원본은 `details.orchestrationError`(compat이 오늘 JSON 문자열을 다시 만든다).
pub fn orchestration_fault(
    request_id: &RequestId,
    failure: OrchestrationFailure,
) -> WorkbenchFault {
    match failure {
        OrchestrationFailure::Domain(error) => domain_fault(request_id, error),
        OrchestrationFailure::Forbidden(message) => {
            WorkbenchFault::new(FaultCode::Forbidden, request_id.clone(), message)
        }
        OrchestrationFailure::Plain(message) => {
            let code = if message.starts_with("Failed to resolve workspace path")
                || message == "Workspace path must be a directory."
            {
                FaultCode::InvalidArgument
            } else {
                FaultCode::PreconditionFailed
            };
            WorkbenchFault::new(code, request_id.clone(), message)
        }
    }
}

pub fn domain_fault(request_id: &RequestId, error: OrchestrationError) -> WorkbenchFault {
    use OrchestrationErrorCode as C;
    let code = match error.code {
        C::InvalidInput | C::InvalidTopology => FaultCode::InvalidArgument,
        C::NotFound => FaultCode::NotFound,
        C::ScopeMismatch | C::Unauthorized | C::ReadOnlyViolation => FaultCode::Forbidden,
        C::RevisionConflict | C::DuplicateConflict | C::InvalidTransition => FaultCode::Conflict,
        C::CapacityExceeded => FaultCode::RateLimited,
        C::CoordinatorInactive | C::CoordinatorBusy | C::WorkerUnavailable | C::RuntimeLost => {
            FaultCode::Unavailable
        }
    };
    let details = serde_json::json!({ "orchestrationError": &error });
    WorkbenchFault::new(code, request_id.clone(), error.message.clone())
        .with_retryable(error.retryable)
        .with_details(details)
}

/// 작업 영역 → DTO. 현재 묶임의 스트림 id를 싣는다(FR-009).
fn session_json(
    runtime: &OrchestrationRuntime,
    session: &OrchestrationSession,
) -> serde_json::Value {
    let mut dto: OrchestrationSessionDto = convert(session);
    dto.event_stream_id = runtime
        .bindings()
        .binding_of(&session.id)
        .map(|binding| StreamKind::Orchestration.stream_id(&binding.binding_id));
    to_json(dto)
}

type Runtime = Arc<OrchestrationRuntime>;

macro_rules! desktop_command {
    ($registry:expr, $services:expr, $runtime:expr, $op:expr, $input:ty, |$rt:ident, $svc:ident, $ctx:ident, $bench:ident, $inp:ident| $body:expr) => {{
        let runtime: Runtime = Arc::clone($runtime);
        $registry.register(
            $op,
            epoch_handler(
                $op,
                $services,
                |input: &$input| Scope::Bench(input.bench_id.clone()),
                move |$svc: Arc<BenchServices>, $ctx, $inp: $input| {
                    let $rt = Arc::clone(&runtime);
                    async move {
                        let view =
                            $svc.resolve(&$ctx.request_id, &$ctx.principal, &$inp.bench_id)?;
                        let $bench = view.id;
                        let result: Result<serde_json::Value, OrchestrationFailure> = $body;
                        result.map_err(|failure| orchestration_fault(&$ctx.request_id, failure))
                    }
                },
            ),
        );
    }};
}

macro_rules! desktop_query {
    ($registry:expr, $services:expr, $runtime:expr, $op:expr, $input:ty, |$rt:ident, $svc:ident, $ctx:ident, $bench:ident, $inp:ident| $body:expr) => {{
        let runtime: Runtime = Arc::clone($runtime);
        $registry.register(
            $op,
            async_query_handler(
                $services,
                move |$svc: Arc<BenchServices>, $ctx, $inp: $input| {
                    let $rt = Arc::clone(&runtime);
                    async move {
                        let view =
                            $svc.resolve(&$ctx.request_id, &$ctx.principal, &$inp.bench_id)?;
                        let $bench = view.id;
                        let result: Result<serde_json::Value, OrchestrationFailure> = $body;
                        result.map_err(|failure| orchestration_fault(&$ctx.request_id, failure))
                    }
                },
            ),
        );
    }};
}

pub fn register(registry: &mut Registry, services: &Arc<BenchServices>, runtime: &Runtime) {
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationBootstrap,
        OrchestrationBootstrapInput,
        |rt, services, ctx, bench, input| {
            // 묶는 동안 작업대 입장권을 쥔다(research R3): 닫기 hook이 먼저 돌아 죽은 작업대에 묶이지 않게.
            let admission = services.admit(&ctx.request_id, Some(&ctx.principal), &bench)?;
            let result = rt
                .bootstrap(
                    &bench,
                    &input.worktree_path,
                    input.resume_workspace_id.clone(),
                )
                .await
                .map(|session| session_json(&rt, &session));
            drop(admission);
            result
        }
    );
    desktop_query!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationGet,
        OrchestrationBenchInput,
        |rt, services, ctx, bench, input| {
            rt.get(&bench).await.map(|session| match session {
                Some(session) => session_json(&rt, &session),
                None => serde_json::Value::Null,
            })
        }
    );
    desktop_query!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationListRecoverable,
        OrchestrationListRecoverableInput,
        |rt, services, ctx, bench, input| {
            let _ = &bench;
            rt.list_recoverable(&input.worktree_path)
                .await
                .map(|sessions| {
                    serde_json::Value::Array(
                        sessions
                            .iter()
                            .map(|session| session_json(&rt, session))
                            .collect(),
                    )
                })
        }
    );
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationBindCoordinator,
        OrchestrationBindCoordinatorInput,
        |rt, services, ctx, bench, input| {
            rt.bind_coordinator(&bench, convert(&input.request))
                .await
                .map(|session| session_json(&rt, &session))
        }
    );
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationDelegateGoal,
        OrchestrationDelegateGoalInput,
        |rt, services, ctx, bench, input| {
            rt.delegate_goal(&bench, convert(&input.request))
                .await
                .map(|outcome| to_json(convert::<_, DelegateGoalOutcomeDto>(&outcome)))
        }
    );
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationAdoptManualChild,
        OrchestrationAdoptManualChildInput,
        |rt, services, ctx, bench, input| {
            rt.adopt_manual_child(&bench, input.panel_id.clone(), input.title.clone())
                .await
                .map(|session| session_json(&rt, &session))
        }
    );
    desktop_query!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationListTasks,
        OrchestrationListTasksInput,
        |rt, services, ctx, bench, input| {
            rt.list_tasks(&bench, input.generation_id.clone())
                .await
                .map(|tasks| {
                    serde_json::Value::Array(
                        tasks
                            .iter()
                            .map(|task| to_json(convert::<_, OrchestrationTaskDto>(task)))
                            .collect(),
                    )
                })
        }
    );
    desktop_query!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationCollectReports,
        OrchestrationBenchInput,
        |rt, services, ctx, bench, input| {
            rt.collect_reports(&bench).await.map(|reports| {
                serde_json::Value::Array(
                    reports
                        .iter()
                        .map(|report| to_json(convert::<_, TaskReportDto>(report)))
                        .collect(),
                )
            })
        }
    );
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationSetPresentation,
        OrchestrationSetPresentationInput,
        |rt, services, ctx, bench, input| {
            rt.set_presentation(&bench, convert(&input.request))
                .await
                .map(|session| session_json(&rt, &session))
        }
    );
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationSendChildCommand,
        OrchestrationSendChildCommandInput,
        |rt, services, ctx, bench, input| {
            let command = &input.input;
            let request = DeliverTaskCommandRequest {
                request_id: command.request_id.clone(),
                task_id: command.task_id.clone(),
                kind: convert(&command.kind),
                message: command.message.clone(),
                input_report_id: command.input_report_id.clone(),
                delivery: convert(&command.delivery),
                source: TaskCommandSource::User,
                expected_task_revision: command.expected_task_revision,
            };
            rt.send_child_command(&bench, request)
                .await
                .map(|command| to_json(convert::<_, TaskCommandDto>(&command)))
        }
    );
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationRespondInput,
        OrchestrationTaskActionInput,
        |rt, services, ctx, bench, input| {
            rt.respond_input(&bench, convert(&input.request))
                .await
                .map(|command| to_json(convert::<_, TaskCommandDto>(&command)))
        }
    );
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationCancelTask,
        OrchestrationTaskActionInput,
        |rt, services, ctx, bench, input| {
            rt.cancel_task(&bench, convert(&input.request))
                .await
                .map(|session| session_json(&rt, &session))
        }
    );
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationRetryTask,
        OrchestrationTaskActionInput,
        |rt, services, ctx, bench, input| {
            rt.retry_task(&bench, convert(&input.request))
                .await
                .map(|session| session_json(&rt, &session))
        }
    );
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationReassignTask,
        OrchestrationTaskActionInput,
        |rt, services, ctx, bench, input| {
            rt.reassign_task(&bench, convert(&input.request))
                .await
                .map(|session| session_json(&rt, &session))
        }
    );
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationHandoffCoordinator,
        OrchestrationHandoffCoordinatorInput,
        |rt, services, ctx, bench, input| {
            rt.handoff_coordinator(&bench, convert(&input.request))
                .await
                .map(|session| session_json(&rt, &session))
        }
    );
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationDispatchPrompt,
        OrchestrationDispatchPromptInput,
        |rt, services, ctx, bench, input| {
            rt.dispatch_prompt(&bench, convert(&input.request))
                .await
                .map(|dispatch| to_json(convert::<_, PromptDispatchDto>(&dispatch)))
        }
    );
    desktop_command!(
        registry,
        services,
        runtime,
        OperationId::OrchestrationRecover,
        OrchestrationBenchInput,
        |rt, services, ctx, bench, input| {
            let _ = &input;
            rt.recover(&bench)
                .await
                .map(|session| session_json(&rt, &session))
        }
    );
    register_run_replay(registry, services, runtime);
    agent::register(registry, services, runtime);
}

/// `run.replay`(research R17): 이 작업대가 소유한 run이거나(끝난 run 포함), 이 작업대에 묶인 작업 영역이 기록한
/// run이면 허용.
/// 보관 한도로 지운 run은 내용이 없으므로 소유 검사 없이 오늘 Evicted 형태를 돌려준다.
fn register_run_replay(registry: &mut Registry, services: &Arc<BenchServices>, runtime: &Runtime) {
    let runtime: Runtime = Arc::clone(runtime);
    registry.register(
        OperationId::RunReplay,
        async_query_handler(services, move |services: Arc<BenchServices>, ctx, input: RunReplayInput| {
            let runtime = Arc::clone(&runtime);
            async move {
                let view = services.resolve(&ctx.request_id, &ctx.principal, &input.bench_id)?;
                let replay = services.hub.replay_run(&input.run_id, input.after_sequence);
                // 보관 한도 제거는 hub의 실제 제거 표식으로만 판정한다 — 응답 모양(terminal·gap·빈 이벤트)으로 추론하면
                // 미발행·미등록 run이나 구독 해지로 비워진 스트림과 섞일 수 있다.
                let evicted = services
                    .hub
                    .is_evicted(&StreamKind::Run.stream_id(&input.run_id));
                // 소유는 hub 기록(journal과 같은 수명)으로 본다 — 끝난 run도 journal이 남은 동안 재생할 수 있다(R17).
                // 입력에 작업대가 있으므로 주체가 아니라 그 작업대 소유로 좁힌다(구독은 작업대 문맥이 없어 주체 기준).
                let owned = services.hub.run_owner(&input.run_id).as_deref() == Some(view.id.as_str());
                let allowed = evicted || owned || node_run_of_bound_workspace(&runtime, &view.id, &input.run_id);
                if !allowed {
                    return Err(WorkbenchFault::new(
                        FaultCode::Forbidden,
                        ctx.request_id.clone(),
                        crate::application::orchestration::runtime::MESSAGE_RUN_OWNED_BY_OTHER_BENCH,
                    ));
                }
                Ok(to_json(convert::<_, RunReplayDto>(&replay)))
            }
        }),
    );
}

fn node_run_of_bound_workspace(
    runtime: &OrchestrationRuntime,
    bench_id: &str,
    run_id: &str,
) -> bool {
    runtime.bench_with_workspace_run(run_id).as_deref() == Some(bench_id)
}
