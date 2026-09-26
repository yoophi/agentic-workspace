//! operation handler 모음과 registry 조립.

pub mod project_create;
pub mod project_list;
pub mod system_describe;

use std::sync::Arc;

use workbench_protocol::{operations::project::ProjectDto, OperationId, RequestId, WorkbenchFault};

use crate::{
    application::{registry::Registry, workbench_runtime::TestHooks},
    domain::{project::Project, project_error::ProjectError},
    infrastructure::{
        sqlite_ledger::SqliteOperationLedger, storage_coordinator::StorageCoordinator,
    },
    ports::operation_ledger::LedgerError,
};

pub fn to_dto(project: &Project) -> ProjectDto {
    ProjectDto {
        id: project.id.clone(),
        name: project.name.clone(),
        working_directory: project.working_directory.clone(),
        description: project.description.clone(),
    }
}

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

pub fn build_registry(
    ledger: Arc<SqliteOperationLedger>,
    coordinator: Arc<StorageCoordinator>,
    hooks: Arc<TestHooks>,
) -> Registry {
    let mut registry = Registry::new();
    registry.register(
        OperationId::ProjectList,
        Arc::new(project_list::ProjectListHandler::new(Arc::clone(
            &coordinator,
        ))),
    );
    registry.register(
        OperationId::ProjectCreate,
        Arc::new(project_create::ProjectCreateHandler::new(
            ledger,
            coordinator,
            hooks,
        )),
    );
    registry.register(
        OperationId::SystemDescribe,
        Arc::new(system_describe::SystemDescribeHandler),
    );
    registry
}
