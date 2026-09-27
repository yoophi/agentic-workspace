#![allow(dead_code)]
// 통합 테스트 crate는 lib의 crate-level allow를 상속하지 않는다. WorkbenchFault는 wire DTO라 Box로 감싸지 않는다.
#![allow(clippy::result_large_err)]

pub mod event_fixtures;
pub mod fixtures;
pub mod git_repo;
pub mod http_harness;
pub mod recording_desktop;
pub mod scripted_run_engine;

use std::{fs, sync::Arc};

use acp_agent_core::{domain::agent::AgentDescriptor, ports::agent_catalog::AgentCatalog};
use serde_json::{json, Value};
use workbench_core::{
    application::workbench_runtime::{RuntimeAdapters, WorkbenchRuntime},
    domain::{
        errors::ProviderSessionError,
        provider_session::{provider_kind_for, ProviderSession, SessionScope},
    },
    infrastructure::data_paths::DataPaths,
    ports::provider_session_repository::ProviderSessionRepository,
};
use workbench_protocol::{
    AuthenticatedPrincipal, CallReply, CallRequest, IdempotencyKey, OperationId, RequestId,
    Workbench, WorkbenchFault, PROTOCOL_VERSION,
};

/// 038 US3: 실행 환경을 읽지 않는 agent catalog stub.
#[derive(Clone, Default)]
pub struct StubCatalog(pub Vec<AgentDescriptor>);

impl AgentCatalog for StubCatalog {
    fn list_agents(&self) -> Vec<AgentDescriptor> {
        self.0.clone()
    }
}

/// provider 세션 stub. fs 어댑터의 계약(미지원 agent → 빈 목록, agent·경로 범위 필터)만 흉내 내고, 정렬·상한은
/// 유즈케이스가 한다.
#[derive(Clone, Default)]
pub struct StubProviderSessions(pub Vec<ProviderSession>);

impl ProviderSessionRepository for StubProviderSessions {
    fn list(
        &self,
        agent_id: &str,
        scope: &SessionScope,
    ) -> Result<Vec<ProviderSession>, ProviderSessionError> {
        if provider_kind_for(agent_id).is_none() {
            return Ok(Vec::new());
        }
        Ok(self
            .0
            .iter()
            .filter(|session| session.agent_id == agent_id)
            .filter(|session| match scope {
                SessionScope::All => true,
                SessionScope::Path(path) => session
                    .cwd
                    .as_deref()
                    .is_some_and(|cwd| std::path::Path::new(cwd) == path),
            })
            .cloned()
            .collect())
    }
}

pub fn stub_adapters(
    agents: Vec<AgentDescriptor>,
    sessions: Vec<ProviderSession>,
) -> RuntimeAdapters {
    stub_adapters_with(agents, sessions, scripted_run_engine::RunScript::default()).0
}

/// 040: 가짜 run 엔진·기록형 데스크톱을 넣은 stub. 테스트가 엔진·데스크톱을 관찰할 수 있게 함께 돌려준다.
pub fn stub_adapters_with(
    agents: Vec<AgentDescriptor>,
    sessions: Vec<ProviderSession>,
    script: scripted_run_engine::RunScript,
) -> (
    RuntimeAdapters,
    Arc<scripted_run_engine::ScriptedRunEngine>,
    Arc<recording_desktop::RecordingDesktop>,
) {
    let engine = Arc::new(scripted_run_engine::ScriptedRunEngine::new(script));
    let desktop = Arc::new(recording_desktop::RecordingDesktop::default());
    let mut adapters = RuntimeAdapters::production();
    adapters.agent_catalog = Arc::new(StubCatalog(agents));
    adapters.provider_sessions = Arc::new(StubProviderSessions(sessions));
    adapters.run_engine = Some(engine.clone());
    adapters.desktop = Some(desktop.clone());
    adapters.terminal_hook = Some(desktop.clone());
    adapters.launch_decorator = Some(desktop.clone());
    (adapters, engine, desktop)
}

pub struct TestRuntime {
    pub dir: tempfile::TempDir,
    pub paths: DataPaths,
    pub runtime: Arc<WorkbenchRuntime>,
    adapters: RuntimeAdapters,
}

impl TestRuntime {
    /// 실행 환경(환경 변수·홈 디렉터리)을 읽지 않도록 빈 stub 어댑터로 기동한다.
    pub fn new() -> Self {
        Self::with_adapters(stub_adapters(Vec::new(), Vec::new()))
    }

