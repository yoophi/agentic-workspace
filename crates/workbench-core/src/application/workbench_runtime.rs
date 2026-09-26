//! `Workbench` 구현체. protocol 검사 → authorization → handler dispatch.
//! 기동 순서(`bootstrap`): 디렉터리 → ledger open·migrate → GC → coordinator → reconcile → revision 캐시.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use workbench_protocol::{
    operations::spec_for, AuthenticatedPrincipal, CallReply, CallRequest, EventStream,
    OperationKind, Subscription, Workbench, WorkbenchFault, PROTOCOL_VERSION,
};

pub const MESSAGE_KEY_ON_QUERY: &str = "idempotencyKey is only accepted for command operations.";

use crate::{
    application::{
        authorization,
        reconcilers::ReconcilerRegistry,
        registry::{CallContext, Registry},
    },
    domain::project_error::ProjectError,
    infrastructure::{
        data_paths::DataPaths,
        sqlite_ledger::SqliteOperationLedger,
        storage_coordinator::{Repositories, StorageCoordinator, STORE_AGGREGATES},
    },
    ports::{
        agent_catalog_reader::AgentCatalogReader,
        operation_ledger::{LedgerError, OperationLedger},
        provider_session_repository::ProviderSessionRepository,
    },
};

/// 실행 환경을 읽는 어댑터(038 US3). 운영은 `production()`, 테스트는 stub을 넣는다.
#[derive(Clone)]
pub struct RuntimeAdapters {
    pub agent_catalog: Arc<dyn AgentCatalogReader>,
    pub provider_sessions: Arc<dyn ProviderSessionRepository>,
}

