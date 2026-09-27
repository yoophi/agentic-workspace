# 043 구현 증거 기록

각 시험은 한 번 실행했고, 로그를 보존한 뒤 원 명령의 종료 코드를 기록했다. 명령에는 `CARGO_INCREMENTAL=0`을 붙였다.

## T004 창 격리 (`crates/workbench-core/tests/window_isolation.rs`)

| 단계 | 명령 | 종료 코드 | 의미 |
|---|---|---|---|
| red (컴파일) | `cargo test -p workbench-core --features test-hooks --test window_isolation` | 101 | `AuthenticatedPrincipal::desktop_window`가 없어 **컴파일 실패**. 권한 동작의 증거가 **아니다** |
| green | 같은 명령 (T007 생성자 추가 뒤) | 0 | 3 passed |
| 동작 변이 | `desktop_window`가 공용 주체 `desktop`을 돌려주게 바꾼 뒤 `... --test window_isolation another_window` | 101 | `desktop run.start must be rejected` — 창별 주체가 없으면 다른 창이 작업대를 조작한다. **이 변이가 권한 판정의 증거다.** 변이를 되돌린 뒤 diff를 확인했다 |

단정 범위:
- 주인 창 `desktop:window:session-1:inc-1`이 작업대를 연다. 이어서 run 시작, 교환 workspace 동기화, orchestration bootstrap을 해 실제 binding과 스트림 id(`eventStreamId`)를 얻는다.
- **대조**: 주인 창은 `run:r1`·`exchange:<bench>`·`bench:<bench>`·`orchestration:<binding>` 네 스트림을 모두 구독할 수 있다. 따라서 침입자의 거절은 스트림이 없어서 난 것이 아니다.
- **다른 창**(`session-2:inc-1`)과 **같은 label의 새 incarnation**(`session-1:inc-2`)은 거절된다.
  - 조작 10종: run.start / sendPrompt / cancel / replay, exchange.syncWorkspace / list / send, orchestration.get / bootstrap, bench.close. 모두 `Forbidden`.
  - 구독 네 스트림: 모두 `Forbidden`. 문구는 run 스트림이 `run is owned by another bench.`, 나머지는 `bench belongs to another principal.`이다.
- 거절 시도 뒤에도 주인 창은 네 스트림을 계속 구독할 수 있고 교환 목록도 조회할 수 있다.
- 판정은 core의 **기존** 작업대 소유 규칙(`workbench_runtime::authorize_bench_streams`, `bench_service` 소유 확인)이 한다. 043은 주체를 창별로 나눌 뿐 판정 코드는 바꾸지 않았다.

## T005 창 토큰 폐기 (`crates/workbench-core/tests/http_window_tokens.rs`)

tasks.md에는 경로가 `crates/workbench-server/tests/`로 적혀 있다. 실제로는 HTTP harness(`tests/support/http_harness.rs`)가 있는 core `tests/`에 두었다. 042의 HTTP 시험과 같은 위치다.

| 단계 | 명령 | 종료 코드 | 의미 |
|---|---|---|---|
| red (컴파일) | `cargo test -p workbench-core --features test-hooks --test http_window_tokens` | 101 | `issue_for`와 두 `revoke_subject`가 없어 **컴파일 실패**. 동작 증거 아님 |
| green | 같은 명령 (T008 뒤) | 0 | 2 passed |
| 변이: 표 폐기 무효 | `EventTicketStore::revoke_subject`를 아무것도 하지 않게 바꿈 | 101 | `connect_ticket(&pending).is_err()` 실패 — 폐기 전에 받은 표로 구독할 수 있었다 |
| 변이: 토큰 폐기 무효 | `DesktopTokenIssuer::revoke_subject`를 아무것도 하지 않게 바꿈 | 101 | 폐기된 토큰의 호출이 401이 아님 |

단정 범위:
- 한 incarnation에 토큰 두 개를 발급하면 둘 다 폐기된다.
- 토큰은 발급 때 받은 창 주체로 풀린다. 다른 창 토큰의 `bench.close`는 `Forbidden`이다.
- 폐기 뒤 호출은 401, 표 발급은 `unauthenticated`이고, 폐기 전에 받은 표는 upgrade가 거절된다.
- 다른 창 토큰은 계속 OK다. 같은 label의 새 incarnation 토큰은 OK지만, 이전 작업대 닫기는 `Forbidden`이다.
- 이미 열린 WebSocket 구독은 폐기 대상이 아니다. 창이 Destroyed되면 WebView와 함께 연결이 닫히고, 이어지는 작업대 닫기가 작업대 스트림에 `evicted` gap을 보낸다. 창 수명 순서는 T050에서 확인한다.

## T006 보관 한도 재구독 (`crates/workbench-core/tests/retention_resubscribe.rs`) · T009

- **성격**: 새 동작을 만드는 시험이 아니다. 클라이언트 복구 절차(R8: live 먼저 → 버퍼 → 스냅샷 병합)가 기대는 **기존** 042 hub 동작을 고정하는 특성 시험이다. 그래서 red 단계가 없다. 이 시험이 실패하면 클라이언트 설계의 전제가 깨진 것이다.
- 명령: `cargo test -p workbench-core --features test-hooks --test retention_resubscribe`, 종료 코드 0, 1 passed.
- 조건: 실제 `EventHub`에 run·교환·orchestration journal 한도를 4로 두고 11건을 발행한다.
  - (a) `after = 0`: `RetentionExceeded`이고 `first = 8`, `last = 11`이다. 한 건을 더 발행해도 구독이 `None`으로 **끝난다**. 등록된 수신자가 없어 송신자가 모두 사라지므로, 시간 대기 없이 결정적으로 단정된다.
  - (b) `after = lastSequence(11)`: 12(이미 발행됨)와 13(live)을 연속으로 받는다.
- T009: hub 한도는 이미 `RuntimeAdapters.event_limits`와 `EventHub::new(epoch, EventHubLimits)`로 주입할 수 있다. **코드 변경 없음**이다. 시험 host(T016)도 이 경로를 쓴다.

## T010–T015 AW 창 주체 조립 · 창 수명 경합 판단 (T050 증거 일부)

`cd apps/agentic-workbench/src-tauri && CARGO_INCREMENTAL=0 cargo test` 종료 코드 0, 121 passed(기준선 117 + 새 시험 4).

### 구성

- `infrastructure/window_principals.rs`: 창 label마다 incarnation을 둔다.
  - `register`는 창 생성 직전에만 부른다.
  - `current`와 `incarnation`은 조회만 한다. **자동 생성하지 않는다.**
  - `retire(label, incarnation)`은 **같은 incarnation일 때만** 지운다.
