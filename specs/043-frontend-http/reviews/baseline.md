# 043 기준선 게이트 (T001)

기준: branch `043-frontend-http` @ 87b6561 (main b682c6b + 설계 문서만). 각 명령은 한 번 실행했고, 로그를 저장한 뒤 원 명령의 종료 코드를 기록했다.

| 범위 | 명령 | 종료 코드 | 결과 |
|---|---|---|---|
| AW 화면 + workbench-client (필터) | `pnpm --filter @yoophi/agentic-workbench --filter @yoophi/workbench-client test` | 0 | AW 81 files / 429 tests 통과, workbench-client 타입 시험 통과 |
| AW 화면 + workbench-client 타입 (필터) | `pnpm --filter @yoophi/agentic-workbench --filter @yoophi/workbench-client check-types` | 0 | 통과 |
| **워크스페이스 전체** 시험 | `pnpm test` (turbo, 전체 패키지) | 0 | Tasks 12/12 successful |
| **워크스페이스 전체** 타입 | `pnpm check-types` (turbo, 전체 패키지) | 0 | Tasks 13/13 successful |
| Rust core/server/protocol | `CARGO_INCREMENTAL=0 cargo test -p workbench-core -p workbench-server -p workbench-protocol --features workbench-core/test-hooks` | 0 | 478 passed, 0 failed |
| AW Rust | `cd apps/agentic-workbench/src-tauri && CARGO_INCREMENTAL=0 cargo test` | 0 | 117 passed, 0 failed |

범위 주의:
- tasks.md의 `pnpm -r test`·`pnpm -r typecheck`는 실제 스크립트 이름이 아니다. 워크스페이스 전체 게이트는 루트 스크립트 `pnpm test`·`pnpm check-types`(turbo)로 실행했다.
- 필터 실행 두 줄은 전체 게이트가 아니다. 워크스페이스 전체 판정은 위의 전체 두 줄로 한다.
- Rust 워크스페이스 전체(`cargo test --workspace`)는 이 기준선에 포함하지 않았다. MA·GE·Hushline 크레이트는 043 변경 범위 밖이다. T056에서 변경된 크레이트 기준으로 다시 정한다.
