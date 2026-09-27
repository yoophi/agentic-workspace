---

description: "Task list for implementing the Workbench event stream (stage 2a)"
---

# Tasks: 이벤트 모델 통합 — Workbench 이벤트 스트림 (서버-클라이언트 전환 2a)

**Input**: Design documents from `/specs/039-workbench-events/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/workbench-events.md](./contracts/workbench-events.md), [contracts/tauri-compat-events.md](./contracts/tauri-compat-events.md), [quickstart.md](./quickstart.md)

**Tests**: spec의 성공 기준(SC-001 race 1,000회, SC-002 재수화, SC-003 gap, SC-004 두 경로 일치, SC-005 감시 참조 수)이 테스트를 요구한다. 각 스토리에서 테스트를 먼저 쓰고 실패를 확인한 뒤 구현한다. 037·038 테스트는 **수정 없이** 계속 통과해야 한다(단, `events_are_unsupported_in_037` 단위 테스트는 이 기능이 뒤집는 계약이므로 교체한다 — T012).

**Organization**: US1(구독 계약·hub)이 MVP이자 나머지 전부의 기반, US2(run 발행·데스크톱 순번·재수화 버퍼링), US3(worktree 알림 스트림), US4(이벤트 계약 생성)가 뒤따른다.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 실행 가능 (다른 파일, 미완료 의존 없음)
- **[Story]**: 해당 사용자 스토리 (US1–US4)
- 모든 태스크에 정확한 파일 경로를 포함한다

## Path Conventions

- **Reusable Rust**: `crates/workbench-protocol/src`, `crates/workbench-core/src`, `crates/workbench-core/tests`, `crates/workbench-protocol/fixtures/events`
- **App Tauri backend**: `apps/agentic-workbench/src-tauri/src/{inbound,infrastructure,ports}`
- **App frontend**: `apps/agentic-workbench/src/{entities,features}/agent-run` — 이 스토리들에서 유일하게 허용된 프론트 변경(grill Q2)
- **Reusable TypeScript**: `packages/workbench-client/src`
- **Documentation**: `docs/[english-file-name].md`

`crates/acp-agent-core`·`packages/agent-client`는 변경하지 않는다(grill Q5, research R6). 워크트리는 `/Users/yoophi/project/worktrees/039-workbench-events`이며 모든 명령은 그 루트에서 실행한다.

### 공통 규칙

- **lock 순서**: hub는 `streams` → 개별 스트림 → `terminal_order`·`evicted` 순으로만 잡는다. 스트림 lock을 쥔 채 `streams`를 잡지 않고, 두 스트림 lock을 동시에 쥐지 않는다(research R2). 정리(eviction)는 스트림 lock을 푼 뒤.
- **deliver 콜백**: `publish_run`의 `deliver`는 스트림 lock 안에서 불린다 — 막히지 않아야 하고 hub를 다시 호출하면 안 된다(research R6).
- **wire 동일성**: DTO 미러는 원본 serde 속성을 그대로 복사하고 `assert_wire_parity`(core `application/dto.rs`)로 모든 variant를 고정한다.
- **fixture**: `crates/workbench-protocol/fixtures/events/<case>.json`, 형식은 [contracts/workbench-events.md §7](./contracts/workbench-events.md#7-fixture-crates-workbench-protocol-fixtures-events-json). `{{epoch}}`는 test support가 치환.
- **테스트 전용 hook**은 cargo feature `test-hooks` 뒤에 둔다(대기열 크기·보관 run 수를 낮추는 설정 포함). 프로덕션 빌드 clippy(`cargo clippy -p workbench-core --lib`)도 clean이어야 한다.
- 커밋은 논리 단위마다 하되 사용자 지시 전에는 push·PR 하지 않는다.

---

## Phase 1: Setup (기준선과 의존성)

- [X] T001 기준선 기록: `cargo test --workspace --all-targets` 통과 수, `pnpm --filter agentic-workbench test` 통과 수, `pnpm run check-types` 결과를 tasks.md Notes에 적는다
- [X] T002 [P] 의존성: `crates/workbench-protocol/Cargo.toml`에 `futures-core`, `crates/workbench-core/Cargo.toml`에 `notify = "6"`·`futures-core`와 dev-dependency `tokio-tungstenite`·`futures-util`, dev `axum`에 `ws` feature. `cargo build -p workbench-core --all-targets` 통과
- [X] T003 [P] `crates/workbench-protocol/fixtures/events/` 디렉터리와 fixture 형식 README 한 단락(`crates/workbench-protocol/fixtures/events/README.md`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 모든 스토리가 쓰는 protocol 타입, hub 골격, 세대, 테스트 경로.

**⚠️ CRITICAL**: 이 Phase가 끝나기 전에 스토리 작업을 시작하지 않는다.

### protocol 타입 (research R1·R3·R4)

- [X] T004 `crates/workbench-protocol/src/workbench.rs`: `GapNotice {stream_id, epoch, reason, first_sequence?, last_sequence?}` + `GapReason`(unknownStream·evicted·epochChanged·retentionExceeded·subscriberLagged·shutdown), `EventItem { Event(EventEnvelope) | Gap(GapNotice) }`(serde `kind` 태그), `EventStream`을 `Pin<Box<dyn futures_core::Stream<Item = EventItem> + Send>>` 래퍼로 교체(`impl Stream`, `EventStream::new`). `events_unsupported`는 제거하지 않고 deprecated 주석. 단위 테스트: 직렬화 형태
- [X] T005 [P] `crates/workbench-protocol/src/principal.rs`: `Scope::RunRead`("run:read") 추가(`ALL` 14, `is_read` true), `desktop()`·`test_readonly()` 포함. 기존 테스트 기대 수 갱신
- [X] T006 [P] `crates/workbench-protocol/src/events/mod.rs`(신규): `EventClass {State, Notification}`, `EventSchemaSpec {schema, stream_kind, class, required_scopes, subscribable}`, `EVENT_SCHEMAS` 표(`run.event.v1` state `RunRead` 구독 가능, `worktree.changed.v1` notification `WorktreeRead` 구독 가능, `orchestration.workspaceUpdated.v1`·`exchange.requested.v1`·`exchange.status.v1` 구독 불가), `EventFrame`(hello·subscribe·event·gap·fault, serde `type` 태그), `StreamId::parse("<kind>:<key>")` → `StreamKind`. `lib.rs` 재노출
- [X] T007 `crates/workbench-protocol/src/descriptor.rs`: `DescribeOutput`에 `epoch: String`, `event_schemas: Vec<EventSchemaDescriptor>` 추가. `system.describe` handler(`crates/workbench-core/src/application/handlers/system/describe.rs`)가 principal scope로 거른 구독 가능 스키마를 채운다(epoch는 runtime에서 주입). describe fixture 2개(`crates/workbench-protocol/fixtures/system-describe-{desktop,readonly}.json`)에 `eventSchemas` 기대 추가 (depends T005, T006)

### hub 골격 (research R2·R5·R10)

- [X] T008 `crates/workbench-core/src/infrastructure/event_hub/mod.rs`(신규): `EventHub { epoch, streams, terminal_order, evicted, subscriptions, watchers }`와 상수(`RUN_JOURNAL_CAPACITY` 512, `MAX_RETAINED_RUNS` 256, `MAX_TOMBSTONES` 4,096, `SUBSCRIBER_QUEUE` 1,024, `MAX_SUBSCRIPTIONS` 256, `MAX_CURSORS` 64). 모듈 주석에 lock 순서 규칙. `test-hooks`로 상한을 낮추는 `EventHubLimits`
- [X] T009 `crates/workbench-core/src/infrastructure/event_hub/stream.rs`(신규): `StreamState {kind, class, sequence, journal, terminal, subscribers}`, `append`(journal 한도), `decide_cursor(cursor, epoch, evicted) -> CursorDecision {Replay(range) | Live | Gap(reason, bounds) | Ahead}` 순수 함수(research R2 표). 단위 테스트: 표의 모든 행 (depends T008)
- [X] T010 `crates/workbench-core/src/infrastructure/event_hub/subscription.rs`(신규): 구독 핸들(`mpsc::Sender<EventItem>`, overflow 플래그), `EventStream` 구현체(replay 목록 → 대기열 중 `sequence > high_water` → overflow면 `Gap(subscriberLagged)` 후 종료), `Drop`에서 등록된 스트림들에서 해제·동시 구독 수 감소 (depends T008)
- [X] T011 `crates/workbench-core/src/ports/event_publisher.rs`(신규): `RunEventPublisher::publish_run(&self, run_id, &RunEvent, terminal, deliver: &mut dyn FnMut(&EventEnvelope)) -> Option<EventEnvelope>`(제거된 run이면 `None`). `ports/mod.rs` 등록
- [X] T012 `crates/workbench-core/src/application/workbench_runtime.rs`: bootstrap에서 `uuid v4` epoch 생성, `EventHub` 소유(`events_hub()` 접근자), describe에 epoch 주입, `Workbench::events`를 hub `subscribe`로 위임. `RunEventPublisher`를 runtime에 구현. 단위 테스트 `events_are_unsupported_in_037`를 `events_reject_unknown_stream_kind`로 교체 (depends T008–T011)

### 테스트 경로 (research R11·R14)

- [X] T013 [P] `crates/workbench-core/tests/support/http_harness.rs`: `GET /v1/events` WebSocket(bearer 인증 → `hello` → 클라이언트 `subscribe` → `event`/`gap` 프레임, fault면 `fault` 후 close, 연결 종료 = 스트림 drop). 클라이언트 helper `Harness::subscribe(token, cursors) -> WsSubscription { next_frame() }` (depends T004, T006)
- [X] T014 [P] `crates/workbench-core/tests/support/event_fixtures.rs`(신규): fixture 로더(`publish`·`subscribe`·`publishAfter`·`expect.items`·`{{epoch}}` 치환), in-memory 실행기(`runtime.events` 소비)와 WS 실행기, 부분 일치 비교. test-hooks 한도 주입 (depends T012, T013)

**Checkpoint**: `cargo test -p workbench-protocol -p workbench-core` 통과(037·038 테스트 무수정), describe에 epoch·eventSchemas.

---

## Phase 3: User Story 1 - 이벤트를 구독하는 호출자가 재연결해도 이벤트를 잃거나 중복 반영하지 않는다 (Priority: P1) 🎯 MVP

**Goal**: hub 구독 계약(수신자 등록 → high-water → replay → drain, gap, 세대, 제거 표식, 한도)이 두 경로에서 같게 동작한다.

**Independent Test**: `event_subscription_race` 1,000회 누락·중복 0, `event_contract_suite` fixture 전부 in-memory·WS 일치, 교착 회귀 테스트 통과.

### Tests for User Story 1 ⚠️ (구현 전 작성, 실패 확인)

- [X] T015 [P] [US1] fixture(`crates/workbench-protocol/fixtures/events/`): `run-replay-from-start.json`, `run-replay-from-middle.json`, `run-replay-at-end.json`, `run-unknown-cursor-zero-waits.json`(구독 뒤 발행 → 1부터), `run-unknown-cursor-positive-gap.json`, `run-retention-exceeded-gap.json`(test-hooks 보관 8), `run-epoch-changed-gap.json`, `run-cursor-ahead-invalid.json`, `run-evicted-cursor-zero-gap.json`·`run-evicted-cursor-positive-gap.json`(test-hooks 보관 run 2), `run-evicted-while-subscribed-gap.json`, `run-late-publish-to-evicted-dropped.json`, `run-subscriber-lagged-gap.json`(test-hooks 대기열 4), `stream-kind-not-available.json`(orchestration), `stream-forbidden-without-scope.json`, `cursors-over-limit.json`, `subscriptions-over-limit-rate-limited.json`
- [X] T016 [P] [US1] `crates/workbench-core/tests/event_contract_suite.rs`(신규): 모든 events fixture를 in-memory와 WS로 실행하고 결과(아이템 목록·gap·fault 코드)를 서로 비교(SC-004)
- [X] T017 [P] [US1] `crates/workbench-core/tests/event_subscription_race.rs`(신규): 발행 thread가 `run:r1`에 연속 발행하는 동안 1,000회 임의 cursor k로 구독 → 각 구독의 처음 N개가 `k+1..` 연속·중복 없음(SC-001). 여러 스트림을 섞은 구독 변형 1개
- [X] T018 [P] [US1] `crates/workbench-core/tests/event_hub_deadlock.rs`(신규): 발행(terminal 포함 → 정리 유발)·구독·drop을 8 thread에서 5초간 반복, watchdog으로 진행 확인(lock 순서 회귀)

### Implementation for User Story 1

- [X] T019 [US1] `event_hub/mod.rs`: `subscribe(principal, cursors)` — cursor 수·동시 구독 수 검사(`invalidArgument`·`rateLimited`), kind 파싱·scope 검사(`forbidden`, 미지원 kind `invalidArgument` "stream kind is not available yet."), cursor마다 `streams`에서 찾거나(`after == 0`이면 빈 run 스트림 생성) 풀고 스트림 lock에서 등록 → high-water → `decide_cursor` → replay 복사. `Ahead`는 `invalidArgument`("cursor is ahead of the stream."). 반환 `EventStream` (depends T009, T010)
- [X] T020 [US1] `event_hub/mod.rs`: `publish_state(stream, schema, body, terminal, deliver)` 공통 경로 — 스트림 lock 안 순번·journal·`try_send`(실패 → overflow 표시·해제)·`deliver`, unlock 뒤 terminal 처리(`terminal_order` push → 보관 run 수 초과 시 `streams` → 대상 스트림 lock 순으로 제거, 구독자에 `Gap(evicted)`, `evicted` 기록·상한). 제거된 run id 발행은 버리고 `tracing`/`eprintln` 진단(core 기존 관례 확인) (depends T019)
- [X] T021 [US1] `event_hub/mod.rs`: `replay_run(run_id, after) -> RuntimeEventSnapshotDto` — 오늘 AW `RuntimeEventSnapshot` 형태(research R5 표, 제거된 run은 `terminal: true, gapDetected: true`). 형태 타입은 core `application`에 두고 AW가 그대로 직렬화 (depends T020)
- [X] T022 [US1] AW 단위 테스트였던 journal 테스트(`in_memory_runtime_event_journal.rs` tests)를 hub 단위 테스트로 이식(`event_hub/mod.rs` tests): run별 순번, 한도, gap 판정 (depends T021)
- [X] T023 [US1] T015–T018 통과, `cargo clippy -p workbench-core --all-targets -- -D warnings`. 커밋(`feat(workbench-core): implement Workbench.events with an event hub (039 US1)`)

**Checkpoint**: 구독 계약이 두 경로에서 성립. 아직 실제 발행자는 테스트뿐.

---

## Phase 4: User Story 2 - agent run 화면이 live와 replay를 같은 번호로 받아 추정 없이 복원한다 (Priority: P2)

**Goal**: run 이벤트가 hub를 거쳐 번호를 갖고, 데스크톱은 번호 순서대로 받으며, 화면은 재수화 중 live를 버퍼링한다.

**Independent Test**: `run_delivery_order` 1,000회, controller vitest 6건, 기존 run 테스트 무수정 통과, 데스크톱 payload 형태 단위 테스트.

### Tests for User Story 2 ⚠️ (구현 전 작성, 실패 확인)

- [ ] T024 [P] [US2] `crates/workbench-core/src/application/event_dto.rs`(신규) 테스트: `run_event_wire_parity` — acp-agent-core `RunEvent` 14 variant 전부(Lifecycle 10 status 포함)를 serde JSON과 `RunEventDto` JSON으로 비교
- [ ] T025 [P] [US2] `crates/workbench-core/tests/run_delivery_order.rs`(신규): 두 thread가 같은 run에 발행, `deliver`가 순번 11에서 50ms 멈추는 동안 다른 thread 발행 → 기록된 전달 순서 오름차순·빈틈 없음. 무작위 지연 1,000회
- [ ] T026 [P] [US2] `apps/agentic-workbench/src/features/agent-run/model/agent-run-controller.test.ts`: research R7 regression 6건(① loading 중 live 11 → snapshot 1–10 → 1–11 ② 역순 12·11 ③ 10 중복 ④ 13 빈틈 → gap ⑤ ready에서 12 → gap ⑥ 재수화 실패 → 버퍼 적용·runtimeLost)
- [ ] T027 [P] [US2] `apps/agentic-workbench/src-tauri/src/infrastructure/tauri_run_event_sink.rs` 단위 테스트: 삽입 payload JSON이 `{runId, event, sequence, epoch, streamId, eventId}`이고 `{runId, event}`가 이전과 바이트 동일(스크립트 생성 함수를 순수 함수로 분리해 테스트)

### Implementation for User Story 2

- [ ] T028 [P] [US2] `crates/workbench-protocol/src/events/run.rs`(신규): `RunEventDto` + 보조 DTO(`LifecycleStatus` 등 acp-agent-core `domain/events.rs`의 serde 속성 그대로). `utoipa::ToSchema`. T024 통과 (depends T006)
- [ ] T029 [US2] runtime `publish_run` 구현: `serde_json::to_value(RunEvent)` → hub `publish_state(run:<id>, "run.event.v1", body, terminal, deliver)`. terminal 판정은 오늘과 같음(`Lifecycle Completed|Cancelled`). T025 통과 (depends T020, T028)
- [ ] T030 [US2] `apps/agentic-workbench/src-tauri/src/infrastructure/tauri_run_event_sink.rs`: journal append·Tauri `emit`·`emit_to`·`target_label=None` 분기 제거, `app.state::<Arc<WorkbenchRuntime>>()`의 `publish_run(…, deliver)`에서 창 `eval`(창 없으면 생략). worktree guard 검증은 `publish_run` 뒤. T027 통과 (depends T029)
- [ ] T031 [US2] `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`: `replay_orchestration_runtime_events`가 runtime hub `replay_run` 사용(입출력 형태 불변). `lib.rs`의 `.manage(InMemoryRuntimeEventJournal)` 제거. 삭제: `infrastructure/in_memory_runtime_event_journal.rs`, `ports/runtime_event_journal.rs`(+ `mod.rs` 정리). `RuntimeEventSnapshot` TS 타입이 기대하는 JSON과 같은지 AW 단위 테스트 (depends T021)
- [ ] T032 [P] [US2] `apps/agentic-workbench/src/entities/agent-run/model/types.ts`: `DeliveredRunEvent = RunEventEnvelope & {sequence: number; epoch: string; streamId: string; eventId: string}`; `entities/agent-run/api/agent-run-repository.ts`: `listenRunEvents` 콜백 타입만 교체
- [ ] T033 [US2] `apps/agentic-workbench/src/features/agent-run/model/agent-run-controller.ts`: `pendingLive` 버퍼·drain·gap 규칙을 순수 함수로(`applyLiveRuntimeEvent`·`applyRuntimeSnapshot` 확장, `drainPendingLive` 신규), 버퍼 상한 512. T026 통과 (depends T032)
- [ ] T034 [US2] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-runtime-host.tsx`: `sequence: envelope.sequence`(추정 제거). `terminal` 계산 유지 (depends T033)
- [ ] T035 [US2] `cargo test -p workbench-core -p agentic-workbench`, `pnpm --filter agentic-workbench test`·`check-types`. 기존 run 테스트 무수정 통과. 커밋(`feat(aw): publish run events through the event hub with ordered, sequenced desktop delivery (039 US2)`)

