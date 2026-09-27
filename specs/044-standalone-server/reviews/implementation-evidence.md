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