- `infrastructure/window_lifecycle.rs`: 두 경로로 창을 등록한다.
  - `build_tracked`: 등록 → 창 만들기(실패하면 거둬들임) → 창별 `on_window_event` 처리기.
  - `adopt`: setup에서 설정 파일로 만든 `main` 창을 등록한다.
  - 창별 처리기는 **자기 incarnation을 붙잡고** 다음 순서로 정리한다: 거둬들이기 → 토큰·표 폐기(`WorkbenchHttpState::revoke_window`) → 네트워크 전달 선언 해제 → (session 창) 작업대 닫기. 작업대 닫기는 연 창 주체로 한다.
- 호환 command는 전부 `Caller { runtime, principal }`로 부른다(D2).
  - 창 인자가 없던 command에는 `window` 인자를 더했다. Tauri가 주입하므로 화면 호출은 그대로다.
  - 등록이 없는 창(닫힌 뒤 늦게 도는 command)은 `Window is no longer available.`로 거절한다. `list_agents`만 오늘 계약대로 빈 목록을 돌려준다.
  - 운영 코드에 남은 공용 `AuthenticatedPrincipal::desktop()`은 0개다. 남은 두 곳은 시험 코드다.
- 새 command:
  - `ensure_window_bench(open, hint)`: `open=false`면 lookup이고 `null`일 수 있다.
  - `declare_network_delivery(incarnation)`: 현재 incarnation일 때만 받는다.
  - `get_workbench_connection`은 창 주체 토큰과 `incarnation`을 돌려준다.
- 전달 표(`tauri_desktop_bridge`): 선언한 창의 현재 incarnation이면 삽입 전달을 건너뛴다. 창 제목 적용(`set_title`)은 표현 상태라 경로와 상관없이 앱이 한다. 화면 알림 삽입만 건너뛴다.

### 경합 판단(사용자 지적: Destroyed 뒤 늦은 command, 같은 label 재개 창과 옛 정리의 엇갈림)

| 경합 | 판단 | 근거·고정 |
|---|---|---|
| 닫힌 뒤 늦게 도는 command가 새 incarnation을 만듦 | **없앰** | 조회는 만들지 않는다. 시험 `window_principals::lookups_never_create_an_incarnation`과 `a_registered_window_keeps_its_principal_until_its_own_retire`(retire 뒤 `current == None`)으로 고정. `caller()`는 이 경우 거절한다 |
| 같은 label로 새 창을 먼저 등록한 뒤 옛 창의 정리가 늦게 돌아 새 등록을 지움 | **없앰** | 정리는 전역 창 이벤트가 아니라 창별 처리기가 자기 incarnation으로 한다. 시험 `a_late_retire_of_the_old_incarnation_does_not_remove_the_reopened_window`, `tauri_desktop_bridge::only_the_current_incarnation_can_declare_and_skip_delivery`(옛 incarnation 선언 거절, 늦은 forget이 새 선언을 지우지 않음) |
| 새 창 등록 뒤 옛 창의 command가 늦게 실행되어 새 창 주체로 풀림(승격) | **이론상 남음 — 작업대에 묶인 호출에는 해당 없음** | Tauri 2.11 async command는 `window`를 포함한 인자를 future 안에서 추출한다(`tauri-macros-2.6.3/src/command/wrapper.rs` `body_async`: `respond_async_serialized(async move { $path(#args?) })`). `Window`에는 label 말고 인스턴스 식별자가 없다. 그래서 수신 시점에 주체를 붙잡을 수 없다. 다만 session 창 label은 매번 새 id라 재사용되지 않는다(`window_manager::open_session_window` → `session_label(&new_session_id())`). 작업대에 묶인 호출(run·교환·orchestration·작업대)은 session 창에서만 나온다. label을 다시 쓰는 창은 `settings`와 `main`(설정 파일)이고, 이 창들의 호출은 작업대와 무관하다(`reviews/command-inventory.md`). 그래서 승격돼도 다른 창의 작업대에 닿지 않는다 |

결정적 시험으로 고정한 것은 위 세 시험이다. 실제 창을 띄우는 순서 시험은 T050(창 닫기 수명)과 T054(앱 스모크)에서 한다.

## T016 시험 host (`crates/workbench-core/examples/http_test_host.rs`, `required-features = ["test-hooks"]`)

- 구성: 운영 router, 실제 런타임과 hub, 가짜 run 엔진(prompt마다 run 이벤트 하나), journal 보관 한도 4(`HOST_JOURNAL_CAPACITY`), 창 주체 두 개에 묶인 고정 토큰.
- 시험 전용 operation은 없다. 시나리오는 운영 operation으로 만든다.
- 수동 확인:
  - 준비되면 JSON 한 줄을 출력한다.
  - `host-window-a` 토큰으로 `project.list` 호출이 `{"kind":"complete","output":[]}`를 돌려준다.
  - stdin을 닫으면 프로세스가 끝난다.
- dev-dependency tokio에 `io-std`·`io-util` 기능을 더했다. 이것은 example이 stdin을 읽기 위한 dev 전용 변경이다.

## T017–T019 · T021–T023 호출 클라이언트·연결 수명·오류 문자열 (`packages/workbench-client/src`)

| 단계 | 명령 | 종료 코드 | 의미 |
|---|---|---|---|
| red | `npx vitest run src/fault-string.test.ts` / `src/call-client.test.ts src/connection.test.ts` | 1 / 1 | 모듈이 없어 import 실패. 동작 증거 아님 |
| green | `npx vitest run src/fault-string.test.ts src/call-client.test.ts src/connection.test.ts` | 0 | 23 passed(재발견·세대 경계 시험 포함) |
| 패키지 | `pnpm --filter @yoophi/workbench-client test`, `check-types` | 0, 0 | — |
| 변이: 재연결 때 끝점 재발견 제거 | `connection.attempt`가 실패 뒤 `fetchConnection` 없이 옛 URL로 handshake | 1 | 재시작 시험 3개 실패(새 끝점을 찾지 못함) |
| 변이: 401 갱신 뒤 세대 확인 제거 | `sendOnce`의 `boundEpoch` 비교 삭제 | 1 | `does not resend an uncertain mutation when a 401 refresh moves it to a new epoch` 실패 |

