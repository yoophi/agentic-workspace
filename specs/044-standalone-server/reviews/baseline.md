# 044 기준선 (T001)

코드는 main `cb0bd4c`와 같다(브랜치 `8beddee`는 문서만 추가). 각 명령을 한 번 실행하고 원 명령의 종료 코드를 적었다. `CARGO_INCREMENTAL=0`.

| 명령 | 종료 코드 | 결과 |
|---|---|---|
| `cargo test --workspace --all-targets` | 0 | 765 passed, 0 failed |
| `pnpm run check-types` | 0 | — |
| `pnpm run test` | 0 | AW 634, workbench-client 73 |
| `pnpm --filter @yoophi/workbench-client test:integration` | 0 | 7 tests |
| `pnpm --filter @yoophi/agentic-workbench test:integration` | 0 | 1 test |

기록할 일: 기준선 `cargo test`가 도는 동안 R8-spike 로거를 AW `lib.rs`에 잠깐 붙였다가 되돌렸다(몇 초). 그 시점의 로그에는 `agentic-workbench` crate 컴파일이 아직 없었다(0건). 따라서 기준선은 되돌린 원본으로 컴파일됐다. `#207`(`acp_permission_exit`)은 이번 실행에서 통과했다(간헐 실패).
