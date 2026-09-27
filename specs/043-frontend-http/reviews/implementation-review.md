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

## T057 OCR 구현 리뷰 (`ocr delegate`, `--from b682c6b --to 0fedafb`)

OCR이 고른 검토 대상은 82개 파일(시험·문서 제외)이고, 운영 코드를 직접 검토했다. 이벤트 클라이언트, 호출 클라이언트, 연결, network-events, http-transport, bootstrap, 교환 원장, 창 주체·수명·토큰 폐기, 전달 선언, Tauri command 호출자를 보았다. Low는 버렸다.

| # | 등급 | 위치 | 문제 | 조치 |
|---|---|---|---|---|
| O1 | High | `event-client.ts` `pump`·`resetListener`·`completeRecovery` | 재동기(재연결 재조회·gap 복구)가 진행 중인 `onEvent`와 겹치면, 늦게 끝난 옛 `onEvent`가 재동기가 새로 정한 대기열의 첫 항목을 지운다(**유실**). cursor도 옛 순번으로 올린다(새 세대 복구에서는 새 세대 이벤트를 건너뛰는 cursor가 된다) | 수신자별 `generation`. 재동기마다 올리고, 늦게 끝난 전달은 대기열·cursor를 건드리지 않는다 |
| O2 | Medium | 같은 파일 | 재동기 스냅샷이 이미 반영한 이벤트의 프레임이 재동기 **뒤에** 도착하면 다시 넘긴다(**중복**). 스냅샷 기준 걸러내기가 그때의 대기열에만 적용됐다 | 수신자별 `covered`(스냅샷 `passes` 반대)를 다음 재동기까지 유지한다. 덮인 이벤트는 넘기지 않고 cursor만 올린다 |
| O3 | Medium | 같은 파일 `resetListener` | 스냅샷 적재가 계속 실패하면, 내려간 수신자나 닫힌 클라이언트도 재시도 타이머를 끝없이 돈다 | 수신자 제거·클라이언트 닫힘·스트림 종결이면 멈춘다 |
| O4 | Medium | 같은 파일 `add` | 재연결 표를 받는 중에 더 앞선 cursor(`after`)로 수신자가 합류하면, 표는 이미 옛 cursor로 요청돼 합류 수신자가 그 사이 순번을 받지 못한다 | `reopen` 표시. 받은 표를 버리고 새 cursor로 다시 연다 |
| O5 | Medium | `tauri_desktop_bridge`·`bootstrap-transport.ts` | 네트워크 전달을 선언한 창을 새로고침했는데(incarnation 그대로) 부팅이 호환 경로로 떨어지면, 선언이 남아 앱 내부 삽입이 꺼진 채 이벤트를 전혀 받지 못한다 | Tauri command `withdraw_network_delivery`를 추가했다. 호환 경로로 부팅하면 부른다(실패해도 호환 경로 유지·기록) |

시험(먼저 작성·실패 확인):
| 시험 | red(종료 코드) | green |
|---|---|---|
| `event-client.races.test.ts` O1 | 1: `timed out waiting for event 2 delivered after the late settle` | 0 |
| 같은 파일 O2 | 1: `expected [ 2, 3 ] to deeply equal [ 3 ]` | 0 |
| 같은 파일 O3 | 1: `expected 11 to be 1`(내려간 뒤 적재 10회 더) | 0 |
| 같은 파일 O4 | 수정과 함께 작성. 수정을 끈 변이에서 1: `timed out waiting for joining listener receives from its own cursor` | 0 |
| `bootstrap-transport.test.ts` O5(철회 호출, 철회 실패에도 호환 유지) | 1(동작 실패) | 0, 10 passed |
| `tauri_desktop_bridge` O5 Rust | 101(컴파일 실패: 함수 없음. 동작 red가 아님) | 0 |

회귀: workbench-client 61 tests·통합 7, AW 624 tests·통합 1, AW `tsc` 0. **O5 철회는 실제 앱에서 새로고침 + 기동 실패로 확인하지 않았다(미검증).**