**Checkpoint**: quickstart §3 1·2 수동 확인 가능.

---

## Phase 5: User Story 3 - worktree 파일 변경 알림이 구독으로 동작한다 (Priority: P3)

**Goal**: 감시는 core로 옮겨 실제 경로별 참조 수로 공유되고, 데스크톱 command 2개는 구독 task가 된다.

**Independent Test**: `worktree_stream` 참조 수·묶음 테스트, worktree fixture 두 경로 일치, 화면 코드 무변경.

### Tests for User Story 3 ⚠️ (구현 전 작성, 실패 확인)

- [ ] T036 [P] [US3] `crates/workbench-core/tests/worktree_stream.rs`(신규): 임시 git 저장소 구독 → 0.5초 안 파일 3개 변경 → 알림 1회(SC-005); 구독자 0→1→2→1→0에서 감시 시작 1회·중지 1회(hub 관측 hook `watcher_count()` test-hooks); 경로 표기 차이(끝 `/`·심볼릭 링크)가 같은 스트림; 없는 경로 → `notFound` 오늘 문구
- [ ] T037 [P] [US3] fixture `crates/workbench-protocol/fixtures/events/worktree-notification-debounced.json`, `worktree-two-subscribers.json`, `worktree-missing-path-not-found.json`, `worktree-cursor-ignored.json`(알림용은 cursor 무시)

