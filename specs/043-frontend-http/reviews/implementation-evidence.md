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
