//! 세대 범위 멱등성 command와 비동기 조회 handler 공통부(040). 작업대·run·교환 handler가 쓴다.

use std::{future::Future, marker::PhantomData, pin::Pin, sync::Arc};

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use workbench_protocol::{CallReply, OperationId, WorkbenchFault};

use crate::application::{
    bench_service::BenchServices,
    epoch_idempotency::{bench_scope, open_scope, EpochCall},
    intent_first::key_required_message,
    registry::{decode_input, CallContext, OperationHandler},
};

pub(crate) type BoxFut<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// 멱등 기록을 둘 범위.
pub(crate) enum Scope {
    /// 작업대 id.
    Bench(String),
    /// 작업대가 아직 없는 호출(`bench.open`): 주체별.
    Open,
    /// run의 소유 작업대(agent 전용 operation). 소유 작업대가 없으면 기록 없이 실행한다(대상이 없어 실패한다).
    RunOwner(String),
    /// 기록하지 않음(자연 멱등: `bench.close`).
    None,
}

type RunFn<I> = Arc<
    dyn Fn(Arc<BenchServices>, CallContext, I) -> BoxFut<Result<serde_json::Value, WorkbenchFault>>
        + Send
        + Sync,
>;

struct EpochHandler<I> {
    operation: OperationId,
    services: Arc<BenchServices>,
    scope: fn(&I) -> Scope,
    run: RunFn<I>,
    _input: PhantomData<fn() -> I>,
}

#[async_trait]
impl<I> OperationHandler for EpochHandler<I>
where
    I: DeserializeOwned + Send + 'static,
{
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let key = ctx.idempotency_key.clone().ok_or_else(|| {
            WorkbenchFault::invalid_argument(
                ctx.request_id.clone(),
                key_required_message(self.operation),
                Some("/idempotencyKey"),
            )
        })?;
        let typed: I = decode_input(&ctx.request_id, &input)?;
        let scope = match (self.scope)(&typed) {
            Scope::Bench(id) => Some(bench_scope(&id)),
            Scope::Open => Some(open_scope(&ctx.principal.subject)),
            Scope::RunOwner(run_id) => self
                .services
                .engine
                .owner_of(&run_id)
                .await
                .map(|bench| bench_scope(&bench)),
            Scope::None => None,
        };
        let run = Arc::clone(&self.run);
        let services = Arc::clone(&self.services);
        let owned_ctx = ctx.clone();
        let execute = move || async move {
            run(services, owned_ctx, typed)
                .await
                .map(|output| CallReply::complete(output, None))
        };
        match scope {
            Some(scope) => {
                self.services
                    .idempotency
                    .run(
                        EpochCall {
                            scope,
                            subject: &ctx.principal.subject,
                            operation: self.operation,
                            key: &key,
                            input: &input,
                            request_id: &ctx.request_id,
                        },
                        execute,
                    )
                    .await
            }
            None => execute().await,
        }
    }
}

pub(crate) fn epoch_handler<I, F, Fut>(
    operation: OperationId,
    services: &Arc<BenchServices>,
    scope: fn(&I) -> Scope,
    run: F,
) -> Arc<dyn OperationHandler>
where
    I: DeserializeOwned + Send + 'static,
    F: Fn(Arc<BenchServices>, CallContext, I) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, WorkbenchFault>> + Send + 'static,
{
    Arc::new(EpochHandler {
        operation,
        services: Arc::clone(services),
        scope,
        run: Arc::new(move |services, ctx, input| Box::pin(run(services, ctx, input))),
        _input: PhantomData,
    })
}

struct AsyncQueryHandler<I> {
    services: Arc<BenchServices>,
    run: RunFn<I>,
    _input: PhantomData<fn() -> I>,
}

#[async_trait]
impl<I> OperationHandler for AsyncQueryHandler<I>
where
    I: DeserializeOwned + Send + 'static,
{
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let typed: I = decode_input(&ctx.request_id, &input)?;
        (self.run)(Arc::clone(&self.services), ctx.clone(), typed)
            .await
            .map(|output| CallReply::complete(output, None))
    }
}

pub(crate) fn async_query_handler<I, F, Fut>(
    services: &Arc<BenchServices>,
    run: F,
) -> Arc<dyn OperationHandler>
where
    I: DeserializeOwned + Send + 'static,
    F: Fn(Arc<BenchServices>, CallContext, I) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, WorkbenchFault>> + Send + 'static,
{
    Arc::new(AsyncQueryHandler {
        services: Arc::clone(services),
        run: Arc::new(move |services, ctx, input| Box::pin(run(services, ctx, input))),
        _input: PhantomData,
    })
}

pub(crate) fn to_json<T: serde::Serialize>(value: T) -> serde_json::Value {
    serde_json::to_value(value).expect("output serializes")
}