    pub fn with_adapters(adapters: RuntimeAdapters) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = DataPaths::new(dir.path());
        let runtime =
            WorkbenchRuntime::bootstrap_with(paths.clone(), adapters.clone()).expect("bootstrap");
        Self {
            dir,
            paths,
            runtime,
            adapters,
        }
    }

    /// 같은 데이터 디렉터리로 runtime을 다시 만든다(프로세스 재시작 흉내). hook은 초기화된다.
    pub fn restart(self) -> Self {
        let TestRuntime {
            dir,
            paths,
            runtime,
            adapters,
        } = self;
        drop(runtime);
        let runtime = WorkbenchRuntime::bootstrap_with(paths.clone(), adapters.clone())
            .expect("bootstrap again");
        Self {
            dir,
            paths,
            runtime,
            adapters,
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

/// 038: 임의 operation의 변경 요청. 키는 호출자가 정한다(재시도·충돌 시나리오용).
pub fn command_request(operation: OperationId, key: &str, input: Value) -> CallRequest {
    CallRequest {
        protocol_version: PROTOCOL_VERSION,
        operation: operation.as_str().to_owned(),
        request_id: RequestId::random(),
        input,
        idempotency_key: Some(IdempotencyKey::new(key).expect("key")),
        expected_revision: None,
        timeout_ms: None,
    }
}

pub fn query_request(operation: OperationId, input: Value) -> CallRequest {
    CallRequest::query(operation, input)
}

/// 저장 파일 하나를 JSON 배열로 읽는다. 없으면 빈 벡터.
pub fn read_store(path: &std::path::Path) -> Vec<Value> {
    if !path.exists() {
        return Vec::new();
    }
    serde_json::from_str(&fs::read_to_string(path).expect("read store")).expect("parse store")
}

impl TestRuntime {
    pub fn saved_prompts(&self) -> Vec<Value> {
        read_store(&self.paths.saved_prompts_file())
    }

    pub fn goals(&self) -> Vec<Value> {
        read_store(&self.paths.goals_file())
    }

    pub fn agent_run_settings(&self) -> Vec<Value> {
        read_store(&self.paths.agent_run_settings_file())
    }
}

/// 040: 가짜 엔진·기록형 데스크톱과 함께 만든 런타임. 작업대 대상 디렉터리도 만든다.
pub struct BenchHarness {
    pub rt: TestRuntime,
    pub engine: Arc<scripted_run_engine::ScriptedRunEngine>,
    pub desktop: Arc<recording_desktop::RecordingDesktop>,
    pub dir: String,
}

impl BenchHarness {
    pub fn new(script: scripted_run_engine::RunScript) -> Self {
        Self::with(|_| {}, script)
    }

    pub fn with(
        configure: impl FnOnce(&mut RuntimeAdapters),
        script: scripted_run_engine::RunScript,
    ) -> Self {
        let (mut adapters, engine, desktop) = stub_adapters_with(Vec::new(), Vec::new(), script);
        configure(&mut adapters);
        let rt = TestRuntime::with_adapters(adapters);
        let dir = rt.paths.app_data_dir().join("bench-work");
        fs::create_dir_all(&dir).unwrap();
        let dir = fs::canonicalize(dir)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        Self {
            rt,
            engine,
            desktop,
            dir,
        }
    }

    /// 같은 데이터 디렉터리로 runtime을 다시 만든다(042 재시작 뒤 재시도). 가짜 엔진·데스크톱은 유지된다.
    pub fn restart(self) -> Self {
        let BenchHarness {
            rt,
            engine,
            desktop,
            dir,
        } = self;
        Self {
            rt: rt.restart(),
            engine,
            desktop,
            dir,
        }
    }

    pub async fn call(
        &self,
        principal: &AuthenticatedPrincipal,
        operation: OperationId,
        input: Value,
    ) -> Result<Value, WorkbenchFault> {
        let request = if matches!(
            workbench_protocol::operations::spec_for(operation).kind,
            workbench_protocol::OperationKind::Query
        ) {
            query_request(operation, input)
        } else {
            command_request(operation, &uuid_key(), input)
        };
        self.rt
            .runtime
            .call(principal.clone(), request)
            .await
            .map(|reply| reply.output().cloned().unwrap_or(Value::Null))
    }

    pub async fn keyed(
        &self,
        operation: OperationId,
        key: &str,
        input: Value,
    ) -> Result<Value, WorkbenchFault> {
        self.rt
            .runtime
            .call(
                AuthenticatedPrincipal::desktop(),
                command_request(operation, key, input),
            )
            .await
            .map(|reply| reply.output().cloned().unwrap_or(Value::Null))
    }

    pub async fn open(&self) -> String {
        let output = self
            .call(
                &AuthenticatedPrincipal::desktop(),
                OperationId::BenchOpen,
                json!({ "workingDirectory": self.dir }),
            )
            .await
            .expect("bench.open");
        output["benchId"].as_str().unwrap().to_owned()
    }

    pub async fn start(&self, bench: &str, run: &str) -> Result<Value, WorkbenchFault> {
        self.call(
            &AuthenticatedPrincipal::desktop(),
            OperationId::RunStart,
            json!({ "benchId": bench, "request": { "goal": "g", "agentId": "codex", "runId": run } }),
        )
        .await
    }

    pub async fn close(&self, bench: &str) -> Result<Value, WorkbenchFault> {
        self.call(
            &AuthenticatedPrincipal::desktop(),
            OperationId::BenchClose,
            json!({ "benchId": bench }),
        )
        .await
    }
}

pub fn uuid_key() -> String {
    format!("k-{}", uuid::Uuid::new_v4())
}