impl RuntimeAdapters {
    /// 환경 변수 기반 agent catalog와 provider 로컬 세션 파일(오늘 command가 쓰던 구현).
    pub fn production() -> Self {
        Self {
            agent_catalog: Arc::new(
                acp_agent_core::infrastructure::agent_catalog::ConfigurableAgentCatalog::from_env(),
            ),
            provider_sessions: Arc::new(
                crate::infrastructure::fs::provider_session_repository::FsProviderSessionRepository::new(),
            ),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error("Failed to create app data directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Ledger(#[from] LedgerError),
    #[error("{0}")]
    Storage(#[from] ProjectError),
}

/// 테스트 전용 주입점. `test-hooks` feature가 꺼진 빌드에서는 항상 no-op이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashPoint {
    AfterPending,
    AfterJsonSave,
    BeforeApplied,
}

impl CrashPoint {
    /// 부작용(JSON 저장 또는 Git 명령) 직후. Git 변경 테스트에서 의미가 드러나도록 붙인 이름이다.
    pub const AFTER_SIDE_EFFECT: CrashPoint = CrashPoint::AfterJsonSave;
}

/// 프로세스는 살아 있지만 특정 저장 단계가 실패하는 상황을 주입한다(crash와 달리 handler가 계속 실행된다).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailPoint {
    /// JSON 저장은 성공했지만 ledger `complete` 트랜잭션이 실패한다.
    LedgerComplete,
}

#[derive(Default)]
pub struct TestHooks {
    #[cfg(feature = "test-hooks")]
    crash_point: std::sync::Mutex<Option<CrashPoint>>,
    #[cfg(feature = "test-hooks")]
    fail_point: std::sync::Mutex<Option<FailPoint>>,
}

impl TestHooks {
    #[cfg(feature = "test-hooks")]
    pub fn set_crash_point(&self, point: Option<CrashPoint>) {
        *self.crash_point.lock().unwrap() = point;
    }

    #[cfg(feature = "test-hooks")]
    pub fn set_fail_point(&self, point: Option<FailPoint>) {
        *self.fail_point.lock().unwrap() = point;
    }

    /// 설정된 지점이면 true. handler는 해당 저장 단계를 실패한 것으로 처리한다.
    pub fn should_fail(&self, point: FailPoint) -> bool {
        #[cfg(feature = "test-hooks")]
        {
            return *self.fail_point.lock().unwrap() == Some(point);
        }
        #[allow(unreachable_code)]
        {
            let _ = point;
            false
        }
    }

    /// 설정된 지점이면 `Err`를 돌려주어 handler를 그 자리에서 중단시킨다(프로세스 종료 흉내).
    pub fn crash_if(&self, point: CrashPoint) -> Result<(), CrashInjected> {
        #[cfg(feature = "test-hooks")]
        {
            if *self.crash_point.lock().unwrap() == Some(point) {
                return Err(CrashInjected(point));
            }
        }
        let _ = point;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrashInjected(pub CrashPoint);

pub struct WorkbenchRuntime {
    paths: DataPaths,
    ledger: Arc<SqliteOperationLedger>,
    coordinator: Arc<StorageCoordinator>,
    registry: Registry,
    reconcilers: ReconcilerRegistry,
    hooks: Arc<TestHooks>,
}

impl WorkbenchRuntime {
    pub fn bootstrap(paths: DataPaths) -> Result<Arc<Self>, BootstrapError> {
        Self::bootstrap_with(paths, RuntimeAdapters::production())
    }

    pub fn bootstrap_with(
        paths: DataPaths,
        adapters: RuntimeAdapters,
    ) -> Result<Arc<Self>, BootstrapError> {
        paths.ensure_dirs()?;

        let ledger = Arc::new(SqliteOperationLedger::open(&paths)?);
        ledger.migrate()?;
        ledger.gc_expired(Utc::now())?;

        let coordinator = Arc::new(StorageCoordinator::new(Repositories::json(&paths)));
        for aggregate in STORE_AGGREGATES {
            coordinator.set_revision_of(aggregate, ledger.current_revision(aggregate)?);
        }

        let hooks = Arc::new(TestHooks::default());
        let (registry, reconcilers) = crate::application::handlers::build_registry(
            Arc::clone(&ledger),
            Arc::clone(&coordinator),
            Arc::clone(&hooks),
            &adapters,
        );

        // 중단된 변경의 적용 여부를 operation별 reconciler로 판정한다. 자동 재실행은 하지 않는다(FR-009).
        // 등록되지 않은 operation(upsert·수정)은 unknown이다(research R6).
        ledger.reconcile_pending(&mut |record| reconcilers.resolve(record))?;
        for aggregate in STORE_AGGREGATES {
            coordinator.set_revision_of(aggregate, ledger.current_revision(aggregate)?);
        }

        Ok(Arc::new(Self {
            paths,
            ledger,
            coordinator,
            registry,
            reconcilers,
            hooks,
        }))
    }

    pub fn reconcilers(&self) -> &ReconcilerRegistry {
        &self.reconcilers
    }

    pub fn paths(&self) -> &DataPaths {
        &self.paths
    }

    pub fn ledger(&self) -> &Arc<SqliteOperationLedger> {
        &self.ledger
    }

    pub fn coordinator(&self) -> &Arc<StorageCoordinator> {
        &self.coordinator
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn hooks(&self) -> &Arc<TestHooks> {
        &self.hooks
    }
}

#[async_trait]
impl Workbench for WorkbenchRuntime {
    async fn call(
        &self,
        principal: AuthenticatedPrincipal,
        request: CallRequest,
    ) -> Result<CallReply, WorkbenchFault> {
        if request.protocol_version != PROTOCOL_VERSION {
            return Err(WorkbenchFault::unsupported_protocol(
                request.request_id,
                request.protocol_version,
                PROTOCOL_VERSION,
            ));
        }
        if let Some(timeout) = request.timeout_ms {
            if !(1..=600_000).contains(&timeout) {
                return Err(WorkbenchFault::invalid_argument(
                    request.request_id,
                    "timeoutMs는 1..=600000 범위여야 합니다.",
                    Some("/timeoutMs"),
                ));
            }
        }

        let operation =
            authorization::resolve_operation(&request.request_id, &principal, &request.operation)?;
        // 조회에 멱등성 키를 실어 보내는 것은 계약 위반이다(contracts §1 규칙 1). 조용히 무시하면 호출자가
        // 재시도 중복 제거가 되는 줄 오해한다.
        if matches!(spec_for(operation).kind, OperationKind::Query)
            && request.idempotency_key.is_some()
        {
            return Err(WorkbenchFault::invalid_argument(
                request.request_id,
                MESSAGE_KEY_ON_QUERY,
                Some("/idempotencyKey"),
            ));
        }
        let handler = self.registry.handler_for(operation).ok_or_else(|| {
            WorkbenchFault::internal(
                request.request_id.clone(),
                format!("operation {operation}의 handler가 등록되지 않았습니다."),
            )
        })?;

        let ctx = CallContext {
            principal,
            request_id: request.request_id,
            idempotency_key: request.idempotency_key,
            expected_revision: request.expected_revision,
        };
        handler.handle(&ctx, request.input).await
    }

    fn events(
        &self,
        _principal: AuthenticatedPrincipal,
        _request: Subscription,
    ) -> Result<EventStream, WorkbenchFault> {
        Err(WorkbenchFault::events_unsupported(
            workbench_protocol::RequestId::random(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use workbench_protocol::{FaultCode, OperationId, RequestId};

    use super::*;

    fn runtime() -> (tempfile::TempDir, Arc<WorkbenchRuntime>) {
        let dir = tempfile::tempdir().unwrap();
        let runtime = WorkbenchRuntime::bootstrap(DataPaths::new(dir.path())).unwrap();
        (dir, runtime)
    }

    fn request(operation: &str, protocol_version: u16) -> CallRequest {
        CallRequest {
            protocol_version,
            operation: operation.into(),
            request_id: RequestId::new("r1").unwrap(),
            input: json!({}),
            idempotency_key: None,
            expected_revision: None,
            timeout_ms: None,
        }
    }

    #[tokio::test]
    async fn rejects_unsupported_protocol_before_anything_else() {
        let (_dir, runtime) = runtime();
        let fault = runtime
            .call(
                AuthenticatedPrincipal::desktop(),
                request("project.list", 2),
            )
            .await
            .unwrap_err();
        assert_eq!(fault.code, FaultCode::UnsupportedProtocol);
        assert_eq!(
            fault.details.unwrap()["supportedProtocolRevisions"],
            json!([1])
        );
    }

    #[tokio::test]
    async fn unknown_operation_is_not_found_and_forbidden_needs_scope() {
        let (_dir, runtime) = runtime();
        let fault = runtime
            .call(
                AuthenticatedPrincipal::desktop(),
                request("project.rename", 1),
            )
            .await
            .unwrap_err();
        assert_eq!(fault.code, FaultCode::NotFound);
        let fault = runtime
            .call(
                AuthenticatedPrincipal::test_readonly(),
                request("project.create", 1),
            )
            .await
            .unwrap_err();
        assert_eq!(fault.code, FaultCode::Forbidden);
    }

    #[tokio::test]
    async fn events_are_unsupported_in_037() {
        let (_dir, runtime) = runtime();
        let fault = runtime
            .events(AuthenticatedPrincipal::desktop(), Subscription::default())
            .unwrap_err();
        assert_eq!(fault.code, FaultCode::UnsupportedSchema);
    }

    #[tokio::test]
    async fn bootstrap_creates_ledger_and_zero_revision() {
        let (_dir, runtime) = runtime();
        assert!(runtime.paths().ledger_file().exists());
        assert_eq!(runtime.coordinator().revision(), 0);
        assert!(runtime
            .registry()
            .handler_for(OperationId::ProjectList)
            .is_some());
    }
}
