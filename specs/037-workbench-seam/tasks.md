---

description: "Task list for Workbench Seam (server-client migration 1a)"
---

# Tasks: Workbench Seam 도입 (서버-클라이언트 전환 1a)

**Input**: Design documents from `/specs/037-workbench-seam/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/workbench-call.md](./contracts/workbench-call.md), [contracts/tauri-compat-commands.md](./contracts/tauri-compat-commands.md), [quickstart.md](./quickstart.md)

**Tests**: 사용자 여정 테스트는 spec이 요구하지 않았지만, **헌장이 요구하는 테스트는 필수**다 — 새 crate 두 개는 공유 코드이고, 입력 정규화·지문·상태 전이·Fault 매핑·authorization은 순수 로직이며, ledger·저장 복구는 안전 경계다. 따라서 각 스토리에서 테스트를 먼저 쓰고 실패를 확인한 뒤 구현한다. contract fixture는 세 Adapter가 공유한다(research R10).

**Organization**: 사용자 스토리별로 묶어 각 스토리를 독립적으로 구현·검증한다. US1(무회귀 + 세 경로 동일)이 MVP, US2(멱등성·복구·동시성), US3(계약 조회·타입 생성)이 뒤따른다.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 실행 가능 (다른 파일, 미완료 의존 없음)
- **[Story]**: 해당 사용자 스토리 (US1, US2, US3)
- 모든 태스크에 정확한 파일 경로를 포함한다

## Path Conventions

- **Reusable Rust**: `crates/workbench-protocol/src`, `crates/workbench-core/src`, `crates/workbench-core/tests`
- **App Tauri backend**: `apps/agentic-workbench/src-tauri/src/{domain,application,inbound,infrastructure}`
- **Reusable TypeScript**: `packages/workbench-client/src`
- **Documentation**: `docs/[english-file-name].md`

프론트엔드 `apps/agentic-workbench/src/**`는 변경하지 않는다(FR-014). 워크트리는 `/Users/yoophi/project/worktrees/037-workbench-seam`이며 모든 명령은 그 루트에서 실행한다.

### 공통 규칙

- Rust 오류 문자열은 [contracts/tauri-compat-commands.md](./contracts/tauri-compat-commands.md)의 골든 문구를 바이트 단위로 유지한다.
- SQLite DDL은 [data-model.md §3](./data-model.md#3-저장-모델--workbench-coreinfrastructure)을 그대로 쓴다(`schema_version`, `operation_ledger`, `aggregate_revision`).
- 입력 검증은 별도 JSON Schema 검증기 없이 `serde_json::from_value` + `#[serde(deny_unknown_fields)]`로 하고, serde 오류를 `invalidArgument`(`details.path`)로 변환한다. descriptor의 `inputSchema`/`outputSchema`는 describe·OpenAPI 노출용이다.
- 테스트 전용 hook(crash point, 복구 지연)은 `workbench-core`의 cargo feature `test-hooks` 뒤에 두고, `[dev-dependencies] workbench-core = { path = ".", features = ["test-hooks"] }`로 integration test에서만 켠다.

---

## Phase 1: Setup (기준선과 골격)

**Purpose**: 회귀 판정 기준선을 남기고, 새 crate·package 골격과 의존성을 만든다. 생성 파이프라인 spike를 가장 먼저 돌려 R1 결정을 실측으로 확인한다.

- [X] T001 `cargo test -p agentic-workbench --lib 2>&1 | tail -3`와 `pnpm run check-types`를 실행해 현재 통과 테스트 수와 타입 검사 결과를 이 파일 하단 Notes에 기록한다 (변경 전 기준선)
- [X] T002 [P] `crates/workbench-protocol/Cargo.toml`(deps: `serde` derive, `serde_json`, `utoipa` 5.x features `preserve_order`, `async-trait` 0.1, `thiserror` 2, `uuid` v4)과 `crates/workbench-protocol/src/lib.rs`(빈 `pub mod` 선언: call, fault, principal, descriptor, workbench, operations, openapi)를 만들고 `cargo check -p workbench-protocol` 통과
- [X] T003 [P] `crates/workbench-core/Cargo.toml`(deps: `workbench-protocol` path, `rusqlite` 0.40 `bundled`, `sha2` 0.10, `tokio` 1 `rt-multi-thread`/`sync`/`macros`, `async-trait`, `thiserror`, `serde`, `serde_json`, `uuid`, `chrono`; features `test-hooks = []`; dev-deps: `axum` 0.7, `reqwest` 0.12 `default-features = false, features = ["json"]`, `tempfile` 3, `tokio` `macros`, `workbench-core = { path = ".", features = ["test-hooks"] }`)과 `crates/workbench-core/src/lib.rs` + `src/{domain,ports,application,infrastructure}/mod.rs` 빈 골격을 만들고 `cargo check -p workbench-core` 통과. `rusqlite` bundled 첫 빌드 시간을 Notes에 기록(research R2 CI 영향)
- [X] T004 [P] `packages/workbench-client/package.json`(name `@yoophi/workbench-client`, private, type module, scripts `generate`/`check-types`/`test`, devDeps `openapi-typescript` ^7.13.0, `typescript` ^5, `vitest` ^4.1.9), `packages/workbench-client/tsconfig.json`(`../../tsconfig.base.json` extends, paths), `packages/workbench-client/src/index.ts`(빈 export)를 만들고 `pnpm install`로 `pnpm-lock.yaml` 갱신, `pnpm --filter @yoophi/workbench-client check-types` 통과
- [X] T005 생성 파이프라인 spike (plan 리스크 1): `crates/workbench-protocol/src/operations/project.rs`에 `ProjectListInput`/`ProjectListOutput`만 `ToSchema`로 정의하고, `crates/workbench-protocol/src/openapi.rs`에 `CallRequest` `oneOf`를 variant object `{operation: enum["project.list"], input: $ref}` 하나로 프로그램적으로 조립해 임시 `openapi.json`을 만든 뒤 `npx openapi-typescript`로 TS를 생성. `Extract<Req, {operation: "project.list"}>["input"]`이 `ProjectListInput` 타입으로 좁혀지는지 `tsc`로 확인하고 결과(성공 여부, 생성 TS 발췌)를 `specs/037-workbench-seam/research.md` R1 끝에 "spike 결과" 소절로 기록. 실패하면 `serde_json::json!` 직접 조립으로 전환하고 같은 소절에 기록 (depends T002, T004)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 세 스토리가 모두 의존하는 wire 계약, 도메인 이동, 저장 조정, ledger 골격, runtime 뼈대, 테스트 지원 코드를 만든다.

**⚠️ CRITICAL**: 이 단계가 끝나기 전에 스토리 작업을 시작하지 않는다.

- [X] T006 [P] `crates/workbench-protocol/src/call.rs`: `OperationId`(enum `ProjectList`/`ProjectCreate`/`SystemDescribe`, serde rename `project.list` 등, `FromStr`/`Display`), `RequestId`, `IdempotencyKey`(newtype, 1..128 검증), `CallRequest`(camelCase: protocolVersion, operation, requestId, input, idempotencyKey?, expectedRevision?, timeoutMs?), `CallReply`(`#[serde(tag = "kind")]` complete/accepted, revision?) + `ToSchema`. 같은 파일 `#[cfg(test)]`에 JSON round-trip과 길이 검증 테스트
- [X] T007 [P] `crates/workbench-protocol/src/fault.rs`: `FaultCode`(10개, data-model §1), `Outcome`, `WorkbenchFault{code,message,retryable,outcome,requestId,details?}`, `FaultCode::http_status()`(정본 코드 표), 생성 helper(`invalid_argument(request_id, message, path)`, `not_found`, `forbidden`, `conflict(outcome)`, `precondition_failed(current_revision)`, `unavailable`, `internal`) + `ToSchema`. 테스트: 코드↔status 표 전수, 직렬화 필드명
- [X] T008 [P] `crates/workbench-protocol/src/principal.rs`: `PrincipalKind{Desktop,Test}`, `Scope{ProjectRead,ProjectWrite,SystemDescribe}`(Display `project:read` 등), `AuthenticatedPrincipal{kind, scopes: BTreeSet<Scope>}`(Serialize 없음 — wire 금지), `AuthenticatedPrincipal::desktop()`(3 scope), `::test_readonly()`(read, describe). 테스트: 생성자 scope 집합
- [X] T009 [P] `crates/workbench-protocol/src/descriptor.rs`: `OperationKind{Query,Command}`, `Effect{Read,Modify}`, `OperationDescriptor{id, kind, effect, idempotent, requiredScopes, inputSchema: Value, outputSchema: Value, cliExposure: Option<String>, mcpExposure: bool}`, `DescribeOutput{protocolVersion, operations}` + `ToSchema`
- [X] T010 `crates/workbench-protocol/src/workbench.rs`: `#[async_trait] pub trait Workbench: Send + Sync { async fn call(&self, principal: AuthenticatedPrincipal, request: CallRequest) -> Result<CallReply, WorkbenchFault>; fn events(&self, principal: AuthenticatedPrincipal, request: Subscription) -> Result<EventStream, WorkbenchFault>; }`, `Subscription`/`StreamCursor`/`EventEnvelope`/`EventStream` 자리 타입(정본 시그니처, 037은 사용 안 함), `WorkbenchFault::events_unsupported()` helper (depends T006, T007, T008)
- [X] T011 [P] `crates/workbench-protocol/src/operations/project.rs`: `ProjectDto{id,name,workingDirectory,description?}`, `ProjectListInput`(빈 struct, `deny_unknown_fields`), `ProjectListOutput = Vec<ProjectDto>`, `ProjectCreateInput{name, workingDirectory, description?}`(`deny_unknown_fields`), `ProjectCreateOutput = ProjectDto`; `crates/workbench-protocol/src/operations/system.rs`: `SystemDescribeInput`(빈 struct), output은 `DescribeOutput`; `operations/mod.rs`에 `OPERATIONS: [(OperationId, OperationKind, Effect, idempotent, &[Scope])]` 정적 표. 테스트: `ProjectDto` JSON이 AW `Project`(camelCase 4필드)와 동일
- [X] T012 [P] `crates/workbench-core/src/domain/project.rs`(AW `apps/agentic-workbench/src-tauri/src/domain/project.rs`의 `Project`·`ProjectDraft` 그대로 이동, serde camelCase 유지)와 `crates/workbench-core/src/domain/project_error.rs`(`thiserror` `ProjectError{NameRequired, WorkingDirectoryRequired, NotFound, Storage(String), Clock(String)}`, Display = "Project name is required." / "Working directory is required." / "Project not found." / 내부 문자열 그대로). 테스트: 다섯 variant의 `to_string()` 골든
- [X] T013 [P] `crates/workbench-core/src/ports/project_repository.rs`(`trait ProjectRepository { fn load_projects(&self) -> Result<Vec<Project>, ProjectError>; fn save_projects(&self, &[Project]) -> Result<(), ProjectError>; fn recover_from_backup(&self) -> Result<(), ProjectError>; }`), `crates/workbench-core/src/ports/operation_ledger.rs`(`LedgerState{Pending,Applied,Failed,Unknown}`, `LedgerRecord{execution_id, principal_kind, operation, idempotency_key, input_fingerprint, aggregate, reserved_resource_id?, state, result_json?, revision?, request_id, created_at, updated_at, expires_at?}`, `LedgerKey{principal_kind, operation, contract_revision, idempotency_key}`, `trait OperationLedger { find, begin, complete, fail, reconcile_pending, gc_expired, current_revision, migrate }` 시그니처와 doc comment), `crates/workbench-core/src/ports/aggregate_lock.rs`(`trait AggregateLock { fn lock(&self, aggregate: &str) -> Guard }`)
- [X] T014 `crates/workbench-core/src/application/project_service.rs`: AW `application/project_service.rs`를 이동하되 오류를 `ProjectError`로, `create_project(repository, draft)`를 `create_project_with_id(repository, id: String, draft)`로 분리(`new_project_id()`는 `pub`으로 유지, 형식 `project-{unix_nanos}`), `normalize_draft`를 `pub`으로. `update_project`/`delete_project` 유지. 테스트: 정규화(trim, 빈 description → None), 이름·디렉터리 누락 오류의 Display 문구, update/delete NotFound 문구 (depends T012, T013)
- [X] T015 [P] `crates/workbench-core/src/infrastructure/data_paths.rs`: `DataPaths::new(app_data_dir: PathBuf)`, `projects_file()` = `<dir>/projects.json`, `ledger_file()` = `<dir>/workbench/ledger.sqlite`, `ensure_dirs()`. 테스트: 경로 조합, 디렉터리 생성
- [X] T016 [P] `crates/workbench-core/src/infrastructure/json_store.rs`: AW `infrastructure/json_store.rs`를 기반으로 하되 (a) `load<T>(path, label) -> Result<T, StoreError>`는 **읽기 전용**이고 파싱 실패 시 `StoreError::PrimaryCorrupt{path, source}` 반환(어떤 경우에도 쓰지 않음), (b) `save<T>`는 기존 temp+rename+`.bak` 로직 유지, (c) `recover_from_backup(path, label)`은 primary를 다시 파싱해 여전히 손상이면 `.bak` 내용을 temp에 쓰고 `rename`으로 교체(`fs::copy` 금지). AW 원본 테스트 2개를 포팅하고 추가 테스트: 손상 primary에 `load`를 호출해도 파일 mtime·내용이 바뀌지 않음, `recover_from_backup` 뒤 primary == backup, primary가 이미 정상이면 recover가 no-op
- [X] T017 `crates/workbench-core/src/infrastructure/json_project_repository.rs`: `JsonProjectRepository::new(paths: &DataPaths)`, `ProjectRepository` 구현(`load_projects`는 `json_store::load`, `PrimaryCorrupt`를 `ProjectError::Storage`로 매핑하되 호출자가 구분할 수 있게 별도 variant `ProjectError::StoreCorrupt` 추가 여부를 결정해 T012에 반영; `recover_from_backup` 위임). 테스트: tempdir에서 저장·재로드, 손상 후 recover (depends T013, T015, T016)
- [X] T018 `crates/workbench-core/src/infrastructure/sqlite_ledger.rs`: `SqliteOperationLedger::open(&DataPaths)`(pragma WAL/synchronous NORMAL/busy_timeout 5000/foreign_keys ON, 단일 `Mutex<Connection>`), `migrate()`(`schema_version` 없으면 v1 DDL 3 테이블 + 인덱스 생성, 있으면 버전 검사), `begin(record) -> execution_id`, `complete(execution_id, result_json, aggregate) -> revision`(**같은 트랜잭션**에서 `aggregate_revision` upsert +1, ledger row `applied`·revision·`expires_at = now+24h`), `current_revision(aggregate)`(row 없으면 0 insert). 테스트: migrate 멱등, begin→complete 전이와 revision 1·2·3, UNIQUE(principal_kind, operation, contract_revision, idempotency_key) 위반 오류, `aggregate_revision`가 ledger 삭제 후에도 유지 (depends T013)
- [X] T019 `crates/workbench-core/src/infrastructure/storage_coordinator.rs`: aggregate별 `std::sync::Mutex<()>` 맵, `with_projects_read(|repo| ...)`와 `with_projects_write(|repo| ...)`(둘 다 lock 획득; read/write 모두 `load_projects`가 `StoreCorrupt`면 **lock 안에서** `recover_from_backup` 후 1회 재시도), revision 캐시(`AtomicU64`, 생성 시 `ledger.current_revision("projects")`로 초기화, `bump()`), `#[cfg(feature = "test-hooks")] recovery_delay: Option<Duration>` hook(복구 직전 sleep). 테스트: 손상 primary에서 read가 복구 후 성공, write 중 다른 스레드 read가 lock 대기 (depends T017, T018)
- [X] T020 [P] `crates/workbench-core/src/application/authorization.rs`: `required_scopes(op) -> &[Scope]`(T011 표), `authorize(principal, op: &str) -> Result<OperationId, WorkbenchFault>`(미존재 → `notFound`, 존재하나 scope 부족 → `forbidden`), `visible_operations(principal) -> Vec<OperationId>`. 테스트: desktop 3개 전부 허용, readonly에 `project.create` forbidden·목록 제외, 미존재 이름 notFound
- [X] T021 [P] `crates/workbench-core/src/application/registry.rs`: `OperationHandler` trait(`async fn handle(&self, ctx: CallContext, input: Value) -> Result<(Value, Option<u64>), WorkbenchFault>`), `Registry{ handlers: HashMap<OperationId, Arc<dyn OperationHandler>> }`, `decode_input<T: DeserializeOwned>(request_id, input) -> Result<T, WorkbenchFault>`(serde 오류 → `invalidArgument`, `details.path`에 serde 메시지), `descriptor_for(op) -> OperationDescriptor`(schemas는 US3에서 채움, 우선 `Value::Null`). 테스트: unknown field → invalidArgument, 타입 불일치 → invalidArgument
- [X] T022 `crates/workbench-core/src/application/workbench_runtime.rs`: `WorkbenchRuntime::bootstrap(paths: DataPaths) -> Result<Arc<Self>>`(ensure_dirs → ledger open+migrate → coordinator → registry; reconcile/GC는 US2에서 추가), `impl Workbench`: `protocolVersion != 1` → `unsupportedProtocol`; `authorize`; `registry.dispatch`; `events` → `events_unsupported()`. `CallContext{principal, request_id, idempotency_key?, expected_revision?}`. 테스트: unsupportedProtocol, notFound, forbidden 경로가 handler 없이도 동작 (depends T010, T019, T020, T021)
- [X] T023 [P] `crates/workbench-core/tests/support/fixtures.rs`: `Fixture{name, principal: "desktop"|"readonly", seed: {projects: Vec<Value>}, request: Value | requests: Vec<Value>, expect: {reply|fault} | expects: Vec<...>, expect_after?: {projects_len?, ledger_applied?}}` 구조체, `load_all()`(`crates/workbench-protocol/fixtures/*.json` 정렬 순회), `apply_seed(paths)`, `assert_matches(actual, expect)`(fault는 code·message·outcome, reply는 output·revision 비교; `requestId`는 무시)
- [X] T024 [P] `crates/workbench-core/tests/support/http_harness.rs`: `spawn(runtime: Arc<dyn Workbench>) -> Harness{addr, shutdown}`; axum 0.7 router `POST /v1/calls`: `Authorization: Bearer test-desktop` → `desktop()`, `test-readonly` → `test_readonly()`, 없음/기타 → 401 `unauthenticated` problem+json; body → `CallRequest`; 성공 200 JSON, 실패 `fault.code.http_status()` + `application/problem+json`(`type`,`title`,`status` + Fault 필드); `127.0.0.1:0` bind. `tests/support/mod.rs`에서 두 모듈 re-export

**Checkpoint**: `cargo test -p workbench-protocol -p workbench-core` 통과(유닛 테스트만). runtime은 handler 없이 unsupportedProtocol/notFound/forbidden을 낸다.

---

## Phase 3: User Story 1 - 프로젝트 목록·생성이 그대로 동작하고, 데스크톱 밖에서도 같은 결과를 낸다 (Priority: P1) 🎯 MVP

**Goal**: `project.list`·`project.create`가 Workbench를 통해 동작하고, Tauri compat·in-memory·HTTP 세 경로가 같은 fixture에 같은 결과를 낸다. 사용자 화면·저장 형식·오류 문구는 변하지 않는다.

**Independent Test**: 기존 AW 테스트가 무수정 통과하고, `contract_suite`가 US1 fixture 전부를 in-memory·HTTP에서 동일 결과로 통과하며, 앱에서 프로젝트 목록·생성·이름 누락 오류가 이전과 같다(quickstart §4 1·2·4·5).

### Tests for User Story 1 ⚠️ (구현 전 작성, 실패 확인)

- [X] T025 [P] [US1] `crates/workbench-protocol/fixtures/` 에 US1 시나리오 JSON 9개 작성: `project-list-empty.json`, `project-list-seeded.json`(seed 3개, 순서 유지), `project-create-ok.json`(output에 id 형식 `^project-\d+$` 매칭 규칙 → fixture는 `"id": {"$regex": ...}` 대신 matcher가 `id` 존재만 확인하도록 `expect.ignoreFields: ["id"]`), `project-create-name-required.json`, `project-create-working-directory-required.json`(공백 문자열), `project-create-missing-idempotency-key.json`(message "idempotencyKey is required for project.create."), `operation-not-found.json`(`"operation": "project.rename"`), `input-unknown-field.json`(`project.list`에 `{"foo": 1}`), `protocol-version-unsupported.json`(protocolVersion 2 → `unsupportedProtocol`)
- [X] T026 [P] [US1] `crates/workbench-core/tests/contract_suite.rs`: fixture마다 (a) tempdir `DataPaths`로 runtime 생성 → seed → `runtime.call(principal, request)` 직접 호출, (b) 새 runtime + `http_harness::spawn` → `reqwest` POST → 응답을 `CallReply`/`WorkbenchFault`로 역직렬화; 두 결과가 `expect`와 일치하고 서로 같은지 assert. US1 fixture 9개가 모두 **실패**하는 것을 확인(handler 미구현)
- [X] T027 [P] [US1] `apps/agentic-workbench/src-tauri/src/inbound/workbench_compat.rs`의 `#[cfg(test)]`: fixture `project-create-*.json`의 `request.input`을 `ProjectInput`으로 역직렬화해 `create_project_request(input, key, request_id)`가 fixture `request`와 동일(requestId·idempotencyKey 제외)한지, `fault_to_string(&fixture.expect.fault)`가 `expect.fault.message`와 같은지. 파일이 없으므로 먼저 함수 시그니처만 선언한 뒤 테스트가 컴파일·실패하는 상태로 둔다

### Implementation for User Story 1

- [X] T028 [P] [US1] `crates/workbench-core/src/application/handlers/project_list.rs`: `decode_input::<ProjectListInput>` → `coordinator.with_projects_read(load)` → `Vec<ProjectDto>`(`From<Project>`), revision `None`
- [X] T029 [US1] `crates/workbench-core/src/application/handlers/project_create.rs`: `idempotency_key` 없으면 `invalidArgument`("idempotencyKey is required for project.create.") → `decode_input::<ProjectCreateInput>` → `ProjectDraft` → `normalize_draft`(오류 → `invalidArgument` with `ProjectError` Display) → `id = new_project_id()` → `ledger.begin(LedgerRecord{pending, reserved_resource_id: id, fingerprint: US2에서 채움(우선 빈 문자열)})` → `coordinator.with_projects_write(|repo| create_project_with_id(repo, id, draft))` → `ledger.complete(execution_id, result, "projects")` → `coordinator.bump()` → `Complete{output: ProjectDto, revision}`; 저장 오류 시 `ledger.fail(execution_id, fault)` → `unavailable`(message = ProjectError Display) (depends T014, T018, T019, T021)
- [X] T030 [US1] `crates/workbench-core/src/application/handlers/mod.rs`와 `workbench_runtime.rs`: 두 handler를 registry에 등록(`system.describe`는 US3), `lib.rs`에서 `pub use application::workbench_runtime::WorkbenchRuntime; pub use infrastructure::data_paths::DataPaths; pub use workbench_protocol::*` 재노출. `contract_suite` US1 fixture 9개 통과 확인 (depends T028, T029)
- [X] T031 [US1] `apps/agentic-workbench/src-tauri/Cargo.toml`에 `workbench-core = { path = "../../../crates/workbench-core" }` 추가; `apps/agentic-workbench/src-tauri/src/lib.rs` `.setup()` 첫 부분에서 `app.path().app_data_dir()` → `DataPaths::new` → `WorkbenchRuntime::bootstrap` → `app.manage(runtime: Arc<WorkbenchRuntime>)`. 실패 시 기존 `Failed to resolve app data directory: …` 문구로 setup 오류 반환
- [X] T032 [US1] `apps/agentic-workbench/src-tauri/src/inbound/workbench_compat.rs`: `desktop_principal()`, `list_projects_request(request_id) -> CallRequest`, `create_project_request(input: ProjectInput, idempotency_key, request_id) -> CallRequest`(`ProjectInput → ProjectCreateInput`), `reply_to_projects(CallReply) -> Result<Vec<Project>, String>`, `reply_to_project(CallReply) -> Result<Project, String>`, `fault_to_string(&WorkbenchFault) -> String`(= `message.clone()`); `inbound/mod.rs`에 `pub mod workbench_compat;`. T027 테스트 통과 (depends T031)
- [X] T033 [US1] `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`: `list_projects`/`create_project`를 `async fn`으로 바꾸고(프론트 `invoke` 호출 방식은 동일) `app.state::<Arc<WorkbenchRuntime>>()` → compat 요청 생성(`requestId`·`idempotencyKey`는 `uuid::Uuid::new_v4()`) → `runtime.call` → compat 변환. `update_project`/`delete_project`는 `runtime.coordinator().with_projects_write(|repo| project_service::update_project(repo, id, draft))` 형태로 **같은 lock·repository**를 쓰게 바꾸고 오류는 `ProjectError` Display를 `String`으로. `impl From<ProjectInput> for ProjectDraft`의 `ProjectDraft`를 `workbench_core::domain::project::ProjectDraft`로 변경 (depends T032)
- [X] T034 [US1] AW에서 이동된 파일 삭제와 import 정리: `apps/agentic-workbench/src-tauri/src/application/project_service.rs`, `src/domain/project.rs`, `src/domain/project_repository.rs`, `src/infrastructure/json_project_repository.rs` 삭제; `src/application/mod.rs`·`src/domain/mod.rs`·`src/infrastructure/mod.rs`의 `pub mod` 제거; `src/domain/mod.rs`에 `pub use workbench_core::domain::project;`를 두어 `crate::domain::project::Project`를 쓰는 다른 모듈(`grep -rn "domain::project" src`로 확인)이 그대로 컴파일되게 한다. `cargo build -p agentic-workbench` 통과 (depends T033)
- [X] T035 [P] [US1] `crates/workbench-core/tests/list_latency.rs`: `#[ignore]` 테스트 — seed 50개, `project.list` 1,000회 in-memory 호출 p95 계산 후 5ms 미만 assert, `--nocapture`로 수치 출력 (depends T030)
- [X] T036 [US1] 검증: `cargo test -p workbench-protocol -p workbench-core -p agentic-workbench` 전부 통과, T001 기준선 대비 AW 통과 수 감소 없음; `cd apps/agentic-workbench && pnpm tauri dev`로 quickstart §4의 1(목록 동일)·2(`projects.json` 형식 불변, `workbench/ledger.sqlite` 생성)·4(이름 누락 → "Project name is required.")·5(수정·삭제 동작) 확인 후 결과를 Notes에 기록

**Checkpoint**: US1 완료 = MVP. 프론트 diff 0(`git diff --stat -- apps/agentic-workbench/src`), 세 경로 동일 결과, 화면 무회귀.

---

## Phase 4: User Story 2 - 변경 요청은 재시도·중단·동시 실행에도 정확히 한 번만 적용된다 (Priority: P2)

**Goal**: 멱등성 재요청·충돌, `expectedRevision`, 중단 뒤 reconciler 판정(`applied`/`unknown`), TTL GC와 revision 영속성, 동시 요청 직렬화, 손상 파일 복구의 lock 직렬화를 구현하고 테스트로 고정한다.

**Independent Test**: `ledger_crash_points`·`concurrency`·`revision_retention`·`recovery_under_lock`와 US2 fixture(`contract_suite`)가 통과한다(SC-003, SC-004, FR-007~FR-010).

### Tests for User Story 2 ⚠️ (구현 전 작성, 실패 확인)

- [X] T037 [P] [US2] fixture 추가(`requests` 시퀀스 형식, T023 loader가 지원): `crates/workbench-protocol/fixtures/project-create-idempotent-replay.json`(같은 key·같은 input 2회 → 두 번째가 첫 번째와 동일 output·revision, `expect_after.projects_len = 1`), `project-create-conflict-different-payload.json`(같은 key·다른 name → 두 번째 `conflict`, outcome `applied`), `project-create-stale-revision.json`(생성 1회 → `expectedRevision: 0`으로 재생성 → `preconditionFailed`, `details.currentRevision = 1`), `project-create-whitespace-same-fingerprint.json`(`" AW "`와 `"AW"`가 같은 지문 → 재생 응답)
- [X] T038 [P] [US2] `crates/workbench-core/tests/ledger_crash_points.rs`: `CrashPoint{AfterPending, AfterJsonSave, BeforeApplied}` hook을 runtime에 설정 → `project.create` 호출이 해당 지점에서 `Err(CrashInjected)`로 중단 → runtime drop → 같은 `DataPaths`로 `bootstrap`(reconcile 실행) → `AfterPending`은 `unknown`(JSON에 id 없음)이고 같은 key 재요청이 `conflict` outcome `unknown`; `AfterJsonSave`·`BeforeApplied`는 `applied`로 확정되고 재요청이 저장 결과 반환, `projects.json`에 정확히 1개
- [X] T039 [P] [US2] `crates/workbench-core/tests/concurrency.rs`: (a) 서로 다른 key 20개 `tokio::spawn` 동시 create → 프로젝트 20, `current_revision` 20, ledger `applied` 20; (b) 같은 key 20개 동시 → 프로젝트 1, `applied` 1, 나머지는 저장 결과 또는 `conflict`(pending) 중 하나이며 어느 경우도 프로젝트가 늘지 않음. in-memory와 HTTP harness 각각
- [X] T040 [P] [US2] `crates/workbench-core/tests/revision_retention.rs`: 생성 3건(revision 3) → 테스트 helper로 모든 row `expires_at`를 과거로 갱신 → `gc_expired` → runtime 재생성 → 4번째 생성 revision == 4, `expectedRevision: 3` 요청은 `preconditionFailed`(`currentRevision` 4), `operation_ledger` row는 1개(새 것)뿐이지만 `aggregate_revision.revision == 4`
- [X] T041 [P] [US2] `crates/workbench-core/tests/recovery_under_lock.rs`: `projects.json`을 잘못된 JSON으로 덮고 `.bak`에 프로젝트 2개 → `recovery_delay = 200ms` hook → read 1개와 create 5개를 동시에 spawn → 최종 `projects.json`에 7개(복구 2 + 생성 5), ledger `applied` 5건의 id가 모두 파일에 존재, `.bak` 복구 프로젝트가 사라지지 않음

### Implementation for User Story 2

- [X] T042 [P] [US2] `crates/workbench-core/src/application/idempotency.rs`: `canonical_json(&Value) -> String`(객체 키 BTreeMap 정렬 재직렬화), `fingerprint(operation, &normalized_input) -> String`(sha256 hex), `ReplayDecision{Proceed, ReturnStored(CallReply|WorkbenchFault), Conflict(WorkbenchFault)}`, `decide(existing: Option<&LedgerRecord>, fingerprint, request_id) -> ReplayDecision`(data-model §3 "같은 키 재요청" 표 7행 그대로; `pending` → conflict outcome unknown retryable true message "같은 요청이 처리 중입니다.", `unknown` → retryable false message "이전 요청의 적용 여부를 확인할 수 없습니다. 프로젝트 목록을 확인하세요."). 테스트: 표 7행 전수, 키 순서 다른 JSON의 지문 동일
- [X] T043 [US2] `crates/workbench-core/src/infrastructure/sqlite_ledger.rs` 확장: `find(&LedgerKey) -> Option<LedgerRecord>`, `fail(execution_id, fault_json)`(`failed`, `expires_at = now+24h`), `reconcile_pending(|reserved_id| -> bool)`(각 `pending`에 대해 closure가 true면 같은 트랜잭션에서 `applied` + `aggregate_revision` +1 + `result_json`은 closure가 돌려준 Project JSON, false면 `unknown`; `pending`/`unknown`은 `expires_at` NULL 유지), `gc_expired(now) -> usize`(`applied`/`failed`만 삭제). 테스트: 각 메서드 + `aggregate_revision`은 GC와 무관
- [X] T044 [US2] `crates/workbench-core/src/application/handlers/project_create.rs` 확장: 정규화 뒤 `fingerprint` 계산 → `ledger.find` → `idempotency::decide` → `ReturnStored`/`Conflict`면 즉시 반환(authorization은 이미 통과) → `Proceed`면 `begin`(fingerprint·request_id 기록) → lock 안에서 `expected_revision`이 있고 `coordinator.revision() != expected`면 `ledger.fail(preconditionFailed{currentRevision})` 후 Fault → 저장 → `complete`. `#[cfg(feature = "test-hooks")] crash_point` 검사 3곳 삽입(`AfterPending`은 begin 직후, `AfterJsonSave`는 save 직후 lock 안, `BeforeApplied`는 complete 직전) (depends T042, T043)
- [X] T045 [US2] `crates/workbench-core/src/application/workbench_runtime.rs` 확장: `bootstrap` 순서를 `ensure_dirs → ledger.open+migrate → gc_expired(now) → coordinator 생성 → reconcile_pending(closure: lock 안에서 `load_projects`(손상 시 recover) 후 reserved id 존재 여부와 해당 Project JSON) → revision 캐시 로드`로 고정; `set_crash_point`/`set_recovery_delay`(`test-hooks`); mutation 50회마다 `gc_expired` 호출하는 카운터 (depends T043, T019)
- [X] T046 [US2] `crates/workbench-core/tests/support/fixtures.rs` 확장: `requests` 시퀀스 실행(같은 runtime에 순차 호출), `expects` 인덱스별 비교, `expect_after.projects_len`을 `projects.json` 파싱으로, `expect_after.ledger_applied`를 ledger 조회 helper로 검증; `ignoreFields`(예: `id`)와 `details.currentRevision` 비교 지원. `contract_suite`가 US1+US2 fixture 13개를 in-memory·HTTP 양쪽에서 통과 (depends T044, T045)
- [X] T047 [US2] `cargo test -p workbench-core --test ledger_crash_points --test concurrency --test revision_retention --test recovery_under_lock --test contract_suite`로 T038~T041과 US2 fixture 통과 확인, `crates/workbench-core/tests/recovery_under_lock.rs`에서 hook 없이도(delay 0) 통과하는지 재확인. 실측 수치(20건 동시 소요 시간, 복구 시간)를 이 파일 Notes에 기록 (depends T045, T046)

**Checkpoint**: US1·US2 모두 독립 통과. `sqlite3 workbench/ledger.sqlite 'select state,count(*) from operation_ledger group by state'`로 상태 분포 확인 가능.

---

## Phase 5: User Story 3 - 새 클라이언트 개발자가 계약을 조회하고 타입 안전하게 호출한다 (Priority: P3)

**Goal**: `system.describe`가 principal별 허용 operation과 스키마를 돌려주고, 권한 없는 호출은 `forbidden`이며, registry에서 OpenAPI 3.1과 TypeScript 타입이 생성·커밋되고 CI가 drift를 잡는다.

**Independent Test**: US3 fixture 3개 통과, `pnpm --filter @yoophi/workbench-client check-types test` 통과(`@ts-expect-error` 포함), `pnpm generate:contracts && git diff --exit-code` 통과, 의도적 스키마 변경 시 실패(SC-005, SC-006).

### Tests for User Story 3 ⚠️ (구현 전 작성, 실패 확인)

- [X] T048 [P] [US3] fixture: `crates/workbench-protocol/fixtures/system-describe-desktop.json`(operations 3개, 각 `id`·`kind`·`effect`·`idempotent`·`requiredScopes` 값, `inputSchema`/`outputSchema`는 non-null만 확인 → `expect.schemaPresent: true`), `system-describe-readonly.json`(principal readonly, `project.create` 없음, 2개), `project-create-forbidden-readonly.json`(readonly가 create → `forbidden`, outcome `notApplied`)
- [X] T049 [P] [US3] `packages/workbench-client/src/operation-map.test-d.ts`(vitest `expectTypeOf`): `OperationMap["project.list"]["input"]`은 `Record<string, never>`류 빈 객체, `["output"]`은 `Project[]`; `OperationMap["project.create"]["output"]`은 `Project`; `// @ts-expect-error`로 `const x: OperationMap["project.create"]["output"] = [] as Project[]`; `OperationId`가 정확히 세 리터럴 union. `vitest.config.ts`에 `typecheck: { enabled: true }` 설정. 생성 파일이 없어 실패하는 상태로 둔다
- [X] T050 [P] [US3] `crates/workbench-protocol/src/openapi.rs`의 `#[cfg(test)]`: `build_openapi()` 직렬화가 `openapi/workbench.openapi.json` 파일 내용과 바이트 동일(golden; 파일 없으면 실패), `CallRequest.oneOf`가 정확히 3 variant이고 각 variant `properties.operation.enum`이 단일 값, `CallReplyByOperation.oneOf`도 3개, `openapi == "3.1.0"`

### Implementation for User Story 3

- [X] T051 [P] [US3] `crates/workbench-core/src/application/handlers/system_describe.rs`: `decode_input::<SystemDescribeInput>` → `authorization::visible_operations(principal)` → 각 `registry.descriptor_for(op)` → `DescribeOutput{protocolVersion: 1, operations}`; registry 등록
- [X] T052 [US3] `crates/workbench-core/src/application/registry.rs` 확장: `descriptor_for`가 `workbench_protocol::operations::schema_for(op) -> (input: Value, output: Value)`를 호출해 `inputSchema`/`outputSchema`를 채움; `workbench-protocol/src/operations/mod.rs`에 `schema_for`를 utoipa `ToSchema::schema()`(또는 `utoipa::openapi::schema`) → `serde_json::to_value`로 구현. `contract_suite`가 US3 fixture 3개 통과 (depends T051)
- [X] T053 [US3] `crates/workbench-protocol/src/openapi.rs`: `build_openapi() -> utoipa::openapi::OpenApi` — `#[derive(OpenApi)]`로 components(`ProjectDto`, `ProjectListInput`, `ProjectCreateInput`, `SystemDescribeInput`, `DescribeOutput`, `OperationDescriptor`, `WorkbenchFault`, `CallReply`) 수집 후 `Modify`로 (a) `CallRequest`를 `OPERATIONS` 표 순회로 variant object(`operation: {type: string, enum: [id]}`, `input: $ref`, 공통 필드 protocolVersion/requestId/idempotencyKey/expectedRevision/timeoutMs) `oneOf`로 조립, (b) `CallReplyByOperation` `oneOf`(`operation` enum 단일값 + `output: $ref`)를 추가, (c) `paths["/v1/calls"].post` requestBody/responses(200 `CallReply`, default `WorkbenchFault` problem+json) 기록, `info.version = "1"`. T005 spike 결과가 `json!` 조립이었다면 그 방식 유지. T050 golden 테스트는 파일 생성(T054) 뒤 통과
- [X] T054 [US3] `crates/workbench-protocol/src/bin/export_openapi.rs`: `build_openapi()`를 `serde_json::to_string_pretty` + 끝 개행으로 stdout에 출력. `cargo run -q -p workbench-protocol --bin export_openapi > crates/workbench-protocol/openapi/workbench.openapi.json` 실행해 커밋 대상 파일 생성, T050 통과 (depends T053)
- [X] T055 [US3] `packages/workbench-client/package.json` `generate` 스크립트: `cargo run -q -p workbench-protocol --bin export_openapi > ../../crates/workbench-protocol/openapi/workbench.openapi.json && openapi-typescript ../../crates/workbench-protocol/openapi/workbench.openapi.json -o src/generated/workbench.ts`; 실행해 `packages/workbench-client/src/generated/workbench.ts` 생성; `packages/workbench-client/src/operation-map.ts`(data-model §6의 `Req`/`Res`/`OperationId`/`OperationMap`/`Call` 타입, 생성 파일의 `components["schemas"]` 참조); `src/index.ts`에서 `export type * from "./generated/workbench"; export * from "./operation-map";`. T049 타입 테스트 통과 (depends T054)
- [X] T056 [US3] 루트 `package.json` scripts에 `"generate:contracts": "pnpm --filter @yoophi/workbench-client generate"`; `.github/workflows/quality.yml`의 `Set up Rust` 단계 **뒤**, `Install dependencies` 뒤에 `- name: Check contract drift` / `run: pnpm run generate:contracts && git diff --exit-code -- crates/workbench-protocol/openapi packages/workbench-client/src/generated` 단계 추가(Rust가 먼저 설치돼야 `cargo run` 가능; 현재 순서는 Node → pnpm → Rust → install이므로 install 뒤가 맞음) (depends T055)
- [X] T057 [US3] 검증: `pnpm --filter @yoophi/workbench-client check-types && pnpm --filter @yoophi/workbench-client test`; `pnpm run generate:contracts && git diff --exit-code -- crates/workbench-protocol/openapi packages/workbench-client/src/generated`; SC-005 확인 — `ProjectCreateInput`의 필드 이름을 임시로 바꾸고 같은 명령이 실패하는지 본 뒤 되돌림; `pnpm run check-types` 전체(다른 앱 영향 없음) 통과. 결과를 Notes에 기록 (depends T056)

**Checkpoint**: 세 스토리 모두 독립 통과. 생성물 2개가 git에 있고 CI 단계가 추가됨.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: 문서, 전체 검증, 잔여 정리.

- [X] T058 [P] `docs/workbench-seam.md` 신규(한국어, 영문 파일명): 범위·비범위, 세 호출 경로와 Seam 관계(Mermaid flowchart), intent-first 상태 전이(Mermaid stateDiagram), 같은 키 재요청 응답 표, 오류 코드 부분집합, principal·scope, 계약 생성·drift 검사 절차, 038 이후 이관 절차(도메인 이동 템플릿 = 037의 project 이동 순서), 완료 기준과 검증 명령(quickstart 링크)
- [X] T059 [P] `docs/client-server-architecture-research.md` 상단 인용 블록 아래에 "진행 상태: 1단계 첫 세로 slice(1a)는 [Workbench Seam](workbench-seam.md)·`specs/037-workbench-seam`으로 완료(2026-09-xx). 나머지 command 이관은 038." 각주 한 문단 추가. 생성 OpenWiki 페이지는 손대지 않음
- [X] T060 [P] 잔여 참조 정리: `grep -rn "JsonProjectRepository\|application::project_service\|from_app" apps/agentic-workbench/src-tauri/src | grep -i project`가 0건인지, `apps/agentic-workbench/src-tauri/src/infrastructure/json_store.rs`에 core로 옮긴 함수와 중복된 주석/코드가 남아 있으면 "038에서 제거" TODO 주석만 남김; `git diff --stat -- apps/agentic-workbench/src`가 비어 있는지 확인
- [X] T061 전체 게이트: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`(새 crate의 `#![deny(...)]` 없이 workspace 기준), `cargo test --workspace --all-targets`, `pnpm run check-types`, `pnpm run test`, `pnpm run build` 모두 통과. 실패 항목은 수정 후 재실행
- [X] T062 quickstart.md §1~§4·§7을 처음부터 끝까지 실행하고 각 항목 결과를 이 파일 Notes에 기록. `list_latency` 수치, bundled 빌드 시간, 앱 기동 시간 체감을 포함. spec의 SC-001~SC-006 각 항목에 대응하는 증거를 한 줄씩 남긴다
- [X] T063 `specs/037-workbench-seam/plan.md` 리스크 표에 실측(빌드 시간, spike 결과)을 반영하고, `specs/037-workbench-seam/checklists/requirements.md` Notes에 구현 중 spec과 어긋난 점이 있으면 기록. 커밋 메시지 초안(`feat(aw): introduce Workbench seam with SQLite operation ledger (037, stage 1a)`)과 PR 본문 초안을 Notes에 작성 — 커밋·PR은 사용자 지시 후

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001~T004 즉시 시작 가능. T005 spike는 T002·T004 뒤.
- **Foundational (Phase 2)**: Setup 완료 뒤. 내부 순서 — T006~T009·T011~T013·T015·T016·T020·T021·T023·T024는 [P]; T010 ← T006~T008; T014 ← T012·T013; T017 ← T013·T015·T016; T018 ← T013; T019 ← T017·T018; T022 ← T010·T019·T020·T021. **모든 스토리를 막는다.**
- **US1 (Phase 3)**: Foundational 뒤. 테스트 T025~T027 [P] 먼저 → T028 [P] → T029 → T030 → T031 → T032 → T033 → T034 → T035 [P] → T036.
- **US2 (Phase 4)**: Foundational 뒤 시작 가능하지만 `project_create` handler(T029)를 확장하므로 실질적으로 US1의 T030 뒤. 테스트 T037~T041 [P] → T042 [P] → T043 → T044 → T045 → T046 → T047.
- **US3 (Phase 5)**: Foundational 뒤 시작 가능. T048~T050 [P] → T051 [P] → T052 → T053 → T054 → T055 → T056 → T057. US1·US2와 파일이 겹치지 않아 병렬 가능(단 `registry.rs` T052는 T021 뒤).
- **Polish (Phase 6)**: 세 스토리 완료 뒤. T058~T060 [P] → T061 → T062 → T063.

### User Story Dependencies

- **US1 (P1)**: Foundational만 필요. 독립 검증 가능(MVP).
- **US2 (P2)**: US1의 handler를 확장한다. US1 없이 시작하면 T029를 US2에서 함께 작성해야 하므로 순서상 US1 뒤를 권장.
- **US3 (P3)**: `system.describe` handler와 protocol `openapi.rs`·TS 패키지는 US1·US2와 독립. `contract_suite` 파일은 공유하지만 fixture만 추가한다.

### Within Each User Story

- 헌장 필수 테스트(fixture·순수 로직·안전 경계)를 먼저 쓰고 **실패를 확인**한 뒤 구현
- protocol 타입 → core domain/ports → infrastructure → application handler → runtime 등록 → AW inbound 배선 순
- 공유 crate 변경 뒤에는 소비 앱(`agentic-workbench`) Rust 검사까지 한 묶음(헌장 V)

### Parallel Opportunities

- Phase 1: T002·T003·T004 동시
- Phase 2: T006·T007·T008·T009·T011·T012·T013·T015·T016·T020·T021·T023·T024 동시(13개), 이후 T010·T014·T017·T018 동시
- US1: T025·T026·T027 동시; T028·T035는 다른 파일이라 T029 진행 중 병렬 가능
- US2: T037~T041 다섯 테스트 파일 동시; T042는 T043과 병렬
- US3: T048·T049·T050 동시; T051은 T053과 병렬
- Polish: T058·T059·T060 동시

---

## Parallel Example: User Story 1

```bash
# 테스트 먼저 (세 파일, 서로 독립)
Task: "US1 fixture 9개 작성 in crates/workbench-protocol/fixtures/"
Task: "contract_suite.rs in crates/workbench-core/tests/"
Task: "workbench_compat.rs 테스트 in apps/agentic-workbench/src-tauri/src/inbound/"

# 구현 (list handler와 latency 테스트는 create handler와 파일이 다름)
Task: "handlers/project_list.rs"
Task: "tests/list_latency.rs"
# 그 뒤 순차: project_create.rs → runtime 등록 → lib.rs 배선 → compat → tauri_commands → AW 파일 삭제 → 검증
```

## Parallel Example: Foundational

```bash
# protocol 4 + core domain/ports/infra 5 + application 2 + test support 2 = 13개 동시
Task: "call.rs" ; Task: "fault.rs" ; Task: "principal.rs" ; Task: "descriptor.rs"
Task: "operations/{project,system}.rs" ; Task: "domain/{project,project_error}.rs" ; Task: "ports/*.rs"
Task: "data_paths.rs" ; Task: "json_store.rs"
Task: "authorization.rs" ; Task: "registry.rs"
Task: "tests/support/fixtures.rs" ; Task: "tests/support/http_harness.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 완료 — **T005 spike 결과를 먼저 확인**한다. 실패하면 R1을 `json!` 조립으로 바꾸고 진행.
2. Phase 2 완료 — runtime이 handler 없이 unsupportedProtocol/notFound/forbidden을 낸다.
3. Phase 3 완료 — 앱이 이전과 똑같이 동작하고 세 경로 결과가 같다.
4. **STOP and VALIDATE**: quickstart §4 수동 확인, 프론트 diff 0.
5. 여기까지가 정본이 말한 "권장 첫 구현 단위"와 등가다. 리뷰를 받아도 된다.

### Incremental Delivery

1. US2 추가 → crash·동시성·revision·복구 테스트 통과 → ledger가 실제로 보호하는 것을 시연
2. US3 추가 → 생성물 커밋, CI drift 단계 → `pnpm generate:contracts` 시연
3. Polish → 문서·전체 게이트 → PR

### Parallel Team Strategy

한 사람이 진행하는 경우 위 순서를 따른다. 둘이면 Foundational 뒤 A: US1→US2, B: US3(protocol `openapi.rs`·TS 패키지)로 나눌 수 있다. `registry.rs`(T021→T052)와 `contract_suite.rs` fixture 추가만 조정한다.

---

## Notes

- [P] = 다른 파일, 미완료 의존 없음
- 커밋은 논리 단위마다 하되 사용자 지시 전에는 push·PR 하지 않는다
- 각 checkpoint에서 멈춰 스토리를 독립 검증한다
- 기준선·실측 기록 (T001, T003, T036, T047, T057, T062):
  - (T001) AW `cargo test --lib` 통과 수: **223** / `pnpm check-types`: **12/12 tasks successful** (2026-09-26)
  - (T003) `rusqlite` 0.40.2 bundled(SQLite 3.53) 첫 빌드: workbench-core 의존성 컴파일 포함 **약 11초**(로컬 Apple Silicon, debug). CI 영향은 작다.
  - (T005) spike 결과: **성공**. utoipa 5.5.0 ToSchema + registry 순회 oneOf → openapi-typescript 7.13.0이 `{operation: "project.list"; input: ProjectListInput} | …` 판별 union 생성. `Extract<CallRequest, {operation: K}>["input"]` 동작. newtype은 수동 `PartialSchema` 필요(research R1 spike 결과 소절).
  - (T036) quickstart §4 결과: 백그라운드 스모크 — 앱 dev 기동 시 `workbench/ledger.sqlite` 생성 여부와 `projects.json` 해시 불변을 확인(결과는 T062 항목).
  - (T047) 동시 20건: `concurrency` 3 테스트 0.19초(in-memory 20건·같은 키 20건·HTTP 20건), `recovery_under_lock` 3 테스트 0.26초(복구 지연 200ms 포함), `ledger_crash_points` 4 테스트 0.04초.
  - (T057) drift 검사: `ProjectCreateInput.name`→`title`로 바꾸고 `pnpm run generate:contracts && git diff --exit-code`가 **실패(2 files changed)**, 복원 후 **통과**. `/workbench-client` tsc 통과, vitest typecheck 5 tests · Type Errors: no errors.
  - (T061) `cargo fmt --check` clean, `cargo clippy --workspace --all-targets -D warnings` clean, `cargo test --workspace --all-targets` **462 passed / 0 failed**(AW lib 227 = 기준선 223 + compat 4).
  - (T062) SC 증거: SC-001 AW 기존 테스트 무수정 통과(227), 프론트 diff 0 · SC-002 fixture 16개 × in-memory/HTTP 동일(`contract_suite`) · SC-003 crash 3지점 판정(`ledger_crash_points`), 중복 생성 0 · SC-004 20건 동시 → 20 projects/revision 20/lost 0(`concurrency`) · SC-005 drift 검출 + `-expect-error`(`operation-map.test-d.ts`) · SC-006 readonly describe에 create 없음 + create forbidden(`system-describe-readonly`, `project-create-forbidden-readonly`)
- (T063) 구현 중 spec 대비 어긋난 점: 없음. 보완 사항 — `ProjectError::StoreCorrupt` variant를 추가해 coordinator가 복구 재시도를 판단(spec Key Entities에는 없던 세부), `events`는 `unsupportedSchema`로 응답(2단계 전), `update_project`/`delete_project`는 operation 없이 runtime lock·repository만 공유(research R4). 프론트엔드 변경 0.
- (T063) 커밋 메시지 초안:
  `feat(aw): introduce Workbench seam with SQLite operation ledger (037, stage 1a)`
  본문: `crates/workbench-protocol`(wire 계약·OpenAPI 3.1 생성)과 `crates/workbench-core`(Workbench 구현·registry·StorageCoordinator·rusqlite ledger)를 신설하고 프로젝트 도메인을 AW에서 core로 이동. `list_projects`/`create_project` Tauri command는 `Workbench.call` 호환 어댑터가 되며 시그니처·오류 문구 불변. `project.create`는 intent-first(pending→JSON atomic save→applied)로 멱등성·중단 복구·동시성 보장. `aggregate_revision`으로 revision을 TTL GC와 분리, 저장 파일 `.bak` 복구는 aggregate lock 안에서만. `packages/workbench-client`에 생성 타입과 `OperationMap`, CI에 contract drift 검사. 프론트엔드 변경 없음.
- (T063) PR 본문 초안: Summary — 위 커밋 본문 요약 + 설계 문서 링크(`docs/workbench-seam.md`, `specs/037-workbench-seam/`) + Codex adversarial review 2건 반영 표(plan.md). Test plan — `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace --all-targets`(462 passed), `pnpm run check-types`, `pnpm run test`, `pnpm run generate:contracts && git diff --exit-code`, 앱 스모크(ledger 생성·projects.json 불변). 범위 밖 — 나머지 command 이관(038), 이벤트(2단계), 운영 HTTP(3단계).
- (T036/T062) 앱 스모크(2026-09-26, `pnpm tauri dev`, 포트 1421): 기동 30초 뒤 `~/Library/Application Support/com.yoophi.agentic-workbench/workbench/ledger.sqlite`가 생성되고 `schema_version=1`, `aggregate_revision(projects)=0`이 기록됨. `projects.json` SHA-1이 기동 전후 동일(`a19ecc2e…`) — 읽기 경로가 파일을 건드리지 않음을 실기기에서 확인. 설치된 정식 앱(다른 코드)이 동시에 떠 있었지만 서로 영향 없음. **UI 조작 항목(quickstart §4의 1·4·5: 목록 육안 확인, 이름 누락 문구, 수정·삭제)은 자동화하지 않았고 compat 유닛 테스트·contract suite·AW 기존 테스트 227개로 대체 검증했다.** 실제 화면에서 프로젝트를 만들어 ledger에 `applied` row가 생기는지는 PR 리뷰 시 수동 확인 항목으로 남긴다.
- (리뷰 2회차 반영, 2026-09-26) Codex adversarial review 3건 + 점검 중 추가 발견 1건을 반영(plan.md "Codex adversarial review 반영" 2회차 표). 변경: `project_create.rs`에 `Locked::SavedButUnconfirmed`(저장 뒤 complete 실패 → pending 유지·unknown 응답), `TestHooks::FailPoint::LedgerComplete`; `json_store::recover_from_backup<T>` typed 검증; `CallReply`/`CallRequest`/`EventEnvelope`의 임의 JSON 필드를 `value_type = Value`로, `CallReply` variant 단위 `rename_all`로 `executionId` 교정. 테스트 +5(`ledger_complete_failure_after_save_is_reported_unknown_and_reconciled`, `recovery_shape_tests` 3, `generic_reply_envelope_accepts_any_output_and_uses_camel_case`), TS test-d +1. 재검증: clippy clean, fmt clean, `cargo test --workspace --all-targets` **467 passed / 0 failed**, TS typecheck 6 tests, 생성물 재생성 후 drift 없음, 프론트 diff 0.
- (OCR 리뷰, 2026-09-26) `ocr review`는 LLM 엔드포인트 미설정으로 실행 불가 → `ocr delegate`(파일 66개·규칙 5그룹) 기준으로 직접 검토. 반영 3건: `project_create.rs` 예약 id 나노초 충돌(`DuplicateReservation`) 시 id 재생성 최대 3회; ledger 테스트 helper 3개(`set_expires_at_for_all`·`count_by_state`·`applied_resource_ids`)를 `#[cfg(any(test, feature = "test-hooks"))]` 뒤로; `quality.yml` `validate` job에 `timeout-minutes: 45`. 버려진 `ledger.fail()` 결과에는 동작 근거 주석. 재검증: clippy clean(test-hooks 없는 production lib 포함), 467 passed / 0 failed, fmt clean.
