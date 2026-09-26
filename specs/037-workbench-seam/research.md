# Research: Workbench Seam 도입 (037)

**Date**: 2026-09-26 | **Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)

Technical Context의 미확정 항목과 기술 선택을 조사해 결정으로 바꾼다. 정본 [서버-클라이언트 전환 조사](../../docs/client-server-architecture-research.md)가 이미 정한 것은 재검토하지 않고, 037 구현에 필요한 세부만 다룬다. 조사 기준일 2026-09-26, 로컬 toolchain `rustc 1.98.1`.

## R1. OpenAPI 3.1 생성과 request `oneOf` 상관 타입

- **Decision**: `utoipa` **5.x 최신**을 `crates/workbench-protocol`에 넣어 각 operation의 input/output 타입에 `ToSchema`를 derive하고, `CallRequest`의 `oneOf` union은 **operation registry에서 프로그램적으로 조립**한다. 각 variant는 `{ operation: enum["project.list"], input: $ref }` 형태의 object schema로, `discriminator` object는 쓰지 않는다. TypeScript 쪽은 `openapi-typescript` **7.13.0**으로 `components`·union 타입을 생성하고, `OperationMap`은 생성 타입 위에 **조건부 타입**(`Extract<Req, {operation: K}>["input"]` / `Extract<Res, {operation: K}>["output"]`)으로 손으로 한 번 정의한다.
- **Rationale**:
  - utoipa 5.0부터 OpenAPI 3.1만 지원하고, `#[schema(discriminator = ...)]`는 `#[serde(untagged)]` enum에만 붙으며 mapping을 명시하지 않으면 variant 이름이 discriminator 값으로 새는 문제가 있다([utoipa 5 migration guide](https://github.com/juhaku/utoipa/discussions/1124), [ToSchema docs](https://docs.rs/utoipa/latest/utoipa/derive.ToSchema.html), [discussion #1227](https://github.com/juhaku/utoipa/discussions/1227), [issue #1456](https://github.com/juhaku/utoipa/issues/1456)).
  - `openapi-typescript`는 `discriminator.mapping`이 없거나 `allOf` 아래에 있을 때 값을 schema 이름으로 만들거나 mapping을 무시한 사례가 있다([issue #2149](https://github.com/openapi-ts/openapi-typescript/issues/2149), [issue #1690](https://github.com/openapi-ts/openapi-typescript/issues/1690)). 반면 variant마다 tag 속성을 단일값 `enum`으로 두면 TypeScript가 그대로 판별 union으로 읽는다([openapi-typescript Advanced](https://openapi-ts.dev/advanced)).
  - 정본도 "각 variant schema에 `operation`의 literal/const 제약을 넣고 golden contract test로 검사하라"고 했다. registry가 유일한 source of truth이므로 union도 registry에서 만드는 것이 drift를 막는다.
  - `utoipa` 6.0.0은 2026-09-22에 나왔다([docs.rs](https://docs.rs/crate/utoipa/latest)). 037은 `utoipa-axum` router를 쓰지 않으므로 6.0의 이점이 없고, 4일 된 major는 부속 crate가 따라오지 않았을 수 있어 5.x를 택한다. 3단계에서 `utoipa-axum` 도입 시 함께 올린다.
- **Alternatives considered**:
  - `#[serde(untagged)]` + `#[schema(discriminator(property_name, mapping(...)))]` derive: 가능하지만 variant마다 tag 필드를 단일 variant enum으로 넣는 우회가 필요하고 registry와 이중 정의가 된다.
  - `schemars`로 JSON Schema만 생성하고 OpenAPI는 손으로 감싸기: TS 생성 파이프라인(openapi-typescript)과 3단계 `utoipa-axum`과의 연결이 끊긴다.
  - OperationMap을 Rust에서 별도 TS 파일로 생성: 생성기가 둘이 된다. 조건부 타입 한 파일이 더 단순하고 spike 목적(상관 타입이 컴파일에서 잡히는가)을 충족한다.
- **검증 방법**: `packages/workbench-client/src/operation-map.test-d.ts`에서 `expectTypeOf<OperationMap["project.list"]["output"]>().toEqualTypeOf<Project[]>()`와 `// @ts-expect-error` 케이스로 상관 타입을 고정한다(SC-005).
- **spike 결과(2026-09-26, T005)**: 성공. `utoipa` 5.5.0 `ToSchema` derive + registry 순회로 조립한 `oneOf`를 `openapi-typescript` 7.13.0에 넣으면 `CallRequest`가 `{ operation: "project.list"; input: components["schemas"]["ProjectListInput"]; ... } | { operation: "project.create"; input: ...ProjectCreateInput; idempotencyKey: ... } | ...` 형태의 판별 union으로 나오고, `CallReplyByOperation`도 `output: components["schemas"]["ProjectDto"][]` / `ProjectDto` / `DescribeOutput`로 짝지어졌다. `Extract<CallRequest, {operation: K}>["input"]` 조건부 타입이 그대로 동작해 `json!` 직접 조립 대안은 필요 없었다. 주의점 두 가지: (1) newtype(`RequestId`, `IdempotencyKey`)에 struct-level `#[schema(value_type)]`는 utoipa 5에서 `PartialSchema`를 만들지 않아 수동 `impl PartialSchema + ToSchema`가 필요했다. (2) `Vec<T>`의 output 스키마는 `ArrayBuilder::items(Ref)`로 직접 만들었다.

## R2. SQLite ledger 라이브러리와 WAL 안전성

- **Decision**: `rusqlite = { version = "0.40", features = ["bundled"] }`. 연결은 **단일 writer 연결 하나**를 `StorageCoordinator`가 소유하고 `std::sync::Mutex`로 감싼다. 열 때 `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000ms`를 설정하고 `foreign_keys=ON`. 모든 SQLite 호출은 `tokio::task::spawn_blocking` 안에서 실행한다.
- **Rationale**:
  - 2026-03에 발견된 SQLite "WAL-reset" 손상 버그는 3.51.3에서 고쳐졌고, rusqlite 0.39부터 3.51.3 이상을 bundled한다. 0.40.2가 최신이며 bundled SQLite는 3.53.x다([rusqlite features](https://docs.rs/crate/rusqlite/latest/features), [WAL-reset 분석](https://github.com/ynishi/eventsdb/issues/18)). 그 버그는 같은 파일에 **두 연결 이상**이 동시에 쓸 때 발생하므로, 단일 writer 연결 설계가 이중 방어가 된다.
  - `bundled`는 시스템 SQLite 버전에 의존하지 않아 macOS/Windows/Linux에서 같은 동작을 보장한다. rusqlite 0.40의 MSRV는 1.85이고 로컬은 1.98.1, CI는 `rustup stable`이라 문제없다.
  - ledger 트랜잭션은 수 밀리초의 짧은 쓰기라 async 드라이버가 필요 없고, 정본이 말한 "blocking JSON I/O는 bounded blocking pool로" 원칙에 `spawn_blocking`으로 맞춘다.
- **Alternatives considered**: `sqlx`(빌드 타임 DB 또는 offline 데이터 필요, CI 복잡), `redb`(SQL 없음, ledger 조회·TTL GC를 직접 인덱싱). 2026-09-26 grill에서 rusqlite로 확정.
- **CI 영향**: `bundled`는 C 컴파일이 필요해 첫 `cargo build`가 길어진다. `quality.yml`의 macOS runner에는 clang이 있어 추가 설정이 없다. 빌드 시간은 tasks 단계에서 실측해 기록한다.

## R3. `Workbench` trait의 async 표현

- **Decision**: `async-trait = "0.1"`을 `workbench-core`와 `workbench-protocol`(trait 정의 위치)에 추가하고 `#[async_trait] pub trait Workbench: Send + Sync { async fn call(...); fn events(...); }`로 정의한다.
- **Rationale**: Rust 1.75+의 native `async fn in trait`은 `dyn Workbench`(Tauri managed state, HTTP harness의 `Arc<dyn Workbench>`)에서 object-safe하지 않다. `async-trait`는 boxed future로 이를 해결하고 ecosystem 표준이다. 워크스페이스에 아직 없지만 의존 크기가 작다.
- **Alternatives considered**: `Pin<Box<dyn Future>>`를 직접 반환하는 수동 시그니처(장황), `call`을 동기로 두기(037의 project 연산은 동기지만 정본 trait 계약이 async이고 038에서 ACP 연산이 들어온다).

## R4. `DataPaths` 주입과 다른 `from_app` 저장소와의 공존

- **Decision**: `workbench-core`에 `pub struct DataPaths { pub app_data_dir: PathBuf }`와 `DataPaths::projects_file()`, `DataPaths::ledger_file()`(`workbench/ledger.sqlite`)를 둔다. AW `lib.rs`의 `.setup()`에서 `app.path().app_data_dir()`로 한 번 만들어 `WorkbenchRuntime`(`Workbench` 구현 + `StorageCoordinator` + ledger)을 조립하고 `app.manage(Arc<WorkbenchRuntime>)`한다. 나머지 9개 `Json*Repository::from_app(&AppHandle)`는 **그대로 둔다**.
- **Rationale**: 정본 1단계가 "`from_app(&AppHandle)` 대신 `DataPaths` 생성자 주입"을 요구한다. 프로젝트 저장소만 core로 옮기므로 `JsonProjectRepository::new(DataPaths)`만 바꾸고, 다른 저장소는 038에서 도메인별 이동과 함께 바꾼다. 두 방식이 한 파일에 공존하는 기간은 038까지로 한정된다.
- **주의**: `update_project`·`delete_project` Tauri command는 037 범위 밖이지만 같은 `project_service`를 쓴다. core로 옮긴 `project_service`를 AW가 직접 import해 기존처럼 호출하도록 유지한다(Workbench를 거치지 않음). 038에서 `project.update`/`project.delete` operation으로 이관한다. 단, 이 두 command가 `JsonProjectRepository`를 별도로 만들어 쓰면 coordinator lock 밖에서 JSON을 써서 lost update가 생긴다. 따라서 037에서 이 둘도 **`WorkbenchRuntime`이 소유한 같은 repository·lock을 통해** 저장하게 배선만 바꾼다(operation 계약은 만들지 않음). 이것이 "서버 런타임이 유일한 쓰기 주체" 원칙(spec Persistence and safety)을 037에서도 지키는 최소 조치다.

## R5. Aggregate revision의 저장 위치 (2026-09-26 Codex 리뷰 반영으로 수정)

- **Decision**: 만료되지 않는 `aggregate_revision(aggregate PK, revision, updated_at)` 테이블을 두고, `applied` 전이와 **같은 SQLite 트랜잭션**에서 `revision + 1`로 갱신한다. ledger row의 `revision`은 그 복사본이다. `StorageCoordinator`가 기동 시 이 테이블에서 읽어 캐시한다.
- **Rationale**: 첫 안은 `MAX(operation_ledger.revision)`으로 유도하는 것이었다. Codex adversarial review가 지적한 대로, TTL GC(24h)가 만료된 `applied` row를 모두 지우면 재시작 뒤 revision이 0으로 되돌아가고, 이전에 발급된 stale `expectedRevision`이 통과해 FR-010의 단조 증가 보장이 깨진다. revision의 수명(영구)과 멱등성 결과의 수명(TTL)이 다르므로 저장소도 분리해야 한다. `projects.json`에 넣는 것은 FR-002(형식 불변)에 어긋나 여전히 제외한다.
- **grill 결정 4 수정**: "`schema_version` + `operation_ledger`만"에서 `aggregate_revision`을 추가한다. 결정 4의 취지는 "쓰는 곳 없는 테이블(outbox)을 미리 만들지 않는다"였고, `aggregate_revision`은 037에서 바로 쓰이므로 취지와 충돌하지 않는다. spec Assumptions도 함께 고쳤다.
- **Alternatives considered**: GC가 aggregate별 최신 `applied` row를 남기도록 예외 규칙 두기(GC 규칙이 revision 정확성을 떠맡아 취약), JSON envelope에 `revision` 추가(037 범위 밖).
- **검증**: `revision_retention` 테스트 — 프로젝트 3개 생성(revision 3) → 모든 row의 `expires_at`을 과거로 갱신 → GC → 프로세스 재시작(runtime 재생성) → 4번째 생성의 revision이 4이고, `expectedRevision: 3`으로 보낸 요청은 `preconditionFailed`.

## R6. intent-first 상태 전이와 reconciler 규칙

- **Decision**: ledger row 상태는 `pending → applied | failed`, 재시작 시 `pending → applied | unknown`. 흐름:
  1. 입력 정규화(`normalize_draft`) → canonical JSON(키 정렬) → SHA-256 지문
  2. 같은 (principal, operation, key) row 조회: `applied`/`failed`면 저장된 결과 반환(지문 다르면 `conflict`), `pending`이면 `conflict`(진행 중), `unknown`이면 `conflict` + `outcome: unknown`
  3. 없으면 `pending` insert(예약 project id 포함) — **commit**
  4. coordinator lock 획득 → `load_projects` → push → `save_projects`(기존 atomic temp+rename)
  5. `applied` update(결과 JSON, revision+1) — **commit** → 응답
  6. 4에서 실패하면 `failed` update(오류) — commit → Fault 반환
- **Reconciler**(기동 시, readiness 전): 모든 `pending` row에 대해 `projects.json`에 예약 id가 있으면 `applied`(결과는 JSON에서 재구성, revision은 현재 MAX+1), 없으면 `unknown`. 자동 재실행하지 않는다.
- **6'. 저장 뒤 `complete` 실패**(2026-09-26 구현 리뷰 반영): 5단계가 실패하면 `fail()`을 부르지 않고 row를 `pending`으로 둔 채 `unavailable` + `outcome: unknown`을 돌려준다. 부작용이 끝난 뒤의 실패를 `failed`(=notApplied)로 기록하면 reconciler가 영구히 건너뛰어 거짓 실패와 중복 생성이 생긴다. 다음 기동의 reconciler가 4단계 결과를 보고 `applied`로 확정한다.
- **Rationale**: 정본의 "외부 side effect가 있는 command는 실행 전에 durable intent를 남긴다"와 "`unknown`은 자동 재실행하지 않는다" 규칙을 그대로 따른다. project id를 3단계 전에 예약해 두어야 5단계 crash 뒤 JSON에서 적용 여부를 판정할 수 있다.
- **지문 라이브러리**: `sha2 = "0.10"`. canonical JSON은 `serde_json::Value`를 `BTreeMap` 기반으로 재직렬화해 얻는다(`serde_json`의 `preserve_order` feature를 켜지 않는다).

## R7. 037의 principal 모델

- **Decision**: `AuthenticatedPrincipal { kind: PrincipalKind, scopes: BTreeSet<Scope> }`, `PrincipalKind = Desktop | Test`, `Scope = ProjectRead | ProjectWrite | SystemDescribe`. AW 조립부가 `Desktop`에 세 scope를 부여해 Tauri compat Adapter에 고정 주입한다. 테스트는 `Desktop` 전체와 `Test{ProjectRead, SystemDescribe}` 두 principal을 쓴다. HTTP harness는 `Authorization: Bearer test-desktop` / `test-readonly` 고정 토큰을 principal로 매핑한다(테스트 전용).
- **Rationale**: spec Assumptions "호출자 정체는 두 종류만". 정본 Invariant 1(principal은 auth Module만 생성)과 2(describe와 call은 같은 authorization 판단)를 037에서 성립시키되 토큰 발급·검증은 3단계로 미룬다.

## R8. test-only HTTP harness 위치와 형태

- **Decision**: `workbench-core`의 `[dev-dependencies]`에 `axum = "0.7"`, `tokio`, `reqwest = { default-features = false, features = ["json"] }`(또는 `hyper` client)를 넣고 `crates/workbench-core/tests/support/http_harness.rs`에 `POST /v1/calls` router를 둔다. 테스트가 `127.0.0.1:0`에 bind해 실제 직렬화를 거친다. 운영 코드에는 포함되지 않는다.
- **Rationale**: grill 결정 1. AW가 이미 axum 0.7·tower-http 0.5를 쓰므로 버전 충돌이 없다. 3단계에서 이 harness의 router 함수를 `workbench-server` crate로 옮긴다.
- **Alternatives considered**: `tower::ServiceExt::oneshot`으로 네트워크 없이 테스트 — 직렬화는 검증되지만 "실제 loopback HTTP 경로"라는 spec 문구와 SC-001의 지연 측정에 못 미친다. 둘 다 둘 수 있으나 037은 실제 bind만 한다.

## R9. 오류 타입과 기존 문구 보존

- **Decision**: core로 옮기는 `project_service`의 오류를 `String`에서 `ProjectError` enum(`NameRequired`, `WorkingDirectoryRequired`, `NotFound`, `Storage(String)`, `Clock(String)`)으로 바꾸고 `Display`가 **기존 문자열을 그대로** 출력하게 한다("Project name is required." 등). `WorkbenchFault::from(ProjectError)`가 `NameRequired`/`WorkingDirectoryRequired` → `invalidArgument`, `NotFound` → `notFound`, `Storage` → `unavailable`, `Clock` → `internal`로 매핑하고 `message = error.to_string()`. Tauri compat Adapter는 `fault.message`만 `String`으로 돌려준다.
- **Rationale**: grill 결정 6과 `App.tsx`가 `String(caughtError)`를 화면에 그대로 보여 주는 사실. 새 실패 모드(`conflict`, `preconditionFailed`, `unavailable`(ledger 잠김))의 message는 한 문장 한국어로 둔다.
- **골든 테스트**: 기존 `project_service` 테스트의 오류 문자열 기대값이 있으면 그대로 유지한다.

## R10. contract fixture와 세 Adapter 공통 테스트

- **Decision**: `crates/workbench-protocol/fixtures/*.json`에 시나리오별 `{ request, expect: { reply | fault } }` 파일을 두고, `workbench-core/tests/contract_suite.rs`가 fixture를 읽어 **in-memory Adapter**와 **HTTP harness**에 같은 요청을 보낸다. Tauri compat Adapter는 `AppHandle`이 필요해 fixture를 직접 돌리기 어려우므로, compat Adapter의 순수 부분(`ProjectInput → CallRequest` 변환, `CallReply/Fault → Result<_, String>` 변환)을 함수로 분리해 AW crate 유닛 테스트에서 같은 fixture로 검증한다. 세 경로의 "관측 결과 동일"은 이 세 테스트가 같은 fixture를 공유함으로써 성립한다(SC-002).
- **시나리오 최소 집합**: list 성공(0개·N개), create 성공, create 이름 누락, create 디렉터리 누락, 없는 operation, 스키마 위반 입력, 권한 없는 principal의 create, describe(desktop/readonly), 멱등 재요청 동일 결과, 같은 키 다른 payload conflict, stale expected_revision.
- **Alternatives considered**: Tauri command를 `tauri::test` mock runtime으로 직접 호출 — 가능하지만 AppHandle mock 비용이 크고 037의 목적(계약 동일성)은 변환 함수 검증으로 충분하다.

## R11. 계약 생성물 커밋과 CI drift 검사

- **Decision**:
  - `crates/workbench-protocol/src/bin/export_openapi.rs`가 `openapi.json`을 stdout에 쓴다. `crates/workbench-protocol/openapi/workbench.openapi.json`에 커밋.
  - `packages/workbench-client/package.json`에 `"generate": "cargo run -q -p workbench-protocol --bin export_openapi > ../../crates/workbench-protocol/openapi/workbench.openapi.json && openapi-typescript ../../crates/workbench-protocol/openapi/workbench.openapi.json -o src/generated/workbench.ts"`.
  - 루트 `package.json`에 `"generate:contracts": "pnpm --filter @yoophi/workbench-client generate"`.
  - `quality.yml`에 `Install dependencies` 뒤 `pnpm run generate:contracts && git diff --exit-code -- crates/workbench-protocol/openapi packages/workbench-client/src/generated` 단계 추가.
- **Rationale**: grill 결정 5. Rust toolchain은 CI에 이미 있다. TS 빌드는 커밋된 생성물만 읽으므로 Rust 없이도 동작한다.
- **주의**: `export_openapi`는 `serde_json::to_string_pretty` + 끝 개행으로 결정적 출력이어야 한다. utoipa의 `preserve_order` feature를 켜 필드 순서를 struct 순서로 고정해 diff 노이즈를 줄인다.

## R12. 동시성 시나리오(SC-004)

- **Decision**: in-memory Adapter에 대해 `tokio::spawn` 20개가 서로 다른 키로 `project.create`를 동시에 호출하는 테스트와, HTTP harness에 대해 같은 테스트를 둔다. 기대: 프로젝트 20개, revision 20, ledger `applied` 20건. 같은 키 20개 동시 호출도 추가: 프로젝트 1개, `applied` 1건, 나머지 19건은 저장된 결과 또는 `conflict`(진행 중) 중 하나이며 어느 쪽이든 부작용은 없다.
- **Rationale**: coordinator의 aggregate lock이 `load→mutate→save` 전체를 감싸는지, ledger의 (principal, operation, key) unique 제약이 경합을 막는지 한 테스트로 확인한다.

## R13. SC-001 지연 예산 측정

- **Decision**: contract suite에 `project.list` 1,000회 in-memory 호출의 p95를 측정해 5ms 미만이면 통과하는 벤치마크성 테스트를 `#[ignore]`로 두고 quickstart에서 수동 실행한다. 사용자 체감 기준(+50ms)은 4단계 Desktop 전환에서 HTTP 경로로 다시 측정한다.
- **Rationale**: 037의 Desktop 경로는 in-process 호출이라 serde round trip 비용만 추가된다. CI에서 시간 기반 테스트를 강제하면 flaky해진다.

## R14. 저장 파일 복구 쓰기와 aggregate lock (2026-09-26 Codex 리뷰 반영으로 추가)

- **사실**: AW `infrastructure/json_store.rs`의 `load_json`은 primary 파싱이 실패하고 `.bak`이 있으면 **읽기 경로에서** `fs::copy(&backup_path, store_path)`로 복구한다. 이 함수는 lock을 모르고, `fs::copy`는 원자 교체가 아니다. `atomic_write`는 저장 전에 primary를 `.bak`으로 복사한다.
- **문제**: 037 첫 안은 query가 lock 없이 읽었다. 손상된 primary를 만난 query가 `.bak`을 읽고 잠시 멈춘 사이, create가 lock 안에서 복구·저장·`applied` commit을 끝내면, query가 재개하며 실행하는 늦은 `fs::copy`가 새 프로젝트를 덮어쓴다. ledger는 성공으로 남고 프로젝트는 사라진다. 즉 query가 사실상 쓰기 연산이었다.
- **Decision**:
  1. core의 `json_store`는 `load`(읽기 전용, 손상 시 `StoreError::PrimaryCorrupt`)와 `recover_from_backup`(temp+rename 교체)을 분리한다. `load`는 어떤 경우에도 파일을 쓰지 않는다.
  2. `recover_from_backup`은 `StorageCoordinator`가 aggregate lock을 잡은 상태에서만 호출하고, 호출 직전에 primary를 다시 파싱해 다른 쓰기가 이미 복구·갱신했으면 복구를 건너뛴다.
  3. 모든 프로젝트 읽기(`project.list`, `update_project`/`delete_project`의 선행 load 포함)는 aggregate lock을 잡는다. contract §3의 "query는 lock 없이 읽는다" 규칙을 폐기하고 "query는 ledger를 거치지 않지만 lock은 잡는다"로 바꾼다.
- **Rationale**: 복구는 쓰기이므로 쓰기와 같은 직렬화 경계 안에 있어야 한다. 프로젝트 목록은 수 KB이고 lock은 in-process `Mutex`라 읽기 지연은 SC-001 예산에 영향이 없다. `RwLock`으로 읽기를 병렬화하면 복구 시 write lock 승격이 필요해 복잡도만 늘고 037 규모에서 이득이 없다.
- **Alternatives considered**: AW `json_store`를 그대로 복제(결함 유지), 읽기 경로에서 복구를 아예 하지 않고 오류만 반환(사용자가 손상 파일을 스스로 못 고침 — 기존 UX 후퇴), `RwLock` + 승격(복잡).
- **검증**: `recovery_under_lock` 테스트 — primary를 잘못된 JSON으로 덮고 `.bak`은 정상인 상태에서, read 1개와 create N개를 동시에 실행 → 최종 `projects.json`에 N개가 모두 있고, ledger `applied` row마다 대응 프로젝트가 존재하며, `.bak`에서 복구된 이전 프로젝트도 보존된다. 복구 함수에 테스트 전용 지연 hook을 두어 인터리빙을 강제한다.
- **AW 원본 `json_store.rs`의 같은 결함**: 037 범위 밖(다른 9개 저장소는 여전히 lock 없이 읽고 복구한다). 038에서 저장소를 core로 옮길 때 같은 분리를 적용하도록 plan "038 이후로 넘기는 것"에 기록했다.

## 정리: 추가되는 의존성

| crate/패키지 | 버전 | 위치 | 용도 |
|---|---|---|---|
| `utoipa` | 5.x (features: `preserve_order`) | workbench-protocol | ToSchema, OpenAPI 3.1 문서 |
| `async-trait` | 0.1 | workbench-protocol, workbench-core | `Workbench` trait |
| `rusqlite` | 0.40 (`bundled`) | workbench-core | operation ledger |
| `sha2` | 0.10 | workbench-core | 입력 지문 |
| `thiserror` | 2 | workbench-protocol, workbench-core | 오류 enum |
| `axum` 0.7, `reqwest`, `tokio` | 기존 버전 | workbench-core dev-deps | HTTP harness |
| `openapi-typescript` | 7.13.0 | packages/workbench-client devDeps | TS 타입 생성 |
| `vitest` | 기존 4.x | packages/workbench-client devDeps | 타입 테스트 |
