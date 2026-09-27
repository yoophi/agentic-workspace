# Research: 나머지 도메인의 Workbench 이관 (1b)

**Date**: 2026-09-26 | **Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)

037의 research R1~R14는 그대로 유효하다. 여기서는 038이 새로 만나는 결정만 다룬다. grill(Q1~Q8)에서 사용자가 확정한 것은 결정 근거로 인용하고 다시 열지 않는다.

## R1. wire DTO 전략 — protocol에 미러 타입을 두고 core가 변환한다

**Decision**: `workbench-protocol/operations/*`에 29개 operation의 input/output 타입을 **독립 DTO**로 정의하고 `utoipa::ToSchema`를 derive한다. core의 handler가 도메인 타입(AW에서 이동한 것, `git-core`·`acp-agent-core`의 것)을 DTO로 변환한다(`From`/`to_dto`). protocol은 `git-core`·`acp-agent-core`에 의존하지 않는다.

**Rationale**: 037이 `Project`→`ProjectDto`로 세운 규칙이며, protocol crate의 존재 이유가 "wire를 내부 모델에서 분리"다. 대안인 "공유 crate 타입에 `ToSchema` derive를 feature로 추가"는 `git-core`(AW·git-explorer 공용)와 `acp-agent-core`를 건드려 grill Q6의 "git-core 불변"과 헌장 V(소비 앱 재검증)를 어긴다. `schema = Value`(불투명)는 FR-010(32개 전부 상관 타입)을 어긴다.

**비용과 완화**: 미러 대상은 약 45개 타입(AW 도메인 28, git-core 15, acp-agent-core `AgentDescriptor`·`AgentOptionDescriptor`·`PermissionMode`·`ContextSizePreset`). 전부 필드만 있는 데이터 타입이다. **wire 동일성 테스트**로 회귀를 막는다: 도메인 타입마다 대표 값(Option None/Some, default, 빈 컬렉션 포함)을 만들어 `serde_json::to_value(domain) == serde_json::to_value(Dto::from(domain))`를 확인하고, input DTO는 오늘 프론트가 보내는 JSON 형태(AW 인바운드 `*Input` 구조체와 동일 필드)로 역직렬화되는지 확인한다. DTO의 serde 속성(`rename_all = "camelCase"`, `skip_serializing_if`, `default`)은 원본과 같게 옮긴다.

**Alternatives considered**: (a) 도메인 타입을 protocol로 이동해 단일 타입으로 — git-core 타입은 이동 불가라 반쪽만 되고, 헌장 III의 domain/wire 분리를 깬다. (b) 매크로로 미러 생성 — 45개에 매크로를 도입할 이득보다 가독성 손실이 크다.

## R2. input 검증 엄격성 — 최상위는 `deny_unknown_fields`, 중첩 도메인 DTO는 관용

**Decision**: 각 operation의 최상위 input 구조체(`GoalCreateInput` 등)는 037 규칙대로 `#[serde(deny_unknown_fields)]`. 도메인 객체를 통째로 받는 입력(`agentRunSettings.save`의 `settings`, `git.createWorktree`의 필드들)에서 **중첩 DTO는 오늘의 도메인 struct와 같은 관용 규칙**(unknown 무시, `#[serde(default)]` 유지)을 쓴다.

**Rationale**: `AgentRunSettings`는 프론트가 전체 객체를 보내고 서버가 `serde(default)`로 빈 필드를 채운다. 여기에 `deny_unknown_fields`를 걸면 프론트 타입에만 있는 필드 하나로 저장이 깨진다(FR-002 위반). 최상위 봉투는 계약이므로 엄격하게 유지한다.

## R3. 저장 단위 4개의 저장소 — generic `JsonCollectionStore<T>` + 도메인별 port

**Decision**: `workbench-core/infrastructure/json_collection_store.rs`에 `JsonCollectionStore<T: Serialize + DeserializeOwned>`를 두고 `load`(읽기 전용, 손상→`StoreError::PrimaryCorrupt`)·`save`(temp+rename+.bak)·`recover_from_backup::<Vec<T>>`(lock 안)를 제공한다. 도메인별 port(`SavedPromptRepository`, `GoalRepository`, `AgentRunSettingsRepository`, 기존 `ProjectRepository`)는 그대로 두고 각 `Json*Repository`가 이 store를 감싼다(각 20줄 내외). `DataPaths`에 `saved_prompts_file()`·`goals_file()`·`agent_run_settings_file()`을 추가한다(파일 이름·위치는 오늘과 동일: `saved-prompts.json`·`goals.json`·`agent-run-settings.json`).

