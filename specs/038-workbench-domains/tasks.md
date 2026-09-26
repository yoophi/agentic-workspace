---

description: "Task list for migrating the remaining server-owned domains behind the Workbench seam (stage 1b)"
---

# Tasks: 나머지 도메인의 Workbench 이관 (서버-클라이언트 전환 1b)

**Input**: Design documents from `/specs/038-workbench-domains/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/workbench-operations.md](./contracts/workbench-operations.md), [contracts/tauri-compat-commands.md](./contracts/tauri-compat-commands.md), [quickstart.md](./quickstart.md)

**Tests**: 사용자 여정 테스트는 spec이 요구하지 않았지만 **헌장이 요구하는 테스트는 필수**다 — 공유 crate 변경(핵심 로직 이동), 순수 로직(정규화·오류 매핑·DTO 변환·재시작 판정), 안전 경계(파일 root 검사·ledger·저장 복구). 각 스토리에서 fixture와 테스트를 먼저 쓰고 실패를 확인한 뒤 구현한다. 037의 contract suite·crash point·동시성 테스트는 **수정 없이** 계속 통과해야 한다(행위 보존 증거).

**Organization**: 사용자 스토리별로 묶는다. US1(저장 단위 4 도메인 13 op)이 MVP, US2(Git·worktree 14 op), US3(agent 2 op), US4(인벤토리·완료 판정)이 뒤따른다.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 실행 가능 (다른 파일, 미완료 의존 없음)
- **[Story]**: 해당 사용자 스토리 (US1–US4)
- 모든 태스크에 정확한 파일 경로를 포함한다

## Path Conventions

- **Reusable Rust**: `crates/workbench-protocol/src`, `crates/workbench-core/src`, `crates/workbench-core/tests`, `crates/workbench-protocol/fixtures`
- **App Tauri backend**: `apps/agentic-workbench/src-tauri/src/{domain,application,inbound,infrastructure,ports}`
- **Reusable TypeScript**: `packages/workbench-client/src`
- **Documentation**: `docs/[english-file-name].md`

프론트엔드 `apps/agentic-workbench/src/**`는 변경하지 않는다(FR-013). 공유 crate `crates/git-core`·`crates/acp-agent-core`는 변경하지 않는다(grill Q6, ADR 0002). 워크트리는 `/Users/yoophi/project/worktrees/038-workbench-domains`이며 모든 명령은 그 루트에서 실행한다.

### 공통 규칙

- **문구 보존**: [contracts/workbench-operations.md §3](./contracts/workbench-operations.md#3-보존-문구골든)의 골든 문구를 바이트 단위로 유지한다. 도메인 오류 enum의 `Display`가 그 문구를 낸다(research R7).
- **wire 동일성**: 도메인 타입을 옮기거나 DTO를 만들 때마다 `crates/workbench-core/src/application/dto.rs`의 `assert_wire_parity(domain, dto)`로 JSON 동일성을 고정한다(R1). DTO serde 속성(`rename_all = "camelCase"`, `skip_serializing_if`, `default`)은 원본에서 그대로 복사한다.
- **input 엄격성**: operation 최상위 input struct만 `#[serde(deny_unknown_fields)]`, 중첩 도메인 DTO는 원본 관용 규칙(R2).
- **변경 operation**: 전부 `application/intent_first.rs::run_command`를 거친다. handler는 `MutationSpec`(aggregate, 정규화 입력, `Reservation`, `apply` closure)만 만든다. 재시작 판정은 [data-model §4](./data-model.md#재시작-판정-규칙-r6) 표.
- **조회 operation**: 저장 단위를 읽는 것만 `with_aggregate` lock 안에서, Git·파일 조회는 lock 없이 `tokio::task::spawn_blocking`(R12).
- **fixture**: `crates/workbench-protocol/fixtures/<domain>-<verb>-<case>.json`. 문자열 안 `{{repo}}`·`{{repoName}}`은 test support가 치환(R8). 환경 의존 값은 `ignoreFields`.
- **테스트 전용 hook**은 037처럼 cargo feature `test-hooks` 뒤에 둔다. 프로덕션 빌드 clippy(`cargo clippy -p workbench-core --lib`)도 clean이어야 한다.
- 커밋은 논리 단위마다 하되 사용자 지시 전에는 push·PR 하지 않는다.

---

## Phase 1: Setup (기준선과 의존성)

**Purpose**: 회귀 판정 기준선을 남기고 core의 새 의존성과 모듈 골격을 만든다.

- [X] T001 기준선 기록: `cargo test --workspace --all-targets 2>&1 | grep -E "^test result" | awk '{s+=$4} END {print s}'`(037 종료 시 467), `cargo test -p agentic-workbench --lib 2>&1 | tail -3`, `pnpm run check-types`, `git diff --stat origin/main -- apps/agentic-workbench/src`(0이어야 함), `ls apps/agentic-workbench/src-tauri/src/{domain,application,infrastructure,ports} | wc -l`을 실행해 이 파일 하단 Notes에 기록한다
- [X] T002 [P] `crates/workbench-core/Cargo.toml` `[dependencies]`에 `git-core = { path = "../git-core" }`, `acp-agent-core = { path = "../acp-agent-core" }`, `anyhow = "1"`, `walkdir = "2"` 추가 후 `cargo check -p workbench-core` 통과(R11). `Cargo.lock` 갱신을 커밋 대상에 포함
- [X] T003 [P] 모듈 골격: `crates/workbench-core/src/domain/errors/mod.rs`, `src/application/{intent_first,dto}.rs`(빈 `//!` 문서 주석만), `src/application/reconcilers/mod.rs`, `src/application/handlers/{project,saved_prompt,goal,agent_run_settings,git,worktree,agent}/mod.rs`, `src/infrastructure/{git,fs}/mod.rs`, `src/infrastructure/json_collection_store.rs`; `crates/workbench-protocol/src/operations/{common,saved_prompt,goal,agent_run_settings,git,worktree,agent}.rs`(빈 모듈) — 각 `mod.rs`/`lib.rs`에 `pub mod` 등록 후 `cargo check --workspace` 통과. 기존 `handlers/{project_list,project_create,system_describe}.rs`는 이 단계에서 옮기지 않는다

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 네 스토리가 모두 의존하는 ledger v2, 일반화된 coordinator, generic JSON 저장소, intent-first runner, reconciler 골격, scope, 테스트 지원, compat generic을 만든다. **037 테스트는 이 단계 끝에서도 수정 없이 통과해야 한다.**

**⚠️ CRITICAL**: 이 단계가 끝나기 전에 스토리 작업을 시작하지 않는다.

### ledger schema v2 (R16)

- [X] T004 테스트 먼저: `crates/workbench-core/src/infrastructure/sqlite_ledger.rs` `#[cfg(test)]`에 (a) `v1_file_upgrades_to_v2_and_releases_terminal_reservations` — 현재 `DDL_V1` 텍스트로만 만든 임시 DB에 `applied` 상태·`reserved_resource_id = "/tmp/wt"` 행을 직접 INSERT → `SqliteOperationLedger::open` + `migrate()` → `schema_version` MAX = 2, 같은 `(aggregate, "/tmp/wt")`로 `begin` 성공; (b) `pending_reservation_still_conflicts` — `pending` 행이 있으면 같은 자원 `begin` → `DuplicateReservation`; (c) `applied_transition_releases_reservation` — begin→complete 뒤 같은 자원 begin 성공; (d) `unsupported_future_schema_is_rejected` — version 3 행 삽입 시 `UnsupportedSchema`. 실행해 실패 확인
- [X] T005 구현: 같은 파일에서 `SCHEMA_VERSION = 2`, `const MIGRATION_V2: &str = "DROP INDEX IF EXISTS operation_ledger_reserved; CREATE UNIQUE INDEX IF NOT EXISTS operation_ledger_reserved_pending ON operation_ledger (aggregate, reserved_resource_id) WHERE reserved_resource_id IS NOT NULL AND state = 'pending';"`, `migrate()`를 "DDL_V1 실행 → 현재 버전 조회 → `None | Some(1)`이면 `MIGRATION_V2` 실행 후 `INSERT schema_version (2, now)` → `Some(2)` 통과 → `Some(n>2)` `UnsupportedSchema { found, supported: 2 }`"로 바꾼다. `DDL_V1` 문자열은 바꾸지 않는다(승격 테스트의 원본). T004 통과, 기존 sqlite_ledger 테스트 통과

### StorageCoordinator 일반화 (R4)

- [X] T006 테스트 먼저: `crates/workbench-core/src/infrastructure/storage_coordinator.rs` `#[cfg(test)]`에 `aggregates_have_independent_locks_and_revisions`(두 aggregate 이름으로 `with_aggregate` 동시 실행이 서로 막지 않고 revision이 각각 증가), `with_aggregate_recovers_once_on_corrupt`(generic 복구 재시도), `git_worktrees_aggregate_is_canonical`(`git_worktrees_aggregate("/a/../b")` == `git_worktrees_aggregate("/b")` 형식 `git-worktrees:<canonical>`). 실패 확인
- [X] T007 구현: `StorageCoordinator`에 `revisions: Mutex<HashMap<String, u64>>`, `pub fn revision(&self, aggregate: &str) -> u64`, `set_revision(aggregate, v)`, `bump(aggregate)`, `pub fn with_aggregate<R, E>(&self, aggregate: &str, mut f: impl FnMut() -> Result<R, E>, is_corrupt: impl Fn(&E) -> bool, recover: impl Fn() -> Result<(), E>) -> Result<R, E>`(lock → f → corrupt면 test-hooks 지연 → recover → f 한 번 더), 상수 `PROJECTS_AGGREGATE`·`SAVED_PROMPTS_AGGREGATE = "saved-prompts"`·`GOALS_AGGREGATE = "goals"`·`AGENT_RUN_SETTINGS_AGGREGATE = "agent-run-settings"`, `pub fn git_worktrees_aggregate(repo_root: &Path) -> String`(canonicalize 실패 시 입력 그대로). 기존 `with_projects`는 `with_aggregate` 위에 다시 구현하고 시그니처 유지. `WorkbenchRuntime::bootstrap`·`project_create.rs`·`revision_retention.rs`의 `revision()`/`set_revision()` 호출을 aggregate 인자 형태로 바꾼다. `cargo test -p workbench-core` 통과(037 테스트 무수정)

### generic JSON 저장소 (R3)

- [X] T008 [P] 테스트 먼저: `crates/workbench-core/src/infrastructure/json_collection_store.rs` `#[cfg(test)]`에 `round_trips_vec`, `load_reports_primary_corrupt_without_writing`(손상 파일 mtime·내용 불변), `recover_from_backup_validates_as_typed_vec`(구조 무효 backup은 복구 거절), `save_writes_backup_of_previous`. 실패 확인
- [X] T009 [P] 구현: `pub struct JsonCollectionStore<T> { path: PathBuf, label: &'static str, _marker }` + `load(&self) -> Result<Vec<T>, StoreError>`, `save(&self, &[T])`, `recover_from_backup(&self)` — 기존 `json_store.rs` 함수를 그대로 위임. `crates/workbench-core/src/infrastructure/data_paths.rs`에 `saved_prompts_file()`(`saved-prompts.json`), `goals_file()`(`goals.json`), `agent_run_settings_file()`(`agent-run-settings.json`) 추가 + 유닛 테스트. T008 통과

### intent-first 공통 runner (R5)

- [X] T010 `crates/workbench-core/src/application/intent_first.rs`: `pub enum Reservation { None, ServerGenerated(Box<dyn FnMut() -> String + Send>), CallerProvided(String) }`, `pub struct MutationSpec<Out, E> { operation: OperationId, aggregate: String, normalized_input: serde_json::Value, reservation: Reservation, supports_expected_revision: bool, apply: Box<dyn FnOnce(&ApplyContext) -> Result<Applied<Out>, E> + Send>, fault: fn(&RequestId, E) -> WorkbenchFault }`, `pub enum Applied<Out> { Ok(Out) , Rejected(WorkbenchFault) }`, `pub struct ApplyContext { reserved_id: Option<String>, hooks, coordinator }`, `pub fn run_command<Out: Serialize, E>(ledger, coordinator, hooks, ctx: &CallContext, spec) -> Result<CallReply, WorkbenchFault>`. 절차: 키 필수(`MESSAGE_KEY_REQUIRED` 형식 "idempotencyKey is required for `<op>`.") → `fingerprint(op, normalized)` → `find`/`decide` → `begin`(`ServerGenerated`는 `DuplicateReservation` 시 재생성 3회, `CallerProvided`는 즉시 `conflict` outcome `unknown` retryable "Another change to this worktree path is still in progress.", `None`은 예약 없음) → `crash_if(AfterPending)` → `with_aggregate`(lock 안: `supports_expected_revision`이면 `ctx.expected_revision` 검사 → `preconditionFailed`; 아니면 `expected_revision.is_some()` → `invalidArgument` "expectedRevision is not supported for git operations.") → `apply` → `crash_if(AfterJsonSave|BeforeApplied)` → `should_fail(LedgerComplete)` → `complete` → `set_revision`(저장 단위만) → `Locked::{Applied,Rejected,SavedButUnconfirmed,Crashed}` 분기(037 동일). 유닛 테스트: `Reservation::CallerProvided` 충돌 응답, `ServerGenerated` 재시도 3회 뒤 `conflict`, expectedRevision 두 정책
- [X] T011 `crates/workbench-core/src/application/handlers/project_create.rs`를 `run_command` 위에 다시 쓴다(`Reservation::ServerGenerated(new_project_id)`, aggregate `PROJECTS_AGGREGATE`, `apply`가 `expectedRevision`을 검사하지 않도록 runner로 이동). **검증**: `cargo test -p workbench-core --test ledger_crash_points --test concurrency --test contract_suite --test revision_retention --test recovery_under_lock` 전부 **수정 없이** 통과. 통과 여부와 diff 줄 수를 Notes에 기록

### reconciler 골격 (R6)

- [X] T012 `crates/workbench-core/src/application/reconcilers/mod.rs`: `pub trait Reconciler: Send + Sync { fn resolve(&self, record: &LedgerRecord) -> Option<serde_json::Value>; }`, `pub struct ReconcilerRegistry(HashMap<OperationId, Arc<dyn Reconciler>>)`; `reconcilers/json_create.rs`: generic `JsonCreateReconciler<T, F: Fn(&T) -> &str>`(coordinator lock 안에서 컬렉션을 읽어 `reserved_resource_id`와 id가 같은 항목을 DTO로) — 기존 `bootstrap`의 project closure를 이것으로 대체; `reconcilers/json_delete.rs`: `JsonDeleteReconciler`(대상 id가 없으면 `Some(null)`). `handlers::build_registry`가 `(Registry, ReconcilerRegistry)`를 돌려주고 `bootstrap`이 `reconcile_pending(|record| reconcilers.get(record.key.operation)?.resolve(record))`로 dispatch. 등록되지 않은 operation의 pending은 `None`(unknown). 037 crash point 테스트 무수정 통과

### 계약 공통 (R1·R9)

- [X] T013 [P] `crates/workbench-protocol/src/principal.rs`: `Scope`에 `SavedPromptRead/Write`, `GoalRead/Write`, `AgentRunSettingsRead/Write`, `GitRead/Write`, `WorktreeRead`, `AgentRead`(serde/`as_str` = `savedPrompt:read` 등) 추가, `desktop()` = 13개 전부, `test_readonly()` = 모든 `:read` + `SystemDescribe`. 기존 테스트 갱신. `crates/workbench-protocol/src/operations/common.rs`: `EmptyOutput`(schema `{"type":"null"}` — 수동 `PartialSchema`), `nullable_schema(inner)` helper(`oneOf [inner, {type: null}]`). 유닛 테스트
- [X] T014 [P] `crates/workbench-core/src/application/dto.rs`: `pub(crate) fn assert_wire_parity<D: Serialize, W: Serialize>(domain: &D, dto: &W)`(test-only, `serde_json::to_value` 비교 + 실패 시 diff 출력)와 `pub(crate) fn assert_input_roundtrip<I: DeserializeOwned + Serialize>(json: Value)`. 기존 `handlers/mod.rs`의 `to_dto`(Project)를 여기로 옮기고 `project_wire_parity` 테스트 추가

### 테스트 지원 (R8)

- [X] T015 `crates/workbench-core/tests/support/fixtures.rs`: `Seed`에 `saved_prompts: Vec<Value>`, `goals: Vec<Value>`, `agent_run_settings: Vec<Value>`, `git_repo: Option<GitRepoSeed>`, `agents: Vec<Value>`, `provider_sessions: Vec<Value>` 추가(`#[serde(default)]`), `apply_seed`가 각 파일을 기록. `tests/support/git_repo.rs`(신규): `GitRepoSeed { commits: Vec<{message, files: BTreeMap<String,String>, branch: Option<String>}>, branches: Vec<String>, worktrees: Vec<{path, branch}>, working_changes: BTreeMap<String, Option<String>> }`, `build(seed, dir) -> BuiltRepo { root, name }` — `git init -q`, 고정 `GIT_AUTHOR_NAME/EMAIL`·`GIT_COMMITTER_*`, `GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE` = `2026-01-01T00:00:0N+00:00`(N = 커밋 순번), `user.name/email` 설정, `-c commit.gpgsign=false`. `Fixture::steps()`에서 request/expect 문자열의 `{{repo}}`·`{{repoName}}` 치환. 유닛 테스트: 같은 seed 두 번 → 같은 HEAD 해시
- [X] T016 `crates/workbench-core/tests/support/mod.rs`: `TestRuntime`에 `with_adapters(TestAdapters)` 생성자(US3에서 stub catalog·provider 세션 주입에 사용, 지금은 production 기본값), `git_repo: Option<BuiltRepo>` 보관. `http_harness`는 변경 없음(principal.rs 변경으로 readonly token이 새 read scope를 자동 획득). `cargo test -p workbench-core --test contract_suite` 통과(기존 16 fixture)

### compat generic (R10)

- [X] T017 `apps/agentic-workbench/src-tauri/src/inbound/workbench_compat.rs`: `pub async fn call_query<Out: DeserializeOwned>(runtime, operation: OperationId, input: Value) -> Result<Out, String>`, `pub async fn call_command<Out: DeserializeOwned>(runtime, operation, input) -> Result<Out, String>`(`IdempotencyKey::random()`), `Out = ()`는 `null` 허용. 기존 `call_list_projects`/`call_create_project`를 이 둘로 재구현. `apps/agentic-workbench/src-tauri/src/infrastructure/perf_log.rs`에 `pub async fn log_async_command<T>(name: &'static str, fut: impl Future<Output = T>) -> T`(`run_ms`만 기록). `cargo test -p agentic-workbench` 통과(compat 4 테스트 무수정)

**Checkpoint**: `cargo test --workspace --all-targets` 전부 통과(037 테스트 무수정), `cargo clippy --workspace --all-targets -- -D warnings` clean. 여기서 한 번 커밋(`refactor(workbench-core): generalize coordinator, extract intent-first runner, ledger schema v2`).

---

## Phase 3: User Story 1 - 프로젝트 수정·삭제, saved prompt, goal, agent 실행 설정이 그대로 동작하고 세 경로에서 같은 결과를 낸다 (Priority: P1) 🎯 MVP

**Goal**: 저장 단위 4개의 13개 command를 `Workbench` operation으로 옮기고 Tauri command를 compat로 바꾼다. 저장 파일 형식·문구 불변, 세 경로 동일, 변경은 intent-first.

**Independent Test**: AW 기존 테스트 무수정 통과 + fixture(성공·검증 실패·없음·멱등 재생·충돌·stale revision·readonly forbidden) 세 경로 일치 + crash point 판정 + 동시 20건.

### Tests for User Story 1 ⚠️ (구현 전 작성, 실패 확인)

- [X] T018 [P] [US1] fixture — project: `crates/workbench-protocol/fixtures/project-update-ok.json`, `project-update-name-required.json`, `project-update-not-found.json`, `project-update-stale-revision.json`, `project-update-idempotent-replay.json`, `project-update-missing-idempotency-key.json`, `project-delete-ok.json`(expectAfter projectsLen −1), `project-delete-not-found.json`, `project-delete-forbidden-readonly.json`
- [X] T019 [P] [US1] fixture — savedPrompt: `saved-prompt-list-empty.json`, `saved-prompt-list-seeded.json`(seed.savedPrompts 2), `saved-prompt-create-ok.json`(ignoreFields `output.id`), `saved-prompt-create-label-required.json`("Button label is required."), `saved-prompt-create-prompt-required.json`, `saved-prompt-create-idempotent-replay.json`, `saved-prompt-create-conflict-different-payload.json`, `saved-prompt-create-missing-idempotency-key.json`, `saved-prompt-update-ok.json`, `saved-prompt-update-not-found.json`, `saved-prompt-update-stale-revision.json`, `saved-prompt-delete-ok.json`, `saved-prompt-delete-not-found.json`, `saved-prompt-list-idempotency-key-rejected.json`(조회에 키 → invalidArgument)
- [X] T020 [P] [US1] fixture — goal: `goal-get-none.json`(output null), `goal-get-seeded.json`, `goal-get-working-directory-required.json`, `goal-create-ok.json`(ignoreFields createdAt/updatedAt), `goal-create-objective-required.json`, `goal-create-replaces-existing.json`(seed G1 → 새 objective → output 새 목표, expectAfter goals 1), `goal-create-idempotent-replay.json`, `goal-create-missing-idempotency-key.json`, `goal-update-ok.json`, `goal-update-token-budget-null-clears.json`(3상태), `goal-update-not-found.json`, `goal-update-stale-revision.json`, `goal-clear-ok.json`, `goal-clear-missing-is-ok.json`, `goal-record-progress-ok.json`(누적 확인 requests 2), `goal-record-progress-not-found.json`, `goal-create-forbidden-readonly.json`
- [X] T021 [P] [US1] fixture — agentRunSettings: `agent-run-settings-get-none.json`, `agent-run-settings-get-seeded.json`, `agent-run-settings-save-ok.json`(전체 객체, `serde(default)` 필드 생략 포함), `agent-run-settings-save-working-directory-required.json`, `agent-run-settings-save-no-builtin-profile.json`("At least one built-in agent profile must stay enabled."), `agent-run-settings-save-idempotent-replay.json`, `agent-run-settings-save-stale-revision.json`, `agent-run-settings-save-missing-idempotency-key.json`, `agent-run-settings-save-unknown-top-level-field.json`(최상위 여분 필드 → invalidArgument; 중첩 여분 필드는 허용되는 쌍 fixture `...-nested-unknown-field-ok.json`)
- [X] T022 [P] [US1] `crates/workbench-core/src/application/dto.rs` 테스트: `saved_prompt_wire_parity`, `goal_wire_parity`(tokenBudget None/Some, 각 status), `agent_run_settings_wire_parity`(빈 overrides/프로필 3개/ralph loop), `goal_update_input_tristate_roundtrip`(없음/`null`/값), `agent_run_settings_save_input_accepts_frontend_shape`(AW `AgentRunSettings` TS 필드 집합 그대로). DTO가 없어 컴파일 실패 → T029 뒤 통과
- [X] T023 [P] [US1] `crates/workbench-core/tests/ledger_crash_points.rs` 추가: `saved_prompt_create_{after_pending,after_json_save,before_applied}`(unknown/applied/applied, 항목 수), `goal_record_progress_after_json_save_is_unknown`(파일 반영됨·같은 키 재요청 conflict unknown), `saved_prompt_delete_after_json_save_is_applied`(대상 없음), `goal_create_over_existing_after_pending_is_unknown_and_keeps_old_goal`(G1 유지, 새 키 재요청 성공), `agent_run_settings_save_ledger_complete_failure_is_unknown_and_stays_pending`
- [X] T024 [P] [US1] `crates/workbench-core/tests/concurrency.rs` 추가: `twenty_saved_prompt_creates_distinct_keys`(20 항목·revision 20), `twenty_goal_progress_same_directory_serialize`(tokensUsed 합계 정확·revision 단조); `tests/revision_retention.rs`·`tests/recovery_under_lock.rs`를 aggregate 4개에 대해 매개화(`for aggregate in [...]`)

### Implementation for User Story 1

- [X] T025 [P] [US1] 도메인 이동: `apps/agentic-workbench/src-tauri/src/domain/{saved_prompt,goal,agent_run_settings}.rs` → `crates/workbench-core/src/domain/` (내용 그대로; `crate::domain::run::` → `acp_agent_core::domain::run::`). `crates/workbench-core/src/domain/errors/{saved_prompt_error,goal_error,agent_run_settings_error}.rs`: data-model §2 enum + `Display`(골든 문구) + `field_path()`. AW `domain/mod.rs`에서 세 모듈을 `pub use workbench_core::domain::{saved_prompt, goal, agent_run_settings};`로 재노출
- [X] T026 [P] [US1] 포트 이동: AW `domain/{saved_prompt_repository,goal_repository,agent_run_settings_repository}.rs` → `crates/workbench-core/src/ports/` (오류 타입을 enum으로, `recover_from_backup(&self)` 추가). AW `domain/mod.rs`에서 세 `*_repository` 모듈 선언 삭제
- [X] T027 [US1] 서비스 이동: AW `application/{saved_prompt_service,goal_service,agent_run_settings_service}.rs` → `crates/workbench-core/src/application/` (`Result<_, String>` → 도메인 오류 enum; `format!("{label} is required.")`류는 `Required(label)` variant로; 기존 유닛 테스트 함께 이동해 통과). `agent_run_settings_service`의 `MAX_RALPH_*`는 `acp_agent_core::domain::run::` 경로. (depends T025, T026)
- [X] T028 [US1] JSON 저장소: `crates/workbench-core/src/infrastructure/{json_saved_prompt_repository,json_goal_repository,json_agent_run_settings_repository}.rs`(`JsonCollectionStore<T>` 위, `new(&DataPaths)`). `StorageCoordinator::new`가 네 저장소를 받도록 확장(`Repositories { projects, saved_prompts, goals, agent_run_settings }`) + `with_saved_prompts`/`with_goals`/`with_agent_run_settings`(각각 `with_aggregate` 위, StoreCorrupt → recover). `WorkbenchRuntime::bootstrap`이 네 aggregate revision을 ledger에서 읽어 설정. (depends T007, T009, T026)
- [X] T029 [US1] 계약: `crates/workbench-protocol/src/operations/project.rs`에 `ProjectUpdateInput`·`ProjectDeleteInput`; `operations/saved_prompt.rs`(inputs 4 + `SavedPromptDto`); `operations/goal.rs`(inputs 5 — `GoalUpdateInput.token_budget: Option<Option<u64>>`에 `#[serde(default, deserialize_with = "double_option")]` + `#[schema(value_type = Option<u64>, nullable)]`, `GoalDto`, `GoalStatus`); `operations/agent_run_settings.rs`(inputs 2, `AgentRunSettingsDto` + 중첩 DTO 4 + `AgentRunSessionMode`·`PermissionMode`·`ContextSizePreset` 미러). `call.rs` `OperationId` +13 variant·`ALL` 16, `operations/mod.rs` `OPERATIONS` +13·`schema_for` +13(null 출력은 `EmptyOutput`, `goal.get`/`agentRunSettings.get`은 `nullable_schema`). `openapi.rs` 골든 테스트가 16 variant를 확인하도록 실행(생성물 갱신은 T034). (depends T013)
- [X] T030 [US1] `crates/workbench-core/src/application/dto.rs`: `From<&SavedPrompt> for SavedPromptDto`, `From<&ThreadGoal> for GoalDto`, `From<&AgentRunSettings> for AgentRunSettingsDto`(+ 역방향 `into_domain()` — save 입력용), `GoalUpdateInput → GoalUpdate` 등 input→draft 변환. T022 통과. (depends T025, T029)
- [X] T031 [US1] handler 13개: `crates/workbench-core/src/application/handlers/project/{update,delete}.rs`, `saved_prompt/{list,create,update,delete}.rs`, `goal/{get,create,update,clear,record_progress}.rs`, `agent_run_settings/{get,save}.rs`. 조회 3개는 `with_*` lock 안 읽기(project_list 형태). 변경 10개는 `MutationSpec`: aggregate 상수, `Reservation::ServerGenerated(new_saved_prompt_id)`(savedPrompt.create), 그 외 `Reservation::None`, `supports_expected_revision: true`. 오류 매핑 함수 `saved_prompt_fault`/`goal_fault`/`agent_run_settings_fault`(R7 표). reconciler: `savedPrompt.create` → `JsonCreateReconciler`, `project.delete`·`savedPrompt.delete`·`goal.clear` → `JsonDeleteReconciler`, 나머지 미등록(unknown). `handlers/mod.rs::build_registry`에 13개 등록. 기존 `handlers/{project_list,project_create,system_describe}.rs`를 `handlers/project/{list,create}.rs`·`handlers/system/describe.rs`로 이동. (depends T010, T012, T028, T030)
- [X] T032 [US1] compat: `apps/agentic-workbench/src-tauri/src/inbound/workbench_compat.rs`에 `project_update_request`/`project_delete_request`/`saved_prompt_*_request`/`goal_*_request`/`agent_run_settings_*_request` 변환 함수(`*Input` → `Value`, contracts 변환 규칙). `inbound/tauri_commands.rs`의 13개 command 본문을 `log_async_command(name, call_query/call_command(...))`로 교체(시그니처·반환 타입 불변, `update_project`/`delete_project`는 coordinator 직접 호출 제거). AW 삭제: `application/{saved_prompt,goal,agent_run_settings}_service.rs`, `infrastructure/json_{saved_prompt,goal,agent_run_settings}_repository.rs`, `domain/{saved_prompt,goal,agent_run_settings}.rs`, `domain/*_repository.rs` 3개; `application/mod.rs`·`infrastructure/mod.rs` 정리. `cargo check -p agentic-workbench` 통과. (depends T031)
- [X] T033 [US1] compat 유닛 테스트: `workbench_compat.rs` `#[cfg(test)]`에 fixture 기반 변환 대조(T018–T021의 request를 `*Input`으로 역직렬화 → 변환 → fixture request와 `operation`·`input` 동일), `null` 출력 → `()`, fault 3종 message만. `cargo test -p agentic-workbench` 전부 통과(기존 테스트 무수정). (depends T032)
- [X] T034 [US1] 생성물: `pnpm run generate:contracts` 실행 → `crates/workbench-protocol/openapi/workbench.openapi.json`·`packages/workbench-client/src/generated/workbench.ts` 갱신. `packages/workbench-client/src/operation-map.ts`에 `SavedPrompt`·`Goal`·`AgentRunSettings` alias, `index.ts` re-export, `operation-map.test-d.ts`에 16키 union·`goal.get` → `Goal | null`·`savedPrompt.create` 상관 테스트. `pnpm --filter @yoophi/workbench-client check-types test` 통과. `system-describe-desktop.json`/`readonly.json` 기대 개수 16/6으로 중간 갱신. (depends T029)
- [X] T035 [US1] `cargo test -p workbench-core --test contract_suite`로 T018–T021 fixture 전부 in-memory·HTTP 일치, T023·T024 통과. `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`. 커밋(`feat(workbench): migrate project update/delete, saved prompts, goals, agent run settings behind Workbench (038 US1)`)

**Checkpoint**: 앱이 이전과 똑같이 동작(quickstart §4 1–4), 프론트 diff 0, 세 경로 결과 동일. 여기까지가 MVP.

---

## Phase 4: User Story 2 - Git·worktree·파일 조회와 worktree 생성·삭제가 서버 경계에서 같은 안전 규칙으로 동작한다 (Priority: P2)

**Goal**: Git·worktree 14개 command를 옮긴다. 조회는 lock 없이, worktree 생성·삭제는 종료 상태 규칙(ADR `crates/workbench-core/docs/adr/0001`)과 예약 수명(R16)을 따른다. 안전 규칙(root 검사·512KB·UTF-8)은 값 그대로 core 경계로.

**Independent Test**: `gitRepo` seed fixture 세 경로 일치, `git_reconcile`·`reservation_lifecycle` 통과, 탈출·상한·UTF-8 fixture가 오늘과 같은 결과.

### Tests for User Story 2 ⚠️ (구현 전 작성, 실패 확인)

- [X] T036 [P] [US2] fixture — git: `git-list-remotes-seeded.json`(seed.gitRepo + `git remote add origin`은 seed `remotes` 필드로), `git-list-remotes-not-a-directory.json`(notFound), `git-list-remotes-working-directory-required.json`, `git-list-branches-seeded.json`, `git-list-worktrees-seeded.json`, `git-list-worktrees-include-status.json`, `git-create-worktree-ok.json`(expectAfter `gitWorktrees: 2` — `ExpectAfter`에 `git_worktrees: Option<usize>` 추가), `git-create-worktree-default-path-and-branch.json`(path `""`, branch 생략 → 성공, ignoreFields 없음/출력 null), `git-create-worktree-missing-idempotency-key.json`, `git-create-worktree-idempotent-replay.json`, `git-create-worktree-expected-revision-rejected.json`, `git-create-worktree-bad-reference-internal.json`(git stderr 그대로), `git-create-worktree-forbidden-readonly.json`, `git-delete-worktree-ok.json`, `git-delete-worktree-has-changes-precondition-failed.json`(seed.worktrees + workingChanges in worktree), `git-delete-worktree-path-required.json`
- [X] T037 [P] [US2] fixture — worktree: `worktree-list-changes-seeded.json`(workingChanges 2, ignoreFields diff 본문 제외 안 함 — 결정적), `worktree-get-changes-seeded.json`, `worktree-get-file-diff-seeded.json`, `worktree-list-files-all.json`, `worktree-list-files-markdown.json`, `worktree-list-files-dir-depth.json`, `worktree-list-files-outside-forbidden.json`(`scope.dir: "../outside"` → forbidden "File path must stay inside the worktree."), `worktree-list-files-not-a-directory.json`, `worktree-read-text-file-ok.json`, `worktree-read-text-file-truncated.json`(seed 파일 600KB → `truncated: true`, `size`), `worktree-read-text-file-non-utf8.json`(seed 바이트 — `files` 값에 `{"base64": ...}` 형태 허용을 git_repo seed에 추가), `worktree-read-text-file-outside-forbidden.json`, `worktree-read-text-file-path-required.json`, `worktree-read-text-file-directory-not-regular.json`, `worktree-list-history-seeded.json`(commits 3 → subject 순서, ignoreFields hash·authoredAt), `worktree-list-history-cursor.json`(maxCount 2 → cursor → 다음 페이지), `worktree-get-graph-seeded.json`, `worktree-get-commit-detail-seeded.json`(`{{headHash}}` 치환 추가 — seed 빌더가 HEAD·각 커밋 해시를 `{{commit:N}}`로 노출), `worktree-get-commit-file-diff-seeded.json`, `worktree-get-commit-detail-unknown-hash-internal.json`
- [X] T038 [P] [US2] `dto.rs` 테스트: `git_remote_wire_parity`, `git_branch_wire_parity`, `git_worktree_wire_parity`(status 각 variant, pruneReason None/Some), `worktree_change_wire_parity`(diff/content None·binary), `git_worktree_changes_wire_parity`(git-core 값), `git_worktree_file_diff_wire_parity`, `worktree_file_entry_wire_parity`, `worktree_text_file_wire_parity`, `worktree_file_list_scope_input_roundtrip`(kind 기본 all), `git_commit_history_wire_parity`, `git_commit_graph_wire_parity`(refs·layout hints), `git_commit_detail_wire_parity`, `git_file_diff_wire_parity`
- [X] T039 [P] [US2] `crates/workbench-core/tests/git_reconcile.rs`(신규): `create_pending_with_registered_path_is_applied`, `create_pending_without_path_is_unknown`, `delete_pending_with_absent_path_is_applied`, `delete_pending_with_present_path_is_unknown`, `partial_directory_without_registration_is_unknown`(디렉터리만 mkdir). `tests/reservation_lifecycle.rs`(신규): `create_delete_create_same_path_with_fresh_keys_succeeds`, `retry_after_failed_create_same_path_succeeds`(첫 시도 bad reference), `concurrent_same_path_creates_one_applied_one_conflict_unknown`. `tests/ledger_crash_points.rs` 추가: `git_create_worktree_after_pending_is_unknown`, `git_create_worktree_after_git_command_is_applied`(`CrashPoint::AfterJsonSave` 재사용 — 이름을 `AfterSideEffect` alias로 노출), `git_delete_worktree_after_git_command_is_applied`

### Implementation for User Story 2

- [X] T040 [P] [US2] 도메인 이동: AW `domain/{git_remote,git_branch,git_worktree,worktree_change,worktree_file}.rs` → `crates/workbench-core/src/domain/`. `domain/errors/{git_error,worktree_file_error}.rs`(data-model §2, `GitError::GitNotFound`·`CommandFailed(String)`·`WorktreeHasChanges`·`StatusUnresolved`·`Required`·`Clock`; `WorktreeFileError::OutsideWorktree`·`NotADirectory`·`NotRegularFile`·`NotFound`·`Required`·`Io`). AW `domain/mod.rs`에서 다섯 모듈 재노출, `git_worktree_changes.rs`·`worktree_git.rs` 재노출은 유지
- [X] T041 [P] [US2] 포트 이동: AW `domain/{git_remote_provider,git_branch_provider,git_worktree_provider,worktree_change_provider,worktree_file_provider,worktree_git_provider}.rs` → `crates/workbench-core/src/ports/`(오류 `GitError`/`WorktreeFileError`). AW `domain/mod.rs`에서 선언 삭제
- [X] T042 [US2] 서비스 이동: AW `application/{git_remote_service,git_branch_service,git_worktree_service,worktree_changes_service,git_worktree_changes_service,worktree_file_service,worktree_git_service}.rs` → `crates/workbench-core/src/application/`(오류 enum, `default_worktree_path`·`new_worktree_name`·`normalize_*` 유지, 기존 유닛 테스트 이동). `git_worktree_service::create_git_worktree`가 **해석된 경로를 돌려주도록** `Result<ResolvedWorktreeDraft, GitError>` 형태의 `resolve_create_draft` 함수를 분리(예약과 실행에 같은 경로 사용). (depends T040, T041)
- [X] T043 [US2] 어댑터 이동: AW `infrastructure/git_cli_{remote,branch,worktree,worktree_change,worktree_git}_provider.rs` → `crates/workbench-core/src/infrastructure/git/cli_{remote,branch,worktree,worktree_change,worktree_git}_provider.rs`. `Command::output()`의 `io::Error`(`ErrorKind::NotFound`) → `GitError::GitNotFound`, `!status.success()` → `GitError::CommandFailed(git_error_message(stderr, fallback))`(stderr 해석 없음, R7). `git_cli_worktree_provider`의 삭제 전 검사 문구 두 개는 `WorktreeHasChanges`/`StatusUnresolved`. AW `infrastructure/fs_worktree_file_provider.rs` → `crates/workbench-core/src/infrastructure/fs/worktree_file_provider.rs`(`canonicalize`·`starts_with` root 검사·`MAX_PREVIEW_BYTES` 그대로, 오류 enum 매핑). 기존 유닛 테스트(탈출 거절 등) 함께 이동해 통과. (depends T041)
- [X] T044 [US2] 계약: `crates/workbench-protocol/src/operations/git.rs`(inputs 5 + `GitRemoteDto`·`GitBranchDto`·`GitWorktreeDto`·`GitWorktreeStatus`), `operations/worktree.rs`(inputs 9 + `WorktreeChangeDto`·`WorktreeChangeType`·`GitWorktreeChangesDto`·`GitChangedFileDto`·`GitChangedFileGroup`·`GitWorktreeFileDiffDto`·`WorktreeFileEntryDto`·`WorktreeTextFileDto`·`WorktreeFileListScopeDto`·`WorktreeFileListKind`·`GitCommitHistoryDto`·`GitCommitPageDto`·`GitCommitSummaryDto`·`GitCommitGraphDto`·`GitGraphCommitDto`·`GitGraphLayoutHintsDto`·`GitGraphRefDto`·`GitGraphRefKind`·`GitCommitDetailDto`·`GitCommitFileChangeDto`·`GitFileDiffDto` — git-core `domain.rs`의 serde 속성 그대로). `OperationId` +14(`ALL` 30), `OPERATIONS` +14(`git.createWorktree`/`deleteWorktree`는 Command·Modify·idempotent·`GitWrite`), `schema_for` +14. (depends T013)
- [X] T045 [US2] `dto.rs`: 위 DTO ↔ 도메인/git-core 타입 `From` 변환 21개. T038 통과. (depends T040, T044)
- [X] T046 [US2] handler 14개: `handlers/git/{list_remotes,list_branches,list_worktrees,create_worktree,delete_worktree}.rs`, `handlers/worktree/{list_changes,get_changes,get_file_diff,list_files,read_text_file,list_history,get_graph,get_commit_detail,get_commit_file_diff}.rs`. 조회 12개: `decode_input` → `spawn_blocking`(lock 없음) → 서비스 → DTO, 사전 검증 오류 매핑(`git_fault`/`worktree_file_fault`, R7). `git.createWorktree`: `resolve_create_draft` → `MutationSpec { aggregate: git_worktrees_aggregate(canonical repo), reservation: Reservation::CallerProvided(resolved.path), supports_expected_revision: false, apply: provider.create_worktree(...) → Applied::Ok(()) }`; `git.deleteWorktree`: `Reservation::None`, apply에서 `WorktreeHasChanges`/`StatusUnresolved` → `Applied::Rejected(preconditionFailed)`. `reconcilers/git_worktree.rs`: `GitWorktreeCreateReconciler`(aggregate 문자열에서 repo 경로 복원 → `list_worktrees` → 경로 존재 → `Some(null)`), `GitWorktreeDeleteReconciler`(경로 부재 → `Some(null)`; delete는 예약이 없으므로 대상 경로를 `result_json`이 아닌 **정규화 입력에서** 알아야 함 → `NewLedgerEntry`에 컬럼을 늘리지 않고 delete도 `Reservation::CallerProvided(path)`를 잡는다 — pending 동안 같은 경로 생성과 상호 배제되는 부수 효과는 바람직함; data-model §4 표의 `git.deleteWorktree` 예약 항목을 이에 맞게 갱신). `build_registry` 등록. (depends T010, T012, T042, T043, T045)
- [X] T047 [US2] compat: `workbench_compat.rs`에 git/worktree 14개 요청 변환(`include_status: None` → 필드 생략, `scope: None` → 생략, `max_count/offset/cursor` 옵션 생략 규칙). `tauri_commands.rs` 14개 command 교체(`run_blocking_command` 제거). AW 삭제: `infrastructure/git_cli_*.rs` 5개, `infrastructure/fs_worktree_file_provider.rs`, `application/{git_*,worktree_*}_service.rs` 7개, `domain/{git_remote,git_branch,git_worktree,worktree_change,worktree_file}.rs`, `domain/*_provider.rs` 6개; `mod.rs` 정리. compat fixture 대조 테스트 추가. `cargo test -p agentic-workbench` 통과. (depends T046)
- [X] T048 [US2] 생성물: `pnpm run generate:contracts`; `operation-map.ts` alias(`GitRemote`·`GitBranch`·`GitWorktree`·`WorktreeChange`·`GitWorktreeChanges`·`WorktreeFileEntry`·`WorktreeTextFile`·`GitCommitHistory`·`GitCommitGraph`·`GitCommitDetail`·`GitFileDiff`), `index.ts`, test-d(30키·`worktree.listFiles` scope optional·`git.createWorktree` output null). `system-describe-*` 기대 30/18. `pnpm --filter @yoophi/workbench-client check-types test`. (depends T044)
- [X] T049 [US2] `cargo test -p workbench-core`(contract_suite에 T036·T037 fixture, git_reconcile, reservation_lifecycle, crash points), clippy·fmt clean. 커밋(`feat(workbench): migrate git, worktree and file operations behind Workbench (038 US2)`)

**Checkpoint**: quickstart §4 5–7·11 수동 확인. `git diff --stat origin/main -- crates/git-core` = 0.

> **(US2, 2026-09-27) 구현 중 결정** — 동작 보존(FR-002)을 계약 초안보다 우선했다. T065에서 contracts·data-model에 반영한다.
> - `git.listWorktrees`의 `includeStatus` 생략 시 서버 기본값은 **true**다(오늘 command의 `unwrap_or(true)`, 프론트가 생략해 호출). 계약 §2의 "기본 false"는 오기.
> - `git.listRemotes`·`listBranches`·`listWorktrees`는 저장소가 아닌 디렉터리에서 오늘처럼 **빈 목록**을 돌려준다(git 비정상 종료 → `Ok(vec![])`). fixture는 `git-list-remotes-not-a-repository-empty`. 사전 검증 `notFound`는 `worktree.listFiles`·`readTextFile`에만 있다.
> - `GitError`: `GitNotFound(String)`(오늘 문구 보존), `Io`·`WorktreeNotFound`("Git worktree not found." → `notFound`)·`Unresolvable`(기본 경로 계산 실패 → `invalidArgument`) 추가. git-core reader(이력·그래프·상세·status/diff)는 `Result<_, String>`이라 실패가 전부 `CommandFailed`(`internal`)다 — git-core를 바꾸지 않는다(Q6).
> - `WorktreeFileError::NotUtf8`("Only UTF-8 text files can be previewed.") → `invalidArgument`(`/path`).
> - `git.deleteWorktree`도 대상 경로를 `CallerProvided`로 예약한다(pending 동안 같은 경로 생성·삭제와 배타, 재시작 판정 증거). data-model §4 표 갱신 필요.
> - Git 변경은 저장 단위가 아니므로 **재생 응답에서도 `revision`을 싣지 않는다**(`intent_first`가 `tracks_revision=false`면 제거). fixture `absent: ["revision"]`로 고정.
> - 생성 지문은 기본값 채우기 **전** 정규화 입력(`CreateWorktreeRequest`)으로 계산한다 — 기본 branch 이름이 시각에서 만들어지므로 같은 키 재요청이 충돌하지 않게.
> - 파일 배치: 서비스 5개(`git_service`·`git_worktree_service`·`worktree_changes_service`·`worktree_git_service`·`worktree_file_service`), 포트 2개(`ports/git_providers.rs`·`worktree_file_provider.rs`), handler는 `handlers/{git,worktree}/mod.rs` + 공통 `query_handler`. `WORKSPACE_EXCLUDED_DIRS`는 core `infrastructure::fs`로 옮기고 AW watcher는 재노출로 같은 값을 쓴다. `perf kind=git`도 core `infrastructure::perf`.
> - crash 시나리오는 037 `ledger_crash_points.rs`를 수정하지 않고 `tests/git_reconcile.rs`에 두었다(`CrashPoint::AFTER_SIDE_EFFECT` 별칭). 같은 경로 동시 생성은 타이밍에 기대지 않도록 pending 기록을 직접 만든 결정적 테스트로 확인했다.
> - `system-describe-readonly` 기대값은 **17**(query 5 + git 3 + worktree 9). T048의 "18"은 오기이며 US3 뒤 19가 된다.
> - test support: `WORKBENCH_FIXTURE_FILTER`(개발용), 두 경로 비교 전 저장소 경로 정규화(`normalize_paths`), worktree seed `files`, git 날짜는 `Z` 표기.


---

## Phase 5: User Story 3 - agent catalog와 provider 세션 조회가 같은 인터페이스에서 제공된다 (Priority: P3)

**Goal**: `agent.list`·`agent.listProviderSessions`. 실행 환경(env catalog, provider 로컬 파일)을 읽는 조회를 port 뒤에 두어 테스트에서 stub으로 바꿀 수 있게 한다.

**Independent Test**: stub catalog·stub provider 세션 seed fixture 세 경로 일치; fs 어댑터는 이동한 기존 유닛 테스트로.

### Tests for User Story 3 ⚠️ (구현 전 작성, 실패 확인)

- [ ] T050 [P] [US3] fixture: `agent-list-seeded.json`(seed.agents 2 → 같은 순서), `agent-list-empty.json`, `agent-list-idempotency-key-rejected.json`, `agent-list-provider-sessions-filtered.json`(seed.providerSessions 3: cwd A 2·B 1 → `cwd: A` → 2), `agent-list-provider-sessions-all.json`(cwd 생략 → 3, 최신순), `agent-list-provider-sessions-unsupported-agent-empty.json`(`opencode` → []), `agent-list-provider-sessions-agent-id-required.json`, `agent-list-provider-sessions-limit-50.json`(seed 60 → 50)
- [ ] T051 [P] [US3] `dto.rs` 테스트: `agent_descriptor_wire_parity`(acp-agent-core 값, models/efforts/contextSizes 비어 있음·채움, runtimeVersion None), `provider_session_wire_parity`(Option 전부 None·전부 Some)

### Implementation for User Story 3

- [ ] T052 [P] [US3] 도메인·포트 이동: AW `domain/provider_session.rs` → `crates/workbench-core/src/domain/provider_session.rs`; AW `ports/provider_session_repository.rs` → `crates/workbench-core/src/ports/provider_session_repository.rs`(`anyhow::Result` → `Result<Vec<ProviderSession>, ProviderSessionError>`); `domain/errors/provider_session_error.rs`. 신규 `crates/workbench-core/src/ports/agent_catalog_reader.rs`: `pub trait AgentCatalogReader: Send + Sync { fn list_agents(&self) -> Vec<AgentDescriptor>; }` + `impl<T: acp_agent_core::ports::agent_catalog::AgentCatalog> AgentCatalogReader for T`(object-safe 포장; `AgentCatalog`는 `Clone` bound라 `dyn` 불가). AW `domain/mod.rs`·`ports/mod.rs` 재노출/삭제
- [ ] T053 [US3] 유즈케이스·어댑터 이동: AW `application/list_provider_sessions.rs` → core(오류 enum); AW `infrastructure/fs_provider_session_repository.rs` → `crates/workbench-core/src/infrastructure/fs/provider_session_repository.rs`(내부 `anyhow` 유지, port 경계에서 `ProviderSessionError::Storage(err.to_string())`; 손상 항목 건너뜀 유지; 기존 유닛 테스트 이동). (depends T052)
- [ ] T054 [US3] 계약: `crates/workbench-protocol/src/operations/agent.rs`(`AgentListInput {}`, `AgentListProviderSessionsInput { agent_id, cwd: Option<String> }`, `AgentDescriptorDto`·`AgentOptionDescriptorDto`(acp-agent-core `agent.rs` serde 속성 그대로), `ProviderSessionDto`). `OperationId` +2(`ALL` 32), `OPERATIONS` +2(`AgentRead`), `schema_for` +2. `dto.rs` 변환 + T051 통과. (depends T013, T052)
- [ ] T055 [US3] 주입점: `crates/workbench-core/src/application/workbench_runtime.rs`에 `pub struct RuntimeAdapters { agent_catalog: Arc<dyn AgentCatalogReader>, provider_sessions: Arc<dyn ProviderSessionRepository> }` + `RuntimeAdapters::production()`(`ConfigurableAgentCatalog::from_env()`, `FsProviderSessionRepository::new()`), `WorkbenchRuntime::bootstrap(paths)` = `bootstrap_with(paths, RuntimeAdapters::production())`. `tests/support/mod.rs`의 `TestRuntime::with_adapters`가 `seed.agents`/`seed.providerSessions`로 만든 stub(`StubCatalog`, `StubProviderSessions` — `SessionScope::Path` 필터·최신순·상한 50은 **유즈케이스**가 처리하므로 stub은 전체 목록만 돌려줌)을 주입. handler 2개 `handlers/agent/{list,list_provider_sessions}.rs`(`spawn_blocking`, lock 없음, `agent_fault`). `build_registry` 등록. (depends T016, T053, T054)
- [ ] T056 [US3] compat: `list_agents`(`Vec<AgentDescriptor>` 반환 유지 — `call_query` 실패 시 `Err(message)`가 아니라 시그니처가 `Vec`이므로 빈 목록 + `eprintln!`이 아닌 **시그니처를 `Result<Vec<AgentDescriptor>, String>`로 바꾸지 않고** 실패 시 빈 목록을 돌려주되 perf 로그에 오류를 남긴다; contracts 표의 주석을 이에 맞게 정정), `list_provider_sessions` 교체. AW 삭제: `application/list_provider_sessions.rs`, `infrastructure/fs_provider_session_repository.rs`, `domain/provider_session.rs`, `ports/provider_session_repository.rs`; AW `Cargo.toml`에서 `walkdir` 제거(`anyhow`는 mcp·acp session store가 사용하므로 유지). compat fixture 대조 테스트. `cargo test -p agentic-workbench` 통과. (depends T055)
- [ ] T057 [US3] 생성물: `pnpm run generate:contracts`; `operation-map.ts`(`AgentDescriptor`·`ProviderSession` alias), `index.ts`, test-d(32키 정확성 — `OperationId` union 전체 나열, `agent.list` → `AgentDescriptor[]`). `system-describe-desktop.json` 32·`system-describe-readonly.json` 19(변경 13개 부재). `pnpm --filter @yoophi/workbench-client check-types test`, `cargo test -p workbench-core --test contract_suite`. 커밋(`feat(workbench): migrate agent catalog and provider session queries behind Workbench (038 US3)`)

**Checkpoint**: quickstart §4 8 수동 확인.

---

## Phase 6: User Story 4 - 다음 단계 담당자가 71개 command 전부의 이관 상태와 이유를 한 곳에서 확인한다 (Priority: P4)

**Goal**: 인벤토리 71 문서화, 정본 각주, 옮긴 도메인이 AW에 남지 않았음과 계약 조회 32/19를 확정.

**Independent Test**: quickstart §7·§8의 grep이 전부 기대값, 문서 표 71행.

- [ ] T058 [P] [US4] `docs/workbench-seam.md`: 상태 줄을 "037·038 구현 완료"로, 범위 절에 038 도메인·operation 29 추가, 비범위를 "이벤트·창 정체 32(2단계)·표현 상태 8(유지)"로, Mermaid에 `intent_first`·`reconcilers`·`JsonCollectionStore`·`infrastructure/git|fs` 추가, "ledger schema v2 — 예약 수명" 소절(R16 표), "재시작 판정 규칙" 표(data-model §4), **"command 인벤토리 (71)" 절**(data-model §6 표를 command 단위 71행으로 펼쳐 기록; 열: command·분류·operation/이유), "038 이후 이관 절차" → "적용된 이관 절차(038)"로 실제 순서 갱신 + "039(2단계) 이관 절차" 안내, ADR 3건 링크
- [ ] T059 [P] [US4] `docs/client-server-architecture-research.md` 진행 상태 각주에 "038(1b) 완료 — 29 command 이관, 32 이연(ADR 0001), 8 유지; ledger schema v2" 추가. `CONTEXT-MAP.md`·`crates/workbench-core/CONTEXT.md`를 구현된 이름과 대조해 어긋난 용어가 있으면 갱신(예: `Reservation` 용어가 생겼으면 "예약" 항목 추가 여부 판단 — 구현 세부면 넣지 않음)
- [ ] T060 [US4] 완료 판정 grep(quickstart §8): 프론트 diff 0, AW `domain/`·`infrastructure/`에 옮긴 파일 0, `tauri_commands.rs`에 `from_app`·`GitCli*`·`Fs*Provider` 0, `git diff --stat origin/main -- crates/git-core crates/acp-agent-core` 0, `packages/workbench-client/src/index.ts` export 목록이 data-model §8과 일치. 결과를 Notes에 기록. AW `infrastructure/json_store.rs`에 "038 뒤 남은 사용처: appearance·orchestration·session window state·layout·acp session(2단계·표현 상태 정리에서 처리)" 주석 갱신

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T061 [P] `crates/workbench-core/tests/list_latency.rs`에 `saved_prompt_list`·`goal_record_progress`·`worktree_list_files`(작은 seed repo) 100회 p95 측정 추가(#[ignore]); `cargo test -p workbench-core --test list_latency -- --ignored --nocapture` 결과를 Notes에 기록(SC-001)
- [ ] T062 [P] drift 검출 실증: `crates/workbench-protocol/src/operations/goal.rs` 필드명 하나 변경 → `pnpm run generate:contracts && git diff --exit-code -- crates/workbench-protocol/openapi packages/workbench-client/src/generated` 실패 확인 → 복원 → 통과. 결과 Notes
- [ ] T063 전체 게이트: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo clippy -p workbench-core --lib -- -D warnings`, `cargo test --workspace --all-targets`(통과 수 Notes), `pnpm run check-types`, `pnpm run test`, `pnpm run generate:contracts && git status --short`(변경 없음)
- [ ] T064 앱 스모크(quickstart §4): `pnpm --filter agentic-workbench tauri dev` 기동 → 기존 v1 `ledger.sqlite`가 `schema_version` 2로 승격됐는지, `saved-prompts.json`·`goals.json`·`agent-run-settings.json` 해시가 기동만으로 바뀌지 않는지, 수동 항목 1–11 중 자동화되지 않은 것을 표로 Notes에 기록(수행 여부 정직하게). 037처럼 UI 조작 항목은 리뷰어 수동 항목으로 남길 수 있음
- [ ] T065 SC 증거 매핑을 Notes에 작성(SC-001~SC-008 각각 테스트/명령/문서), spec 대비 어긋난 점 기록(예: `git.deleteWorktree`도 예약을 잡기로 한 T046 결정 → data-model §4 갱신), 커밋 메시지·PR 본문 초안 작성(push·PR은 사용자 지시 후). 커밋(`docs(aw): record 038 inventory and migration status`)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: 의존 없음. T002·T003 병렬.
- **Phase 2 (Foundational)**: T001–T003 뒤. 내부 순서: T004→T005, T006→T007, T008→T009(병렬 가능), T010→T011(T007 뒤), T012(T011 뒤), T013·T014 병렬, T015→T016, T017 독립. **Phase 2 끝에 037 테스트 무수정 통과가 게이트.**
- **Phase 3 (US1)**: Phase 2 뒤. MVP.
- **Phase 4 (US2)**: Phase 2 뒤. US1과 독립(다른 도메인·파일)이지만 `call.rs`/`operations/mod.rs`/`handlers/mod.rs`/`workbench_compat.rs`/`tauri_commands.rs`/`operation-map.ts`는 공유 파일이므로 순차 편집.
- **Phase 5 (US3)**: Phase 2 뒤 + T016(주입점). US1·US2와 독립.
- **Phase 6 (US4)**: US1–US3 완료 뒤(인벤토리는 최종 상태 기준).
- **Phase 7 (Polish)**: 전부 뒤.

### User Story Dependencies

- US1(P1): Foundational만. 독립 검증 가능.
- US2(P2): Foundational만. US1 없이도 완결(git·worktree fixture만으로 검증).
- US3(P3): Foundational + T016. 독립.
- US4(P4): US1–US3.

### Within Each User Story

- fixture·wire parity·crash/reconcile 테스트 먼저 → 도메인·포트 이동(병렬) → 서비스 → 어댑터 → 계약(protocol) → dto → handler+reconciler → compat+AW 삭제 → 생성물 → 통합 확인·커밋.

### Parallel Opportunities

- Phase 1: T002 ∥ T003
- Phase 2: (T004→T005) ∥ (T006→T007) ∥ (T008→T009) ∥ T013 ∥ T014 ∥ T017
- US1: T018 ∥ T019 ∥ T020 ∥ T021 ∥ T022 ∥ T023 ∥ T024; T025 ∥ T026
- US2: T036 ∥ T037 ∥ T038 ∥ T039; T040 ∥ T041
- US3: T050 ∥ T051
- US4/Polish: T058 ∥ T059, T061 ∥ T062

---

## Parallel Example: Foundational

```text
# 동시에 시작 (다른 파일):
T004 "sqlite_ledger.rs v2 테스트"       T006 "storage_coordinator.rs 일반화 테스트"
T008 "json_collection_store.rs 테스트"  T013 "principal.rs Scope +10, common.rs"
T014 "dto.rs 골격"                      T017 "workbench_compat.rs generic"
# 그 뒤 순차: T005 → T007 → T009 → T010 → T011(037 테스트 무수정 게이트) → T012 → T015 → T016
```

## Parallel Example: User Story 2

```text
# 테스트 먼저 (병렬):
T036 "git-* fixture"  T037 "worktree-* fixture"  T038 "git/worktree wire parity"  T039 "git_reconcile · reservation_lifecycle · crash points"
# 이동 (병렬): T040 "domain 5 + errors"  T041 "ports 6"
# 순차: T042 services → T043 adapters → T044 protocol → T045 dto → T046 handlers+reconcilers → T047 compat/AW 삭제 → T048 생성물 → T049 게이트·커밋
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1–2 완료 — **T011의 037 테스트 무수정 통과가 첫 게이트**. 실패하면 runner를 고치고 진행.
2. Phase 3 완료 — 저장 단위 4 도메인이 Workbench 뒤에 있고 앱은 이전과 같다.
3. **STOP and VALIDATE**: quickstart §4 1–4, 프론트 diff 0, 세 경로 일치.

### Incremental Delivery

1. US2 → Git·worktree 14 + 종료 상태 규칙·예약 수명 시연(`git_reconcile`, `reservation_lifecycle`)
2. US3 → agent 2 + 주입점, 32 operation 완성, describe 32/19
3. US4 → 인벤토리 71·문서
4. Polish → 게이트·스모크·PR 초안

### Parallel Team Strategy

한 사람이면 위 순서. 둘이면 Foundational 뒤 A: US1→US4, B: US2→US3. 공유 파일(`call.rs`·`operations/mod.rs`·`handlers/mod.rs`·`workbench_compat.rs`·`tauri_commands.rs`·`operation-map.ts`·`system-describe-*.json`)은 스토리 단위로 순차 병합한다.

---

## Notes

- [P] = 다른 파일, 미완료 의존 없음
- 커밋은 논리 단위마다(Phase 2 끝, 각 US 끝, 문서). push·PR은 사용자 지시 후.
- 각 checkpoint에서 멈춰 스토리를 독립 검증한다.
- 기준선·실측 기록 (T001, T011, T060, T061, T062, T063, T064, T065):
  - (T001, 2026-09-26) 변경 전 기준선: `cargo test --workspace --all-targets` **467 passed / 0 failed**(24 test binaries), AW lib 227, `pnpm run check-types` 통과, 프론트 diff 0. AW 백엔드 파일 수: domain 30 · application 24 · infrastructure 29 · ports 9.
  - (T004/T005) v2 승격 테스트가 실제 결함을 잡음: 첫 구현은 `migrate()`가 매번 `DDL_V1`(v1 예약 index `IF NOT EXISTS`)을 실행해, 승격 뒤 두 번째 기동에서 applied+pending이 같은 자원을 갖는 순간 index 재생성이 constraint 위반으로 실패했다. 버전 무관 DDL(`DDL_BASE`)과 v1 전용 DDL(`#[cfg(test)] DDL_V1`, 승격 재현용)을 분리해 해결. `v1_file_upgrades_to_v2_and_releases_terminal_reservations`가 두 번째 migrate와 그 뒤 begin까지 확인한다.
  - (T011) `project_create.rs` 250줄 → 90줄(`MutationSpec`만 조립). 037 테스트(`ledger_crash_points` 4·`concurrency` 3·`contract_suite` 2·`revision_retention` 1·`recovery_under_lock` 3) **수정 없이 통과**. 유일한 support 변경: `tests/list_latency.rs`의 `Seed` 리터럴에 `..Default::default()`(seed 필드 추가로 인한 컴파일 수정, 행위 무관). `SavedButUnconfirmed` 문구는 "프로젝트는 저장되었지만…" → "변경은 저장되었지만…"으로 일반화(테스트는 코드·outcome만 검증).
  - (T016) `TestRuntime::with_adapters`는 주입점(`RuntimeAdapters`)이 생기는 T055에서 함께 만든다. Phase 2에서는 `apply_seed → SeedContext`와 `steps_with(ctx)` 치환, `git_repo` 빌더까지.
  - (US1, 2026-09-27) 구현 중 결정: handler는 op별 파일 대신 **도메인별 모듈**(`handlers/{project,saved_prompt,goal,agent_run_settings,system}/`, project만 op별 파일)로 두었다. crash point·동시성·revision·복구 테스트는 037 파일을 손대지 않고 `tests/us1_crash_points.rs`(7)·`tests/us1_stores.rs`(4, 저장 단위 4개 매개화)에 추가. `goal.clear`는 대상 없음이 오늘도 `Goal not found.`(contracts §2 "성공" 표기는 오기 → T065에서 정정). `goal.update`의 `tokenBudget`은 AW `Option<Option<_>>` serde 규칙 그대로(`null` = 변경 없음; 3상태는 오늘 wire로 표현 불가 — spec 표기 정정 대상). 삭제 3개(`project.delete`·`savedPrompt.delete`·`goal.clear`)는 대상 id를 `Reservation::CallerProvided`로 잡아 reconciler가 종료 상태를 판정. 조회에 `idempotencyKey`가 오면 runtime이 `invalidArgument`로 거절(contracts 규칙 1, `MESSAGE_KEY_ON_QUERY`). `start_agent_run`(2단계 대상)의 설정 읽기는 `coordinator.with_agent_run_settings`로 옮겨 lock 안에서 읽는다. 037 유닛 테스트 중 `authorization::readonly_is_forbidden…`은 표가 커져 "Query 수와 같다"로 일반화(행위 동일). compat 변환 테스트 `us1_inputs_match_fixture_shapes`는 fixture 29종 이상을 AW `*Input`으로 왕복.
  - (Phase 2 게이트, 2026-09-27) `cargo test -p workbench-protocol -p workbench-core -p agentic-workbench` 343 passed / 0 failed(protocol 골든은 계약 재생성 뒤 통과). AW compat generic 6개·perf_log 2개는 US1까지 dead_code 경고(의도).
