//! `project.*` handler. list·create는 037, update·delete는 038.

pub mod create;
pub mod delete;
pub mod list;
pub mod update;

use std::sync::Arc;

use workbench_protocol::OperationId;

use crate::{
    application::{
        handlers::{json_create_reconciler, json_delete_reconciler, projects_as_dto_json},
        intent_first::IntentFirst,
        reconcilers::ReconcilerRegistry,
        registry::Registry,
    },
    infrastructure::storage_coordinator::StorageCoordinator,
};

pub fn register(
    registry: &mut Registry,
    reconcilers: &mut ReconcilerRegistry,
    coordinator: &Arc<StorageCoordinator>,
    runner: &Arc<IntentFirst>,
) {
    registry.register(
        OperationId::ProjectList,
        Arc::new(list::ProjectListHandler::new(Arc::clone(coordinator))),
    );
    registry.register(
        OperationId::ProjectCreate,
        Arc::new(create::ProjectCreateHandler::new(Arc::clone(runner))),
    );
    registry.register(
        OperationId::ProjectUpdate,
        Arc::new(update::ProjectUpdateHandler::new(Arc::clone(runner))),
    );
    registry.register(
        OperationId::ProjectDelete,
        Arc::new(delete::ProjectDeleteHandler::new(Arc::clone(runner))),
    );
    // 재시작 판정: 생성은 예약 id 증거, 삭제는 종료 상태(대상 없음). update는 unknown(등록 안 함).
    reconcilers.register(
        OperationId::ProjectCreate,
        json_create_reconciler(projects_as_dto_json(Arc::clone(coordinator)), "id"),
    );
    reconcilers.register(
        OperationId::ProjectDelete,
        json_delete_reconciler(projects_as_dto_json(Arc::clone(coordinator)), "id"),
    );
}