**Rationale**: 037 `json_store.rs`의 load/recover 분리(R14)를 복사 없이 네 저장소에 적용한다. port를 도메인별로 유지하는 것은 헌장 III(port는 시그니처 타입만)과 기존 서비스 코드(`goal_service`가 `GoalRepository`를 받음)를 그대로 옮기기 위해서다.

**Alternatives considered**: 저장소 하나에 `Vec<T>` generic port — 서비스 시그니처를 전부 바꿔야 하고 도메인 오류 타입 매핑이 흐려진다.

## R4. `StorageCoordinator` 일반화 — aggregate 이름으로 lock·revision·복구 재시도

**Decision**: `with_projects`를 일반화한 `with_aggregate<R, E>(aggregate: &str, f: impl FnMut() -> Result<R, E>, recover: impl Fn() -> Result<(), E>, is_corrupt: impl Fn(&E) -> bool)`를 추가하고, 편의 메서드 `with_projects`·`with_saved_prompts`·`with_goals`·`with_agent_run_settings`가 이를 호출한다. revision은 `HashMap<String, AtomicU64>`로 aggregate별 보관하며 `revision(aggregate)`/`set_revision(aggregate, v)`로 바뀐다. 상수 `PROJECTS_AGGREGATE`·`SAVED_PROMPTS_AGGREGATE`·`GOALS_AGGREGATE`·`AGENT_RUN_SETTINGS_AGGREGATE`.

Git 변경은 aggregate 이름을 **`git-worktrees:<저장소 루트의 canonical 경로>`**로 동적으로 만든다. 같은 저장소의 생성·삭제가 직렬화되고, 다른 저장소는 서로 막지 않는다. `aggregate_revision` 행은 생기지만 의미가 없으므로 Git 변경에 `expectedRevision`이 오면 `invalidArgument`("expectedRevision is not supported for git operations.")로 거절한다.

**Rationale**: grill Q3(파일 = 저장 단위 4개). 동적 aggregate 이름은 ledger 스키마 변경 없이(컬럼이 TEXT) 저장소별 직렬화를 얻는다.

## R5. intent-first 공통 runner — `project_create.rs`에서 추출

**Decision**: `application/intent_first.rs`에 `run_command<Out>(ledger, coordinator, hooks, ctx, MutationSpec) -> Result<CallReply, WorkbenchFault>`를 만든다. `MutationSpec`은 operation id, aggregate 이름, 정규화된 입력(지문용), `reserved_resource_id: Option<String>`, 그리고 lock 안에서 실행할 `apply: FnOnce() -> Result<Applied<Out> | Rejected(WorkbenchFault), DomainError>`를 가진다. runner가 키 필수 검사 → 지문 → `find`/`decide` → `begin`(`DuplicateReservation` 처리는 R16: 서버가 만든 id는 새로 만들어 3회 재시도, 호출자가 준 자원(worktree 경로)은 즉시 `conflict` outcome `unknown` retryable) → `crash_if(AfterPending)` → `with_aggregate` 안에서 expectedRevision 검사·`apply`·`crash_if(AfterJsonSave/BeforeApplied)`·`should_fail(LedgerComplete)`·`complete`·`set_revision` → `SavedButUnconfirmed`/`Rejected`/`Applied` 분기를 맡는다. `project_create.rs`는 이 runner를 쓰도록 리팩터링하고 **기존 crash point·동시성·멱등성 테스트를 수정 없이 통과**시켜 행위 보존을 증명한다.

**Rationale**: 변경 12개에 250줄짜리 절차를 복사하면 Codex 리뷰가 잡아낸 세 결함(unknown 처리, typed 복구, 예약 재시도)이 한 곳에서 어긋날 위험이 12배가 된다. 지문·상태 전이·응답 규칙은 이미 `idempotency.rs`에 순수 함수로 있으므로 runner는 그 위의 얇은 오케스트레이션이다.

