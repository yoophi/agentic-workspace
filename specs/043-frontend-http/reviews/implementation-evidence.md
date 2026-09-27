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
