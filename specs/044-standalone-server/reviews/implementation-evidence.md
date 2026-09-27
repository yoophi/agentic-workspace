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
