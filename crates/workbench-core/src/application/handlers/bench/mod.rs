//! `bench.open`·`bench.close`·`bench.requestTitle` handler(040). 제목 요청은 서버 상태를 바꾸지 않고 작업대 알림
//! 스트림에 발행해 데스크톱이 자기 창에 적용한다(ADR 0007).

use std::sync::Arc;

use workbench_protocol::{
    events::{StreamKind, BENCH_TITLE_REQUESTED_V1},
    operations::bench::{
        BenchCloseInput, BenchOpenInput, BenchRequestTitleInput, TitleChangeResultDto,
    },
    FaultCode, OperationId, RequestId, WorkbenchFault,
};

use crate::{
    application::{
        bench_service::BenchServices,
        handlers::{
            epoch::{epoch_handler, to_json, Scope},
            exchange::MESSAGE_RUN_MISMATCH,
        },
        registry::Registry,
    },
    domain::window_title::ValidatedWindowTitle,
    ports::desktop_bridge::DesktopDelivery,
};

pub const MESSAGE_RUN_ID_REQUIRED: &str = "Agent run id is required.";
pub const MESSAGE_RUN_NOT_OWNED: &str =
    "Agent run is not active or is not owned by a session window.";

/// 제목 실패 fault. MCP 도구가 오늘 `TitleChangeResult.code`로 되돌릴 수 있게 `details.titleCode`를 싣는다.
fn title_fault(
    request_id: &RequestId,
    code: FaultCode,
    title_code: &str,
    message: impl Into<String>,
) -> WorkbenchFault {
    WorkbenchFault::new(code, request_id.clone(), message)
        .with_details(serde_json::json!({ "titleCode": title_code }))
}

pub fn register(registry: &mut Registry, services: &Arc<BenchServices>) {
    registry.register(
        OperationId::BenchOpen,
        epoch_handler(
            OperationId::BenchOpen,
            services,
            |_: &BenchOpenInput| Scope::Open,
            |services, ctx, input: BenchOpenInput| async move {
                services
                    .open(&ctx.request_id, &ctx.principal, &input.working_directory)
                    .map(to_json)
            },
        ),
    );
    registry.register(
        OperationId::BenchClose,
        epoch_handler(
            OperationId::BenchClose,
            services,
            |_: &BenchCloseInput| Scope::None,
            |services, ctx, input: BenchCloseInput| async move {
                services
                    .close(&ctx.request_id, &ctx.principal, &input.bench_id)
                    .await
                    .map(to_json)
            },
        ),
    );
    registry.register(
        OperationId::BenchRequestTitle,
        epoch_handler(
            OperationId::BenchRequestTitle,
            services,
            |input: &BenchRequestTitleInput| Scope::RunOwner(input.run_id.trim().to_owned()),
            |services, ctx, input: BenchRequestTitleInput| async move {
                if ctx.principal.agent_run_id() != Some(input.run_id.as_str()) {
                    return Err(title_fault(
                        &ctx.request_id,
                        FaultCode::Forbidden,
                        "unauthorized",
                        MESSAGE_RUN_MISMATCH,
                    ));
                }
                let title = ValidatedWindowTitle::parse(&input.title)
                    .map_err(|error| {
                        title_fault(
                            &ctx.request_id,
                            FaultCode::InvalidArgument,
                            "invalidTitle",
                            error.reason(),
                        )
                    })?
                    .into_string();
                let run_id = input.run_id.trim();
                if run_id.is_empty() {
                    return Err(title_fault(
                        &ctx.request_id,
                        FaultCode::NotFound,
                        "unknownRun",
                        MESSAGE_RUN_ID_REQUIRED,
                    ));
                }
                let bench = match services.engine.active_owner_of(run_id).await {
                    Some(owner) => services.registry.resolve_any(&owner).ok(),
                    None => None,
                }
                .ok_or_else(|| {
                    title_fault(
                        &ctx.request_id,
                        FaultCode::NotFound,
                        "unknownRun",
                        MESSAGE_RUN_NOT_OWNED,
                    )
                })?;
                let desktop = services.desktop.clone();
                let bench_id = bench.id.clone();
                let requested = title.clone();
                services.hub.publish_notification(
                    StreamKind::Bench,
                    &bench.id,
                    BENCH_TITLE_REQUESTED_V1,
                    serde_json::json!({ "title": title }),
                    &mut |_| {
                        if let Some(desktop) = &desktop {
                            desktop.deliver(DesktopDelivery::TitleRequested {
                                bench_id: bench_id.clone(),
                                title: requested.clone(),
                            });
                        }
                    },
                );
                Ok(to_json(TitleChangeResultDto {
                    ok: true,
                    applied_title: Some(title),
                    reason: None,
                    code: None,
                }))
            },
        ),
    );
}