규칙(사용자 검토 반영):
- **재연결**: 각 시도는 지금 끝점으로 handshake하고, 실패하면(401뿐 아니라 연결 거부 포함) `fetchConnection`으로 연결 정보를 다시 받아 새 끝점으로 handshake한다. 서버가 다른 포트로 다시 떠도 찾는다.
- **자격 증명 갱신**: 갱신으로 `baseUrl`이 바뀌면 곧바로 handshake해 세대를 갱신하고 `epochChanged`를 알린다.
- **변경의 시도별 세대 경계**: 첫 시도가 응답 유실이면 이후 모든 재시도는 처음 보낸 세대에 묶인다. 재연결 handshake의 세대가 다르면 재전송 0회다. 재시도 중 401 갱신으로 세대가 바뀌어도 재전송 0회다. 401 자체는 적용 전 거절이므로, 앞선 불확실 시도가 없는 첫 시도는 갱신 뒤 다시 보낸다.
- **조회·변경 구분**: `OPERATION_KINDS: Record<OperationId, …>`(85개, 시험 host의 `system.describe`에서 뽑음). 누락이나 초과가 있으면 타입 검사가 실패한다. 실제 서버와의 대조는 통합 suite(T041)에서 한다.
- **`faultToString`**: compat Rust와 같다. 교환은 `details.exchangeCode`가 있을 때만 JSON이다(계약 문서의 `?? code`는 compat 코드와 달라 T055에서 문서를 고친다).
- **`onState`**: 구독 즉시 현재 상태를 한 번 알린다(화면 표시용). 처음 쓴 시험 기대값에 이 첫 알림이 빠져 있어 계약을 명시하는 쪽으로 고쳤다.

## T020 · T024 · T025 command 표 동등성·transport·저장소 이관

| 단계 | 명령 | 종료 코드 | 의미 |
|---|---|---|---|
| golden 생성 | `cd apps/agentic-workbench/src-tauri && UPDATE_GOLDEN=1 cargo test --lib compat_parity` | 101 → 수정 뒤 0 | 처음 실패는 제 추정 command 수(60)가 틀려서다. 실제 서버 소유 command는 **61개**다. 파일은 compat 코드로 계산됐다 |
| golden 확인 | `cargo test --lib compat_parity` | 0 | 72 사례, 61 command. DTO를 통째로 넘기는 사례 6개(`save_agent_run_settings`, `start_agent_run`×2, `list_agent_tool_command_candidates`, `sync_agent_workspace`, `send_agent_exchange`, `acknowledge_agent_exchange`)는 화면 원본(`wire`)과 compat 입력이 **서버의 operation 입력 DTO로 같다**는 것을 Rust가 확인한다 |
| TS red | `npx vitest run src/shared/api/transport/command-table.parity.test.ts` | 1 | **실제 불일치 1건**: `update_goal`의 `tokenBudget: null`. compat은 생략하고, TS 표는 코드 주석("null 포함 그대로")을 따라 null을 보냈다 |
| TS green | 같은 명령 | 0 | 73 passed(72 사례 + 표와 golden의 command 목록 일치) |
| transport red→green | `npx vitest run src/shared/api/transport/` | 1 → 0 | `http-transport` 모듈 없음 → 82 passed |
| AW 전체 | `pnpm --filter @yoophi/agentic-workbench test`, `tsc --noEmit` | 0, 0 | 83 files / 511 tests(기준선 429 + 82). 기존 화면 시험은 기본값(호환 경로)으로 통과 |

관찰(기존 동작, 043이 바꾸지 않음):
- `update_goal`의 compat 인자 타입은 `Option<Option<usize>>`지만 serde 기본 역직렬화는 JSON null을 바깥 `None`으로 읽는다. 그래서 화면이 `tokenBudget: null`을 보내도 **예산 지우기가 서버에 전달되지 않는다**. `workbench_compat::goal_update_input`의 주석과 실제 동작이 다르다.
- 네트워크 경로는 동등성을 위해 같은 동작을 따른다. 고칠지는 별도 이슈로 판단한다.

transport:
- `shared/api/transport`:
  - `invoke`는 창이 정한 transport로 간다. 기본은 호환 경로다.
  - `HttpTransport`는 command 표로 작업대를 확보하고(`ensure_window_bench(open, hint)`), 작업대가 없을 때의 결과·오류, 결과 변환(`sessionForWindow`), 오류 문자열(교환·orchestration), 항상 성공하는 command(`list_agents`, replay의 Missing)를 compat과 같게 돌려준다.
  - 적용 안 됨(`notApplied`)과 결과 불명(`unknown`)은 새 문구로 보여 준다.
- 저장소 13개가 공용 `invoke`를 쓴다. 데스크톱 표현 command(`open_worktree_window`, worktree 감시)는 Tauri `invoke`를 그대로 쓴다.

## T025–T028 · T051(선행) 부팅 경로·화면 시험 두 경로·직접 호출 가드

| 항목 | 명령 | 종료 코드 | 결과 |
|---|---|---|---|
| 부팅 선택(T026, T051 시험 선행 작성) | `npx vitest run src/app/bootstrap-transport.test.ts` | 0 | 7 passed. **시험과 구현을 함께 써서 red를 따로 기록하지 못했다.** 대신 변이로 확인했다: 전달 선언 실패를 무시하게 바꾸면 `stays on the compat path when the delivery declaration fails` 실패(종료 1) |
| 가짜 서버 역변환 | `npx vitest run src/shared/api/transport/testing` | 0 | golden 72 사례에서 정변환(역변환(입력)) = 입력 |
| 화면 시험 두 경로(T027) | `npx vitest run src/features/agent-run/ui/agent-run-panel.test.tsx` | 1 → 0 | 네트워크 경로에서 처음 3개 실패 → 대기 조건을 고친 뒤 26 passed(9 시나리오 × 2 경로 + 기존 harness 시험) |
| 반복 | 같은 명령 10회, 회차마다 로그·종료 코드 | 10 × 0 | 10/10 `26 passed` |
| 직접 호출 가드(T028) | `npx vitest run src/shared/api/transport/no-direct-invoke.test.ts` | 0 | 4 passed. 변이(`project-repository`를 Tauri `invoke`로 되돌림)에서 실패(종료 1), 위반 목록에 `list_projects`·`create_project`… |
| AW 전체 | `pnpm --filter @yoophi/agentic-workbench test`, `tsc --noEmit` | 0, 0 | 86 files / 607 tests |

T027 범위를 정직하게 적는다:
- 기존 AW 화면 시험 중 서버 소유 command를 **실제로 거치는** 시험은 `agent-run-panel.test.tsx` 하나다.
  - Tauri `invoke`를 command별 가짜 응답으로 흉내 낸다.
  - 나머지 화면 시험은 소스 문자열 검사, 순수 모델 시험, props 기반 컴포넌트 시험이라 transport와 무관하다.
  - Tauri를 흉내 내는 다른 두 시험(`appearance-preferences-repository`, `settings-window-repository`)은 데스크톱 표현 command다.