## R6. 재시작 판정(reconciler) 규칙을 operation 종류별로 정의한다

**Decision**: `reconcile_pending`의 판정 closure를 operation별 `Reconciler`(trait, `fn resolve(&self, record: &LedgerRecord) -> Option<Value>`)로 분리해 handler와 함께 등록한다. 규칙:

| 변경 종류 | 판정 근거 | 결과 |
|---|---|---|
| **서버가 새 id를 만드는** 생성(`project.create`·`savedPrompt.create`) | `reserved_resource_id`(`project-*`·`prompt-*`)가 저장 파일에 있음. 이 id는 이 실행만 만들 수 있으므로 존재 = 적용 | `applied`(저장된 항목을 결과로) / 없으면 `unknown` |
| 삭제(`project.delete`·`savedPrompt.delete`·`goal.clear`) | 대상 id가 저장 파일에 **없음** | `applied`(결과 `null`) / 있으면 `unknown` |
| **upsert**(`goal.create`·`agentRunSettings.save`)·수정(`project.update`·`savedPrompt.update`·`goal.update`·`goal.recordProgress`) | 관찰로 구별 불가 — `goal.create`는 같은 worktree의 기존 목표를 **교체**하므로 "목표가 있다"는 이전 목표일 수 있다 | 항상 `unknown`, 예약 없음 |
| Git 생성(`git.createWorktree`) | `reserved_resource_id`(해석된 worktree 경로)가 `git worktree list`에 있음 | `applied`(결과 `null`) / 없으면 `unknown` |
| Git 삭제(`git.deleteWorktree`) | 경로가 `git worktree list`에 **없음** | `applied` / 있으면 `unknown` |

**Rationale**: grill Q4가 Git 변경에 확정한 종료 상태 규칙을 "대상이 명확히 관찰되는 변경"(삭제)에 일관되게 적용하고, 관찰로 구별할 수 없는 수정·upsert는 037 FR-009대로 `unknown`이다. 초안은 `goal.create`를 "workingDirectory의 goal이 있으면 applied"로 두었으나 Codex 리뷰(2026-09-26)가 지적한 대로 `goal_service::create_goal`은 기존 목표를 `retain`으로 지우고 새 목표를 넣는 **교체**여서, 이관 전부터 있던 목표가 "적용됨" 증거로 오판되어 요청된 교체가 조용히 사라진다. 그래서 upsert로 분류하고 `unknown`으로 판정한다. 수정·upsert에 `unknown`이 남는 빈도는 "JSON 저장 뒤 ledger 확정 전 강제 종료"라는 좁은 창에 한정되고, 사용자가 재조회하면 실제 상태가 보인다.

**Alternatives considered**: (a) 수정 전 스냅샷 해시를 pending에 저장해 사후 비교 — `NewLedgerEntry` 확장과 저장소 읽기 2회가 필요하고 revision과 중복된다. (b) `goal.create`에 대해 저장된 목표의 (objective, tokenBudget)로 `input_fingerprint`를 재계산해 대조 + `createdAt ≥ ledger.created_at` — 같은 objective로 진행을 초기화하는 교체를 구별하지 못해 여전히 거짓 `applied`가 가능하다. 2단계 이후 저장이 SQLite로 갈 때 자연스럽게 해결되므로 지금 넣지 않는다.

## R7. 도메인 오류 enum과 FaultCode 매핑

**Decision**: 도메인마다 `thiserror` enum을 두고 `Display`는 기존 문구를 그대로 낸다(037 R9). 매핑:

