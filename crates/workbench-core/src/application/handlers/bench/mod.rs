//! `bench.open`·`bench.close` handler(040). `bench.requestTitle`은 US3.

use std::sync::Arc;

use workbench_protocol::{
    operations::bench::{BenchCloseInput, BenchOpenInput},
    OperationId,
};

use crate::application::{
    bench_service::BenchServices,
    handlers::epoch::{epoch_handler, to_json, Scope},
    registry::Registry,
};

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
}
