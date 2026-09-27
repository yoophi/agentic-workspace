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