- 따라서 FR-012/SC-002의 "새 경로에서 기존 시험 통과"는 이 시험의 9개 시나리오를 네트워크 경로로 돌린 것이 근거다. 이벤트 화면 시험은 T038에서 늘린다.
- 네트워크 경로 구성:
  - 시험 안에서 실제 루프백 HTTP 서버(node `http`, 042 서버와 같은 CORS 응답)를 띄운다.
  - 실제 `createConnection`(handshake), `createWorkbenchClient`, `HttpTransport`, 실제 `fetch`(happy-dom, CORS 적용)를 거친다.
  - 가짜 서버는 operation 입력을 command 인자로 되돌려 같은 가짜 응답 함수를 부른다. 그래서 `invocationsFor(...)` 단정이 두 경로에서 같은 뜻이다.
- **첫 실행의 CORS 거절**: happy-dom `fetch`가 CORS를 적용해 가짜 서버 요청이 막혔다. 가짜 서버에 042 `CorsLayer`와 같은 응답(요청 출처 반사, `POST,GET`, `authorization,content-type`)을 넣었다. 실제 WebView 요청이 042 preflight 응답으로 통과한다는 근거는 core 시험 `preflight_answers_only_allowed_origins_without_credentials`와 042 probe다.
- **대기 조건 수정(단정은 그대로)**: 실패 3개의 원인은 transport 동작이 아니라 시험의 대기 조건이 부정확했던 것이다.
  - 목록 상자는 먼저 "Loading commands..."로 뜨는데, 시험은 목록이 나타나자마자 내용을 단정했다.
  - 모델 버튼은 agent 목록을 불러온 뒤 생기는데, 시험은 그 전에 Enter를 눌렀다.
  - 호환 경로의 가짜 `invoke`는 마이크로태스크 안에 끝나 이 차이가 가려져 있었다.
  - 기대값은 바꾸지 않았다. 대기 조건을 "후보를 다 불러옴"과 "모델 버튼 있음"으로 정확히 했다.

## T029 · T030 · T033 이벤트 클라이언트 기본·수신자 계약 (`packages/workbench-client/src/event-client.ts`)

| 단계 | 명령 | 종료 코드 | 의미 |
|---|---|---|---|
| red | `npx vitest run src/event-client.test.ts src/event-client.listeners.test.ts` | 1 | 모듈 없음(import 실패). 동작 증거 아님 |
| green | 같은 명령 | 0 | 10 passed. 처음 15로 보인 것은 기본 시험 파일을 import해 기본 5개가 두 번 돈 것이다. 도우미를 `testing/event-client-harness.ts`로 옮겼다 |
| 변이: 수신자 실패를 반영 완료로 침 | `pump`의 catch에서 재동기 대신 계속 진행 | 1 | 재동기 시험 2개 실패(거절, 동기 예외) |
| 변이: settle 전에 cursor 전진 | `onEvent` 호출 전에 `delivered` 갱신 | 1 | 순차 처리 시험, 재연결 cursor 시험 실패 |
| 변이: 재연결 cursor를 최댓값으로 | `cursor()`를 max로 | 1 | `reconnects from the minimum applied cursor…` 실패 |

- 가짜 hub(`testing/fake-event-hub.ts`)는 042 `decide_existing`·`decide_missing`의 cursor 판정과 "hello는 등록 뒤" 순서를 흉내 낸다. 전달은 시험이 `publish`할 때 동기로 일어나고, 기다림은 마이크로태스크만 돈다(시간 대기 없음).
- 단정 범위:
  - Promise settle 뒤에만 그 수신자의 cursor가 전진하고, 한 번에 하나씩 처리한다.
  - 거절·동기 예외는 그 수신자만 스냅샷으로 재동기한다. 스냅샷이 덮은 순번은 다시 받지 않고 그 뒤만 받는다. 다른 수신자는 계속 받는다.
  - 재연결 cursor는 반영 완료의 최솟값이다(받았지만 반영 전에 끊긴 경우 포함). 이미 반영한 수신자에게는 다시 넘기지 않는다.
  - 수신자 교체 중에 도착한 이벤트는 새 수신자가 받는다.
  - 대기열 상한을 넘으면 소켓을 닫고, 다음 수신자가 붙을 때 cursor에서 다시 구독한다.
  - 마지막 수신자가 떠나고 유예가 지나면 소켓을 닫는다.
- T032(전달 끄기 단위 시험)는 Foundational에서 Rust `tauri_desktop_bridge::only_the_current_incarnation_can_declare_and_skip_delivery`로 넣었다. T037(부팅의 전달 선언)은 `bootstrap-transport`에 있다(선언 실패 → 호환 경로, T051 시험).

## T031 · T034 · T035(모델) 교환 원장·구독 이관·네트워크 이벤트 계층

| 항목 | 명령 | 종료 코드 | 결과 |
|---|---|---|---|
| 교환 원장 red→green | `npx vitest run src/features/agent-run/model/exchange-reconciler.test.ts` | 1 → 0 | 모듈 없음 → 7 passed |
| 네트워크 이벤트 | `npx vitest run src/shared/api/transport/network-events.test.ts` | 1 → 0 | 첫 실패는 가짜 hub가 모든 이벤트를 `test.v1` schema로 발행해서다. schema를 받게 고친 뒤 2 passed |
| 변이: 교환 `passes`를 항상 참(검토한 초안 상태) | — | 1 | `does not replay requested or status events older than the snapshot` 실패 |
| 변이: worktree 재연결 재설정 끔(검토한 초안 상태) | — | 1 | `sends a full re-read signal after reconnecting …` 실패 |
| 변이: 네트워크 창에 창 삽입으로 run 이벤트를 넣음 | `agent-run-panel.test.tsx -t "run-event contract"` | 1 | [http]에서 시간 초과 — 네트워크 창은 창 삽입을 받지 않는다(두 경로 중복 없음, SC-003 화면 측) |
| AW 전체 / 패키지 | `pnpm test`, `tsc --noEmit` / `pnpm test`, `check-types` | 0, 0 / 0, 0 | 88 files / 616 tests / 6 files / 50 tests |

사용자 검토 반영:
- **교환 병합**: `exchangePasses`는 스냅샷에 같은 requestId가 있으면 스냅샷보다 늦은(`updatedAt`) 상태만 넘긴다. 요청 이벤트는 스냅샷에서 아직 `accepted`일 때만 넘긴다.
  - 시험은 실제 보관 gap을 가짜 hub로 만든다(한도 3, 끊긴 동안 5건 발행). 스냅샷 로드를 붙잡아 둔 사이 버퍼에 옛 요청·옛 `accepted`·`delivered`를 쌓는다.
  - x1이 스냅샷의 `delivered`에서 `accepted`로 되돌아가지 않고, x1 요청이 다시 넘어가지 않는 것을 단정한다.
