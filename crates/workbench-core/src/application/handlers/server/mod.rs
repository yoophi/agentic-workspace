//! `server.status`·`lease.*`·`desktop.*`·`bench.list` handler(044, contracts/server-lifecycle.md §4). 소유자 전용 operation은
//! scope(`server:admin`·`server:read`)로 막는다 — 창·agent principal은 dispatcher에서 `forbidden`을 받는다.
//! `server.stop`은 비우기·정지(T041)와 함께 등록한다.

use std::sync::Arc;

use workbench_protocol::{
    operations::{
        bench::{BenchListInput, BenchRunDto, BenchSummaryDto},
        desktop::{
            DesktopIssueWindowTokenInput, DesktopIssueWindowTokenOutput, DesktopRetireWindowInput,
            DesktopRetireWindowOutput,
        },
        lease::{
            LeaseAcquireInput, LeaseAcquireOutput, LeaseReleaseInput, LeaseReleaseOutput,
            LeaseRenewInput, LeaseRenewOutput,
        },
        server::{ActiveWorkDto, ServerStateDto, ServerStatusInput, ServerStatusOutput},
    },
    AuthenticatedPrincipal, FaultCode, OperationId, RequestId, WorkbenchFault,
};

use crate::{
    application::{
        bench_service::is_owner,
        handlers::epoch::{async_query_handler, epoch_handler, to_json, Scope},
        registry::Registry,
        server_control::ServerControl,
        work_gate::{DrainMode, GateState},
    },
    ports::server_host::{ServerHost, WindowTokenError},
};

pub const MESSAGE_NO_SERVER_HOST: &str = "Workbench server host is not attached.";
pub const MESSAGE_UNKNOWN_LEASE: &str = "Lease is not active.";
pub const MESSAGE_WINDOW_IDENTITY: &str =
    "Window label and incarnation must be non-empty and must not contain ':'.";

/// 아직 파생하지 않는 `server.status` 필드(T041에서 파생). 이 필드는 `null`·부재로 싣고 목록에 올린다 — 정지 판정은
/// 모르는 값을 활동 작업으로 본다(`ActiveWorkDto::blocks_stop`).
pub const NOT_YET_DERIVED: [&str; 9] = [
    "activeWork.orchestrationTasks",
    "activeWork.queuedTasks",
    "activeWork.pendingExchanges",
    "activeWork.pendingNotifications",
    "activeWork.pendingOperations",
    "unresolvedOperations",
    "undeliverableExchanges",
    "failedExchangeDeliveries",
    "idleSince",
];

fn host<'a>(
    control: &'a ServerControl,
    request_id: &RequestId,
) -> Result<&'a Arc<dyn ServerHost>, WorkbenchFault> {
    control
        .host()
        .ok_or_else(|| WorkbenchFault::unavailable(request_id.clone(), MESSAGE_NO_SERVER_HOST))
}

/// 창 주체. label·incarnation에 `:`가 있으면 다른 창의 주체 문자열을 흉내 낼 수 있으므로 거절한다.
fn window_principal(
    request_id: &RequestId,
    label: &str,
    incarnation: &str,
) -> Result<AuthenticatedPrincipal, WorkbenchFault> {
    let valid = |part: &str| !part.trim().is_empty() && !part.contains(':');
    if !valid(label) {
        return Err(WorkbenchFault::invalid_argument(
            request_id.clone(),
            MESSAGE_WINDOW_IDENTITY,
            Some("/input/label"),
        ));
    }
    if !valid(incarnation) {
        return Err(WorkbenchFault::invalid_argument(
            request_id.clone(),
            MESSAGE_WINDOW_IDENTITY,
            Some("/input/incarnation"),
        ));
    }
    Ok(AuthenticatedPrincipal::desktop_window(label, incarnation))
}

fn state_dto(state: GateState) -> ServerStateDto {
    match state {
        GateState::Serving => ServerStateDto::Serving,
        GateState::Draining(DrainMode::Idle) => ServerStateDto::DrainingIdle,
        GateState::Draining(DrainMode::Wait) => ServerStateDto::DrainingWait,
        GateState::Stopping => ServerStateDto::Stopping,
    }
}

