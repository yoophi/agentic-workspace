//! operation handler 모음과 registry 조립. 도메인별 하위 모듈에 handler가 있고, 여기서는 공통 helper·오류 매핑과
//! handler + reconciler 등록을 한다.

pub mod agent_run_settings;
pub mod goal;
pub mod project;
pub mod saved_prompt;
pub mod system;

use std::sync::Arc;

use serde::Serialize;
use workbench_protocol::{CallReply, FaultCode, OperationId, RequestId, WorkbenchFault};

use crate::{
    application::{
        dto::{goal_dto, saved_prompt_dto},
        intent_first::IntentFirst,
        reconcilers::{
            JsonCreateReconciler, JsonDeleteReconciler, LoadCollection, ReconcilerRegistry,
        },
        registry::Registry,
        workbench_runtime::TestHooks,
    },
    domain::{
        errors::{AgentRunSettingsError, GoalError, SavedPromptError},
        project_error::ProjectError,
    },
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
        ProjectError::NotFound => {
            WorkbenchFault::new(FaultCode::NotFound, request_id.clone(), error.to_string())
        }
        ProjectError::Storage(_) | ProjectError::StoreCorrupt(_) => {
            WorkbenchFault::unavailable(request_id.clone(), error.to_string())
        }
        ProjectError::Clock(_) => WorkbenchFault::internal(request_id.clone(), error.to_string()),
    }
}

pub fn saved_prompt_fault(request_id: &RequestId, error: SavedPromptError) -> WorkbenchFault {
    match &error {
        SavedPromptError::Required(_) => WorkbenchFault::invalid_argument(
            request_id.clone(),
            error.to_string(),
            error.field_path(),
        ),
        SavedPromptError::NotFound => {
            WorkbenchFault::new(FaultCode::NotFound, request_id.clone(), error.to_string())
        }
        SavedPromptError::Storage(_) | SavedPromptError::StoreCorrupt(_) => {
            WorkbenchFault::unavailable(request_id.clone(), error.to_string())
        }
        SavedPromptError::Clock(_) => {
            WorkbenchFault::internal(request_id.clone(), error.to_string())
        }
    }
}

pub fn goal_fault(request_id: &RequestId, error: GoalError) -> WorkbenchFault {
    match &error {
        GoalError::Required(_) => WorkbenchFault::invalid_argument(
            request_id.clone(),
            error.to_string(),
            error.field_path(),
        ),
        GoalError::NotFound => {
            WorkbenchFault::new(FaultCode::NotFound, request_id.clone(), error.to_string())
        }
        GoalError::Storage(_) | GoalError::StoreCorrupt(_) => {
            WorkbenchFault::unavailable(request_id.clone(), error.to_string())
        }
    }
}

pub fn agent_run_settings_fault(
    request_id: &RequestId,
    error: AgentRunSettingsError,
) -> WorkbenchFault {
    match &error {
        AgentRunSettingsError::Required(_)
        | AgentRunSettingsError::NoBuiltInProfile
        | AgentRunSettingsError::NoCommandConfigured(_) => WorkbenchFault::invalid_argument(
            request_id.clone(),
            error.to_string(),
            error.field_path(),
        ),
        AgentRunSettingsError::Storage(_) | AgentRunSettingsError::StoreCorrupt(_) => {
            WorkbenchFault::unavailable(request_id.clone(), error.to_string())
        }
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

/// 조회 handler 공통: blocking pool에서 `f`를 돌리고 도메인 오류를 Fault로 바꾼다.
pub(crate) async fn blocking<T, E, F>(
    request_id: &RequestId,
    fault: fn(&RequestId, E) -> WorkbenchFault,
    f: F,
) -> Result<T, WorkbenchFault>
where
    T: Send + 'static,
    E: Send + 'static,
    F: FnOnce() -> Result<T, E> + Send + 'static,
{
    let request_id = request_id.clone();
    let result = tokio::task::spawn_blocking(f)
        .await
        .map_err(|error| WorkbenchFault::internal(request_id.clone(), error.to_string()))?;
    result.map_err(|error| fault(&request_id, error))
}

/// 조회 응답. revision은 조회에 싣지 않는다.
pub(crate) fn complete<T: Serialize>(output: T) -> CallReply {
    CallReply::complete(
        serde_json::to_value(output).expect("output serializes"),
        None,
    )
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

    project::register(&mut registry, &mut reconcilers, &coordinator, &runner);
    saved_prompt::register(&mut registry, &mut reconcilers, &coordinator, &runner);
    goal::register(&mut registry, &mut reconcilers, &coordinator, &runner);
    agent_run_settings::register(&mut registry, &coordinator, &runner);
    registry.register(
        OperationId::SystemDescribe,
        Arc::new(system::describe::SystemDescribeHandler),
    );
    (registry, reconcilers)
}

/// reconciler용: projects 컬렉션을 lock 안에서 읽어 DTO JSON 배열로.
pub(crate) fn projects_as_dto_json(coordinator: Arc<StorageCoordinator>) -> LoadCollection {
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

pub(crate) fn saved_prompts_as_dto_json(coordinator: Arc<StorageCoordinator>) -> LoadCollection {
    Box::new(move || {
        let prompts = coordinator
            .with_saved_prompts(|repo| repo.load_saved_prompts())
            .ok()?;
        Some(
            prompts
                .iter()
                .map(|prompt| serde_json::to_value(saved_prompt_dto(prompt)).expect("dto"))
                .collect(),
        )
    })
}

pub(crate) fn goals_as_dto_json(coordinator: Arc<StorageCoordinator>) -> LoadCollection {
    Box::new(move || {
        let goals = coordinator.with_goals(|repo| repo.load_goals()).ok()?;
        Some(
            goals
                .iter()
                .map(|goal| serde_json::to_value(goal_dto(goal)).expect("dto"))
                .collect(),
        )
    })
}

pub(crate) fn json_create_reconciler(
    load: LoadCollection,
    id_field: &'static str,
) -> Arc<JsonCreateReconciler> {
    Arc::new(JsonCreateReconciler::new(load, id_field))
}

pub(crate) fn json_delete_reconciler(
    load: LoadCollection,
    id_field: &'static str,
) -> Arc<JsonDeleteReconciler> {
    Arc::new(JsonDeleteReconciler::new(load, id_field))
}