### Implementation for User Story 3

- [X] T038 [US3] 이동: AW `infrastructure/fs_worktree_watcher.rs` → `crates/workbench-core/src/infrastructure/fs/worktree_watcher.rs`(`perf_log` → core `infrastructure::perf`, `WORKSPACE_EXCLUDED_DIRS` core 값 사용, 기존 단위 테스트 이동). `fs/mod.rs` 등록
- [ ] T039 [US3] hub worktree kind: 구독 시 `canonicalize`(실패 → `notFound` 오늘 문구), `watchers` 참조 수 증가·첫 구독이면 `watch_worktree(canonical, publish_notification)` 시작, 구독 drop에서 감소·0이면 handle drop. `publish_notification(worktree:<canonical>, "worktree.changed.v1", body)`는 journal 없음. T036·T037 통과 (depends T019, T038)
- [ ] T040 [P] [US3] `crates/workbench-protocol/src/events/worktree.rs`: `WorktreeChangedDto {workingDirectory, changedPath, kind: file|git}` + core wire parity(`WorktreeChangedEvent` ↔ DTO)
- [ ] T041 [US3] AW `inbound/tauri_commands.rs`: `start_worktree_watcher`가 blocking pool에서 `runtime.events(desktop, [worktree:<wd>])` → tauri async task로 스트림 소비, 이벤트 본문 `workingDirectory`를 호출자 문자열로 바꿔 `emit_to(label, WORKTREE_CHANGED_EVENT)`. `WorktreeWatcherState.handles: HashMap<label, JoinHandle>`(교체 시 abort), `stop_for_window`는 abort. fault는 message 문자열로. AW에서 `notify` 의존 제거(`Cargo.toml`) (depends T039)
- [ ] T042 [US3] `cargo test -p workbench-core -p agentic-workbench`, 프론트 worktree 코드 diff 0 확인. 커밋(`feat(workbench-core): serve worktree change notifications as a shared subscription stream (039 US3)`)