- **worktree 재조회**:
  - 이벤트 클라이언트에 `resyncOnReconnect`(알림 스트림: 첫 연결 뒤 매 재연결 hello에서 스냅샷 재설정)를 더했다. worktree 구독은 재연결마다 `{kind: "git", reason: "resync"}`를 보낸다.
  - 패널의 무효화 처리를 `features/worktree-workspace/model/worktree-change-invalidation.ts`로 뽑았고, 패널이 그 함수를 부른다.
  - 시험은 재연결 뒤 이 신호가 실제 `QueryClient`에서 파일 목록·변경·Git 이력·그래프 query를 무효화하는 것까지 단정한다.
- **소스 문자열 시험 4개 재지정**(App 제목, speckit 무효화, 교환·orchestration 저장소의 fallback): 검사하던 문자열이 옮겨 간 파일(호환 transport, 무효화 모델)을 보도록 대상만 바꿨고, 원래 검사 내용은 모두 유지했다.
- **화면 이벤트 시험(T038 일부)**: `agent-run-panel` [http]의 run 이벤트는 가짜 서버 harness의 이벤트 계층으로 들어온다. 이 계층은 042 cursor 규칙을 흉내 내는 **메모리 hub 소켓**이고, 호출만 실제 HTTP다. 실제 WebSocket과 실제 042 서버의 조합은 T041 통합 suite가 맡는다.

## T035 연결 · T036 교환 전달 키 · T043 agent 1회 전달 · 세대 변경 재동기(T046 일부)

| 항목 | 명령 | 종료 코드 | 결과 |
|---|---|---|---|
| 화면: 교환 prompt의 run 시작 키 | `npx vitest run src/features/agent-run/ui/agent-run-panel.test.tsx` | 1 → 0 | 첫 실패는 서버 기록이 앞선 시험의 `run.start` 3건까지 쌓여서였다. 이 시험 이후 기록만 보게 고친 뒤 28 passed. [http]에서 서버가 받은 `run.start`의 키가 `["exchange-delivery:x-42"]` |
| 변이: `startRun`에 키를 넘기지 않음 | 같은 파일 `-t "exchange delivery key"` | 1 | [http] 실패 |
| **범위 A — core → RunEngine 호출 1회** | `cargo test -p workbench-core --features test-hooks --test exchange_delivery_once` | 0 | `ScriptedRunEngine.prompts` 카운터: 같은 키 두 번 → 1, 다른 키 → 2. 이 시험은 **core가 엔진을 한 번 부른다는 근거**이고, 실제 agent 전달 증거가 아니다 |
| **범위 B — 실제 AcpRunEngine + 가짜 ACP agent 프로세스** | `cargo test -p workbench-core --features test-hooks --test exchange_delivery_acp` | 0 | 실제 runner가 띄운 `fake_acp_permission_agent.py`의 기록에서 x-1 본문을 받은 횟수 1, 전체 prompt 3(목표, x-1, x-2) |
| 범위 B 대조 변이(첫 판) | 중복 전송을 새 키로 | **0(변이를 못 잡음)** | 장벽이 잘못됐다. `end_turn` 3개 조건이 (목표, x-1, 중복)으로 먼저 채워져 x-2 전에 셌다 |
| 범위 B 장벽 수정 | 가짜 agent가 `prompt-text:<id>:<본문>`을 추가로 기록(기존 줄 형식 유지), 장벽 = "x-2 본문 prompt의 `end_turn`" | 0 | 2 passed |
| 범위 B 대조 변이(수정 뒤) | 중복 전송을 새 키로 | 101 | agent 기록에 `prompt-text:3`과 `prompt-text:4`가 모두 x-1이다(중복 전달 드러남). 실패 지점은 x-1 횟수 단정이 아니라 "x-2가 끝나지 않음(10초 상한)"이다. **변이 상태에서 x-2가 agent에 가지 않은 원인은 미확인**이다. 정상 경로에서는 중복 전송이 저장 결과 재생이라 x-2를 보낼 때 agent가 쉬고 있다 |
| 가짜 agent 변경 회귀 | `cargo test … --test acp_permission_exit` | 0 | 4 passed |
| 세대 변경 재동기 | `npx vitest run src/app/bootstrap-transport.test.ts` | 0 | 8 passed. 재연결 handshake의 세대가 바뀌면 `onEpochChanged`를 **한 번** 부르고 진단 기록을 남긴다. 기본 동작은 창 다시 불러오기(새 작업대·새 구독·화면 상태 초기화) |

- **T035**: `worktree-agent-run-area`의 교환 요청 수신자가 원장(`handleRequested`)을 거치고, 상태 수신자는 서버의 종결 상태를 원장에 알린다(`observeStatus`). 확인에 실패했을 때 화면 문구는 오늘과 같고, 이제 다음 재조정 때 확인만 다시 시도한다.
- **T036**: 교환에서 라우팅된 prompt(`exchangeRequestId`)는 실제로 보내는 두 지점에 키 `exchange-delivery:<requestId>`를 싣는다. 대기열 전송(`sendPromptToRun`)과 즉시 전달(`startAgentRun`)이다. 교환이 아닌 prompt에는 키가 없다(기존처럼 호출마다 새 키).
- **SC-004d 범위**: 서버 쪽은 범위 B로 실제 agent 프로세스 경계까지 확인했다. 화면에서 원장 없이 다시 라우팅되는 새로고침 시나리오를 실제 앱에서 끝까지 보는 것은 T054 앱 스모크에서 다룬다(아직 안 함).

## T039 · T040 · T044 · T045 재연결·gap 복구 (이벤트 클라이언트)

| 항목 | 명령 | 종료 코드 | 결과 |
|---|---|---|---|
| 재연결·gap | `npx vitest run src/event-client.reconnect.test.ts src/event-client.gap.test.ts` | 1 → 0 | 첫 실패는 제 시험의 소켓 수 단정이었다(수신자 교체 때 클라이언트가 스스로 다시 연결해 소켓 수로는 셀 수 없음). 전달 단정은 처음부터 통과했다. 실제로 끊은 횟수를 세도록 고친 뒤 7 passed |
| 반복 | 같은 명령 10회 | 10 × 0 | — |
| 변이: 복구 재구독 cursor를 0으로(H1 원래 설계) | — | 1 | live 확보 시험, "hello는 성공이 아님" 시험 실패 |
| 변이: 버퍼를 스냅샷 기준으로 거르지 않음 | — | 1 | 병합 시험 실패(스냅샷이 덮은 7을 다시 받음) |

- **강제 끊김 120회**: 한 번에 1–3건씩 발행하고 매 회차 소켓을 끊는다.
  - 느린 async 수신자는 받았지만 반영 전에 끊기는 구간을 만든다.
  - 10회마다 수신자를 교체한다. 교체 중 끊김이다.
  - 결과: 안정 수신자와 느린 수신자는 1..N을 빠짐·중복 없이 받는다. 교체된 수신자들이 받은 구간을 이어 붙이면 1..N을 정확히 덮는다.