/// 작업대의 살아 있는 run(엔진이 아직 이 작업대 소유로 두는 run)과 상태. 끝난 run(journal만 남음)은 뺀다.
async fn bench_runs(control: &ServerControl, bench_id: &str) -> Vec<BenchRunDto> {
    let services = control.benches();
    let mut runs = Vec::new();
    for run_id in services.hub.runs_of_bench(bench_id) {
        if services.engine.active_owner_of(&run_id).await.as_deref() != Some(bench_id) {
            continue;
        }
        let state = if control.work_gate().busy_run_count(&run_id) > 0 {
            "busy"
        } else {
            "idle"
        };
        runs.push(BenchRunDto {
            run_id,
            state: state.to_owned(),
        });
    }
    runs
}

async fn bench_list(
    control: &ServerControl,
    principal: &AuthenticatedPrincipal,
) -> Vec<BenchSummaryDto> {
    let services = control.benches();
    let owner = is_owner(principal);
    let mut open = services.registry.open_benches();
    open.retain(|(_, opened_by)| owner || opened_by == &principal.subject);
    open.sort_by(|a, b| a.0.cmp(&b.0));
    let mut summaries = Vec::new();
    for (bench_id, opened_by) in open {
        // 목록을 뜬 뒤 닫힌 작업대는 뺀다.
        let Ok(view) = services.registry.resolve_any(&bench_id) else {
            continue;
        };
        summaries.push(BenchSummaryDto {
            runs: bench_runs(control, &bench_id).await,
            bench_id,
            working_directory: view.working_directory,
            owner: opened_by.as_str().to_owned(),
        });
    }
    summaries
}

async fn server_status(control: &ServerControl) -> ServerStatusOutput {
    let gate = control.work_gate();
    let active = gate.active_work();
    let accepted_calls = control
        .host()
        .map(|host| host.accepted_calls())
        .unwrap_or(0)
        + active.accepted_calls as u64;
    let mut idle_runs = 0u64;
    for (bench_id, _) in control.benches().registry.open_benches() {
        idle_runs += bench_runs(control, &bench_id)
            .await
            .iter()
            .filter(|run| run.state == "idle")
            .count() as u64;
    }
    ServerStatusOutput {
        state: state_dto(gate.state()),
        instance_id: control
            .host()
            .and_then(|host| host.instance_id())
            .unwrap_or_default(),
        server_epoch: control.epoch().to_owned(),
        active_work: ActiveWorkDto {
            busy_runs: active.busy_runs as u64,
            // `NOT_YET_DERIVED` 참고: T041(유휴·wait 정지)이 파생한다. 그때까지 `null`(모름) — 0(없음)으로 보고하지 않는다.
            orchestration_tasks: None,
            queued_tasks: None,
            pending_exchanges: None,
            pending_notifications: None,
            pending_operations: None,
            accepted_calls,
            reservations: gate.reservation_total() as u64,
        },
        idle_runs,
        leases: control.leases().count() as u64,
        unresolved_operations: None,
        undeliverable_exchanges: None,
        failed_exchange_deliveries: None,
        idle_since: None,
        not_yet_derived: NOT_YET_DERIVED
            .iter()
            .map(|path| (*path).to_owned())
            .collect(),
    }
}

