# 043 구현 리뷰 기록

## T056 최종 게이트 (커밋 `f2b1bee`, 각 1회 실행·원 명령 종료 코드)

CI(`.github/workflows/quality.yml`)와 같은 명령에 통합 suite 두 개를 더했다. `CARGO_INCREMENTAL=0`.

| 게이트 | 명령 | 종료 코드 | 결과 |
|---|---|---|---|
| 서식 | `cargo fmt --all -- --check` | 0 | — |
| 정적 검사 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | — |
| Rust 시험 | `cargo test --workspace --all-targets` | 0 | 764 passed, 0 failed. core/server/protocol은 dev-dependency로 `test-hooks`가 켜진다. AW Tauri crate 포함 |
| 타입 | `pnpm run check-types` | 0 | 13/13 tasks |
| JS 시험 | `pnpm run test` | 0 | 12/12 tasks. AW 624 tests(90 files, 화면 시험 두 경로 포함), workbench-client 57 tests |
| 빌드 | `pnpm run build` | 0 | 5/5 tasks |
| 클라이언트 통합 | `pnpm --filter @yoophi/workbench-client test:integration` | 0 | 3 files / 7 tests(실제 042 시험 host) |
| AW 통합 | `pnpm --filter @yoophi/agentic-workbench test:integration` | 0 | 1 file / 1 test(교환 재조정, 실제 시험 host) |

**무효·실패한 이전 실행**(같은 방식으로 기록):
- 1차(`2a97f29`): `fmt` 1(`exchange_delivery_acp.rs`·`exchange_delivery_once.rs` 서식), `clippy` 101(`window_lifecycle.rs` 접을 수 있는 `if`). 통합 두 줄은 filter 이름을 잘못 적어(`@agentic-workspace/...`) "No projects matched"로 **아무것도 실행하지 않고 0을 돌려줬다. 무효**.
- 2차(`c01110c`, 서식·let chain 수정 뒤): `clippy` 101(`http_window_tokens.rs`의 쓸모없는 `.into()`). clippy는 첫 실패 crate에서 멈춰 1차에서 이 오류가 가려져 있었다. `--keep-going`으로 남은 오류가 없음을 확인하고 고쳤다.
- 3차(`f2b1bee`)가 위 표다. 수정은 모두 서식·lint였고 동작 변경은 없다.

## 성공 기준 증거 표

| 기준 | 증거 | 범위·한계 |
|---|---|---|
| SC-001 호환 경로 서버 호출 0 | `command-inventory.md`(S/D 분류), `no-direct-invoke.test.ts`(저장소가 Tauri `invoke`를 직접 부르면 실패, 변이 확인), command 표 Rust/TS golden 동등성 | 데스크톱 표현(D) command는 명시적으로 남는다. 네트워크 경로가 켜진 창 기준. 끝점 기동 실패 시 부팅 대체 경로는 의도된 호환 경로다(SC-007) |
| SC-002 기존 화면 시험 기대값 불변 | `agent-run-panel.test.tsx` 9 시나리오 × 두 경로(실제 루프백 HTTP + `HttpTransport`), 10회 반복 10/10 | 서버 소유 command를 실제로 거치는 기존 화면 시험은 이 하나다(`implementation-evidence.md` T027 범위). 대기 조건만 고쳤다 |
| SC-003 이중 표시 0 | 네트워크 창에 창 삽입 run 이벤트를 넣는 변이가 실패(T035), 창별 전달 선언(`declare_network_delivery`), 수신자 `lastQueued` 중복 방지 | — |
| SC-004 강제 끊김 100회+ | `event-client.reconnect.test.ts`(120 회차, 반영 전 끊김·수신자 교체 섞음), `event-client.gap.test.ts`(100회), 앱 스모크 강제 끊김 | 대량 반복은 가짜 hub 위 이벤트 클라이언트 시험이다. 실제 앱에서는 스모크 몇 차례다 |
| SC-004a 다른 창 자격 증명 거절 | core `window_isolation.rs`·`http_window_tokens.rs`, TS `window-isolation.integration.test.ts`(forbidden·스트림 fault) | — |
| SC-004b 응답 유실 재시도 | `call-client` 시험(같은 세대 같은 키, 401 갱신 넘어 세대 경계 포함), 통합 suite lost-response·epoch 변경 | — |
| SC-004c 보관 한도 복구 | core `retention_resubscribe.rs`, `retention-recovery.integration.test.ts`(run·교환·orchestration) | — |
| SC-004d agent 전달 1회 | core `exchange_delivery_once.rs`·`exchange_delivery_acp.rs`(실제 AcpRunEngine + 가짜 ACP 프로세스), AW `exchange-recovery.itest.ts`, 앱 스모크 새로고침(개발 출처) | 새로고침 스모크의 패널 라우팅은 probe가 운영 원장·키로 대신했다. 배포 출처 새로고침은 미검증 |
| SC-005 실제 앱 개발·배포 출처 | `app-smoke.md`, `app-smoke/*.json` | macOS만. Windows 출처 미검증 |
| SC-006 8시간 자격 증명 | `connection.test.ts`(가짜 시계 8시간, 80% 갱신, 만료 토큰 보유 0) | 실시간 8시간 실행은 하지 않았다 |
| SC-007 기동 실패 시 오늘 경로 | T051 부팅 시험, 앱 스모크 `AW_WORKBENCH_HTTP_FAIL_START`(debug)에서 조회 동작 | 실제 앱 compat 경로의 run 흐름은 확인하지 않았다(목록 조회만) |

## 미검증·이후 항목

- Windows 출처, 배포 출처 새로고침 시나리오, 실제 앱 compat 경로 run 흐름
- 최종 release 산출물 검증, standalone 서버 수명 검증(5단계)
