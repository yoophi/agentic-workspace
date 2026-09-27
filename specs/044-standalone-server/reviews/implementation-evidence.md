# 044 구현 증거

각 검증은 한 번 실행하고, 로그를 저장하고, 원 명령의 종료 코드를 적는다. `CARGO_INCREMENTAL=0`.

## T002·T003 R8-spike

결과 표와 확정 규칙은 `research.md` R8에 있다. 요약:
- Cmd+Q·Dock Quit·AppleScript `quit`은 `RunEvent::Exit`만 오고, 그 전에 창 이벤트가 없다.
- `SIGTERM`은 이벤트가 없다.
- 빨간 버튼·Close Window 메뉴·마지막 창 닫기는 `CloseRequested` → `Destroyed` 순서다.
- System Events Cmd+W 키 입력은 두 창을 닫았다(원인 미확인, 중복 메뉴 가설 기각).
- 로그아웃은 관측하지 않았다.

## T005·T006 #207 (닫힌 작업대의 멱등 기록 부활)

- 시험: `crates/workbench-core/tests/bench_close_idempotency.rs`. 시험 엔진에 완료 문(`prompt_gate`, `test-hooks`)을 두고 효과 → `close_all_benches` → 문 열기 순서로 뒤집는다. 시간 지연을 쓰지 않는다.

