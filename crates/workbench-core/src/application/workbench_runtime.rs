//! `Workbench` 구현체. protocol 검사 → authorization → handler dispatch.
//! 기동 순서(`bootstrap`): 디렉터리 → ledger open·migrate → GC → coordinator → reconcile → revision 캐시.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use workbench_protocol::{
    AuthenticatedPrincipal, CallReply, CallRequest, EventStream, Subscription, Workbench,
    WorkbenchFault, PROTOCOL_VERSION,
};

use crate::{
    application::{
        authorization,
        handlers::to_dto,
        registry::{CallContext, Registry},
    },
    domain::project_error::ProjectError,
    infrastructure::{
        data_paths::DataPaths,
        json_project_repository::JsonProjectRepository,
        sqlite_ledger::SqliteOperationLedger,
        storage_coordinator::{StorageCoordinator, PROJECTS_AGGREGATE},
    },
    ports::operation_ledger::{LedgerError, OperationLedger},
};

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
    hooks: Arc<TestHooks>,
}

impl WorkbenchRuntime {
    pub fn bootstrap(paths: DataPaths) -> Result<Arc<Self>, BootstrapError> {
        paths.ensure_dirs()?;

        let ledger = Arc::new(SqliteOperationLedger::open(&paths)?);
        ledger.migrate()?;
        ledger.gc_expired(Utc::now())?;

        let repository = Arc::new(JsonProjectRepository::new(&paths));
        let coordinator = Arc::new(StorageCoordinator::new(
            repository,
            ledger.current_revision(PROJECTS_AGGREGATE)?,
        ));

        // 중단된 변경의 적용 여부를 판정한다. 자동 재실행은 하지 않는다(FR-009).
        {
            let coordinator = Arc::clone(&coordinator);
            ledger.reconcile_pending(&mut |record| {
                let reserved = record.reserved_resource_id.as_deref()?;
                let projects = coordinator
                    .with_projects(|repo| repo.load_projects())
                    .ok()?;
                projects
                    .iter()
                    .find(|project| project.id == reserved)
                    .map(|project| serde_json::to_value(to_dto(project)).expect("dto serializes"))
            })?;
        }
        coordinator.set_revision(ledger.current_revision(PROJECTS_AGGREGATE)?);

        let hooks = Arc::new(TestHooks::default());
        let registry = crate::application::handlers::build_registry(
            Arc::clone(&ledger),
            Arc::clone(&coordinator),
            Arc::clone(&hooks),
        );

        Ok(Arc::new(Self {
            paths,
            ledger,
            coordinator,
            registry,
            hooks,
        }))
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
