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