pub fn register(registry: &mut Registry, control: &Arc<ServerControl>) {
    let services = control.benches();

    let c = Arc::clone(control);
    registry.register(
        OperationId::DesktopIssueWindowToken,
        epoch_handler(
            OperationId::DesktopIssueWindowToken,
            services,
            // 토큰 비밀을 멱등 기록에 남기지 않는다.
            |_: &DesktopIssueWindowTokenInput| Scope::None,
            move |_, ctx, input: DesktopIssueWindowTokenInput| {
                let control = Arc::clone(&c);
                async move {
                    let principal =
                        window_principal(&ctx.request_id, &input.label, &input.incarnation)?;
                    let issued = host(&control, &ctx.request_id)?
                        .issue_window_token(principal, &input.origin)
                        .map_err(|error| match error {
                            WindowTokenError::OriginNotAllowed | WindowTokenError::Retired => {
                                WorkbenchFault::new(
                                    FaultCode::Forbidden,
                                    ctx.request_id.clone(),
                                    error.to_string(),
                                )
                            }
                        })?;
                    Ok(to_json(DesktopIssueWindowTokenOutput {
                        token: issued.token,
                        expires_at: issued.expires_at,
                    }))
                }
            },
        ),
    );

    let c = Arc::clone(control);
    registry.register(
        OperationId::DesktopRetireWindow,
        epoch_handler(
            OperationId::DesktopRetireWindow,
            services,
            |_: &DesktopRetireWindowInput| Scope::None,
            move |_, ctx, input: DesktopRetireWindowInput| {
                let control = Arc::clone(&c);
                async move {
                    let principal =
                        window_principal(&ctx.request_id, &input.label, &input.incarnation)?;
                    // 먼저 폐기해 닫는 동안 그 창이 새 호출을 넣지 못하게 한다(tombstone은 이후 발급도 막는다).
                    let revoked_tokens =
                        host(&control, &ctx.request_id)?.retire_window(&principal.subject);
                    let closed_benches = if input.close_bench {
                        control
                            .benches()
                            .close_opened_by(&ctx.request_id, &principal.subject)
                            .await
                    } else {
                        Vec::new()
                    };
                    Ok(to_json(DesktopRetireWindowOutput {
                        revoked_tokens,
                        closed_benches,
                    }))
                }
            },
        ),
    );

    let c = Arc::clone(control);
    registry.register(
        OperationId::LeaseAcquire,
        epoch_handler(
            OperationId::LeaseAcquire,
            services,
            |_: &LeaseAcquireInput| Scope::None,
            move |_, _ctx, input: LeaseAcquireInput| {
                let control = Arc::clone(&c);
                async move {
                    let leases = control.leases();
                    let lease_id = leases.acquire(input.client_kind, input.client_id);
                    Ok(to_json(LeaseAcquireOutput {
                        lease_id,
                        ttl_seconds: leases.ttl().as_secs(),
                    }))
                }
            },
        ),
    );

    let c = Arc::clone(control);
    registry.register(
        OperationId::LeaseRenew,
        epoch_handler(
            OperationId::LeaseRenew,
            services,
            |_: &LeaseRenewInput| Scope::None,
            move |_, ctx, input: LeaseRenewInput| {
                let control = Arc::clone(&c);
                async move {
                    let leases = control.leases();
                    if !leases.renew(&input.lease_id) {
                        return Err(WorkbenchFault::new(
                            FaultCode::NotFound,
                            ctx.request_id.clone(),
                            MESSAGE_UNKNOWN_LEASE,
                        ));
                    }
                    Ok(to_json(LeaseRenewOutput {
                        ttl_seconds: leases.ttl().as_secs(),
                    }))
                }
            },
        ),
    );

    let c = Arc::clone(control);
    registry.register(
        OperationId::LeaseRelease,
        epoch_handler(
            OperationId::LeaseRelease,
            services,
            |_: &LeaseReleaseInput| Scope::None,
            move |_, _ctx, input: LeaseReleaseInput| {
                let control = Arc::clone(&c);
                async move {
                    Ok(to_json(LeaseReleaseOutput {
                        released: control.leases().release(&input.lease_id),
                    }))
                }
            },
        ),
    );

    let c = Arc::clone(control);
    registry.register(
        OperationId::BenchList,
        async_query_handler(services, move |_, ctx, _: BenchListInput| {
            let control = Arc::clone(&c);
            async move { Ok(to_json(bench_list(&control, &ctx.principal).await)) }
        }),
    );

    let c = Arc::clone(control);
    registry.register(
        OperationId::ServerStatus,
        async_query_handler(services, move |_, _ctx, _: ServerStatusInput| {
            let control = Arc::clone(&c);
            async move { Ok(to_json(server_status(&control).await)) }
        }),
    );
}
