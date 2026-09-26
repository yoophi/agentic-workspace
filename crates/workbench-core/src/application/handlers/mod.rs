//! operation handler 모음과 registry 조립. 도메인별 하위 모듈에 handler가 있고, 여기서는 공통 helper·오류 매핑과
//! handler + reconciler 등록을 한다.

pub mod agent;
pub mod agent_run_settings;
pub mod git;
pub mod goal;
pub mod project;
pub mod saved_prompt;
pub mod system;
pub mod worktree;

use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use workbench_protocol::{CallReply, FaultCode, OperationId, RequestId, WorkbenchFault};

use crate::{
    application::{
        dto::{goal_dto, saved_prompt_dto},
        intent_first::IntentFirst,
        reconcilers::{
            JsonCreateReconciler, JsonDeleteReconciler, LoadCollection, ReconcilerRegistry,
        },
        registry::{decode_input, CallContext, OperationHandler, Registry},
        workbench_runtime::{RuntimeAdapters, TestHooks},
    },
    domain::{
        errors::{AgentRunSettingsError, GitError, GoalError, SavedPromptError, WorktreeFileError},
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

/// Git 오류 → Fault(research R7). stderr는 해석하지 않는다: 비정상 종료는 전부 `internal`(재시도 불가).
pub fn git_fault(request_id: &RequestId, error: GitError) -> WorkbenchFault {
    match &error {
        GitError::Required(_) | GitError::Unresolvable(_) => WorkbenchFault::invalid_argument(
            request_id.clone(),
            error.to_string(),
            error.field_path(),
        ),
        GitError::GitNotFound(_) => {
            WorkbenchFault::unavailable(request_id.clone(), error.to_string())
        }
        GitError::WorktreeNotFound => {
            WorkbenchFault::new(FaultCode::NotFound, request_id.clone(), error.to_string())
        }
        GitError::WorktreeHasChanges | GitError::StatusUnresolved => WorkbenchFault::new(
            FaultCode::PreconditionFailed,
            request_id.clone(),
            error.to_string(),
        ),
        GitError::CommandFailed(_) | GitError::Io(_) | GitError::Clock(_) => {
            WorkbenchFault::internal(request_id.clone(), error.to_string())
        }
    }
}

/// 파일 목록·미리보기 오류 → Fault. worktree 밖 경로는 `forbidden`, 사전 확인 가능한 없음은 `notFound`.
pub fn worktree_file_fault(request_id: &RequestId, error: WorktreeFileError) -> WorkbenchFault {
    match &error {
        WorktreeFileError::Required(_) | WorktreeFileError::NotUtf8 => {
            WorkbenchFault::invalid_argument(
                request_id.clone(),
                error.to_string(),
                error.field_path(),
            )
        }
        WorktreeFileError::OutsideWorktree => {
            WorkbenchFault::new(FaultCode::Forbidden, request_id.clone(), error.to_string())
        }
        WorktreeFileError::NotADirectory
        | WorktreeFileError::NotRegularFile
        | WorktreeFileError::NotFound(_) => {
            WorkbenchFault::new(FaultCode::NotFound, request_id.clone(), error.to_string())
        }
        WorktreeFileError::Io(_) => WorkbenchFault::internal(request_id.clone(), error.to_string()),
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

/// 저장 단위가 없는 조회(Git·파일시스템)의 공통 handler: input 역직렬화 → blocking pool에서 `run` → output
/// 직렬화. lock을 잡지 않는다.
struct QueryHandler<I, O, E> {
    run: Arc<dyn Fn(I) -> Result<O, E> + Send + Sync>,
    fault: fn(&RequestId, E) -> WorkbenchFault,
    _input: PhantomData<fn() -> I>,
}

#[async_trait]
impl<I, O, E> OperationHandler for QueryHandler<I, O, E>
where
    I: DeserializeOwned + Send + 'static,
    O: Serialize + Send + 'static,
    E: Send + 'static,
{
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let input: I = decode_input(&ctx.request_id, &input)?;
        let run = Arc::clone(&self.run);
        let output = blocking(&ctx.request_id, self.fault, move || run(input)).await?;
        Ok(complete(output))
    }
}

pub(crate) fn query_handler<I, O, E, F>(
    fault: fn(&RequestId, E) -> WorkbenchFault,
    run: F,
) -> Arc<dyn OperationHandler>
where
    I: DeserializeOwned + Send + 'static,
    O: Serialize + Send + 'static,
    E: Send + 'static,
    F: Fn(I) -> Result<O, E> + Send + Sync + 'static,
{
    Arc::new(QueryHandler {
        run: Arc::new(run),
        fault,
        _input: PhantomData,
    })
}

/// handler와 reconciler를 함께 조립한다. 둘은 같은 operation 목록을 공유해야 하므로 한곳에서 등록한다.
pub fn build_registry(
    ledger: Arc<SqliteOperationLedger>,
    coordinator: Arc<StorageCoordinator>,
    hooks: Arc<TestHooks>,
    adapters: &RuntimeAdapters,
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
    git::register(&mut registry, &mut reconcilers, &runner);
    worktree::register(&mut registry);
    agent::register(
        &mut registry,
        &adapters.agent_catalog,
        &adapters.provider_sessions,
    );
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