| 항목 | 명령 | 종료 코드 | 결과 |
|---|---|---|---|
| red(엔진 문 추가 전) | `cargo test -p workbench-core --test bench_close_idempotency` | 101 | 컴파일 red(`prompt_gate` 필드 없음) — 엔진 편집 스크립트가 패턴 불일치로 적용되지 않았다 |
| red(동작) | 같음 | 101 | `the closed bench's record must not come back: Null` — 닫은 뒤 같은 키 재시도가 성공 결과를 재생했다(#207 증상) |
| green | `… --test bench_close_idempotency --test epoch_idempotency` | 0 | 2 + 3 passed |
| 원 시험 반복 | `cargo test -p workbench-core --test acp_permission_exit` 20회, 회차별 로그·종료 코드 | 20 × 0 | 20/20 통과. 원래 실패가 간헐적이었으므로 반복 통과만으로 해결 근거로 삼지 않는다. 근거는 위 결정적 재현이다 |

- 수정: `EpochIdempotency.closed_benches`(세대 범위 tombstone). `drop_bench`가 `scopes` 잠금 아래에서 세우고, `record`가 같은 잠금 아래에서 확인한다. 닫힌 작업대와 그 아래 run 범위에는 기록하지 않는다. 실행 전 조회는 닫힌 범위의 표가 이미 지워져 비어 있다. 그래서 handler가 작업대를 풀며 `notFound`가 된다.

## T007 protocol: operation 8개·`continuation`·소유자 주체

- 새 operation: `server.status`(query, `server:read`), `server.stop`·`lease.acquire`·`lease.renew`·`lease.release`·`desktop.issueWindowToken`·`desktop.retireWindow`(epoch command, `server:admin`), `bench.list`(query, `bench:read`). DTO는 `operations/{server,lease,desktop}.rs`, `bench.rs`. `RunPromptInput.continuation?: {exchangeRequestId}`(serde default, `run.sendPrompt`에서만 받음 — 판정은 T037).
- 주체: `PrincipalKind::Owner`(`local:owner`, 모든 scope), 새 scope `server:read`·`server:admin`. **설계 문서와 다른 점(기록)**: contracts는 `server:admin` 하나만 적었다. 그런데 `server.status`는 query이고, query는 조회 scope만 요구한다는 기존 불변식(`commands_are_idempotent_and_need_write_scope`)이 있다. 그래서 소유자 전용 조회 scope `server:read`를 더했다. `desktop()`·창·시험 주체는 두 소유자 전용 scope를 뺀 `desktop_scopes()`를 갖는다(창 토큰이 서버 정지·토큰 발급을 못 함).
- **handler 미등록(T026에서 채움, 완료로 세지 않음)**: 새 operation 8개는 아직 registry에 handler가 없다. 부르면 기존 규칙대로 `internal`("operation …의 handler가 등록되지 않았습니다.")을 돌려준다. 임시 stub handler는 만들지 않았다.
- 생성물: `pnpm --filter @yoophi/workbench-client generate`(종료 0)로 OpenAPI·`generated/workbench.ts` 재생성, `operation-kinds.ts`에 8개 추가(93).
- 기존 시험 갱신(044 규칙 반영, 기대 완화 아님):
  - `principal` scope 시험: 소유자 전용 scope 제외를 명시하고 소유자 단정을 추가했다.
  - `authorization`: 데스크톱은 소유자 전용 7개가 `forbidden`, 창 주체도 같다. 소유자는 전부 호출할 수 있다(새 시험).
  - `operation-map.test-d.ts`: 8개 추가.
  - WC 통합 describe 시험: 보이는 operation은 표와 같고, 안 보이는 것은 정확히 소유자 전용 집합이다.
  - describe fixture: desktop·readonly에 `bench.list` 추가(agent는 `bench:read`가 없어 그대로 — 처음에 잘못 추가했다가 계약 시험 실패(43≠44)로 되돌렸다).
  - server `queries_only` 수: 32→34.

| 명령 | 종료 코드 | 결과 |
|---|---|---|
| `cargo test -p workbench-protocol`(1차) | 101 | `server.status`: query에 쓰기 scope → `server:read` 추가 |
| 같음(2차) | 101 | scope 시험 22·readonly 가정, openapi 생성물 불일치 |
| generate 뒤 3차 | 0 | 43 passed |
| `cargo test --workspace --all-targets`(1차) | 101 | 518 passed, 2 failed(authorization 시험 2) |
| `pnpm check-types` / WC test / WC 통합 | 2 / 1 / 1 | type 시험·describe 시험(위 갱신 대상) |
| 갱신 뒤 workspace cargo test | 101 | 548 passed, 1 failed(contract_suite describe fixture 86≠85) |
| check-types / WC test / WC 통합 / AW 통합 | 0 / 0 / 0 / 0 | 73 / 7 / 1 |
| fixture 갱신 뒤 contract_suite | 0 | 5 passed(agent fixture 오갱신 1회 실패 뒤) |
| workspace cargo test(최종 전) | 101 | 767 passed, 1 failed(server `queries_only` 32≠34) |
| `cargo test -p workbench-server` | 0 | 18 passed |

## T008·T009 비우기 분류 대조

- 시험 `crates/workbench-core/tests/drain_classification.rs`: 계약 문서의 두 표(기존 85 + 새 8)를 파싱한다. 모든 operation이 정확히 한 행인지, 종류 열이 `spec_for`와 같은지, 분류 열이 `drain_class`와 같은지, query가 Q인지 본다.
- 구현 `crates/workbench-core/src/application/drain.rs`: `DrainClass{Query, Control, Continuation, NewWork}`, 93개 operation 전부를 빠짐없는 match(와일드카드 없음)로 분류한다. Q 34 · C 24 · K 2 · N 33.
- **설계 문서와 다른 점**: tasks는 `drain_class(OperationId, &input)`라고 적었다. 정적 분류만 표와 대조하므로 `drain_class(OperationId)`로 두었다. K의 입력 조건 판정은 입구가 operation별로 한다(T037·T038·T040).

| 항목 | 종료 코드 | 결과 |
|---|---|---|
| red | 101 | 컴파일 red(`application::drain` 없음) |
| green | 0 | 1 passed |
| 변이(`run.cancel`을 N으로) | 101 | `run.cancel: class column vs drain_class` — 시험이 공허하지 않음(변이 복원 확인) |

## T010–T013 작업 관문·엔진 실행 수명 계약·acp-agent-core 선택 인자

로그: `scratchpad/044/`(파일마다 끝줄에 원 명령의 `exit=`). 모든 판정은 그 로그의 원 명령 종료 코드와 `test result` 줄로 했다.

- T011 `application/work_gate.rs`: 잠금 G 하나 아래 상태(`Serving`·`Draining{Idle|Wait}`·`Stopping`), 예약 표(Turn·Deliver·TaskStart·Notify·Call, drop 해제 guard), 교환 소비 표, task 기동 토큰 표(`Pending→Registered{run}`/`Cancelled`/`Failed`, `register_launch`가 T-start를 A-turn으로 같은 G 아래에서 인계), 정지 판정 `try_stop`(G 아래), `active_work()`(예약 파생분). `WorkbenchRuntime`: `work_gate()`·`server_state()`·`active_work()`, 조립에서 `engine.attach_work_gate`.
- T012 엔진: `AcpRunEngine`의 `start`(초기 순서 guard를 runner로)·`send_prompt`(검사·오류 문구는 `SendPromptUseCase`와 같고, 세션 future를 직접 spawn해 guard를 그 future에 묶음)·`queue_prompt`(spawn 전에 예약)·`send_and_wait`·`steer_prompt`·`cancel_current_prompt_and_send`에 동기 A-turn 예약. 정지 중이면 `RunErrorKind::Unavailable`("server is stopping") → `FaultCode::Unavailable`. `ScriptedRunEngine` 같은 계약(권한 대기 중 초기 turn 유지, 마지막 응답에서 해제).
- T013 acp-agent-core: `AcpAgentRunner::with_initial_turn_guard`(초기 prompt 순서·Ralph 반복을 `run_prompt_sequence` 끝까지 덮고 `child.wait()` 전에 놓음), `StartAgentRunUseCase::execute_gated(.., start_gate)`(`execute`는 `None`으로 위임 — 다른 소비자 변화 없음).

| 단계 | 명령 | 종료 | 결과 |
|---|---|---|---|
| T010 red(컴파일) | `cargo test -p workbench-core --test work_gate` | 101 | `unresolved import workbench_core::application::work_gate`, `no method named work_gate` |
| T010 red(동작, 관문 핵심만 있고 엔진 미연결) | 같음(`t010-red-behavior.log`) | 101 | 2 passed; 3 failed — rpc 오류·대기열·Ralph 순서가 "the run was never reserved"/바쁨 단정 실패. (iii) 정지·예약 교차 1000회는 관문 자체 시험이라 이때 이미 green |
| T010 green | 같음 + `--features test-hooks`(`t010-green-1.log`) | 0 | 5 passed |
| 기동 토큰·`active_work` red(컴파일) | 같음(`t011-token-red-compile.log`) | 101 | `LaunchCancel`·`LaunchState`·`issue_launch`·`active_work`·`server_state` 없음 |
| (무효) | `--test work_gate --lib work_gate`(`t011-token-green-1.log`) | 0 | **이름 필터 때문에 통합 시험 7개가 걸러졌다(단위 3개만 실행). 기동 토큰 완료 근거로 쓰지 않는다** |
| 기동 토큰 green | `cargo test -p workbench-core --features test-hooks --test work_gate`(`-2`) / 필터 없는 `cargo test -p workbench-core --test work_gate`(`t011-token-green-3-nofilter.log`) | 0 / 0 | 7 passed / 7 passed(기동 토큰·`active_work` 포함) |
| start_gate 시험(구현 뒤 추가한 회귀 시험 — TDD red 없음) | `cargo test -p acp-agent-core`(`t013-acp-gate-1.log`) | 0 | 97 passed(`a_gated_start_launches_only_after_the_gate_opens`, `a_dropped_start_gate_never_launches_and_finishes_the_run`) |

### 전체 실행 중 실패: `exchange_delivery_acp` x-2 미전송(043 시험의 기존 경합)

- 관찰: `t013-final-wc-1.log` 종료 101. `the_same_exchange_delivery_key_reaches_the_acp_agent_process_once`에서 agent 기록이 `first`·x-1 뒤 멈추고 x-2가 오지 않았다. 직전 전체 실행(`t013-wc-all-2.log`)은 종료 0이었다.
- 원인: 가짜 agent는 `end_turn:<id>`를 **응답을 보내기 전에** 기록한다. `AcpSession::send_prompt`는 `in_flight.try_lock()`이라, 시험이 기록만 보고 다음 `send_prompt`를 보내면 세션이 아직 이전 응답을 처리 중일 때 "agent is still responding to the previous prompt"로 거절된다. 이 오류는 spawn된 task의 `RunEvent::Error`로만 나가고 prompt는 버려진다. `SendPromptUseCase`도 같은 try-lock 의미라 T012의 직접 spawn이 바꾼 것이 아니다.
- 결정적 재현: 가짜 agent에 `--respond-gate <path> --respond-gate-text <text>`(기록 뒤 문 파일이 생길 때까지 응답 보류)를 더하고, 시험이 x-1 응답을 보류한 채 원래 순서로 x-2를 보내게 했다.
  - `race-repro-red-1.log`: 종료 101, 원래 실패와 같은 기록 `[… "prompt-text:3:\"peer message x-1\"", "end_turn:3"]`.
  - `race-repro-base-red-1.log`: 이번 T011/T012 소스 변경을 stash한 기준에서도 종료 101로 같다 → 기존 경합임을 확인(그 뒤 stash pop).
- 수정(시험 동기화, 기대 완화 아님): 다음 전송을 agent 기록이 아니라 **엔진의 실행 종료**(A-turn 해제 = 세션 `in_flight` 해제 뒤, `work_gate().busy_run_count(run) == 0`)를 보고 보낸다. x-1 응답 보류 문은 남겨 그 구간을 매번 결정적으로 연다. 같은 경합이 있던 `tests/work_gate.rs`의 두 곳("first" 기록 뒤 곧바로 `run.sendPrompt`)도 같은 방식으로 고쳤다.
  - `race-fix-green-1.log`: 종료 0, 2 passed.
- 반복 실행 10회(`flake-mine-*.log`, 0/10 실패)는 결정적 근거로 쓰지 않았다.

### 최종 검증(수정 뒤 1회씩, `*-3.log`)

| 명령 | 종료 | 결과 |
|---|---|---|
| `cargo test -p workbench-core --features test-hooks` | 0 | 53 결과 줄, 441 passed, 0 failed |
| `cargo test -p workbench-core --test work_gate --test exchange_delivery_acp` | 0 | 9 passed |
| `cargo test -p acp-agent-core` | 0 | 97 passed |
| `apps/agentic-workbench/src-tauri`: `cargo test` | 0 | 125 passed |
| `apps/ask-code/src-tauri`: `cargo test && cargo check` | 0 | 시험 0개(빌드·check 통과) |
| `apps/hushline/src-tauri`: `cargo test && cargo check` | 0 | 7 passed |
| `cargo fmt -p workbench-core -p acp-agent-core -- --check` | 1 | 남은 차이는 `tests/bench_close_idempotency.rs`(#207 부모 작업 파일, 이 범위 밖)뿐. 이 작업 파일은 rustfmt 적용 |

### 설계와 다른 점

- `RunErrorKind::Unavailable` 추가(정지 중 예약 거절을 `unavailable`로 돌려주려고). `run_service::engine_fault`에서 `FaultCode::Unavailable`로 대응.
- `AcpRunEngine::start`는 run id가 없으면 엔진에서 uuid를 정한다(초기 순서 예약을 run으로 세기 위해. 유스케이스의 `build_run`과 같은 형식).
- `active_work()`는 관문 예약 파생분만 돌려준다(`GateActiveWork`). 저장소·ledger 파생 수(`orchestrationTasks`·`queuedTasks`·`pendingExchanges`·`pendingOperations`)는 `server.status` 조립(T026 이후)이 채운다 — 이 범위에서는 미완.
- 가짜 agent 선택 인자 추가: `--end-turn-gate`, `--rpc-error-text`, `--respond-gate`/`--respond-gate-text`.

## fork 결과 확인 (T007–T013, 메인 세션)

- 기동 토큰 시험: 필터 없는 `cargo test -p workbench-core --test work_gate`(`t011-token-green-3-nofilter.log`)에서 7개가 모두 실행돼 통과했다(0 filtered out). 이 로그를 직접 확인했다. 필터 때문에 단위 시험 3개만 돈 `t011-token-green-1.log`는 근거로 쓰지 않는다.
- `exchange_delivery_acp` 실패: 결정적 재현(`race-repro-red-1.log`, 변경을 stash한 기준에서 `race-repro-base-red-1.log`) 두 로그 모두 `exchange_delivery_acp.rs:62`에서 실패했다. 수정 뒤 `race-fix-green-1.log`는 2 passed다. 전체 `t013-wc-all-3.log`는 441 passed, 0 failed다. 모두 직접 확인했다.
- **남은 기존 제품 결함(추적)**: 앞 prompt의 완료 로그 직후(응답 처리 중) 보낸 `run.sendPrompt`는 세션 in-flight 잠금을 한 번만 시도해 거절되고, Error 이벤트만 남기고 버려진다. `SendPromptUseCase`도 같다. 044 이전부터 있던 동작이다. 교환 전달(K)은 엔진 대기열 경로라 이 경쟁을 피한다(R7·R14). 일반 `sendPrompt`의 이 경쟁은 이 증분에서 고치지 않는다. 구현 리뷰에서 다시 보고, 후속 추적 항목으로 둔다.
- `cargo fmt`: fork 커밋에 남은 서식 차이 3개 파일(`bench_close_idempotency.rs`, `operations/mod.rs`, `principal.rs`)을 정리했다.

## T014–T017 조립·MCP를 `workbench-host`로, 창 무관 MCP 주입, 시험 host

- **이동(동작 보존)**:
  - AW `src-tauri`의 MCP 모듈 전체(`infrastructure/mcp/*`)와 `domain/mcp_title_control.rs`, `acp_agent_launch_factory.rs` → `crates/workbench-host/src/{mcp/*, mcp/title_control.rs, launch.rs}`(`git mv`, 이력 유지).
  - `workbench_http.rs`의 Tauri 무관 부분(출처 목록, MCP resolver, 발급기·표, `WorkbenchHttpState`, `ExitGate`, drain) → `crates/workbench-host/src/http.rs`.
  - AW `workbench_http.rs`에는 재노출과 Tauri에 묶인 부분(`origin_of(tauri::Url)`, 창 incarnation으로 토큰을 발급하는 `WorkbenchHttp`)만 남았다.
  - `tauri::async_runtime::spawn`은 호출자가 넘기는 tokio 핸들(`spawner`)로 바꿨다. AW는 `tauri::async_runtime::handle().inner()`를 넘긴다.
- **조립**: `workbench_host::assembly::{assemble, assemble_core}`는 런타임 → MCP(`McpLaunchDecorator` 묶음) → HTTP 순이다. AW `lib.rs`가 이것을 쓴다. 043 동작은 그대로다: 창 주체·토큰·compat command·Tauri 브리지의 창 삽입 전달을 유지하고, 기동 실패 주입은 `HttpStart::Fail`로 한다.
- **시험 수 대조**: 옮긴 파일들의 시험은 이동 전 40개다(`e8ef04a` 기준 파일별 셈). 이동 뒤 host `src` 38개와 AW `workbench_http.rs` 2개(Tauri URL·창 등록 의존)를 합치면 40개다. AW Rust 시험은 125 → 87로, 옮긴 38개만큼 줄었다.

| 항목 | 명령 | 종료 코드 | 결과 |
|---|---|---|---|
| T016 red | `cargo test -p workbench-host --test mcp_launch` | 101 | 컴파일 red(`workbench_host::assembly`·`mcp` 없음). 첫 실행은 시험 엔진 편집 스크립트의 패턴 불일치로 `start_requests`도 없었다(재편집 뒤 다시 red) |
| T016 green | 같음 | 0 | 1 passed, 0 filtered out |
| T016 변이 | `assemble_core`에서 decorator 설치를 뺌 | 101 | `MCP env injected` 단정에서 실패. 복원 확인 |
| host 전체 | `cargo test -p workbench-host --features test-hooks` | 0 | 39 passed(단위 38 + 통합 1) |
| core | `cargo test -p workbench-core --features test-hooks` | 0 | 441 passed |
| AW Rust | `cargo test -p agentic-workbench` | 0 | 87 passed |
| clippy | `cargo clippy -p workbench-host -p workbench-core -p agentic-workbench -p agentic-workbench-server --all-targets --features workbench-host/test-hooks -- -D warnings` | 0 | — |
| fmt | `cargo fmt --all -- --check` | 0 | — |
| 통합(TS) | `pnpm --filter @yoophi/workbench-client test:integration` | 0 | 7 passed(시험 host는 host crate 예제로 빌드) |
| AW 화면 | `pnpm --filter @yoophi/agentic-workbench test` / `test:integration` | 0 / 0 | 634 / 1 passed |
| 타입 | `pnpm run check-types` | 0 | — |

- 세 Rust 대상의 모든 `test result` 줄이 `0 filtered out`이다(확인 결과 0이 아닌 것 0건).

**설계와 다른 점(이유)**:
1. 시험 host를 `crates/workbench-core/examples`에서 `crates/workbench-host/examples/http_test_host.rs`로 옮겼다. core가 host에 의존하면 순환이 된다. 예제 이름이 같아 바이너리 경로(`target/debug/examples/http_test_host`)는 그대로다. `global-setup.ts`의 빌드 명령만 `-p workbench-host --example http_test_host --features test-hooks`로 바꿨다. host crate에 `test-hooks` feature(→ core `test-hooks`)를 더했다.
2. 시험 host는 런타임·MCP만 host 조립(`assemble_core`)으로 만든다. HTTP router는 오늘처럼 시험 전용 설정(고정 창 토큰 `StaticResolver`, 빈 출처 정책, 작은 journal 한도)을 쓴다. TS 통합 시험이 고정 토큰에 기대기 때문이다.
3. `McpLaunchDecorator`는 옛 Tauri decorator의 "작업대에 창이 있어야 함"(`MESSAGE_WINDOW_UNAVAILABLE`) 검사를 하지 않는다(R2 의도). embedded 모드에서 창을 닫으면 작업대가 닫히므로, 닫힌 창의 run.start는 작업대 조회에서 먼저 거절된다.
4. 시험 엔진(`ScriptedRunEngine`, `test-hooks`)에 `start_requests`(받은 시작 요청 기록)를 더했다.

## T018–T024 단일 writer·안내 파일·신원 증명·ensure·서버 앱 (US3)

| 항목 | 명령 | 종료 코드 | 결과 |
|---|---|---|---|
| T021 red | `cargo test -p workbench-host --test identify` (`t021-red-1.log`) | 101 | 컴파일 red: `workbench_host::lifecycle` 없음, `HostOptions.owner` 없음 |
| T018 red | `cargo test -p agentic-workbench-server --test process` (`t018-red-1.log`) | 101 | 컴파일 red: `workbench_host::lifecycle` 없음 |
| T021 첫 실행 | 같음(`t021-green-1.log`) | 101 | 실제 서버 시험 통과, 가짜 서버 시험 실패(`Unreachable("Connection reset by peer")`). **원인은 시험 도구**: 가짜 서버가 요청을 한 번만 읽고 닫아, 헤더와 본문을 두 번에 나눠 쓴 클라이언트 쪽이 RST를 받았다. 가짜 서버가 요청 전체(헤더 + content-length 본문)를 읽게 고치고, 클라이언트도 요청을 한 번에 쓰게 했다 |
| T021 green | 같음(`t021-green-2.log`) | 0 | 2 passed, 0 filtered out |
| T021 변이 | 클라이언트의 증명 검증을 끔(`t021-mut-1.log`) | 101 | 가짜 서버 시험 실패(검증 없이 handshake로 넘어가 `Incompatible`, 변형 단정에서 잡힘). 원본 복원 |
| T018 green | `cargo test -p agentic-workbench-server --test process` (`t018-green-1.log`, 클라이언트 한 번 쓰기 수정 전) | 0 | 5 passed, 0 filtered out |
| T018 green(최종 코드) | `cargo test -p agentic-workbench-server` (`t024-final/server-app-2.log`) | 0 | 5 passed, 0 filtered out |

T018 시험 내용(프로세스, 고정 sleep 없이 프로세스 종료 대기·상한 있는 polling):
- (i) 같은 데이터 디렉터리에 `serve` 10개를 동시에 띄운다 → 9개가 종료 코드 3, 1개만 서빙한다. 안내 파일의 `pid`가 그 서버이고, 신원 증명·handshake·ready를 통과한다. `SIGTERM` → 종료 코드 0, 안내 파일 삭제.
- (ii) `ensure` → `kill -9` → 남은 안내 파일 확인 → 다음 `ensure`가 5초 안에 새 인스턴스로 준비된다(측정값 단정). 표준 출력에 자격 증명 없음.
- (iii) `workbench/server/` 0700, `server.json`·`owner.lock`·`startup.lock` 0600.
- (iv) 다른 데이터 디렉터리는 서로 다른 인스턴스·끝점으로 뜬다.
- (v) 저장 형식 99인 ledger → `serve` 종료 코드 4, ledger 바이트 불변, 안내 파일 없음.
- 모든 시험은 자기가 띄운 PID만 `Cleanup` drop에서 끝낸다.

최종 실행(`t024-final/`, 각 1회):

| 게이트 | 종료 코드 | 결과 |
|---|---|---|
| `cargo test -p workbench-host --features test-hooks` | 0 | 48 passed |
| `cargo test -p agentic-workbench-server` | 0 | 5 passed(clippy 수정 뒤 재실행 `server-app-2.log`) |
| `cargo test -p workbench-server` | 0 | 18 passed |
| `cargo test -p workbench-core --features test-hooks` | 0 | 441 passed |
| `cargo test -p agentic-workbench` | 0 | 87 passed |
| clippy `-D warnings`(host·server·core·server 앱·AW) | 101 → 0 | 첫 실행: 시험 코드 2곳(`while let`, 접을 수 있는 `if`). 고친 뒤 `clippy-2.log` 0. identify 시험 재실행 0(2 passed) |
| `cargo fmt --check` | 0 | — |
| `pnpm --filter @yoophi/workbench-client test:integration` | 0 | 7 passed |

구현 요약:
- **workbench-server**: `ServerInfo::instance_id`·`identity_proof`(기본 `None`)를 더했다. router는 인스턴스 식별자를 서버 정보에서 받는다. 인증 없는 `POST /v1/system/identify`: `{nonce}` 16–256자 → `{instanceId, proof}`, 증명 수단이 없으면 `notFound`.
- **workbench-core**: `sqlite_ledger::read_schema_version`(읽기 전용).
- **workbench-host `lifecycle`**:
  - `lock`: `owner.lock`·`startup.lock`, 표준 파일 잠금, 0700·0600.
  - `descriptor`: 원자적 쓰기, 자기 인스턴스일 때만 삭제, 자격 증명 뺀 공개 JSON.
  - `identity`: 인스턴스·32바이트 자격 증명, RFC 2104 HMAC-SHA256(RFC 4231 벡터 시험), Origin 없는 소유자 자격 증명만 받는 `OwnerResolver`.
  - `client`: 루프백 HTTP/1.1, **신원 증명 → handshake·ready**.
  - `ensure`: contracts §3, `process_group(0)`, null stdio, stderr → `server.log`, 종료 회수 스레드.
  - `server`: `serve`.
- **HostOptions.owner / HttpAssembly.owner**: 소유자 resolver와 서버 정보(인스턴스·증명)를 넣는다. AW embedded는 `None`(T028).
- **서버 앱**: `serve`·`ensure`·`status`·`stop`.

중간 구현·미완료(완료로 세지 않음):
- `stop` 명령: "not implemented yet"과 종료 코드 1을 돌려준다. T041·T044에서 구현한다.
- `serve`의 신호 처리: 열린 작업대를 닫고(run 취소) 받아들인 호출을 drain한 뒤 끝낸다(오늘 조립의 종료 순서). 상태 기계·비우기 분류·정지 세 방식·SIGTERM = force(30초 상한)는 T041.
- `--idle-timeout`·`--log` 인자는 아직 받지 않는다. 서버 로그는 `ensure`가 서버 stderr를 `server.log`로 돌린다. 유휴는 T041.
- `ensure`의 서버 상태 확인은 `/health/ready`까지다. `draining`·`stopping` 상태를 기다리는 부분은 `server.status`(T026)·상태 기계(T041) 뒤에 넣는다.

설계와 다른 점:
- 신원 증명 입력은 `nonce ‖ "\n" ‖ instanceId`다(구분자를 둬 이어 붙임의 모호함을 없앴다).
- 소유자 자격 증명은 uuid v4 두 개의 바이트(32바이트) hex다(새 난수 crate를 더하지 않았다). v4는 버전 비트를 빼면 244비트 엔트로피다.
- 저장 형식 거절은 조립 전에 읽기 전용으로 검사한다(데이터 무변경을 보장). 다만 `workbench/server/`(잠금·로그 디렉터리)는 소유 잠금을 위해 먼저 만든다. 도메인 파일·ledger는 건드리지 않는다.
- 소유자 resolver는 Origin이 있는 요청을 받지 않는다(WebView가 자격 증명을 얻어도 쓰지 못하게). contracts에 명시돼 있지 않던 규칙이다.

## T025–T027 (US2 서버 측: 창 토큰 tombstone·소유자 전용 op·소유자 우회)

로그: `scratchpad/044/`(세션 scratchpad). 한 번씩 실행하고 종료 코드를 `.status`에 남겼다.

red(시험 먼저):

| 로그 | 종료 | 종류 | 내용 |
|---|---|---|---|
| `t027-red-1.log` | 101 | 컴파일 | `server_control()` 없음 → 최소 `ServerControl`·임대 표를 더해 동작 red로 넘김 |
| `t027-red-2.log` | 101 | 시험 오류 | agent 전용 op 입력 키를 `input`으로 적음(`AgentToolInput`은 `arguments`) → 시험 수정 |
| `t027-red-3.log` | 101 | 동작 | 3 실패: `lease.acquire`·`bench.list` "handler가 등록되지 않았습니다". `owner_only_forbidden`·`agent_only`는 기존 동작(scope·`ensure_run`)으로 이미 통과 |
| `t025-red-1.log` | 101 | 동작 | 5 실패 전부: `desktop.issueWindowToken`/`retireWindow` 500 "handler가 등록되지 않았습니다" |

green·최종:

| 명령 | 종료 | 결과 |
|---|---|---|
| `cargo test -p workbench-core --features test-hooks --test owner_principal` (`t027-green-1`) | 0 | 7 passed, 0 filtered out |
| `cargo test -p workbench-host --test window_tokens` (`t025-green-2`) | 0 | 6 passed, 0 filtered out |
| `cargo test -p workbench-protocol` (`t027-final-protocol-1`) | 0 | 45 passed, 모든 target 0 filtered out |
| `cargo test -p workbench-server` (`-1` 101: 단위 시험이 옛 `issuer.entries` 참조 → 고침, `-2`) | 0 | 18 passed, 0 filtered out |
| `cargo test -p workbench-core --features test-hooks` (`t027-final-core-2`) | 0 | 449 passed, 54 target 모두 0 filtered out |
| `cargo test -p workbench-host` (`t027-final-host-2`) | 0 | 54 passed, 5 target |
| `cargo test` agentic-workbench-server (`-2`) | 0 | 5 passed |
| `cargo test` AW src-tauri (`-2`) | 0 | 87 passed |
| clippy `-D warnings` core·protocol·server·host(`-1` 101: `build_registry` 인자 8개 → `benches`를 `server_control.benches()`로, `-2` 0)·AW server·AW src-tauri | 0 | clippy 수정 뒤 core·host·AW server·src-tauri 전체 재실행(위 `-2`) |
| `pnpm --filter @yoophi/workbench-client generate` 재실행 후 `cmp` | 0 | OpenAPI·TS 동일(최신) |
| `pnpm --filter @yoophi/workbench-client test` / `test:integration` / `check-types` | 0 | 73 passed / 7 passed / 0 |

구현 요약:
- **workbench-server**: `DesktopTokenIssuer`와 `EventTicketStore`가 폐기 주체 tombstone을 항목과 **같은 잠금** 아래에 둔다. `issue_window`는 잠금 안에서 tombstone을 확인하고, `retire_subject`는 tombstone을 세운 뒤 항목을 지운다. 폐기 주체의 표는 `take`에서도 무효다. `routes/events`는 `IssueError::Retired` → 401.
- **core port `ServerHost`**(`ports/server_host.rs`): 창 토큰 발급·폐기·인스턴스 식별자·받아들인 호출 수. core는 `workbench-server`에 의존하지 않는다. host `HttpServerHost`(`http.rs`)가 구현하고, `assembly::assemble`이 HTTP 기동 뒤 `runtime.attach_server_host`로 넣는다.
- **core handler**(`handlers/server/mod.rs`):
  - `desktop.issueWindowToken`: 출처가 허용 목록 밖이거나 tombstone이면 `forbidden`, label·incarnation이 비었거나 `:`를 포함하면 `invalidArgument`(주체 문자열 위조 방지).
  - `desktop.retireWindow`: 먼저 폐기(토큰+표 수 = `revokedTokens`)하고, `closeBench`면 그 주체가 연 작업대를 모두 닫는다(`BenchServices::close_opened_by`).
  - `lease.acquire`/`renew`(모르면 `notFound`)/`release`.
  - `bench.list`: 소유자는 전부, 그 밖은 자기 것만. run은 hub claim(`EventHub::runs_of_bench`) 중 엔진이 아직 그 작업대 소유로 두는 것만, 상태는 WorkGate Turn 예약으로 `busy`/`idle`.
  - `server.status`.
  - 새 command는 모두 `Scope::None`이다(토큰 비밀을 멱등 기록에 남기지 않음). 소유자 전용 여부는 기존 scope(`server:admin`/`server:read`)로 판정한다.
- **소유자 우회**: `bench_service`의 `resolve`(→ `resolve_any`)·`admit`(주체 검사 없음)·`close`(연 주체로 `close_as`), `workbench_runtime`의 `owns_bench`·`authorize_bench_streams`. agent 전용 op는 run 대조(`ensure_run`)로 소유자도 `forbidden`.

`server.status`에서 아직 파생하지 않는 필드(coordinator 지적 반영):
- 0으로 채우지 않는다. 프로토콜을 바꿔 `activeWork.{orchestrationTasks, queuedTasks, pendingExchanges, pendingNotifications, pendingOperations}`, `unresolvedOperations`, `undeliverableExchanges`, `failedExchangeDeliveries`를 nullable로 했고, `null`로 싣는다. `idleSince`는 생략한다.
- 새 필드 `notYetDerived: string[]`에 그 JSON 경로를 싣는다. OpenAPI·TS를 재생성했고 contracts §4를 갱신했다.
- 파생되는 값: `state`(WorkGate), `instanceId`(host 소유자 신원; embedded·host 없음은 빈 문자열), `serverEpoch`, `busyRuns`, `acceptedCalls`(HTTP+MCP 분리 호출 + WorkGate Call 예약), `reservations`, `idleRuns`, `leases`.
- 정지 판정: `ActiveWorkDto::blocks_stop()`은 `null`인 수를 활동 작업으로 본다(보수적). T041의 유휴·`default`·`wait` 정지는 이것을 써야 한다. 이 규칙이 아니면 정지 판단을 T041에 둔다.
- 고정 시험:
  - `owner_principal::server_status_reports_underived_fields_as_unknown_not_zero`(core): 목록의 필드는 `null`/부재이고 `notYetDerived`와 같으며, 알려진 수가 0이어도 `blocks_stop` 참.
  - `window_tokens::server_status_carries_the_instance_id_and_marks_underived_fields`(host): 실제 instanceId, 창 토큰으로는 403.
  - protocol 단위 시험 2개.
  - 이 시험들은 구현 뒤에 더해서 red 기록이 없다.

설계와 다른 점:
- 임대 표는 plan의 host `lifecycle/lease.rs`가 아니라 core `application/lease.rs`(`ServerControl` 안)에 둔다. 임대가 `server.status`와 T041 유휴 판정(core WorkGate와 함께 읽음)에 쓰이고, core가 host에 의존할 수 없기 때문이다.
- 폐기 결과 `revokedTokens`는 폐기한 토큰 수와 이벤트 표 수의 합이다.
- `server.stop`은 등록하지 않았다(T041).

## T028–T033 — 데스크톱 외부 서버 모드(US2 데스크톱 쪽)

로그: `scratchpad/044/`(세션 임시 디렉터리). 각 검증은 한 번 실행했고 로그 끝에 `exit=`를 남겼다. 전체 대상은 `0 filtered out`이다.

| 검증 | 로그 | exit | 결과 |
|---|---|---|---|
| T030 red(스텁: 항상 CloseBench) | `t030-red-1.log` | 101 | 행동 red: 3 passed / 4 failed |
| T030 green | `t030-green-1.log` | 0 | 7 passed |
| host `owner_calls` red(모듈 없음) | `t028-calls-red-1.log` | 101 | 컴파일 red: `E0432 unresolved import lifecycle::calls` |
| host `owner_calls` green | `t028-calls-green-1.log`, `t028-host-calls-1.log` | 0 | 1 passed |
| `server_client` 변이(임대 해제 생략) | `t028-client-mut-1.log` | 101 | 1 failed(해제 단언), 원본 복원 |
| `server_client` 변이(폐기 수를 spawn 안에서 셈) | `t031-mutant-1.log` | 101 | `counted before the task runs` 실패, 원본 복원(cmp 확인) |
| 새 데스크톱 단위(`server_client`·`workbench_mode`·`window_close_intent`) | `t028-unit-1.log` | 0 | 14 passed |
| T032 red | `t032-red-1.log` | 1 | 행동 red: bootstrap 4, App 1 실패 / 컴파일 red: `window-title`, `connection-failure` 모듈 없음 |
| T032 green | `t032-green-1.log` | 0 | 5 files, 24 passed |
| AW src-tauri `cargo test`(lint 수정 뒤 재실행) | `final-aw-test-2.log` | 0 | lib 100 passed, orchestration_smoke_agent 1 passed |
| `cargo test -p workbench-host` | `final-host-test-1.log` | 0 | 45 + 2 + 1 + 1 + 6 passed |
| `cargo clippy -p agentic-workbench -p workbench-host --all-targets -D warnings` | `final-clippy-1.log` → `final-clippy-2.log` | 101 → 0 | `cloned_ref_to_slice_refs` 3건(시험 코드) 수정 |
| `cargo fmt --check`(AW, host) | `final-fmt-1.log` → `final-fmt-2.log` | host 1 → 0 | host 서식 적용 |
| AW `pnpm test` | `final-aw-vitest-1.log` | 0 | 92 files, 641 passed |
| AW `pnpm test:integration` | `final-aw-itest-1.log` | 0 | 1 file, 1 passed |
| workbench-client `pnpm test:integration` | `final-client-itest-1.log` | 0 | 3 files, 7 passed |
| AW `check-types` / workbench-client `check-types` | `final-types-1.log`, `final-client-types-1.log` | 0 / 0 | |
| AW `pnpm build` / Storybook build | `final-aw-build-1.log`, `final-storybook-1.log` | 0 / 0 | |

구현 요약:
- **모드(T028)**: `infrastructure/workbench_mode.rs`. 기본은 `external`이다. `AW_WORKBENCH_MODE=embedded`만 043 경로를 쓴다. 기동할 때 `[workbench] mode: …`를 기록한다.
  - external: 앱 안에 런타임·MCP·HTTP를 두지 않는다. `ExternalServer`(`infrastructure/server_client.rs`)만 관리한다.
  - `ExternalServer`는 첫 `get_workbench_connection`에서 서버를 `ensure`하고(탐색 순서: `AW_WORKBENCH_SERVER_PATH` → 실행 파일 옆 → `target/debug|release`), 소유자 자격으로 `lease.acquire`를 보낸다. 임대는 10초마다 갱신한다.
  - embedded: `EmbeddedOwnership::claim`이 같은 `owner.lock`을 잡는다. 못 잡으면 setup 오류로 부팅하지 않는다. host 조립에 `owner` 신원을 넣고, HTTP가 뜨면 안내 파일(`mode: "embedded"`)을 쓴다. `Exit`에서 자기 안내만 지운다.
- **호출 client(T028)**: host `lifecycle::calls::call`(bearer + 선택 Origin). `CallError::Transport`와 `Fault{status,code,message}`를 나눈다.
  - 창 토큰 호출은 WebView 출처를 Origin 헤더로 싣는다. 창 토큰은 Origin이 없으면 401이다(host 시험으로 고정).
  - `Descriptor::for_endpoint`는 독립 서버와 embedded가 같이 쓴다.
- **command(T029)**:
  - `get_workbench_connection`: external에서 `desktop.issueWindowToken` 결과를 돌려준다. 출력 모양은 043과 같다(`incarnation` 포함).
  - `ensure_window_bench`: external에서 창 토큰 + Origin으로 `bench.open`. `open:false`는 매핑만 조회한다.
  - `declare_network_delivery`·`withdraw_network_delivery`: external에서 no-op.
  - 호환 command: `workbench_runtime()`가 `try_state`로 `Result`를 돌려준다. 런타임이 없으면 정확히 `"Workbench server is external; this command is unavailable."`이다(panic 없음).
  - 신규 `apply_window_title`(창 제목 + 네이티브 Window 메뉴 동기화)와 `get_workbench_mode`.
- **닫기 의도(T030)**: `application/window_close_intent.rs`는 순수 상태다. (label, incarnation)별 의도를 두고, 종료 의도 뒤의 `CloseRequested`는 무시한다.
  - R8 관측 순서 (a)/(b1)/(b2)/(c)(d)(e)/(f)를 시험으로 고정했다.
  - (b2) Cmd+W 두 창 닫힘은 원인 미확정이다. 두 창 모두 `CloseRequested`가 있으면 둘 다 CloseBench로 기대한다.
- **생명주기(T031)**: `window_lifecycle::track`이 `CloseRequested`에서 의도를 기록한다.
  - `Destroyed`에서 external은 `retire_window_detached(label, incarnation, closeBench = 의도)` + `desktop_benches::forget_window`를 부른다. embedded는 기존 043 `on_destroyed` 그대로다.
  - `mark_quitting()`은 `ExitRequested`·`Exit`에서 부른다(두 모드).
  - external 종료는 `close_all_benches`를 부르지 않는다. `ExitRequested`는 종료를 미루지 않는다. `Exit`에서 `release_for_exit(EXIT_FLUSH_LIMIT = 2s)`가 진행 중 폐기를 흘려보낸 뒤 임대를 놓는다.
  - 폐기 수는 spawn 전에 동기로 올린다. 그래서 `Destroyed` 직후의 `Exit`도 그 폐기를 기다린다(변이로 고정).
  - 서버에 붙은 적이 없으면 폐기 때문에 서버를 띄우지 않는다.
- **화면(T032)**:
  - `bootstrapTransport`가 `get_workbench_mode`를 한 번 묻는다. 물을 수 없으면 external로 본다.
  - external에서 실패하면 `{kind:"failed", reason}`이고 `[workbench-client] connection failed: <이유>`를 기록한다. transport는 바꾸지 않고, 철회도 부르지 않는다.
  - `main.tsx`는 실패하면 `ConnectionFailure`(이유 + "다시 시도" = 창 다시 불러오기)만 그린다. Storybook `WorkbenchConnectionFailure`.
  - `App.tsx` 제목은 `applyWindowTitle` → `apply_window_title`로 적용한다(두 모드).
  - no-direct-invoke 허용 목록에 `get_workbench_mode`·`apply_window_title`를 더했다.
- **T033 범위(정직하게)**: 043 화면 시험(AW `pnpm test` 641)과 통합 시험(AW 1, workbench-client 7)은 기대값을 바꾸지 않고 통과했다. bootstrap 043 시험은 `getMode: embedded`를 넣어 그대로 남겼다.
  - 통합 시험이 붙는 대상은 별도 프로세스인 시험 host(`workbench-host --example http_test_host`, 독립 서버와 같은 host 조립)다. 데스크톱이 실제로 띄우는 `agentic-workbench-server` 바이너리는 아니다.
  - 실제 앱 스모크(T045–T047)는 하지 않았다.

설계와 다른 점·남은 것:
- 외부 모드 창 토큰은 화면이 서버에 직접 묻지 않는다. 데스크톱이 소유자 자격으로 `desktop.issueWindowToken`을 부르고, `get_workbench_connection`의 모양을 유지한다(contracts와 같음).
- `bootstrapTransport`가 예외로 끝나면 이제 연결 실패 화면을 그린다. 043은 예외여도 App을 그렸다. 정상 경로에서는 예외가 없다.
- 호환 command의 external 오류 문구는 Tauri command 시험 틀이 없어 Rust 단위 시험으로 고정하지 않았다. 상수 `MESSAGE_EXTERNAL_UNAVAILABLE` 하나로 만든다.
- `EmbeddedOwnership`의 안내 파일은 HTTP 기동에 실패하면 쓰지 않는다(잠금은 유지).

## fork 결과 확인 (T028–T033, 메인 세션)

- 최종 로그를 직접 확인했다. AW `cargo test` 101 passed(lint 수정 뒤 재실행), clippy·fmt는 첫 실행 실패(101·1) 뒤 수정해 0, AW vitest 641, 두 통합 suite, check-types, AW build, Storybook build는 모두 0이다. filtered out은 모두 0이다.
- lint 수정 커밋 `e04eb49`이 host 코드(`lifecycle/server.rs`, `tests/owner_calls.rs`)를 건드렸다. 그런데 host 시험 로그는 수정 전의 `final-host-test-1.log`뿐이었다. 그래서 메인 세션이 `cargo test -p workbench-host`를 한 번 다시 실행했다(`t033-host-after-lint-1.log`): 종료 0, 55 passed, filtered out 0.
- 범위(fork 보고대로): T033 통합 시험이 붙는 대상은 시험 host 프로세스다. 데스크톱이 실제로 띄우는 `agentic-workbench-server` 바이너리 연결은 실제 앱 스모크(T036·T045–T047)에서 확인한다. 실제 창·종료 동작은 아직 미확인이다.

## T034 — 데스크톱이 떠난 뒤 소유자가 같은 run을 이어 본다 (host 통합)

- 시험 `crates/workbench-host/tests/owner_after_desktop.rs` (dev-dep `tokio-tungstenite`). 시험 host가 실제 `server.json`을 쓰고, 소유자는 그 파일에서 `verify()`(신원 증명 → handshake → ready)를 통과한 뒤에만 자격을 쓴다.
- 흐름: 이 순서로 진행한다.
  1. 창 쪽: `lease.acquire` → 창 토큰 → 창 `bench.open`·`run.start`.
  2. 앱 종료 흉내: `desktop.retireWindow {closeBench:false}`(`closedBenches == []`) → `lease.release`. 이후 옛 창 토큰은 401이다.
  3. 소유자 쪽: `bench.list`에서 run 확인 → `run.replay`로 시작 출력(`lastSequence > 0`) → 그 순번부터 event ticket·WebSocket 구독 → 소유자 `run.sendPrompt` → 새 agentMessage를 받는다. live 순번은 모두 replay 마지막 순번보다 크다.
  4. 소유자 `run.cancel` → run이 목록에서 사라진다.
- 두 엔진으로 검증했다.
  - 시험 엔진(ScriptedRunEngine): 끝에서 `run_count == 0`을 확인한다.
  - 실제 `AcpRunEngine` + 가짜 ACP agent(`--echo`, 고유 `--log` 표식): agent 프로세스가 뜬 것을 확인한 뒤, 취소 후 프로세스가 끝나는지를 상한 있는 polling으로 확인한다.
- 실행 기록(`scratchpad/044/`):
  - `t034-first-1.log` 종료 101 — **시험 작성 오류**. `closedBenches`를 숫자로 가정했으나 실제 응답은 배열 `[]`이다. 기대값을 수정했다.
  - `t034-green-1.log` 종료 101 — **시험 타이밍 오류**. `run.start` 직후 agent 프로세스를 바로 확인했다. 상한 있는 대기로 바꿨다.
  - `t034-green-2.log` 종료 0, 2 passed, filtered out 0.
  - **제품 red는 없다.** 제품 변경 없이 바로 통과했다(서버 lease/retire가 이미 작업대를 남겼다). 대신 mutation으로 시험의 판별력을 확인했다.
  - `t034-mut-1.log` 종료 101 — `lease.release` 처리기가 `benches().close_all()`까지 하도록 임시 변조했다. 두 시험 모두 실패했다.
    - 시험 엔진: 소유자 prompt가 404 `bench not found.`
    - ACP: agent 프로세스 대기 timeout.
    - 변조는 백업으로 되돌렸다(`git diff` 깨끗함).
- lint 수정(redundant closure) 뒤 재실행: `final-host-owner-2.log` 종료 0, 2 passed, filtered out 0.

## T035 — 앱 스모크 probe `quit` 시나리오와 소유자 확인 스크립트

- probe(`http_probe.rs`, `#[cfg(debug_assertions)]` 모듈): `AW_APP_PROBE_SCENARIO=quit`이면 `APP_QUIT_PROBE_SCRIPT`를 한 번 넣는다.
  - 모드(`get_workbench_mode`)와 transport를 기록한다.
  - 앱 transport(`__awDebug.invoke('start_agent_run')`)로 에코 agent run을 시작하고, 시작 에코와 그 뒤 `promptCompleted`를 받는다.
  - `ensure_window_bench {open:false}`로 작업대 id를 얻는다.
  - `{scenario:"quit", runId, benchId, phase:"ready-to-quit", mode, transport, result}`를 `report_app_probe`로 쓴다.
  - run은 취소하지 않는다. 토큰도 싣지 않는다.
  - 단위 시험 `quit_probe_reports_ready_to_quit_and_leaves_the_run_alive`는 템플릿 치환, 보고 필드, `cancel`·`token`·`get_workbench_connection` 부재를 확인한다(`t035-probe-unit-1.log` 종료 0).
  - **실제 앱 실행은 하지 않았다**(T036·T045–T047에서 메인 세션이 수행).
- 스크립트 `specs/044-standalone-server/reviews/app-smoke/owner-check.py`(stdlib만 사용):
  1. `server.json`을 읽는다.
  2. `POST /v1/system/identify`의 HMAC 증명을 확인한다. 틀리면 토큰을 보내지 않고 `identity-failed`로 종료 2.
  3. handshake(instance 일치, epoch).
  4. `bench.list`에서 run을 찾는다.
  5. `run.replay`.
  6. event ticket을 받고 최소 RFC6455 WebSocket(마스킹, ping/pong, 조각 처리)으로 hello를 받는다.
  7. 소유자 prompt의 에코를 live로 받는다(replay 뒤 순번).
  8. `run.cancel` → run이 목록에서 사라지는지 확인한다.
  - JSON 보고를 한 줄로 출력한다. `result == "ok"`일 때만 종료 0이다. loopback 외 주소는 거부하고, Origin을 보내지 않으며, 프록시를 쓰지 않는다.
- 실제 바이너리 검증 `apps/agentic-workbench-server/tests/owner_check_script.rs`(dev-dep `serde_json`):
  - `owner_check_script_observes_and_cancels_a_run_after_the_desktop_left`:
    - 실제 `agentic-workbench-server serve`를 띄우고 PID를 추적한다(실패 시 SIGKILL). 안내 파일 `verify`를 통과할 때까지 기다린다.
    - 창 토큰으로 실제 ACP 에코 run `r1`을 띄우고, 시작 에코 정착을 기다린다.
    - retire(`closeBench:false`)와 lease release를 한다. 이때 agent는 살아 있다.
    - `owner-check.py --data-dir <data> --run-id r1`을 실행한다. 종료 0, `result/identify/liveEcho/liveAfterReplay/cancelled`를 확인한다. 출력에 소유자 토큰이 없다.
    - agent 프로세스가 끝난다. 서버는 SIGTERM에 종료 0.
  - `owner_check_script_sends_no_owner_token_to_an_impostor_endpoint`:
    - 가짜 TCP 끝점이 틀린 증명을 준다. 스크립트는 종료 2 / `identity-failed`이다.
    - 끝점이 받은 요청에는 `authorization` 헤더도 토큰 문자열도 없다.
  - `t035-script-1.log` 종료 0, 2 passed, filtered out 0.
  - **바로 통과했다**(스크립트와 시험을 함께 작성). impostor 시험이 증명 실패 경로를 판별한다.
- 운영 빌드 제외:
  - `cargo check --release`(AW src-tauri, `t035-release-check-1.log`) 종료 0. probe 모듈과 command 등록은 `debug_assertions`에서만 존재한다.
  - `VITE_AW_DEBUG_PROBE` 없이 `pnpm build`(`t035-prod-dist-1.log`) 종료 0. `dist`에서 `__awDebug`·`report_app_probe`·`ready-to-quit`·`dropEventSockets`는 각각 0개 파일이다.

## 최종 실행 (T034·T035 fork)

- `cargo test -p workbench-host` (`final-host-1.log`, status 파일 기록) 종료 0. 모든 target의 filtered out은 0이다(45·2·1·2·1·6·0 passed).
  - 뒤이은 lint 수정은 시험 파일 한 줄만 바꿨다. 해당 target만 다시 돌렸다(`final-host-owner-2.log` 종료 0, 2 passed).
- `cargo test -p agentic-workbench-server`: 첫 실행 `final-server-1.log` 종료 0. lint 수정 뒤 재실행 `final-server-2.log` 종료 0(0·2·5 passed, filtered out 0).
- `cargo clippy -p workbench-host -p agentic-workbench-server --all-targets -D warnings`:
  - 첫 실행 `final-clippy-ws-1.log` 종료 101. `while_let_loop`(owner_check_script.rs), `redundant_closure`(owner_after_desktop.rs).
  - 수정 뒤 `final-clippy-ws-2.log` 종료 0.
- AW src-tauri `cargo test` (`final-aw-test-1.log`) 종료 0, 101·0·1·0 passed, filtered out 0.
- AW src-tauri `cargo clippy --all-targets -D warnings` (`final-aw-clippy-1.log`) 종료 0.
- 끝난 뒤 `fake_acp_permission_agent`·`agentic-workbench-server serve` 잔여 프로세스는 없다.

## fork 결과 확인 (T034–T035, 메인 세션)

- 로그 직접 확인: server 7/0, host owner 2/0(lint 수정 뒤), AW `cargo test` 102/0, clippy 두 대상 0. filtered out은 모두 0이다. 남은 서버·agent 프로세스는 없다.
- lint 수정 뒤 host 전체 로그가 없어, 메인 세션이 `cargo test -p workbench-host`를 한 번 실행했다(`t035-host-full-after-lint-1.log`): 종료 0, 57 passed, filtered out 0.
- T034에는 제품 red가 없다(구성 요소가 이미 있었음). mutation(`lease.release`가 작업대를 닫게 함)으로 시험이 잡아냄을 확인했다(fork 기록).

## T037 교환 전달 continuation (K) — fork

- 시험 `crates/workbench-core/tests/exchange_delivery_drain.rs` 7개(유효 전달 1회 수락·소비 표시, 같은 교환 동시 전달 8개 → 효과 1, 같은 키 다른 prompt 동시 → 효과 1, 조건별 거절(키·run·없는 교환·다른 작업대·draft·rejected 확인·steer) → 소비 0, 비우는 중에는 유효 K만 수락, E2 실제 `AcpRunEngine`+가짜 ACP agent로 다른 prompt가 turn을 쥔 동안 온 전달이 그 뒤에 1회 전달).
- red 1 `t037-red-1.log` 종료 101: **컴파일 red**(시험 코드의 `Workbench` trait import 누락). 제품 red 아님.
- red 2 `t037-red-2.log` 종료 101: **행동 red** 5 failed / 2 passed(비우는 중 일반 prompt가 수락됨, 교환 소비 표시 없음, 동시 전달 단일 효과 실패, 틀린 키 수락, E2 소비 없음). 같은 키 동시 시험은 기존 epoch 멱등으로 이미 통과(red 아님).
- 구현: `WorkGate::begin_exchange_delivery`(G 아래 `stopping` 거절·소비 표시·X-deliver 예약), `record_failed_delivery`; `run_service::deliver_exchange`(K 조건 판정, 엔진 **대기열** 경로 `queue_prompt`, 등록 실패 시 소비 유지 + 실패 기록, X는 대기열 등록이 A-turn을 잡은 뒤 해제); `BenchServices::attach_work_gate`; handler에서 continuation은 `send`만, 비우는 중 일반 prompt·steer·cancelAndSend는 `draining`(notApplied).
- green `t037-green-1.log` 종료 0: 7 passed, 0 failed, 0 filtered out.
- mutation `t037-mut-1.log` 종료 101: 전달을 `send_prompt`(즉시 전송)로 바꾸면 E2 시험이 교환 prompt가 agent에 도달하지 않아 실패한다(원본 복원 확인).
- 이탈: T040(입구 판정) 중 run prompt 계열의 비우는 중 거절만 handler에 최소로 넣었다(T037 시험의 "비우는 중 유효 K만 수락"에 필요). `WorkbenchRuntime::call`의 operation 공통 판정은 T040에 남는다.
- 커밋 715ec2d.

## T038 대기 task 배정 원자성·시작 장벽 (E3·E4·F1) — fork

- 시험 `crates/workbench-core/tests/child_assign_atomic.rs` 6개(+ support 1):
  - E3: 서로 다른 키 동시 배정 100회 → 엔진 기동 1회, 모든 응답이 같은 run, 노드 run 일치.
  - 배정·취소 경합 20회 → 취소 성공이면 그 task의 run이 살아 있지 않음, 관문 예약 0.
  - 시작 장벽 지점별 취소(registry 예약 전 = `BeforePrepare`, spawn 뒤·attach 전 = 스크립트 엔진 `prepare_hook`, attach 뒤·전이 전 = `AfterPrepare`, 전이 뒤·장벽 전 = `AfterRegister`) → 취소 성공, launcher·prompt 표지 0, 관문 예약 0, 준비한 run 정리, 노드 예약 해제, task `cancelled`.
  - 같은 지점별 배정 future abort → 실행 0, 예약 0, task는 다시 배정 가능(`ready`).
  - 바인딩 뒤 취소 → 실제 run 취소. `bind_child_run`이 취소된 task 거절.
  - 지점 멈춤은 결정적 gate(semaphore). abort 뒤 정리(guard가 뒤에서 하는 저장소 예약 해제·닫힌 장벽의 run 정리)는 제한 시간 안 조건 polling으로 관찰한다(순서 제어용 지연 없음).
  - "spawn 뒤·attach 전"은 엔진 내부 지점이라 스크립트 엔진으로 모사했다. 운영 `AcpRunEngine`의 같은 지점은 acp-agent-core `start_gate` 단위 시험(T013)이 덮는다. `AcpRunEngine::start_gated`는 `execute_gated`로 이어진다.
- red 1 `t038-red-1.log` 종료 101: **컴파일 red**(관측 지점 API 없음).
- red 2·3 `t038-red-2.log`·`t038-red-3.log` 종료 101: 관측 지점만 추가(원자성 수정 없음)한 뒤의 실행. 시험의 기준선 버그(첫 과제의 기동 표지·run 수를 세던 것)가 섞여 무효로 두고 시험을 고쳤다.
- red 4 `t038-red-4.log` 종료 101: **행동 red** 4 failed / 3 passed — 100회 배정 기동 2회 이상, `BeforeReserve` 취소 뒤 배정이 active run 보고, abort 뒤 노드 예약 남음, 취소된 task에 bind 성공. 경합 시험(20회)과 바인딩 뒤 취소 시험은 수정 전에도 통과(red 아님; 경합 시험은 회귀 방지용으로 둔다).
- 구현:
  - `RunEngine::start_gated`(필수 메서드): `AcpRunEngine`은 `StartAgentRunUseCase::execute_gated`, 스크립트 엔진은 준비(슬롯·실행 task spawn) 뒤 장벽이 열려야 실행(`launch:` 표지), 닫힌 채 drop이면 슬롯 삭제.
  - `EngineAgentWorker::prepare_worker`(시작 장벽 경로), `start_worker`는 오늘과 같다.
  - `service.reserve_child_run` 비교 후 변경(`ready`/`running` + 노드 run 없음 → 예약, 있으면 `Existing(run)`, 종료 task 거절), `bind_child_run`이 `Cancelled` 거절, `cancel_launching_task`(토큰 전이가 선형화 지점이라 revision 비교 없음).
  - `OrchestrationRuntime`: 기동 토큰 표(task → 토큰·예정 run, 단일 비행), `prevent_task_launch`, 관문 접근(관문 없는 조립은 자체 관문), 시험용 `set_launch_probe`(`test-hooks`).
  - `start_child`: `issue_launch`(T-start) → 단일 비행 → 저장소 RMW → 준비 → G 아래 `register_launch`(T→A 인계) → 장벽 열기 → 바인딩. 취소면 5'(장벽 닫힌 채 drop + 준비한 run 취소). 정리 guard가 abort 때 저장소 예약 해제·준비 run 취소·scheduler 자리 반납을 뒤에서 한다.
  - 취소 두 경로(coordinator 도구, 데스크톱 `cancelTask`): 노드에 예정 run이 있으면 토큰을 먼저 본다. `Prevented`면 task만 취소하고 scheduler 자리를 반납한다. `Registered`면 오늘 경로가 실제 run을 취소한다.
- 기존 시험 조정: `orchestration_agent.rs`의 첫 턴 hook 시험 2개는 실행이 장벽 뒤 따로 돌게 되어, `AfterOpen` 지점에서 hook 완료 허가를 기다리게 했다("엔진 등록 직후·노드 기록 전" 순서를 결정적으로 유지, 기대값 변경 없음).
- green `t038-green-1.log` 종료 0: 7 passed, 0 failed, 0 filtered out.
- mutation `t038-mut-1.log` 종료 101: `prevent_task_launch`가 항상 `Unknown`을 돌려주게 하면 `BeforeReserve` 지점에서 실행 표지가 생겨 실패(원본 복원 확인).
- 회귀: `cargo test -p workbench-core --features test-hooks` (`t038-core-full-1.log`) 종료 0, 56 target, 463 passed, 0 failed, 모든 target filtered out 0.
- 이탈·설명: 기동 토큰은 저장소에 쓰지 않고 메모리 표에 둔다(토큰은 이 프로세스의 작업 관문에서만 뜻이 있고, 저장소에는 노드 `Starting` + 예정 run이 남는다). 스크립트 엔진의 `active_owner_of`는 장벽 전 준비 run도 소유자로 본다(운영은 세션이 붙어야 active) — 기존 시험의 타이밍 의존을 늘리지 않기 위함.

## T039 알림 전달 예약·회수 (F2·G1·G2) — fork

- 시험 `crates/workbench-core/tests/notification_reservation.rs` 6개(+ support 1):
  - F2: 전달기 첫 poll(`FirstPoll`)을 막은 채 보고 호출이 돌아가고 자식 turn도 없음 → `active_work().notifications == 1`, `try_stop(|| 0)` 거절 → 풀면 `delivered`(시도 1, coordinator prompt 1) → 예약 0 → `try_stop` 통과.
  - 재시도 가능 실패(스크립트 엔진 `send_and_wait` 1회 실패) → 외부 호출 없이 서버가 backoff로 재전달 → `delivered`, 시도 2.
  - G1: `Dispatching{attemptId}` 저장 직후(`AfterDispatchingSaved`) 전달 future abort → 회수·재전달, 시도 2, attemptId 바뀜, coordinator prompt 1.
  - G1: 결과 저장 실패 주입(`BeforeResultSave` → `FailResultSave`) → 회수·재전달, 시도 2, prompt 2.
  - G2: 결과 transaction 직전에 회수 한 바퀴 → 회수 0, 알림 변경 0 → 풀면 같은 attemptId로 `delivered`, prompt 1(재전달 0).
  - G2: 같은 지점 abort → 회수되어 재전달 1회(prompt 2), 예약 0.
  - "wait-stop이 끝나지 않음"은 `WorkGate::try_stop`(파생 활동 0으로 넘김)과 `active_work`로 단정했다. host 정지 상태 기계와 저장소 파생 활동(미전달 알림 수)은 T041이다.
- red 1 `t039-red-1.log` 종료 101: **컴파일 red**(probe·회수·실패 주입 API 없음).
- red 2 `t039-red-2.log` 종료 101: 관측 지점만 추가(예약·회수 없음)한 뒤 **행동 red** 5 failed / 2 passed — N-notify 없음, attemptId 없음, 재시도 실패가 `failed`로 남음, abort·저장 실패 뒤 `dispatching`으로 남음. G2 "살아 있는 시도 비회수" 시험은 회수 stub(0)으로 통과(red 아님, 구현 뒤 의미 있는 단정).
- 구현:
  - 도메인 `CoordinatorNotification.attempt_id`(serde default·None이면 직렬화 생략), 프로토콜 DTO `attemptId`(선택), `workbench.openapi.json`·`packages/workbench-client/src/generated/workbench.ts` 재생성(`pnpm run generate`, `t039-generate-1.log` 종료 0; `check-types` `t039-client-types-1.log` 종료 0).
  - `WorkGate::reserve_notify(attemptId)`(N-notify, `stopping`이면 실패)·`notify_attempt_live`.
  - `CoordinatorNotificationDispatcher::dispatch_pending_with(bench, first)`: 시작 때 회수 한 바퀴, 전달마다 시도 id + N-notify(`NotifyAttempt`) → `Dispatching{attemptId}` 저장 → 전달 → 결과는 이 시도가 소유한 `Dispatching`일 때만 저장 → **commit 뒤에만** 예약 해제. 시도 guard가 결과 저장 없이 drop되면 예약을 먼저 놓고 `Orphaned`를 알린다. 한 바퀴 끝에 재시도 가능 실패가 남으면 `RetryableFailures`.
  - `reclaim_orphaned`: 저장소에서 `Dispatching{attemptId}` 후보를 읽고(경계 밖에서) 예약 없는 시도만 같은 id일 때 `Failed(retryable, runtimeLost)`로 되돌린다(관문 G를 저장소 경계 안에서 잡지 않아 잠금 순서 역전 없음).
  - `OrchestrationRuntime`: 보고 도구가 C-call을 놓기 전에 첫 시도(N-notify)를 잡아 전달기로 넘김, `Orphaned` → 새 한 바퀴(시작 때 회수), `RetryableFailures` → backoff(50ms부터 두 배, 상한 5초) 재전달(대상 coordinator run이 살아 있을 때만, 작업대당 하나), 재시작 복구 경로도 같은 전달 함수를 쓴다. 시험용 `set_dispatch_probe`·`last_notification_pass`(`test-hooks`), `reclaim_notifications`.
- green `t039-green-1.log` 종료 0: 7 passed, 0 failed, 0 filtered out.
- mutation:
  - `t039-mut-1.log` 종료 101: 전달이 돌아오자마자 N-notify를 놓게 하면(A-turn 인계와 같은 구간 생김) G2 "살아 있는 시도 비회수"가 실패.
  - `t039-mut-2.log` 종료 101: 보고 도구가 첫 N-notify를 넘기지 않게 하면 F2가 실패. 두 경우 모두 원본 복원 확인.
- 회귀: `cargo test -p workbench-core --features test-hooks` (`t039-core-full-1.log`) 종료 0, 57 target, 470 passed, 0 failed, 모든 target filtered out 0. `cargo test -p workbench-protocol` (`t039-protocol-1.log`) 종료 0, 45 passed, filtered out 0.
- 남은 것: 비우기 진입 때 서버가 알림 한 바퀴를 도는 연결은 비우기 상태 기계(T041)에서 `spawn_notification_pass`를 부르면 된다 — 이번에는 넣지 않았다.

## 최종 실행 (T037–T039 fork)

- `cargo clippy -p workbench-core -p workbench-protocol -p workbench-host -p acp-agent-core --features workbench-core/test-hooks --all-targets -D warnings`:
  - 첫 실행 `f3739-clippy-1.log` 종료 101: `notification_reservation.rs`의 쓰지 않는 대입(`last`).
  - 시험 helper를 고친 뒤 `f3739-clippy-2.log` 종료 0.
- lint 수정 뒤 workbench-core 전체 재실행 `cargo test -p workbench-core --features test-hooks` (`f3739-core-1.log`) 종료 0: 57 target, 470 passed, 0 failed, 모든 target filtered out 0.
- `cargo test -p acp-agent-core` (`f3739-acp-1.log`) 종료 0: 2 target, 97 passed, filtered out 0. 이번 fork는 acp-agent-core를 바꾸지 않았다(ask-code·hushline 재검증 대상 아님).
- `cargo test -p workbench-host` (`f3739-host-1.log`) 종료 0: 7 target, 57 passed, filtered out 0.
- AW src-tauri `cargo test` (`f3739-aw-1.log`) 종료 0: 4 target, 102 passed, filtered out 0. `cargo clippy --all-targets -D warnings` (`f3739-aw-clippy-1.log`) 종료 0.
- 끝난 뒤 가짜 ACP agent·서버 잔여 프로세스 없음.

## fork 결과 확인 (T037–T039, 메인 세션)

- green 로그 직접 확인: T037·T038·T039 각각 7 passed, 0 filtered out.
- 최종 실행 `f3739-*`(clippy 수정 뒤): core 470/0, acp-agent-core 97/0, host 57/0, AW 102/0, AW clippy 0. filtered out은 모두 0이다.
- fork가 돌리지 않은 `agentic-workbench-server`를 메인 세션이 실행했다: `cargo test -p agentic-workbench-server` 종료 0(7/0), clippy `-D warnings` 종료 0.
- 디스크 여유가 4.6GB로 줄어, 재빌드로 복구 가능한 산출물을 정리했다(main 저장소 `target` 전체, 044 `target/release`). 정리 뒤 여유는 11GB다.
