#![allow(dead_code)]
// 통합 테스트 crate는 lib의 crate-level allow를 상속하지 않는다. WorkbenchFault는 wire DTO라 Box로 감싸지 않는다.
#![allow(clippy::result_large_err)]

pub mod fixtures;
pub mod http_harness;

use std::{fs, sync::Arc};

use serde_json::{json, Value};
use workbench_core::{
    application::workbench_runtime::WorkbenchRuntime, infrastructure::data_paths::DataPaths,
};
use workbench_protocol::{
    AuthenticatedPrincipal, CallReply, CallRequest, IdempotencyKey, OperationId, RequestId,
    Workbench, WorkbenchFault, PROTOCOL_VERSION,
};

pub struct TestRuntime {
    pub dir: tempfile::TempDir,
    pub paths: DataPaths,
    pub runtime: Arc<WorkbenchRuntime>,
}

impl TestRuntime {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = DataPaths::new(dir.path());
        let runtime = WorkbenchRuntime::bootstrap(paths.clone()).expect("bootstrap");
        Self {
            dir,
            paths,
            runtime,
        }
    }

    /// 같은 데이터 디렉터리로 runtime을 다시 만든다(프로세스 재시작 흉내). hook은 초기화된다.
    pub fn restart(self) -> Self {
        let TestRuntime {
            dir,
            paths,
            runtime,
        } = self;
        drop(runtime);
        let runtime = WorkbenchRuntime::bootstrap(paths.clone()).expect("bootstrap again");
        Self {
            dir,
            paths,
            runtime,
        }
    }

    pub async fn call(&self, request: CallRequest) -> Result<CallReply, WorkbenchFault> {
        self.runtime
            .call(AuthenticatedPrincipal::desktop(), request)
            .await
    }

    pub fn projects(&self) -> Vec<Value> {
        read_projects(&self.paths)
    }
}

pub fn read_projects(paths: &DataPaths) -> Vec<Value> {
    let path = paths.projects_file();
    if !path.exists() {
        return Vec::new();
    }
    serde_json::from_str(&fs::read_to_string(path).expect("read projects.json")).expect("parse")
}

pub fn create_request(key: &str, name: &str, working_directory: &str) -> CallRequest {
    CallRequest {
        protocol_version: PROTOCOL_VERSION,
        operation: OperationId::ProjectCreate.as_str().to_owned(),
        request_id: RequestId::random(),
        input: json!({ "name": name, "workingDirectory": working_directory }),
        idempotency_key: Some(IdempotencyKey::new(key).expect("key")),
        expected_revision: None,
        timeout_ms: None,
    }
}

pub fn list_request() -> CallRequest {
    CallRequest::query(OperationId::ProjectList, json!({}))
}