| 도메인 오류 | FaultCode | 비고 |
|---|---|---|
| `*::Required(field)` — "`{Label} is required.`" | `invalidArgument` | `details.fieldPath` |
| `SavedPromptError::NotFound` "Saved prompt not found.", `GoalError::NotFound` "Goal not found.", `ProjectError::NotFound` | `notFound` | |
| `AgentRunSettingsError::NoBuiltInProfile` "At least one built-in agent profile must stay enabled." | `invalidArgument` | |
| `WorktreeFileError::OutsideWorktree` "File path must stay inside the worktree." | `forbidden` | grill Q5 |
| `WorktreeFileError::NotADirectory` "Working directory must be a directory.", `::NotRegularFile` "Only regular files can be previewed.", 경로 없음 | `notFound` | 사전 확인 가능한 것만 |
| `GitError::GitNotFound`(spawn `ErrorKind::NotFound`) | `unavailable`, retryable | |
| `GitError::CommandFailed(stderr)` | `internal`, not retryable | message = stderr 그대로(`git_error_message`) |
| `GitError::WorktreeHasChanges` "Worktree has changes and cannot be deleted.", `::StatusUnresolved` "Worktree status is not resolved yet and cannot be deleted." | `preconditionFailed` | 삭제 전 확인 실패. outcome `notApplied` |
| `ProviderSessionError::Storage` | `internal` | 손상 항목은 건너뛰므로 실제로는 드묾 |
| `*::Storage`/`StoreCorrupt`(복구 실패) | `unavailable` / `internal` | 037과 동일 |

Git 어댑터의 `Result<_, String>`은 core로 옮기면서 `GitError`로 바꾼다. 실행 파일 없음과 비정상 종료를 구분하기 위해 `Command::output()`의 `io::Error`와 `!status.success()`를 각각 매핑한다. stderr 본문은 해석하지 않는다(grill Q5).

## R8. fixture 형식 확장 — 저장 단위 seed와 Git 저장소 seed

**Decision**: `tests/support/fixtures.rs`의 `Seed`에 `savedPrompts`·`goals`·`agentRunSettings`(각 JSON 배열, 파일에 그대로 기록)와 `gitRepo`를 추가한다. `gitRepo`는 `{ "commits": [{ "message", "files": {path: content}, "branch"? }], "branches": [...], "worktrees": [...], "workingChanges": {path: content|null} }` 형태로, test support가 임시 디렉터리에 `git init` 후 **고정 author/committer 이름·이메일·날짜**(`GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE`를 커밋 순서대로 1초씩 증가)로 만든다. 그러면 커밋 해시가 결정적이다. fixture의 request/expect 문자열 안 `{{repo}}`는 임시 저장소 경로로, `{{repoName}}`은 디렉터리 이름으로 치환한다. 파일 `modifiedMs`·`size`처럼 환경 의존 값은 `ignoreFields`로 뺀다.

**Rationale**: 037의 fixture 비교기(부분 일치·`ignoreFields`·`requests/expects` 시퀀스·`expectAfter`)를 그대로 쓰면서 Git 결과를 결정적으로 비교할 수 있다. Git 명령이 없는 CI는 없다(이미 AW 테스트가 git CLI를 쓴다).

**Alternatives considered**: Git 결과를 스냅샷 파일로 저장 — 경로가 환경마다 달라 치환이 필요한 건 같고, 세 경로 비교의 본질(in-memory vs HTTP 동일성)에는 스냅샷이 필요 없다.

## R9. 테스트 HTTP harness와 principal

**Decision**: harness는 바꾸지 않는다. `AuthenticatedPrincipal::desktop()`에 새 scope 10개(`savedPrompt:read/write`, `goal:read/write`, `agentRunSettings:read/write`, `git:read/write`, `worktree:read`, `agent:read`)를 추가하고 `test_readonly()`에는 `:read` 전부 + `system:describe`를 준다. `system-describe-desktop.json`·`system-describe-readonly.json` fixture의 기대 개수는 32/19로 갱신한다(변경 13개 = 038의 12 + `project.create`가 readonly에서 빠진다).

## R10. Tauri 호환 어댑터 일반화와 perf 로그 유지

**Decision**: `workbench_compat.rs`에 generic `call_query<Out: DeserializeOwned>(runtime, operation, input: Value) -> Result<Out, String>`와 `call_command<Out>(runtime, operation, input)`(멱등성 키를 호출마다 `random()`)를 두고, command 29개는 `*Input → serde_json::Value` 변환 후 이 둘을 호출한다. 출력이 `()`인 operation은 `Out = ()`로 `null`을 받는다. 기존 `run_blocking_command`가 남기던 perf 로그(specs/007 R1)는 `perf_log::log_async_command(name, future)`로 대체해 `run_ms`를 계속 남긴다(`wait_ms`는 blocking pool 진입 대기였으므로 handler 내부 `spawn_blocking`으로 옮겨진 뒤에는 측정 대상이 없어 0으로 기록하지 않고 필드를 생략한다).

