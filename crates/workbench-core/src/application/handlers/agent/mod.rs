//! `agent.*` handler(038 US3). 실행 환경(환경 변수 catalog, provider 로컬 파일)을 읽는 조회라 lock 없이
//! blocking pool에서 실행한다. 어댑터는 `RuntimeAdapters`로 주입되어 테스트가 stub으로 바꿀 수 있다.

use std::{convert::Infallible, sync::Arc};

use workbench_protocol::{
    operations::agent::{
        AgentDescriptorDto, AgentListInput, AgentListProviderSessionsInput, ProviderSessionDto,
    },
    FaultCode, OperationId, RequestId, WorkbenchFault,
};

use crate::{
    application::{
        agent_dto::{agent_descriptor_dto, provider_session_dto},
        handlers::query_handler,
        provider_session_service::{self, PROVIDER_SESSION_LIMIT},
        registry::Registry,
    },
    domain::errors::ProviderSessionError,
    ports::{
        agent_catalog_reader::AgentCatalogReader,
        provider_session_repository::ProviderSessionRepository,
    },
};

/// catalog 조회는 실패하지 않는다(환경 설정 오류는 기본값·빈 목록, 오늘과 같음).
fn never(_: &RequestId, error: Infallible) -> WorkbenchFault {
    match error {}
}

/// 손상 세션은 건너뛰므로 여기 오는 것은 목록 전체를 만들 수 없는 경우뿐이다(research R7 → `internal`).
pub fn provider_session_fault(
    request_id: &RequestId,
    error: ProviderSessionError,
) -> WorkbenchFault {
    WorkbenchFault::new(FaultCode::Internal, request_id.clone(), error.to_string())
}

pub fn register(
    registry: &mut Registry,
    agent_catalog: &Arc<dyn AgentCatalogReader>,
    provider_sessions: &Arc<dyn ProviderSessionRepository>,
) {
    let catalog = Arc::clone(agent_catalog);
    registry.register(
        OperationId::AgentList,
        query_handler(never, move |_: AgentListInput| {
            Ok::<Vec<AgentDescriptorDto>, Infallible>(
                catalog
                    .list_agents()
                    .iter()
                    .map(agent_descriptor_dto)
                    .collect(),
            )
        }),
    );
    let sessions = Arc::clone(provider_sessions);
    registry.register(
        OperationId::AgentListProviderSessions,
        query_handler(
            provider_session_fault,
            move |input: AgentListProviderSessionsInput| {
                let scope = provider_session_service::scope_for(input.cwd);
                let found = provider_session_service::list_provider_sessions(
                    sessions.as_ref(),
                    &input.agent_id,
                    &scope,
                    Some(PROVIDER_SESSION_LIMIT),
                )?;
                Ok::<Vec<ProviderSessionDto>, ProviderSessionError>(
                    found.iter().map(provider_session_dto).collect(),
                )
            },
        ),
    );
}
