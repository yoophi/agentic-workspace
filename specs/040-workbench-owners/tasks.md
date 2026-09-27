---

description: "Task list for introducing benches and migrating run/exchange commands (stage 2b-1)"
---

# Tasks: 작업대(Bench) 도입과 run·교환 command 이관 (서버-클라이언트 전환 2b-1)

**Input**: Design documents from `/specs/040-workbench-owners/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/workbench-benches.md](./contracts/workbench-benches.md), [contracts/tauri-compat.md](./contracts/tauri-compat.md), [quickstart.md](./quickstart.md)

**Tests**: spec 성공 기준(SC-003 두 경로 일치, SC-004 교차 거절, SC-005 창 격리, SC-006 닫기·연결 끊김)과 Codex 리뷰(닫기 경합, 멱등 보존)가 테스트를 요구한다. 각 스토리에서 테스트를 먼저 쓰고 실패를 확인한 뒤 구현한다. 037–039 테스트는 **기대값 수정 없이** 계속 통과해야 한다(principal·scope 수를 세는 테스트, describe fixture처럼 계약 확장으로 값이 바뀌는 것은 예외로 Notes에 기록).

**Organization**: Foundational(작업대·principal 주체·`RunEngine`·데스크톱 포트·세대 멱등·fixture `steps`)이 모든 스토리의 기반이다. US1(run 8개)이 MVP, US2(교환 4개 + MCP 교환 도구), US3(제목), US4(계약 생성)가 뒤따른다.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 실행 가능 (다른 파일, 미완료 의존 없음)
- **[Story]**: 해당 사용자 스토리 (US1–US4)

## Path Conventions

- **Reusable Rust**: `crates/workbench-protocol/src`, `crates/workbench-core/src`, `crates/workbench-core/tests`, `crates/workbench-protocol/fixtures`
- **App Tauri backend**: `apps/agentic-workbench/src-tauri/src/{inbound,infrastructure,application,domain,ports}`
- **App frontend**: 변경 없음 목표(`apps/agentic-workbench/src`)
- **Reusable TypeScript**: `packages/workbench-client/src`
- **Documentation**: `docs/*.md`

워크트리는 `/Users/yoophi/project/worktrees/040-workbench-owners`. `crates/acp-agent-core`·`packages/agent-client`는 변경하지 않는다.

### 공통 규칙

- **입장 경계**(research R1): 작업대 아래 새 자원을 등록하는 동작(`run.start` 소유 기록까지, 교환 쓰기, 과도기 orchestration 기동)만 입장권(`admission` read guard)을 잡는다. registry `std::sync::Mutex` 안에서 `Open` 확인 + `try_read_owned()`, lock 밖에서 await. 닫기는 `Open → Closing`(lock 안) → write guard 대기 → 소유 run 취소 → 교환 삭제 → 스트림 제거 → 삭제.
- **lock 순서**: bench registry → (해제 후) admission → hub(039 순서). hub `deliver`는 스트림 lock 안 — `DesktopBridge::deliver`는 막히지 않아야 한다(`set_title`처럼 메인 스레드로 가는 일은 async task로).
- **오류 문구**: 오늘 문구 유지(contracts 표). 새 문구는 contracts에 적힌 것만.
- **wire 동일성**: protocol DTO 미러는 원본 serde 속성을 복사하고 core/AW 단위 테스트로 모든 variant를 고정한다.
- **fixture**: call fixture는 `crates/workbench-protocol/fixtures/<case>.json`(`steps`·`capture`·`runScript` 확장, research R13), 이벤트 fixture는 `fixtures/events/`. 생성 스크립트를 쓰면 세션 scratchpad에 둔다.
- 커밋은 논리 단위마다 하되 사용자 지시 전에는 push·PR 하지 않는다.

---

## Phase 1: Setup

- [X] T001 기준선 기록: `cargo test --workspace --all-targets` 통과 수, `pnpm run test`의 agentic-workbench·workbench-client 수, `pnpm run check-types`를 Notes에
- [X] T002 [P] 오늘 동작 캡처(회귀 비교용): AW 교환·run command의 오류 문자열 목록을 `specs/040-workbench-owners/contracts/workbench-benches.md` 표와 대조해 누락이 없는지 확인하고 차이를 Notes에

---

## Phase 2: Foundational (Blocking Prerequisites)

**⚠️ CRITICAL**: 이 Phase가 끝나기 전에 스토리 작업을 시작하지 않는다.

### protocol (research R2·R7, contracts)

- [X] T003 `crates/workbench-protocol/src/principal.rs`: `PrincipalKind::Agent`(문자열 `"agent"`), `AuthenticatedPrincipal.subject: PrincipalSubject`, 생성자 `desktop()`(subject `desktop`)·`test_readonly()`(`test:readonly`)·`test_as(name)`(데스크톱과 같은 scope, `test:<name>`)·`agent(run_id)`(`agent:<runId>`, scope `exchange:read`·`exchange:write`·`presentation:write`)·`agent_run_id()`. 새 scope 6개(`run:write`, `bench:read`, `bench:write`, `exchange:read`, `exchange:write`, `presentation:write`), `ALL` 20, readonly 11. 단위 테스트 갱신
- [X] T004 `crates/workbench-core/src/infrastructure/sqlite_ledger.rs`: `principal_kind` 파싱에 `"agent"` 추가(쓰기는 기존). 단위 테스트
- [X] T005 [P] `crates/workbench-protocol/src/descriptor.rs`·`operations/mod.rs`: `IdempotencyScope{Durable, Epoch}`, `OperationSpec.idempotency_scope: Option<…>`(command만), descriptor `idempotencyScope`. 기존 command 전부 `Durable`. describe fixture 기대값 갱신(Notes)
- [X] T006 [P] `crates/workbench-protocol/src/operations/bench.rs`(신규): `BenchOpenInput{workingDirectory}`/`BenchOpenOutput{benchId, workingDirectory}`, `BenchCloseInput{benchId}`/`BenchCloseOutput{closed, cancelledRuns}`, `BenchRequestTitleInput{runId, title}`/`TitleChangeResultDto`. `OperationId::{BenchOpen, BenchClose, BenchRequestTitle}`, `OPERATIONS` 항목(scope·epoch)
- [X] T007 [P] `crates/workbench-protocol/src/events/mod.rs`: `StreamKind::Bench`(`bench:<id>`, 알림용, `bench:read`), `StreamKind::Exchange` 구독 가능(`exchange:read`), `BENCH_TITLE_REQUESTED_V1`, `EVENT_SCHEMAS`에 추가(body_schema는 US2·US3에서 채움). 기존 `stream-kind-not-available` fixture는 `orchestration:`으로 유지되는지 확인

### core 작업대 (research R1)

- [X] T008 `crates/workbench-core/src/infrastructure/bench/in_memory_bench_registry.rs`(신규) + `mod.rs`: `Bench{id, working_directory, opened_by, opened_at, state, admission: Arc<RwLock<()>>}`, `BenchLimits{max_benches: 256}`, `open`, `admit(id, subject) -> Result<BenchAdmission, BenchError>`(lock 안 `Open` 확인 + `try_read_owned`), `begin_close(id, subject) -> CloseTicket | AlreadyClosing(wait) | Unknown`, `finish_close`. 단위 테스트: 주체 불일치·상한·`Closing` 중 입장 거절·두 번째 닫기 대기
- [X] T009 `crates/workbench-core/src/application/bench_service.rs`(신규): `open`(canonicalize, 오늘 문구), `close`(write guard 대기 → `RunEngine::cancel_runs_owned_by` → 교환 삭제 hook → hub `remove_stream` 2개 → `finish_close`), `resolve(benchId, principal)`(공통 검사 문구 `"bench not found."`/`"bench belongs to another principal."`)
- [X] T010 `crates/workbench-core/src/infrastructure/event_hub/mod.rs`: `remove_stream(kind, key)`(구독자 `Gap(evicted)` + 상태 복원용이면 제거 표식), `publish_notification`에 `deliver` 인자(구독자 없어도 호출; worktree 호출부는 no-op 전달), 교환용 `EventHubLimits.exchange_journal_capacity = 512`(보관 run 수 계산 제외). 039 테스트 수정 없이 통과 + 새 단위 테스트

### core run 기계 포트 (research R3·R4)

- [X] T011 `crates/workbench-core/src/ports/run_engine.rs`(신규): 객체 안전 `RunEngine`(`start`, `send_prompt`, `steer_prompt`, `cancel_current_prompt_and_send`, `set_permission_mode`, `cancel`, `respond_permission`, `owner_of`, `active_owner_of`, `cancel_runs_owned_by`, `acp_registry`, `acp_session_store`)과 `RunEngineError{kind, message}`(문구 보존), `RunSink` 타입
- [X] T012 [P] `crates/workbench-core/src/ports/desktop_bridge.rs`(신규): `DesktopBridge::deliver(DesktopDelivery)`(`Run`·`ExchangeRequested`·`ExchangeStatus`·`TitleRequested`), `RunTerminalHook::on_terminal(run_id)`, `RunLaunchDecorator::decorate(&mut AgentRunRequest, LaunchContext) -> Result<(), String>`
- [X] T013 `crates/workbench-core/src/infrastructure/fs/acp_session_store.rs`: AW `infrastructure/json_acp_session_store.rs`를 이동(`from_app` 제거, `DataPaths`로 경로), 기존 단위 테스트 이동
- [X] T014 `crates/workbench-core/src/infrastructure/run/workbench_run_sink.rs`(신규): acp `RunEventSink` 구현 — `publish_run(... deliver = desktop.deliver(Run{bench, delivered_payload}))`, 종료 이벤트면 lock 밖에서 `terminal_hook.on_terminal`. payload 형태는 039 `delivered_payload`와 같음(단위 테스트로 고정)
- [X] T015 `crates/workbench-core/src/infrastructure/run/acp_run_engine.rs`(신규): `AppState` + `AcpAgentRunner` + `JsonAcpSessionStore`로 `RunEngine` 구현(기존 유스케이스 호출, 권한은 `PermissionBroker::respond_for_run`), 소유자는 `BenchId` 문자열
- [X] T016 `crates/workbench-core/src/application/workbench_runtime.rs`: `RuntimeAdapters`에 `run_engine: Option<Arc<dyn RunEngine>>`(None → bootstrap이 `AcpRunEngine`), `desktop`, `terminal_hook`, `launch_decorator`(없으면 no-op), `bench_limits`. 필드 `benches`, 접근자 `run_engine()`·`run_sink(bench_id)`·`admit(bench_id)`(과도기, doc에 "041에서 제거"). `stub_adapters` 갱신

### 세대 범위 멱등 (research R7)

- [X] T017 `crates/workbench-core/src/application/epoch_idempotency.rs`(신규): 작업대별 결과 기록 1,024 → 요약 강등, 요약 65,536 → 새 command `rateLimited`, `bench.open` 주체별 기록, 키별 in-flight 대기, 다른 payload `conflict`, 요약 적중 `conflict(applied)` 문구. 작업대 닫기 시 폐기 hook. 단위 테스트(각 분기)
- [X] T018 `crates/workbench-core/src/application/registry.rs`·`workbench_runtime.rs`: dispatch가 `idempotency_scope == Epoch`인 command를 `epoch_idempotency`로 감싸도록 연결(키 필수 검사는 기존)

### 테스트 지원 (research R13)

- [X] T019 `crates/workbench-core/tests/support/scripted_run_engine.rs`(신규): 메모리 run 슬롯·소유·권한 대기, `RunScript`(이벤트 목록·권한 요청·실패 주입·`start` 지연), 호출 기록(엔진 호출 수)
- [X] T020 [P] `crates/workbench-core/tests/support/recording_desktop.rs`(신규): `DesktopBridge`·`RunTerminalHook`·`RunLaunchDecorator` 기록형 가짜
- [X] T021 `crates/workbench-core/tests/support/`(contract suite 로더): call fixture `steps`(순차 호출), `capture`(JSON pointer → `{{name}}`), principal `desktop`·`readonly`·`desktop2`(`test_as("desktop2")`)·`agent:{{run}}`, `runScript`. in-memory·HTTP 두 경로 모두 지원. 로더 자체 단위 테스트

### 작업대 operation (Foundational에서 먼저 여는 이유: 모든 스토리의 전제)

- [X] T022 `crates/workbench-core/src/application/handlers/bench/`(신규): `bench.open`·`bench.close` handler, `build_registry` 등록
- [X] T023 [P] fixture `bench-open-ok`, `bench-open-missing-path`, `bench-open-not-directory`, `bench-open-readonly-forbidden`, `bench-close-unknown-ok`, `bench-close-other-principal-forbidden`, `bench-close-twice-ok`, `bench-limit-rate-limited`(`crates/workbench-protocol/fixtures/`)
- [X] T024 `crates/workbench-core/tests/bench_close_race.rs`(신규, 이 시점엔 `admit`만): 입장 guard 보유 중 닫기가 기다리는지, `Closing` 중 입장 `notFound`, 닫기 두 개 겹침. `cargo test -p workbench-core` 통과
- [X] T025 커밋 `feat(workbench-core): introduce benches, principal subjects, run engine port and epoch idempotency (040 foundation)`

**Checkpoint**: 작업대 열기·닫기가 두 경로에서 동작하고, 가짜 엔진·데스크톱으로 run·교환 서비스를 붙일 수 있다.

---

## Phase 3: User Story 1 - run의 소유와 제어가 작업대로 판단된다 (Priority: P1) 🎯 MVP

**Goal**: run command 8개를 operation으로 이관하고 모든 제어에 작업대 소유 검사.

**Independent Test**: 작업대 열기 → 시작 → 권한 → 프롬프트 → 닫기 fixture 두 경로 일치, 교차 작업대·교차 주체 거절, 데스크톱 run 동작 불변.

### Tests for User Story 1 ⚠️ (먼저 작성, 실패 확인)

- [ ] T026 [P] [US1] fixture: `run-start-ok`(`steps`: open → start, `AgentRun` 반환), `run-start-idempotent-replay`, `run-start-key-conflict`, `run-start-concurrent-limit`, `run-start-readonly-forbidden`, `run-send-prompt-ok`, `run-send-prompt-empty`, `run-send-prompt-inactive`, `run-send-prompt-other-bench-forbidden`, `run-steer-unsupported`, `run-cancel-and-send-other-bench-forbidden`, `run-set-permission-mode-other-bench-forbidden`, `run-cancel-finished-ok`, `run-cancel-other-bench-forbidden`, `run-respond-permission-ok`, `run-respond-permission-non-owner`(오늘 문구), `run-respond-permission-unknown-run`, `run-list-tool-candidates-non-owner`, `run-bench-other-principal-forbidden`, `run-epoch-retry-same-result`
- [ ] T027 [P] [US1] `crates/workbench-core/tests/bench_run_flow.rs`(신규): 수명 흐름, 닫기 → 소유 run 전부 취소·`cancelledRuns`, 구독 끊김 후 run 유지(SC-006), run 이벤트가 `RecordingDesktop`에 작업대 단위로 순번 순서 전달
- [ ] T028 [P] [US1] `crates/workbench-core/tests/run_start_reconcile.rs`(신규): `run.start` apply 중단 → 재기동 → `unknown`, 완료된 `run.start` 재생은 저장 결과
- [ ] T029 [P] [US1] `crates/workbench-core/tests/bench_close_race.rs` 확장: `run.start`(가짜 엔진 `start` 지연)와 `bench.close` 동시 1,000회 — 닫기 반환 뒤 그 작업대 소유 run 0개, start는 성공(닫기 전) 또는 `notFound`; `cancelAndSend` 진행 중에도 닫기가 막히지 않음
- [ ] T030 [P] [US1] `crates/workbench-core/tests/epoch_idempotency.rs`(신규): 결과 1,024 초과 뒤 첫 키 재시도 → 엔진 호출 수 불변 + `conflict(applied)`, 요약 한도(테스트 한도 낮춤) → `rateLimited`, 닫은 뒤 재시도 `notFound`, 동시 같은 키 한 번 실행

### Implementation for User Story 1

- [ ] T031 [P] [US1] `crates/workbench-protocol/src/operations/run.rs`(신규): 입력 8개(`benchId` 포함), DTO 미러 `AgentRunRequestDto`·`AgentRunDto`·`AgentToolCandidateQueryDto`·`AgentToolCandidateResponseDto`·`PermissionModeDto`(기존 것 재사용 가능하면 재사용), `OperationId` 8개, `OPERATIONS`(`run.start` durable, 나머지 epoch, `run.listToolCandidates` query)
- [ ] T032 [US1] `crates/workbench-core/src/application/run_service.rs`(신규): operation별 로직 — 작업대 resolve, 소유 검사(`owner_of`, 새 문구 `"run is owned by another bench."`, 유지 문구 2종), `run.start` 입장권 + decorator + 엔진 `start`, 엔진 오류 → fault 매핑(contracts 표)
- [ ] T033 [US1] `crates/workbench-core/src/application/handlers/run/`(신규) + `reconcilers/run_start.rs`: `run.start`는 `MutationSpec`(aggregate `run:<runId>`, 서버가 run id 확정, reservation 배타, 결과 `AgentRun`), reconciler `pending → unknown`. 나머지는 epoch handler. `build_registry` 등록
- [ ] T034 [US1] `crates/workbench-core/src/application/dto.rs`(또는 `run_dto.rs`): 원본 ↔ DTO wire parity 테스트(모든 variant)
- [ ] T035 [US1] AW `apps/agentic-workbench/src-tauri/src/infrastructure/desktop_benches.rs`(신규): `DesktopBenches{by_label, by_bench, closed_labels}` + label별 lock, `ensure(label, hint)`(`window_manager` 경로 → hint), `lookup(label)`, `close(label)`(닫힌 표시 → `bench.close` → 대응 제거). 단위 테스트: single-flight, ensure 중 close 경합, 닫힌 창 ensure 실패
- [ ] T036 [US1] AW `infrastructure/tauri_desktop_bridge.rs`(신규): `DesktopBridge`(run → `agent-run-event-fallback` 삽입, `by_bench`로 창 찾기), `RunTerminalHook`(worktree 가드 + orchestration `fail_task_for_runtime` — 기존 `tauri_run_event_sink.rs` 로직 이동), `RunLaunchDecorator`(Main Coordinator principal 해석 + MCP 토큰·env 주입, `OnceLock<McpServerState>`)
- [ ] T037 [US1] AW `lib.rs`: `AppState::default()`·`.manage(app_state)` 제거, 런타임을 `RuntimeAdapters`(desktop·hook·decorator)로 bootstrap, `DesktopBenches` manage, MCP 시작 후 decorator에 bind, 창 `Destroyed` → `DesktopBenches::close(label)` → orchestration `release_window(label)`(그대로)
- [ ] T038 [US1] AW `inbound/tauri_commands.rs`·`workbench_compat.rs`: run command 8개를 compat으로(contracts/tauri-compat 표: `ensure` vs 조회만, 오늘 반환·오류). `start_agent_run`의 principal·MCP·settings 조립 코드 제거(decorator·core로 이동)
- [ ] T039 [US1] AW 과도기 orchestration(research R12): `infrastructure/acp_agent_worker_adapter.rs`·`tauri_commands.rs`의 orchestration 경로가 `AppState`·세션 저장소를 런타임 접근자에서 얻고, 자식·Main 위임 run 소유자를 `DesktopBenches::ensure`의 `BenchId`로, sink를 `runtime.run_sink(bench)`로, 기동을 `runtime.admit(bench)` guard 안에서. orchestration 테스트(`tests/orchestration_delegation.rs`) 수정 없이 통과 확인
- [ ] T040 [US1] AW 삭제: `infrastructure/tauri_run_event_sink.rs`, `infrastructure/json_acp_session_store.rs`, `application/agent_tool_candidate_service.rs`(core로 이동한 로직 확인 후). 남은 `AppState` 직접 참조가 과도기 경로뿐인지 grep
- [ ] T041 [US1] AW compat 단위 테스트: run 오류 문자열(유지 2종·새 1종), `cancel_agent_run` 작업대 없음 `Ok`, `respond_agent_permission` 작업대 없음 `"unknown or finished run: …"`
- [ ] T042 [US1] 게이트 `cargo test -p workbench-protocol -p workbench-core -p agentic-workbench`, clippy. 커밋 `feat(aw): migrate run commands behind benches with ownership checks (040 US1)`

**Checkpoint**: quickstart §3의 1·4·5 수동 확인 가능.

---

## Phase 4: User Story 2 - 교환이 작업대 기준으로 동작하고 그 작업대에만 전달된다 (Priority: P2)

**Goal**: 교환 command 4개 + MCP 교환 도구 3개 이관, 교환 스트림 개방, 네이티브 방송 제거.

**Independent Test**: 교환 fixture 두 경로, 두 작업대 격리, 확인 멱등, agent principal 경로.

### Tests for User Story 2 ⚠️

- [ ] T043 [P] [US2] fixture: `exchange-sync-ok`, `exchange-sync-invalid-panels`, `exchange-sync-stale-source-run`, `exchange-sync-lower-revision-ignored`, `exchange-send-ok`, `exchange-send-duplicate-same`, `exchange-send-duplicate-conflict`, `exchange-send-target-closing`, `exchange-send-message-too-large`, `exchange-ack-ok`, `exchange-ack-twice-same`(이벤트 1회), `exchange-ack-invalid-transition`, `exchange-list-ok`, `exchange-list-peers-agent`, `exchange-list-peers-desktop-forbidden`, `exchange-send-from-run-agent`, `exchange-send-from-run-other-run-forbidden`, `exchange-get-for-run-agent`, `exchange-other-bench-forbidden`. 오류 fixture는 `details.exchangeCode`까지 검사
- [ ] T044 [P] [US2] 이벤트 fixture `crates/workbench-protocol/fixtures/events/`: `exchange-stream-requested-then-status`, `exchange-stream-bench-closed-gap`, `exchange-stream-closed-bench-cursor-zero-gap`, `exchange-stream-readonly-can-subscribe`
- [ ] T045 [P] [US2] `crates/workbench-core/tests/exchange_flow.rs`(신규): 두 작업대 격리(`RecordingDesktop`에 다른 작업대 전달 0), 확인 두 번 → 상태 이벤트 1회, 작업대 닫기 → 작업 영역·이력 삭제, `exchange.send`·`bench.close` 경합(입장 경계)

### Implementation for User Story 2

- [ ] T046 [US2] core로 이동: AW `domain/agent_exchange.rs` → `crates/workbench-core/src/domain/agent_exchange.rs`(또는 `application/exchange/domain.rs`), `application/agent_exchange_service.rs` → `application/exchange/service.rs`, `ports/agent_workspace_registry.rs` → core ports, `infrastructure/in_memory_agent_workspace_registry.rs` → `infrastructure/exchange/`. 키 `BenchId`, `AgentExchange.window_label` 제거, 소유 조회 = `RunEngine::active_owner_of`, 발행 = hub `exchange:<bench>` + `DesktopBridge`(`ExchangeRequested`·`ExchangeStatus`), 같은 결과 재확인은 이벤트 없음. 기존 단위 테스트 이동·갱신
- [ ] T047 [P] [US2] `crates/workbench-protocol/src/operations/exchange.rs`·`events/exchange.rs`(신규): 입력 7개, DTO(`AgentWorkspaceSyncRequestDto`, `SendAgentExchangeRequestDto`, `AgentExchangeAckRequestDto`, `AgentExchangeDto`, `ExchangeRequestedDto`, `AgentPanelEndpointDto` …), `OperationId` 7개, `EVENT_SCHEMAS` body_schema, wire parity 테스트
- [ ] T048 [US2] `crates/workbench-core/src/application/handlers/exchange/`(신규): handler 7개(쓰기 3개 입장권), agent 전용 3개는 `principal.agent_run_id() == runId` 검사, 도메인 오류 → fault(`details.exchangeCode`, 코드 매핑표)
- [ ] T049 [US2] AW compat: `sync/send/acknowledge/list` command → operation(`ensure` vs 조회만), 오류를 `{"code","message"}` JSON 문자열로 재구성(오늘 `exchange_error`와 바이트 동일, 단위 테스트), `TauriDesktopBridge`에 교환 삽입(`agent-exchange-*-fallback`, 오늘 payload + 순번 필드), 네이티브 `emit` 제거
- [ ] T050 [US2] AW MCP: `infrastructure/mcp/mod.rs`에서 토큰 → `AuthenticatedPrincipal::agent(run_id)`, `agent_exchange_tool.rs`의 3개 도구를 `Workbench.call`로(도구 쪽 run 일치 검사·문구 유지, fault → `structuredContent{code,message}`). `McpServerState`가 `InMemoryAgentWorkspaceRegistry`·`AppState` 대신 런타임을 받도록
- [ ] T051 [US2] AW 삭제: 이동한 교환 파일 4개, `lib.rs`의 교환 registry manage. 교환 단위 테스트가 core에서 통과
- [ ] T052 [US2] 게이트·커밋 `feat(aw): migrate agent exchange behind benches with an exchange stream (040 US2)`

**Checkpoint**: quickstart §3의 2 수동 확인 가능.

---

## Phase 5: User Story 3 - agent의 창 제목 요청이 그 작업대의 창에만 적용된다 (Priority: P3)

### Tests for User Story 3 ⚠️

- [ ] T053 [P] [US3] fixture: `bench-request-title-ok`, `bench-request-title-too-long`(오늘 문구), `bench-request-title-inactive-run`, `bench-request-title-other-run-forbidden`, `bench-request-title-desktop-forbidden`; 이벤트 fixture `bench-stream-title-requested`, `bench-stream-no-replay`(알림용)

### Implementation for User Story 3

- [ ] T054 [US3] `crates/workbench-protocol/src/events/bench.rs`(신규) `TitleRequestedDto{title}` + body_schema; core handler `bench.requestTitle`(AW `application/mcp_title_control_service.rs`·`domain/mcp_title_control.rs`의 검증을 core로 이동, 기존 테스트 이동) → `publish_notification(bench:<id>, deliver = TitleRequested)`
- [ ] T055 [US3] AW: `TauriDesktopBridge`의 `TitleRequested` → async task에서 `set_title` + `sync_window_menu` + `mcp-window-title-fallback` 삽입(네이티브 `emit` 제거); MCP `set_window_title` 도구 → `Workbench.call(bench.requestTitle)`, 결과를 `TitleChangeResult`로. 이동한 AW 파일 삭제
- [ ] T056 [US3] 게이트·커밋 `feat(aw): deliver agent title requests through the bench notification stream (040 US3)`

**Checkpoint**: quickstart §3의 3 수동 확인 가능.

---

## Phase 6: User Story 4 - 계약 조회·생성 타입에 포함 (Priority: P4)

- [ ] T057 [US4] `crates/workbench-protocol/src/openapi.rs`: 새 operation 18개 component·variant, 이벤트 DTO와 `EventBySchema` 3개 variant, golden 테스트 갱신
- [ ] T058 [US4] describe fixture: 데스크톱 50개, readonly 조회 operation만, agent principal은 agent 전용 3개·교환 조회·`bench.requestTitle`만, `idempotencyScope` 표시, `eventSchemas` 추가분
- [ ] T059 [US4] `pnpm run generate:contracts`; `packages/workbench-client/src/operation-map.ts`·`index.ts`에 alias(`Bench*`, `AgentRun`, `AgentExchange`, `EventMap` 키 추가), `operation-map.test-d.ts`에 `OperationMap["run.start"]`·`EventMap["exchange.status.v1"]` 판별·오용 `@ts-expect-error`, operation id 50개
- [ ] T060 [US4] `pnpm --filter @yoophi/workbench-client check-types test`, drift 확인. 커밋 `feat(workbench-protocol): generate bench, run and exchange contracts (040 US4)`

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T061 [P] `crates/workbench-core/tests/run_latency.rs`(#[ignore]): `run.sendPrompt` 경유 p95 증가(가짜 엔진 대비 직접 호출) — 결과 Notes(R14)
- [ ] T062 [P] `docs/workbench-seam.md`: 범위·인벤토리(이관 45, 이연 18, 유지 8), "작업대" 절(수명·입장 경계·주체·데스크톱 대응, Mermaid), 이벤트 스트림 절에 교환·작업대 스트림, MCP agent principal 절, ADR 5건 링크, 2단계 안내(041). `docs/client-server-architecture-research.md` 진행 각주 "040(2b-1) 완료"
- [ ] T063 전체 게이트(quickstart §1·§2): fmt, clippy `-D warnings`(workspace·core lib), `cargo test --workspace --all-targets`, `pnpm run check-types`, `pnpm run test`, 생성물 drift 0, 경계 grep(acp-agent-core·agent-client diff 0, 화면 diff 0, core·protocol의 창 label 0). 결과 Notes
- [ ] T064 앱 스모크(quickstart §3): 수행 가능한 항목은 수행, UI 조작 항목은 리뷰어 수동 항목으로 정직하게 기록
- [ ] T065 SC 증거 매핑(SC-001~008)과 spec·contract 대비 어긋난 점을 Notes에, PR 본문 초안(push·PR은 사용자 지시 후). 커밋 `docs(aw): record 040 bench migration status`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 바로 시작.
- **Foundational (Phase 2)**: Setup 뒤. 모든 스토리를 막는다.
- **US1 (Phase 3)**: Foundational 뒤. MVP. US2·US3의 데스크톱 bridge·compat·MCP 조립(T035–T037)을 먼저 만든다.
- **US2 (Phase 4)**: US1 뒤(`DesktopBenches`, `TauriDesktopBridge`, `lib.rs` 조립 재사용).
- **US3 (Phase 5)**: US2 뒤(MCP agent principal 경로 재사용).
- **US4 (Phase 6)**: US1–US3의 protocol 정의 뒤.
- **Polish (Phase 7)**: 전부 뒤.

### Within Each User Story

- 테스트(fixture·흐름) → protocol DTO → core 서비스·handler → AW compat·bridge → AW 파일 삭제 → 게이트·커밋.
- 같은 파일(`tauri_commands.rs`, `workbench_runtime.rs`, `lib.rs`, `mcp/mod.rs`)을 건드리는 태스크는 순차.

### Parallel Opportunities

- Foundational: T005 ∥ T006 ∥ T007, T012 ∥ T013, T019 ∥ T020.
- US1 테스트: T026 ∥ T027 ∥ T028 ∥ T029 ∥ T030; 구현 T031 ∥ T035.
- US2: T043 ∥ T044 ∥ T045, T047 ∥ T046(다른 crate).
- Polish: T061 ∥ T062.

---

## Parallel Example: User Story 1

```text
T026 run fixture 20개
T027 bench_run_flow.rs
T028 run_start_reconcile.rs
T029 bench_close_race.rs 확장
T030 epoch_idempotency.rs
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1–2 → 작업대·엔진 포트·멱등·fixture 확장
2. Phase 3(US1) → run 8개 이관, 과도기 orchestration 연결
3. **STOP and VALIDATE**: 게이트 + quickstart §3의 1·4·5

### Incremental Delivery

1. Foundation → 커밋
2. US1 → 커밋(run 소유 = 작업대, 사용자 가치 첫 등장)
3. US2 → 커밋(교환 격리 결함 해소)
4. US3 → 커밋(제목 격리)
5. US4 → 커밋(생성 계약)
6. Polish → 게이트·스모크·PR 초안

---

## Notes

- [P] = 다른 파일, 미완료 의존 없음
- 커밋은 논리 단위마다(Foundation, 각 US 끝, 문서). push·PR은 사용자 지시 후.
- 각 checkpoint에서 멈춰 스토리를 독립 검증한다.
- 기준선·실측 기록 (T001, T061, T063, T064, T065):
  - (T001, 2026-09-27) 기준선: `cargo test --workspace --all-targets` 592 passed / 0 failed / 6 ignored, agentic-workbench 429, workbench-client 15, `pnpm run check-types` 13/13.
  - (T002) 오늘 오류 문구는 research 사실 요약·contracts 표와 대조 완료(run 13종·교환 21종·MCP 6종). 차이 없음.
  - (Foundation 구현 중 결정) principal 주체는 `AuthenticatedPrincipal.subject`(`new`는 종류 이름을 주체로 — 기존 호출부 불변). ledger 멱등성 키 namespace는 여전히 `principal_kind`(주체 아님): 영속 command는 `run.start` 하나이고 데스크톱 주체가 하나라 충분하다(3단계에서 재검토). 작업대 operation id는 `system.describe` 앞에 끼워 넣어 계약 순서를 도메인별로 유지. hub: 제거 표식을 worktree 외 모든 kind로 확장, 교환은 `exchange_journal_capacity`(512, 보관 run 수 제외), 작업대 알림 스트림은 구독자가 없어도 발행 시 만들어 데스크톱 전달(닫힌 작업대는 표식으로 전달 안 함), idle 정리 대상 = 발행 전 상태 복원용 + 구독자 없는 작업대 스트림. 세대 멱등은 성공만 기록(실패 재시도는 재실행). `bench.close`는 멱등 기록 없이 자연 멱등. fixture `steps` 형식(요청별 principal·`capture` JSON pointer·`{{dir}}` seed)과 두 경로 비교 시 포착값 정규화. describe fixture는 scratchpad `gen_describe.py`로 registry 소스에서 재생성. T023 중 `not-directory`·`limit` fixture는 fixture로 재현하기 어려워(파일 seed·한도 주입 없음) registry 단위 테스트로 대신했다. `epoch.rs`의 일시 `allow(dead_code)`는 US1에서 제거.