**Rationale**: 29개 command 본문이 5줄 내외로 줄고(FR-002), 변환 유닛 테스트는 fixture로 한다(037 방식). perf 로그를 조용히 없애면 007에서 진단하던 IPC 직렬화 회귀를 볼 수 없게 된다.

## R11. `acp-agent-core`·`anyhow`·`walkdir` 의존 추가

**Decision**: `workbench-core`가 `acp-agent-core`(PermissionMode·ContextSizePreset·AgentDescriptor·ConfigurableAgentCatalog·`MAX_RALPH_*` 상수)와 `git-core`(상태·이력 리더와 모델), `anyhow`·`walkdir`·`chrono`(provider 세션 어댑터)를 의존한다. `acp-agent-core`는 Tauri에 의존하지 않는다(Cargo.toml 확인: agent-client-protocol, anyhow, chrono, libc, percent-encoding, serde, shell-words, tokio, uuid). provider 세션 port는 `anyhow::Result`를 `Result<_, ProviderSessionError>`로 바꾸고 어댑터 내부에서만 `anyhow`를 쓴다.

**Rationale**: 정본 1단계 "`acp-agent-core`는 그대로 의존한다". `git-core`는 이미 AW가 쓰는 공용 crate이므로 core의 의존은 grill Q6과 충돌하지 않는다(확장하지 않고 소비만).

## R12. Git·파일 조회는 lock을 잡지 않는다

**Decision**: `git.*`·`worktree.*` 조회 17개 중 사용자 저장소를 읽는 것은 aggregate lock을 잡지 않는다(오늘과 같음). 저장 단위를 읽는 조회(`savedPrompt.list`·`goal.get`·`agentRunSettings.get`)만 037 R14대로 해당 aggregate lock 안에서 읽는다. `git.createWorktree`·`git.deleteWorktree`는 R4의 동적 aggregate로 서로만 직렬화한다.

**Rationale**: FR-006은 저장 파일에 대한 규칙이다. 사용자 저장소는 외부 프로세스도 동시에 바꾸므로 lock이 정합성을 보장하지 못하고 지연만 늘린다.

## R13. 계약 생성물 크기와 `OperationId::ALL`

**Decision**: `OperationId`에 29개 variant를 추가하고 `ALL` 배열·`OPERATIONS` 표를 32개로 늘린다. `openapi.rs`의 골든 테스트는 그대로 두고(variant 수 = `OPERATIONS.len()`) 생성물을 재생성해 커밋한다. `openapi-typescript` 7.13은 수천 줄 스키마를 문제없이 처리한다(로컬 실측은 tasks의 검증 항목).

## R14. 인벤토리 표의 위치

**Decision**: `docs/workbench-seam.md`에 "command 인벤토리(71)" 절을 추가한다. 열: command, 분류(`이관됨(037)`·`이관됨(038)`·`2단계로 이연`·`데스크톱 유지`), operation 또는 이유. 초안은 [data-model.md §6](data-model.md)에 있다.

## R15. SC-001 지연 예산