- **subscriberLagged**: 같은 cursor로 다시 연결한다.
- **보관 gap**:
  - 표 cursor 순서가 `[1, 6]`이다(gap의 lastSequence로 live 먼저). 재설정 전에는 넘기지 않고, 스냅샷(7까지)이 덮은 7은 빼고 8만 넘긴다.
  - 복구 중 새 gap이면 재시작하고, 3회 뒤 `stream recovery failed repeatedly`를 알린다.
- **세대 변경**: `onEpochChanged`를 알린 뒤 새 세대 cursor 0부터 재설정한다.
- **evicted**: 재설정한 뒤 다시 연결하지 않는다.
- T047(교환 재조정 트리거)은 교환 스트림의 `onReset`이 확인 전 교환을 요청으로 다시 넘기는 방식으로 구현했다(T034 `network-events`, T035 원장). 구독 시작, gap 복구, 수신자 재동기에서 모두 같은 경로를 탄다.

## T047 정정 — 교환 재조정 트리거(사용자 검토)

**정정**: 앞 절(T039–T047)에서 T047을 "구독 시작·gap 복구·수신자 재동기에서 같은 경로"로 완료 처리했다. 그러나 실제로는 **보관 gap 복구와 수신자 재동기에만** 연결돼 있었다.
- 교환 구독에는 `resyncOnStart`와 `resyncOnReconnect`가 없었다.
- 원장은 확인 실패를 삼켜 수신자가 성공으로 처리되고 cursor가 앞서간다. 그래서 같은 세대에서 보관 gap 없이 다시 연결하면 hello만 오고 재조정이 없었다. `accepted` 교환의 확인 재시도가 일어나지 않았다.
- 구독 시작 재조정의 트리거도 없었다.

수정:
- 이벤트 클라이언트에 `resyncOnStart`(첫 hello에서 스냅샷 재설정)를 더했다.
- 교환 구독은 `resyncOnStart`와 `resyncOnReconnect`를 모두 켠다. 트리거는 넷이다: 구독 시작, 같은 세대 재연결, 보관 gap, 수신자 재동기.

| 항목 | 명령 | 종료 코드 | 결과 |
|---|---|---|---|
| 트리거 시험 | `npx vitest run src/shared/api/transport/network-events.test.ts` | 1 → 0 | 4 passed. 중간 실패 원인은 셋이다. (1) AW 시험 환경(happy-dom)의 `Response.json()`이 마이크로태스크만으로 끝나지 않는데 제 대기 도우미가 마이크로태스크만 돌렸다 → 이 파일은 `vi.waitFor`와 실제 타이머로 바꿨다. (2) 확인 재시도 시험이 서버 상태를 처음부터 `accepted`로 둬서 구독 시작 재조정이 먼저 처리했다(시험 설계 오류). (3) 재설정은 수신자(요청·상태)마다 스냅샷을 읽는다 — 재설정 한 번에 `exchange.list`가 2회다 |
| 변이: 교환 구독의 재조정 트리거 제거 | — | 1 | 트리거 시험 2개 실패(병합 시험도 호출 수 기대 때문에 함께 실패) |
| 변이: 교환 병합 기준을 항상 참으로 | — | 1 | 병합 시험 실패 |
| AW 전체 / 패키지 | `pnpm test`, `tsc` / `pnpm test`, `check-types` | 0, 0 / 0, 0 | — |

시험 시나리오:
- **확인 재시도**: 라우팅 성공 → 확인 1회 실패 → 새 이벤트도 보관 gap도 없이 소켓만 재연결 → 재연결 재조정(`exchange.list` +2) → 라우팅 추가 0회, 확인만 재시도해 성공. 서버가 확인을 반영한 뒤(`delivered`)의 재연결에서는 추가 확인 0회다.
- **구독 시작 재조정**: 스트림에 요청 이벤트가 없고 스냅샷에만 `accepted` 교환이 있어도, 구독 시작 재조정으로 라우팅 1회·확인 1회가 일어난다.
- **병합 시험 기대값 조정(성질은 그대로)**: 재연결 hello의 재조정과 뒤이은 gap 복구가 같은 스냅샷을 두 번 적용할 수 있다(x1 `delivered`가 두 번 넘어옴). 실제 WebSocket에서는 hello와 gap이 별개의 메시지로 오기 때문에, hello에서 뒤따를 gap을 알고 재조정을 건너뛸 방법이 없다. 단정은 요구 성질인 "스냅샷 이후 역행 없음(x1 상태 집합 = {delivered})"과 "종결된 교환의 요청 재라우팅 없음"으로 정확히 했다. 상태 수신자는 requestId로 덮어쓰므로 화면 결과는 같다.

## T041 · T042 실제 042 서버 통합 suite (`packages/workbench-client/src/test/integration`, `pnpm --filter @yoophi/workbench-client test:integration`)

구성:
- vitest global setup이 `cargo build -p workbench-core --example http_test_host --features test-hooks`로 시험 host를 빌드한다.
- 각 시험은 host 프로세스를 따로 띄운다. host는 운영 router·실제 런타임·hub(journal 보관 한도 4)로 되어 있다.
- TS 클라이언트는 실제 `fetch`와 Node 22 내장 `WebSocket`으로 붙는다. 이벤트 클라이언트의 기본 소켓이다.
- 기본 `test`에서는 제외한다(`vitest.config.ts` exclude).

| 항목 | 종료 코드 | 결과 |
|---|---|---|
| 첫 실행 | 1 | 2 passed, 2 failed. 실패 둘 다 시험 준비 오류다. 교환은 `syncWorkspace`의 `worktreePath`가 작업대 경로와 달랐는데 제가 넣은 `catch`가 실패를 삼켰다. orchestration은 `setPresentation`이 직접 자식 노드에만 적용되는데 메인 노드에 걸었다 |
| 수정 뒤 | 0 | 4 passed(retention 파일) |
| 변이: 복구 재구독 cursor 0(H1 원래 설계) | 1 | run·교환·orchestration 3개 모두 실패(10초 상한) — **실제 042 hub에서** after 0 재구독이 복구되지 않음을 확인 |
| 반복 | 5 × 0 | — |
| 응답 유실 파일 | 0 | 2 passed. 처음에 처리되지 않은 rejection 1건(서버를 죽일 때 진행 중이던 요청의 Promise). 처리 표시를 붙인 뒤 전체 6 passed, 0 unhandled |
| 변이: 응답 유실 뒤 세대 확인 제거 | 1 | 새 세대 시험 실패 — 세대가 바뀐 서버로 변경이 다시 간다 |

