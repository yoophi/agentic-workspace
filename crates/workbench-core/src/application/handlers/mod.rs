//! operation handler 모음과 registry 조립.

pub mod project_create;
pub mod project_list;
pub mod system_describe;

use std::sync::Arc;

use workbench_protocol::{OperationId, RequestId, WorkbenchFault};

use crate::{
    application::{
        intent_first::IntentFirst,
        reconcilers::{JsonCreateReconciler, ReconcilerRegistry},
        registry::Registry,
        workbench_runtime::TestHooks,
    },
    domain::project_error::ProjectError,
    infrastructure::{
        sqlite_ledger::SqliteOperationLedger, storage_coordinator::StorageCoordinator,
    },
    ports::operation_ledger::LedgerError,
};

pub use crate::application::dto::to_dto;

/// `ProjectError` → Fault. message는 Display 그대로(compat 골든), 검증 오류는 `details.path`를 채운다.
pub fn project_fault(request_id: &RequestId, error: ProjectError) -> WorkbenchFault {
    match &error {
        ProjectError::NameRequired | ProjectError::WorkingDirectoryRequired => {
            WorkbenchFault::invalid_argument(
                request_id.clone(),
                error.to_string(),
                error.field_path(),
            )
        }
        ProjectError::NotFound => WorkbenchFault::not_found(request_id.clone(), "project"),
        ProjectError::Storage(_) | ProjectError::StoreCorrupt(_) => {
            WorkbenchFault::unavailable(request_id.clone(), error.to_string())
        }
        ProjectError::Clock(_) => WorkbenchFault::internal(request_id.clone(), error.to_string()),
    }
}

pub fn ledger_fault(request_id: &RequestId, error: LedgerError) -> WorkbenchFault {
    match &error {
        LedgerError::Storage(_) | LedgerError::UnsupportedSchema { .. } => {
            WorkbenchFault::unavailable(request_id.clone(), error.to_string())
        }
        LedgerError::DuplicateKey
        | LedgerError::DuplicateReservation
        | LedgerError::NotFound(_)
        | LedgerError::InvalidState(_) => {
            WorkbenchFault::internal(request_id.clone(), error.to_string())
        }
    }
}

/// handler와 reconciler를 함께 조립한다. 둘은 같은 operation 목록을 공유해야 하므로 한곳에서 등록한다.
pub fn build_registry(
    ledger: Arc<SqliteOperationLedger>,
    coordinator: Arc<StorageCoordinator>,
    hooks: Arc<TestHooks>,
) -> (Registry, ReconcilerRegistry) {
    let runner = Arc::new(IntentFirst::new(
        Arc::clone(&ledger),
        Arc::clone(&coordinator),
        hooks,
    ));

    let mut registry = Registry::new();
    let mut reconcilers = ReconcilerRegistry::new();

    registry.register(
        OperationId::ProjectList,
        Arc::new(project_list::ProjectListHandler::new(Arc::clone(
            &coordinator,
        ))),
    );
    registry.register(
        OperationId::ProjectCreate,
        Arc::new(project_create::ProjectCreateHandler::new(Arc::clone(
            &runner,
        ))),
    );
    reconcilers.register(
        OperationId::ProjectCreate,
        Arc::new(JsonCreateReconciler::new(
            projects_as_dto_json(Arc::clone(&coordinator)),
            "id",
        )),
    );
    registry.register(
        OperationId::SystemDescribe,
        Arc::new(system_describe::SystemDescribeHandler),
    );
    (registry, reconcilers)
}

/// reconciler용: projects 컬렉션을 lock 안에서 읽어 DTO JSON 배열로.
fn projects_as_dto_json(
    coordinator: Arc<StorageCoordinator>,
) -> crate::application::reconcilers::LoadCollection {
    Box::new(move || {
        let projects = coordinator
            .with_projects(|repo| repo.load_projects())
            .ok()?;
        Some(
            projects
                .iter()
                .map(|project| serde_json::to_value(to_dto(project)).expect("dto serializes"))
                .collect(),
        )
    })
}