**Checkpoint**: quickstart §3 3·4 수동 확인 가능.

---

## Phase 6: User Story 4 - 이벤트 계약이 계약 조회·생성 타입에 포함되어 클라이언트가 실행 전에 검증한다 (Priority: P4)

**Goal**: 이벤트 스키마·프레임이 OpenAPI와 TS `EventMap`으로 생성되고 drift 검사가 잡는다.

**Independent Test**: describe fixture의 eventSchemas, test-d 상관 타입, drift 실증.

- [ ] T043 [P] [US4] `crates/workbench-protocol/src/events/orchestration.rs`: `OrchestrationEventDto {workspaceId, revision, reason, taskId?, nodeId?}`(2b 예약, 구독 불가) + AW 단위 테스트로 AW `OrchestrationEvent`와 wire parity
- [ ] T044 [US4] `crates/workbench-protocol/src/openapi.rs`: components에 `GapNotice`·`GapReason`·`EventItem`·`EventFrame`·`EventSchemaDescriptor`·`EventClass`·`RunEventDto`(+보조)·`WorktreeChangedDto`·`OrchestrationEventDto`, `EventBySchema` oneOf를 `EVENT_SCHEMAS`(본문 DTO가 있는 것)로 조립(variant마다 `schema` 단일값 enum + typed `body`). 골든 테스트 갱신 (depends T028, T040, T043)
- [ ] T045 [US4] `pnpm run generate:contracts`; `packages/workbench-client/src/operation-map.ts`에 `EventSchemaId`·`EventMap`·`EventFrame`·`GapNotice`·`EventEnvelope`·`RunEvent` alias, `index.ts` 재노출, `operation-map.test-d.ts`에 `EventMap["run.event.v1"]` 판별·worktree 본문 오용 `@ts-expect-error`·`EventFrame` 판별 (depends T044)
- [ ] T046 [US4] `pnpm --filter @yoophi/workbench-client check-types test`, drift 확인. 커밋(`feat(workbench-protocol): generate event schemas and EventMap (039 US4)`)

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T047 [P] `crates/workbench-core/tests/event_latency.rs`(#[ignore]): `publish_run`(deliver no-op) p95와 구독자 수신 p95 1,000회, 기준선 = 같은 `Value`를 AW 방식(Mutex+VecDeque append)으로 넣는 비용. 결과를 Notes에(SC-006)
- [ ] T048 [P] `docs/workbench-seam.md`: "이벤트 스트림" 절(봉투·구독 순서·cursor 판정 표·gap·세대·제거 표식·한도·분류·lock 순서), 데스크톱 전달(삽입 경로·순번 순서·재수화 버퍼링), worktree 구독 호환, Mermaid 갱신, 인벤토리 표에서 watcher 2개를 "이관됨(039)"로, ADR 4건 링크. `docs/client-server-architecture-research.md` 진행 각주에 "039(2a) 완료"
- [ ] T049 전체 게이트: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo clippy -p workbench-core --lib -- -D warnings`, `cargo test --workspace --all-targets`, `pnpm run check-types`, `pnpm run test`, `pnpm run generate:contracts && git status --short`(변경 없음), quickstart §2 경계 grep. 결과 Notes
- [ ] T050 앱 스모크(quickstart §3): 수행 가능한 항목은 수행, UI 조작 항목은 리뷰어 수동 항목으로 정직하게 기록
- [ ] T051 SC 증거 매핑(SC-001~008)과 spec·contract 대비 어긋난 점을 Notes에, PR 본문 초안(push·PR은 사용자 지시 후). 커밋(`docs(aw): record 039 event seam status`)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1**: 의존 없음. T002 ∥ T003.
- **Phase 2**: Phase 1 뒤. T004·T005·T006 병렬 → T007(T005·T006 뒤) · T008 → T009·T010(병렬) → T011 → T012 → T013·T014.
- **Phase 3 (US1)**: Phase 2 뒤. MVP이자 US2·US3의 전제(hub 구독·발행 경로).
- **Phase 4 (US2)**: US1 뒤(T020·T021 사용).
- **Phase 5 (US3)**: US1 뒤(T019 사용). US2와 독립이지만 `tauri_commands.rs`·`event_hub/mod.rs` 공유 파일은 순차 편집.
- **Phase 6 (US4)**: T028·T040 뒤. US2·US3 끝나고 하는 것이 자연스럽다.
- **Phase 7**: 전부 뒤.

### User Story Dependencies

- US1(P1): Foundational만.
- US2(P2): US1(hub 발행·replay).
- US3(P3): US1(hub 구독). US2와 독립.
- US4(P4): US2의 `RunEventDto`, US3의 `WorktreeChangedDto`.

### Within Each User Story

- 테스트(fixture·race·순서·vitest) 먼저 → protocol DTO → hub/runtime → AW 어댑터 → 프론트(US2만) → 게이트·커밋.

### Parallel Opportunities

- Phase 2: T004 ∥ T005 ∥ T006; T009 ∥ T010; T013 ∥ T014(각각 선행 뒤)
- US1: T015 ∥ T016 ∥ T017 ∥ T018
- US2: T024 ∥ T025 ∥ T026 ∥ T027; T028 ∥ T032
- US3: T036 ∥ T037; T040
- Polish: T047 ∥ T048

---

## Parallel Example: User Story 1

```text
T015 fixture 17개
T016 event_contract_suite.rs
T017 event_subscription_race.rs
T018 event_hub_deadlock.rs
```

## Parallel Example: User Story 2

```text
T024 run_event_wire_parity   (crates/workbench-core/src/application/event_dto.rs)
T025 run_delivery_order.rs   (crates/workbench-core/tests)
T026 agent-run-controller.test.ts (apps/agentic-workbench/src/features/agent-run/model)
T027 sink payload 단위 테스트 (apps/agentic-workbench/src-tauri/src/infrastructure)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 → Phase 2(describe epoch·eventSchemas, hub 골격, WS 하네스)
2. Phase 3(US1) → race·deadlock·fixture 통과
3. **STOP and VALIDATE**: 구독 계약이 두 경로에서 같고 누락·중복 0

### Incremental Delivery

1. US1 → 커밋
2. US2 → 커밋(데스크톱 run 순번·재수화 버퍼링 — 사용자 가치 첫 등장)
3. US3 → 커밋(watcher command 2개 이관)
4. US4 → 커밋(생성 계약)
5. Polish → 게이트·스모크·PR 초안

---

## Notes

- [P] = 다른 파일, 미완료 의존 없음
- 커밋은 논리 단위마다(각 US 끝, 문서). push·PR은 사용자 지시 후.
- 각 checkpoint에서 멈춰 스토리를 독립 검증한다.
- 기준선·실측 기록 (T001, T047, T049, T050, T051):
  - (T001, 2026-09-27) 기준선: `cargo test --workspace --all-targets` 567 passed / 0 failed / 5 ignored, `pnpm --filter agentic-workbench test` 81 files / 423 tests, `pnpm run check-types` 13/13.
  - (Foundational·US1 구현 중 결정) `WorkbenchFault::events_unsupported`는 deprecated로 남기지 않고 제거했다(사용처가 037 runtime 하나뿐이고 계약이 뒤집혔다). cursor `afterSequence == 0`은 **세대를 보지 않는다** — "처음부터"는 어느 세대에서든 의미가 있다(contracts §4에 반영 필요). 존재하는 스트림을 cursor 0으로 구독했는데 앞부분이 보관 한도로 사라졌으면 `Gap(retentionExceeded)`. 정리(eviction)와 경합해 고아가 된 스트림은 `removed` 표시로 발행·구독을 거절한다. 대기열이 넘치면 그 구독 전체를 `Gap(subscriberLagged)`로 즉시 닫는다(남은 대기열도 버림 — 빠진 이벤트 뒤의 것은 신뢰하지 않음). hub 한도는 `RuntimeAdapters.event_limits`로 주입한다(테스트가 낮춤). worktree 감시 이동(T038)은 runtime이 `start_watch`를 주입해야 해서 Foundational로 앞당겼다(AW 쪽 삭제·command 교체는 T041 그대로). describe fixture 2개는 runtime마다 세대가 달라 `ignoreFields: ["epoch"]`. 테스트 WebSocket은 세대를 `system.describe`로 얻는다(Workbench trait만 사용). WS 경로에서 대기열 초과는 재현할 수 없어(서버가 동시에 비움) `run-subscriber-lagged-gap`은 `inMemoryOnly`. fixture 21개, race 1,000회 + 다중 스트림 100회, deadlock watchdog 3초 통과. 체커가 틀린 순번·빠진 끝을 잡는지 일부러 깨뜨려 확인했다.