단정 범위:
- **조회/변경 표**: `OPERATION_KINDS`가 실제 서버 `system.describe`의 operation 종류와 같다(85개).
- **run 보관 초과**: 끊긴 동안 prompt 6건(한도 4). 스냅샷(`run.replay`)이 덮은 순번은 다시 받지 않고, 다음 live는 스냅샷 `lastSequence + 1`이다.
- **교환 보관 초과**: 스냅샷(`exchange.list`)이 끊긴 동안의 교환 7건(x-0..x-6)을 모두 `accepted`(재조정 대상)로 담는다. 이후 live는 새 교환(x-100)만이다.
- **orchestration 보관 초과**: 수동 자식의 표시 상태를 번갈아 바꾼다. 스냅샷 revision ≥ bootstrap + 7이고, 이후 live의 revision은 모두 스냅샷보다 크다. 마지막 live revision이 서버의 마지막 revision과 같다.
- **같은 세대 응답 유실**:
  - `HOST_PROMPT_SETTLE_MS=1500`. 효과 표식(run replay에 그 prompt의 AgentMessage) 뒤에 요청을 끊는다.
  - 클라이언트가 같은 키로 한 번 재시도해 `ok`를 받는다(run.sendPrompt 요청 2건, 키 동일). 효과는 1건이다.
- **새 세대 응답 유실**:
  - 효과 표식 뒤 host를 `SIGKILL`하고, 같은 데이터 디렉터리로 새 host를 다른 포트에 띄운다.
  - 클라이언트 결과는 `unknown/epochChanged`이고, 연결은 새 끝점·새 세대로 옮겨 가 있다.
  - 새 서버로 간 run.sendPrompt는 0건이다.

## T048 연결 상태 표시 · 앱 진입점에서 네트워크 경로 켜기

- `shared/api/transport/connection-status.ts`(상태 저장소)와 `shared/ui/connection-status.tsx`(표시)를 더했다. AW에는 widgets 계층이 없어 shared에 둔다.
  - 표시는 `reconnecting`과 `disconnected`일 때만 보이고, 연결되면 아무것도 그리지 않는다(기존 배치·문구 불변).
  - Storybook: `molecules.stories.tsx`의 `WorkbenchConnectionStatus`.
- 시험 `shared/ui/connection-status.test.tsx`: 상태 전이에 따라 표시가 나타나고 사라진다.
- **앱 진입점**: `main.tsx`가 렌더 전에 `bootstrapTransport()`로 창의 경로를 한 번 정하고, 모든 창에 `<ConnectionStatus />`를 둔다. 부팅은 연결 상태를 저장소로 알린다.
- 이 시점부터 앱은 네트워크 경로로 부팅한다. **실제 앱에서 동작한다는 증거는 아직 없다** — T053·T054(개발·배포 출처 스모크)에서 확인한다.
- `pnpm --filter @yoophi/agentic-workbench test` 종료 코드 0(622 tests), `tsc --noEmit` 0.

## T049 · T050 · T051 · T052 창 수명·격리·부팅 대체 경로

| 항목 | 명령 | 종료 코드 | 결과 |
|---|---|---|---|
| 두 창 격리(TS 클라이언트, 실제 서버) | `npx vitest run --config vitest.integration.config.ts src/test/integration/window-isolation.integration.test.ts` | 0 | 창 B 토큰으로 창 A 작업대의 `exchange.list`·`orchestration.get`·`run.start`·`bench.close`는 모두 `forbidden`이다. 창 B의 `exchange:<A작업대>` 구독은 fault 프레임을 받아 스트림 오류(`bench belongs to another principal.`)가 된다. 창 A는 계속 조회할 수 있다 |
| 창 닫기 정리 순서 | `cargo test --lib window_lifecycle` | 0 | session 창: 토큰·표 폐기 → 전달 선언 해제 → 연 창 주체로 작업대 닫기. 다른 창: 폐기 → 해제(작업대 없음) |
| AW Rust 전체 | `cargo test` | 0 | 아래 합계 |
| 끝점 기동 실패 주입(T052) | debug 빌드 `AW_WORKBENCH_HTTP_FAIL_START` | — | 기동을 실패로 두면 `get_workbench_connection`이 오류를 돌려준다. 이때 부팅이 호환 경로로 가는 것은 T051 시험(`connection info` 실패)이 확인한다. **실제 앱에서의 확인은 T054에서 한다** |

- T050: `window_lifecycle::teardown`을 `Teardown` trait 주입으로 뽑아 Tauri 없이 순서를 고정했다. 운영은 `AppTeardown`이다.
- 토큰 폐기 자체(폐기 뒤 401, 폐기 전 표 거절)는 T005(core `http_window_tokens`)가 운영 발급기로 확인한다.
AW Rust 합계: 124 passed, 0 failed

## T053 · T054(부분) 앱 스모크

자세한 내용은 `reviews/app-smoke.md`. 개발 출처와 배포 출처(`tauri://localhost`) 모두 네트워크 경로에서 SC-005 흐름이 확인됐다: 프로젝트 조회, run 시작·출력(에코) 수신, 이벤트 소켓 강제 종료, 자동 재연결, 끊긴 뒤 prompt 출력의 이어 받기, 중복·빈 순번 없음. 끝점 기동 실패 주입에서는 호환 경로로 부팅하고 조회가 동작했다. **SC-004d 새로고침 시나리오는 아직 미검증이라 T054는 완료로 표시하지 않는다.**

## SC-004d 새로고침 실제 앱 증거 · T041 교환 재조정 통합(사용자 검토 반영)

- **T041 보강**(`apps/agentic-workbench/src/shared/api/transport/exchange-recovery.itest.ts`, `pnpm --filter @yoophi/agentic-workbench test:integration`):
  - 구성: 실제 시험 host(보관 한도 4) 위에서 화면의 `createNetworkEvents`와 `createExchangeReconciler`를 쓴다.
  - 시나리오: 구독 전에 교환 3건을 보내 첫 요청 이벤트가 journal에 없게 한다. 구독이 보관 gap 복구와 재조정을 거친다.
  - 결과: 교환마다 라우팅 1회·실제 `exchange.acknowledge` 1회, 서버 상태 `delivered` 3건. 같은 세대 재연결 뒤 추가 라우팅·확인 0회.
  - 재연결 뒤 단정의 장벽(사용자 검토 반영): 스냅샷 조회 **응답 해결** 수 +2, 상태 수신자 재설정이 세 교환 모두의 콜백을 끝냄, 그 뒤 한 차례 양보.
  - 변이 두 개(종료 코드 1):
    - 확인을 보내지 않음 → 실패.
    - 중복 라우팅(원장의 확인 여부 무시 + 요청 재설정에 `delivered`도 넘김) → 실패. 이 변이는 **처음 복구 단계**의 라우팅 수 단정(`x-1`이 두 번)에서 잡혔고, 재연결 뒤 단정에서 잡힌 것이 아니다.