**Decision**: 037 `list_latency.rs`(#[ignore]) 방식으로 `savedPrompt.list`·`goal.recordProgress`·`worktree.listFiles`(작은 저장소)를 in-memory 경로에서 100회 측정해 p95를 기록한다. 변경 operation은 ledger 2 commit + JSON 1 write가 추가되므로 037 `project.create` 측정값(수 ms)과 같은 자릿수여야 한다.

## R16. 예약(reservation)의 수명 — `pending` 동안만 배타, 종료 상태에서 해제 (2026-09-26 Codex 리뷰 반영으로 추가)

**Decision**: ledger의 자원 예약 unique index를 **`state = 'pending'`인 행에만** 적용하도록 바꾼다(schema **v2**). 예약은 두 역할을 갖는데 — (1) 같은 자원에 대한 **동시 진행 배제**, (2) 재시작 판정의 **증거**(`reserved_resource_id` 값) — (1)은 진행 중에만 필요하고 (2)는 컬럼 값만 있으면 되므로 index에서 상태를 한정해도 증거는 잃지 않는다(reconciler는 `pending` 행만 본다). 종료 상태(`applied`·`failed`·`unknown`) 전이는 예약을 자동으로 해제한다.

| 상태 | 예약 배타 | 근거 |
|---|---|---|
| `pending` | 유지 | 같은 자원의 두 실행이 동시에 부작용을 내면 안 됨 |
| `applied` | 해제 | 자원이 존재. 같은 자원에 대한 다음 변경(삭제 뒤 재생성 등)은 새 실행 |
| `failed` | 해제 | 부작용 없음. 같은 자원으로 재시도(새 키)는 정당 |
| `unknown` | 해제 | create가 unknown = 자원이 관찰되지 않음, delete가 unknown = 자원이 아직 있음. 어느 쪽도 새 실행을 막을 이유가 없다 |

`DuplicateReservation`(= 다른 `pending` 실행이 같은 자원을 잡음)의 처리는 자원의 출처로 나눈다: 서버가 만든 id(`project-*`·`prompt-*`)는 새 id로 최대 3회 재시도(037 그대로); 호출자가 준 자원(worktree 경로)은 재시도하지 않고 `conflict`, outcome `unknown`, retryable=true("Another change to this worktree path is still in progress.")로 응답한다.

**Migration**: `SCHEMA_VERSION = 2`. `migrate()`는 `DDL_V1`(변경 없음)을 실행한 뒤 버전이 없거나 1이면 `MIGRATION_V2`를 실행하고 버전 2를 기록한다: `DROP INDEX IF EXISTS operation_ledger_reserved; CREATE UNIQUE INDEX IF NOT EXISTS operation_ledger_reserved_pending ON operation_ledger (aggregate, reserved_resource_id) WHERE reserved_resource_id IS NOT NULL AND state = 'pending';`. 버전 2면 통과, 3 이상이면 `UnsupportedSchema`. 037이 만든 v1 파일은 첫 기동에서 자동 승격되며 행 데이터는 바뀌지 않는다.

**Rationale**: Codex 리뷰가 지적한 대로 v1 index는 상태와 무관하게 non-null 예약 전부에 배타적이어서, 재사용되는 자원 식별자(worktree 경로, 초안의 goal `workingDirectory`)를 예약하면 `applied`·`failed`는 24h GC 전까지, `unknown`은 영구히 같은 자원의 정당한 다음 변경(만들고 지우고 다시 만들기, 실패 뒤 재시도)을 `DuplicateReservation`으로 막는다. 037은 `project-{nanos}`가 유일해 이 결함이 드러나지 않았다.

**Tests**: `tests/reservation_lifecycle.rs` — (a) `git.createWorktree` → `git.deleteWorktree` → 같은 경로 `git.createWorktree`(전부 새 키) 성공; (b) 잘못된 reference로 `failed`된 뒤 같은 경로 새 키 재시도 성공; (c) 같은 경로 두 create 동시 → 하나 `applied`, 하나 `conflict` outcome `unknown`; (d) v1 DDL로 만든 ledger 파일(예약이 남은 `applied` 행 포함)을 열면 버전 2로 승격되고 같은 예약으로 `begin`이 성공한다.

**Alternatives considered**: (a) 종료 상태 전이 시 `reserved_resource_id`를 `NULL`로 지움 — 증거가 사라져 사후 진단(어느 경로였는지)이 불가능하고 `applied`의 `result_json`이 `null`인 Git 변경은 경로를 잃는다. (b) index를 그대로 두고 자원 예약을 Git 변경에 쓰지 않음 — 같은 경로 동시 생성을 막을 수단과 재시작 판정 증거를 함께 잃는다.

## 정리: 추가·변경되는 의존성

| crate/package | 추가 | 이유 |
|---|---|---|
| `workbench-core` | `git-core`(path), `acp-agent-core`(path), `anyhow`, `walkdir` | 어댑터·도메인 이동(R11) |
| `workbench-protocol` | 없음 | DTO 미러(R1) |
| `apps/agentic-workbench/src-tauri` | 제거: `walkdir`(사용처가 이동하는 `fs_worktree_file_provider.rs`·`fs_provider_session_repository.rs` 둘뿐임을 확인) | 이동 |
| `packages/workbench-client` | 없음 | 생성물 갱신만 |