- **SC-004d 실제 앱(개발 출처)**: `reviews/app-smoke.md`의 새로고침 절.
  - 운영 원장·키를 쓴 확인 전 새로고침 → 재조정·같은 키 재전송 → 앱 스트림 에코 1회, agent prompt 1회(장벽 뒤 셈), 서버 상태 `delivered`.
  - 패널 UI 라우팅은 probe가 대신했다(범위 명시).
  - 첫 새로고침 스모크 두 번은 무효다. 하나는 `install_probe`의 시나리오 선택 치환이 `rustfmt` 줄바꿈과 맞지 않아 적용되지 않아서(기본 시나리오 결과가 나옴)다. 다른 하나는 zsh 읽기 전용 변수(`status`) 오류로 대기 루프가 시작되지 않아서다. 치환에 적용 단정을 넣고, 대기에 5분 상한과 "시나리오 불일치 = 실패" 판정을 넣은 뒤 다시 실행했다.

## T038 화면 이벤트 통합(범위 명시)

| 항목 | 명령 | 종료 코드 | 결과 |
|---|---|---|---|
| orchestration 비동기 재조회 수신자 | `npx vitest run src/shared/api/transport/network-events.test.ts` | 0 | 5 passed |
| 변이: 수신자 실패를 반영 완료로 침 | — | 1 | 이 시험 실패 |

- 시험 구성: 화면 수신자(`worktree-agent-run-area`의 orchestration 수신자)와 같은 모양, 즉 이벤트를 받으면 작업 영역을 await로 다시 읽는 수신자를 네트워크 이벤트 계층에 붙였다.
- 결과:
  - 첫 재조회가 실패하면 그 수신자만 스냅샷(`orchestration.get`)의 재설정 신호(`reason: "resync"`, 현재 revision)를 받아 다시 읽는다.
  - 동기 예외를 낸 다른 수신자도 재설정 신호로 따라잡는다. 둘은 서로 막지 않는다.
  - 이후 이벤트는 둘 다 받는다.
- **범위**:
  - `worktree-agent-run-area` 컴포넌트 자체를 렌더링한 시험은 아니다. 이 컴포넌트의 기존 시험은 소스 문자열 검사뿐이고, 렌더링 harness가 없다.
  - 화면 경로를 실제로 거친 이벤트 시험은 세 가지다. `agent-run-panel.test.tsx`의 [http] run 이벤트(네트워크 창은 창 삽입을 받지 않음 변이 포함), 교환 키 시험, 앱 스모크의 run 이벤트·새로고침 교환(실제 앱)이다.
- 수신자 교체 중 도착은 이벤트 클라이언트 시험(T030, T039의 교체 중 끊김)이 확인한다.

## T038 정정 — 운영 화면 구독자를 렌더링한 통합 시험(사용자 검토)

**정정**: 앞 절(T038, dd59ea7)은 `worktree-agent-run-area`의 수신자와 **같은 모양의 시험용 수신자**를 만들어 확인했다. 운영 화면 구독자를 거치지 않았으므로 T038 완료 근거가 아니었다. T038을 다시 열고 아래로 닫았다.

- **운영 코드 추출**: `worktree-agent-run-area`의 orchestration 갱신 구독을 동작 그대로 `features/agent-run/ui/use-orchestration-workspace-updates.ts`로 옮겼다. 운영 컴포넌트는 이 hook을 쓴다(`useOrchestrationWorkspaceUpdates(orchestrationSessionRef, setOrchestrationSession, worktree.path)`). hook의 `resetKey`가 `null`이면 구독하지 않는다(운영은 늘 Worktree 경로를 넘긴다).
- **시험**: `use-orchestration-workspace-updates.test.tsx`. 운영 hook과 운영 저장소 함수(`getOrchestrationWorkspace`)를 쓰는 최소 화면을 네트워크 경로로 렌더링한다. 호출은 실제 루프백 HTTP의 가짜 서버로 가고, 이벤트는 가짜 hub 스트림에서 온다. 판정은 **화면에 그려진 revision**이다.
  - **Promise 거절**: 다시 읽기가 한 번 실패하면 이벤트 클라이언트가 이 수신자를 스냅샷(revision 알림)으로 재동기하고, 화면이 `revision:2`를 그린다. 읽기는 실패 1회와 복구 1회 이상이다.
  - **수신자 교체**(사용자 검토 반영): 화면을 다시 마운트하지 않고(화면 상태 revision 2 유지) 구독자만 내린다. 그 사이 revision 3이 도착하고, 구독자를 다시 올리면 **운영 hook을 통해서만** 화면이 `revision:3`이 된다. 그 뒤 revision 4도 이어 받는다.
  - **동기 예외**: 운영 수신자는 async라 내부 예외(재조회 실패 포함)가 모두 거절된 Promise가 된다. 이 수신자에는 동기 예외 경로가 없다. 동기 예외는 이벤트 클라이언트 시험(T030 `treats a synchronous throw like a rejection`)이 맡는다.

| 항목 | 종료 코드 | 결과 |
|---|---|---|
| 첫 판(화면 재마운트로 교체) | 1 → 0 | 처음에는 소켓 재사용 단정만 실패했다(아래 이벤트 클라이언트 수정). **이 첫 판은 새 화면이 마운트하며 최신을 스스로 읽어 대기열 이벤트를 버려도 통과하는 약한 시험이었다**(사용자 검토). 구독자만 교체하는 설계로 바꿨다 |
| 현재 판 | 0 | 1 passed |
| 변이 A: 수신자 없을 때 대기열에만 넣지 않음(최고 순번은 갱신) | **0(못 잡음)** | 새 구독자가 대기열로 받지 못한 순번이 있음을 알고(`lastQueued < highest`) cursor에서 다시 연결해 replay로 받는다. 이 변이는 실제 유실을 만들지 못한다 |
| 변이 B: 수신자 없을 때 도착한 이벤트를 통째로 버림(존재도 모름) | 1 | `expected 'revision:2' to be 'revision:3'` — 교체 단정에서 정확히 실패 |

- **이벤트 클라이언트 수정**: 새 수신자가 붙을 때의 재연결 판정을 `delivered < highest`에서 `lastQueued < highest`로 바꿨다. 대기열을 넘겨받아 최고 순번까지 받은 수신자는 다시 연결하지 않는다. 이전 판정은 불필요한 재연결을 만들었다. 유실은 없었다(중복은 `lastQueued`가 막음).
- 패키지 57 tests, AW 624 tests, 두 패키지 타입 검사 통과. 패키지 시험의 종료 코드 1은 통합 시험 파일의 쓰지 않는 import 때문이었다(타입 검사가 모든 파일을 본다). 고친 뒤 0이다.
