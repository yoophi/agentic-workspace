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

회귀: O1–O4 뒤에 workbench-client 61 tests·통합 7, AW 624 tests·통합 1, AW `tsc` 0. **O5 철회는 실제 앱에서 새로고침 + 기동 실패로 확인하지 않았다(미검증).**

**정정**: 처음 기록은 위 회귀 수치를 O5까지 포함한 것처럼 적었다. 실제로 AW 전체 시험은 O5 변경 **전**에 돌렸고, O5 뒤에는 `bootstrap-transport` 시험과 `tsc`만 돌렸다. O5 커밋(`5f920d2`)은 직접 호출 가드(`no-direct-invoke.test.ts`)를 깨뜨렸다. 새 데스크톱 command `withdraw_network_delivery`가 허용 목록에 없었기 때문이다(아래 Codex 반영 뒤 전체 실행에서 발견, 종료 1). 허용 목록과 command 인벤토리에 넣어 고쳤다.

## T057 Codex 적대적 구현 리뷰 (`/codex:adversarial-review --wait --base b682c6b`, OCR 반영 뒤 `5f920d2`)

판정: needs-attention. High 2건:

| # | 위치 | 문제(Codex 메모리 내 재현) | 조치 |
|---|---|---|---|
| C1 | `event-client.ts` `resetListener` | 재동기 두 개가 겹치면, 늦게 끝난 옛 스냅샷이 새 스냅샷 뒤에 적용된다(`onReset` 순서 `[2, 1]`). 교환 상태가 옛 `accepted`로 되돌아가고 `covered`도 옛 스냅샷으로 돌아간다. O1의 세대 검사는 `onEvent` 완료에만 있었다 | 수신자별 재동기 사슬(`serialize`)로 차례로 돌린다. 작업 시작·스냅샷 적재 뒤·`onReset` 뒤마다 세대·제거 여부를 확인하고, 옛 작업은 콜백·상태 변경 없이 끝낸다. hello 재동기와 gap 복구 재설정도 같은 사슬을 쓴다. 실패 재시도도 옛 세대면 멈춘다 |
| C2 | 같은 파일 `completeRecovery`·`add` | gap 복구의 `onReset`을 기다리는 동안 합류한 수신자는 재설정 대상 목록에 없다. 버퍼는 전역 스냅샷 기준으로 걸러져 새 수신자가 스냅샷도, 지난 상태도 받지 못한 채 cursor만 전진한다 | 복구 중 합류하면 합류 cursor를 복구 기준점으로 두고 재설정 중 상태로 둔다. 복구가 끝나면 버퍼 전체를 대기열로 받고, 자기 스냅샷으로 재설정하며 걸러낸다(스냅샷 없는 스트림은 그대로 넘긴다) |

시험(`event-client.races.test.ts`, 먼저 작성·실패 확인):
| 시험 | red | green |
|---|---|---|
| C1 옛 스냅샷이 늦게 와도 새 스냅샷 뒤에 적용하지 않음 | 1: `expected 1 to be 2`. 직렬화 뒤에는 두 번째 적재가 첫 적재를 기다리므로, 처음 쓴 대기 조건("두 번째 적재 시작")이 성립하지 않았다. 대기 조건을 "재연결 hello 도착"으로 바꿨다. 바꾼 시험도 수정 전 코드(`git stash`)에서 `expected 1 to be 2`로 실패함을 확인했다 | 0 |
| C2 복구 중 합류 수신자가 자기 스냅샷 뒤 live를 받음(보관 한도 2) | 1: `timed out waiting for B gets its own snapshot` | 0 |

회귀(각 1회): workbench-client 63 tests(첫 실행은 시험 파일의 `Array.at` 타입 오류로 종료 1 → `[length - 1]`로 고침 → 0), 통합 7(0). AW 626 tests(첫 실행은 위 O5 가드 실패로 종료 1 → 고친 뒤 0), 통합 1(0).

## T057 Codex 후속 집중 리뷰 1 (`--wait --base 5f920d2`, 대상 `4f4f939`의 이벤트 클라이언트 변경)

사용자 요청으로 C1·C2 수정만 따로 검토했다. 주 리뷰 증거는 위의 전체 변경 리뷰다. 판정: needs-attention. High 2건:

| # | 문제(Codex 메모리 내 재현) | 조치 |
|---|---|---|
| F1 | C1의 직렬화가 스냅샷 **적재**까지 사슬에 넣었다. 끝나지 않는 옛 적재(HTTP 호출에 timeout 없음) 뒤에 새 재동기가 무기한 묶이고, `resetting`이 true로 남아 전달이 멈춘다. gap 복구의 재설정도 이 사슬을 기다린다 | 적재를 사슬 밖으로 뺐다. 사슬에는 적용(`onReset`·상태 변경)만 잇는다. 적재 뒤·적용 차례에 세대를 확인해 늦은 결과는 버린다 |
| F2 | C2 합류자 처리가 종결(evicted) 복구에서 `terminal`을 먼저 세운 뒤 `resetListener`를 불렀다. 이 함수는 terminal이면 바로 돌아가므로 합류자는 스냅샷을 받지 못하고 `resetting`이 풀리지 않는다 | 합류자에게 복구 스냅샷을 **같은 데이터로** 적용한다(`applyRecoverySnapshot`). 적용을 기다리는 동안 또 합류하면 다음 묶음으로 반복한 뒤 버퍼를 나눈다. 따로 적재하지 않으므로 종결 여부와 상관없다 |

시험(먼저 작성·실패 확인, `event-client.races.test.ts` "resync liveness"):
| 시험 | red | green |
|---|---|---|
| F1 옛 적재가 끝나지 않아도 새 재동기가 적용되고 전달이 이어짐 | 1: `timed out waiting for the newer resync applies despite the pending load` | 0 |
| F2 종결 복구 중 합류자가 최종 스냅샷을 받음 | 1: `timed out waiting for B gets the final snapshot` | 0 |

회귀(각 1회, 종료 0): workbench-client 65 tests·통합 7, AW 626 tests·통합 1.
남은 한계: `onReset` 콜백 자체가 끝나지 않으면 그 수신자의 뒤 적용은 기다린다(`onEvent`가 끝나지 않으면 그 수신자 전달이 멈추는 것과 같은 수신자 계약).

## T057 Codex 후속 집중 리뷰 2 (`--wait --base 4f4f939`, 대상 `a637adc`)

판정: needs-attention. F1·F2·C1·C2의 기본 경로는 닫혔다고 확인했다. High 1건:

| # | 문제(Codex 메모리 내 재현) | 조치 |
|---|---|---|
| G1 | F2 수정이 복구 완료를 모든 수신자(합류자 포함)의 `onReset` 완료에 묶었다. 합류자의 `onReset`이 끝나지 않으면 그 합류자가 떠난 뒤에도 복구가 끝나지 않는다. 다른 수신자의 live 이벤트가 복구 버퍼에만 쌓인다(기준 커밋은 cursor 6, HEAD는 5). 앞 절의 "그 수신자만 기다린다"는 한계 설명을 넘는 회귀다 | 복구 완료가 어떤 수신자의 `onReset`도 기다리지 않게 했다. 스냅샷 적재 뒤 곧바로 복구를 끝낸다. 수신자마다 버퍼를 대기열에 먼저 넣어 이후 live가 뒤에 이어 붙게 하고, 스냅샷 적용은 수신자별 사슬에서 돈다. 적용 뒤에 `covered`를 세우고 전달을 재개한다. 또 복구 기준점까지를 스트림의 최고 순번으로 올린다. 복구가 끝난 뒤 더 앞선 cursor로 합류한 수신자는 다시 구독해 자기 스냅샷을 받는다(C2 시험이 이 경로로 통과) |

시험: "recovery isolation" — B 재설정 보류 → B 제거 → A가 live 6을 받고 cursor 6. red 1(`timed out waiting for A keeps receiving live events`) → green 0. C2 시험은 첫 구현에서 종료 1(`B gets its own snapshot` 시간 초과: gap 기준점이 최고 순번에 반영되지 않아 합류자가 다시 구독하지 않음) → 최고 순번 반영 뒤 0.
회귀(각 1회, 종료 0): workbench-client 66 tests·통합 7, AW 626 tests·통합 1.

## T057 Codex 후속 집중 리뷰 3 (`--wait --base 4f4f939`, 대상 `23d9859`)

판정: needs-attention. G1의 전역 막힘은 해소됐다고 확인했다. G1 수정이 만든 회귀 High 2건:

| # | 문제(Codex 메모리 내 재현) | 조치 |
|---|---|---|
| H1 | 복구 재설정 **전에** 반영 cursor를 gap 기준점으로 올려 `onReset`에 기준점(5)을 넘겼다(실제 반영 0). run 소비자는 `delivered` 뒤의 스냅샷 이벤트만 다시 반영하므로 1–5를 잃는다 | 재설정에는 실제 반영 순번을 넘기고, 기준점으로의 전진은 재설정 성공 뒤에 한다. 세대 변경 복구는 0을 넘긴다(`epochReset`). 대기열 중복 방지(`lastQueued`)는 곧바로 기준점·버퍼 기준이다 |
| H2 | 복구 버퍼를 대기열에 그대로 복사해 중복 제거가 사라졌다. 적재 중 재연결로 기준점부터 다시 재생되면 `[6,7,6,7]`이 된다 | 버퍼에는 순번이 늘어나는 이벤트만 둔다. 대기열에 옮길 때도 순번 증가만 받는다 |

시험(먼저 작성·실패 확인, "recovery cursors"):
| 시험 | red | green |
|---|---|---|
| H1 재설정 context가 실제 반영 순번(0), 재설정 뒤 cursor 5 | 1: `expected [ 5 ] to deeply equal [ +0 ]` | 0 |
| H2 적재 중 재연결에도 `[6, 7]` 한 번씩 | 1: `expected [ 6, 7, 6, 7 ] to deeply equal [ 6, 7 ]` | 0 |

회귀(각 1회, 종료 0): workbench-client 68 tests·통합 7, AW 626 tests·통합 1.

## T057 Codex 후속 집중 리뷰 4 (`--wait --base 4f4f939`, 대상 `f60edc6`)

판정: needs-attention. 단일 복구에서는 H1·H2가 의도대로 동작한다고 확인했다. High 1건:

| # | 문제(Codex 재현) | 조치 |
|---|---|---|
| I1 | 세대 변경 복구 뒤 보관 gap이 이어지면, 두 번째 복구가 세대 변경 표시(`epochReset`)를 지워 옛 세대 반영 순번(10)을 재설정에 넘긴다. run 소비자는 새 세대 1–5를 잃고, 이후 재연결은 새 세대에 cursor 10을 요청해 cursor-ahead fault로 스트림이 종결된다 | 세대 변경 gap을 받는 즉시 스트림·수신자의 순번 상태(최고 순번, 빈 스트림 cursor, 대기열, 반영 cursor, 대기열 중복 기준, 걸러내기)를 0으로 되돌린다. 진행 중 전달은 세대를 올려 무효로 한다. 복구 객체의 표시(`epochReset`)는 없앴다. 뒤이은 gap·재시도·재설정이 모두 이 값에서 시작한다 |

**실제 run 소비자 회귀 시험(사용자 요청)**: `network-events.test.ts` "run snapshot replay after retention recovery". 실제 `createEventClient` + `createNetworkEvents`의 `agent-run-event` 수신자와, 서버 run 기록 전체를 돌려주는 `run.replay` 가짜 호출을 쓴다. 기존 실제 host 보관 시험은 `snapshot.lastSequence`와 다음 live 경계만 보므로, 소비자가 틀린 반영 순번으로 replay 출력을 모두 걸러도 잡지 못한다.

| 시험 | 결과 |
|---|---|
| 보관 한도 2에서 구독 → 스냅샷의 1–5 출력 → live 6·7, 정확히 `[1..7]` 한 번씩·순서대로 | H1 수정 뒤 0. **H1을 되돌린 변이**(재설정에 `max(실제 반영, 기준점)` 전달)에서 종료 1: `expected [] to deeply equal [ 'eepoch-1-1', 'eepoch-1-2', …(3) ]`(replay 출력 전부 걸러짐). 변이 복원 확인 |
| 세대 변경 → 새 세대 보관 gap → 새 세대 1–5, 그 뒤 live 6 | red 1: `expected 5 to be greater than or equal to 8`(옛 cursor 3으로 걸러져 새 세대 4·5만 나옴) → I1 수정 뒤 0 |
| 이벤트 클라이언트 단위: 세대 변경 뒤 gap에서 재설정 context 0, 새 세대 cursor 5 → live 6 | red 1: `expected 3 to be +0` → 0 |

회귀(각 1회, 종료 0): workbench-client 69 tests·통합 7, AW 628 tests·통합 1.

## T057 Codex 후속 집중 리뷰 5 (`--wait --base 4f4f939`, 대상 `5cd115c`)

판정: needs-attention. I1(세대 변경 뒤 옛 cursor 재사용)은 해소됐다고 확인했다. High 1건:

| # | 문제(Codex 재현, 실제 `createNetworkEvents` 소비자) | 조치 |
|---|---|---|
| J1 | 복구 재설정(replay 콜백)이 끝나기 전에 재연결하면, 재연결이 아직 전진하지 않은 cursor 0을 써서 같은 보관 gap 복구가 다시 시작된다. 새 복구가 앞선 적용을 무효로 만들고 반영 0을 잡아, 출력 1–5를 두 번 내보낸다(`[1..5,1..5]`, 기준 `4f4f939`는 한 번) | 수신자 상태를 둘로 나눴다. **재연결 cursor(`delivered`)**: 적용 중인 스냅샷이 덮는 지점. 복구 완료 즉시 기준점으로 올려, 재설정 중 재연결이 같은 gap을 다시 일으키지 않는다. **실제 반영 순번(`applied`)**: 이벤트 반영과, 끝난 `onReset`(세대가 지났어도)으로만 오른다. 재설정 context는 적용 차례에 `applied`를 읽는다 |

앞선 교대 수정(G1 → H1 → I1 → J1)이 한 증상씩 옮겨 가서, 이번에는 cursor의 두 의미(재연결 기준 대 소비자 반영)를 분리하는 설계로 바꿨다.

시험:
| 시험 | red | green | 변이 |
|---|---|---|---|
| 소비자: replay 콜백 보류 중 재연결 → 출력 1–5 한 번씩, 그 뒤 6 | 1: 출력 11개(중복) | 0 | 두 방어를 모두 되돌림(C, 분리 전 동작) → 1(출력 11개). **재연결 cursor만 되돌리면(B) 통과한다.** `applied`가 중복을 따로 막기 때문이다(추가 복구 1회만 생긴다). 이 소비자 시험은 두 방어가 모두 빠질 때만 실패한다 |
| 단위: 복구 재설정 중 cursor = 기준점 5, 재연결 표 `afterSequence` 5 | 수정과 함께 작성 | 0 | 즉시 cursor를 되돌리면 1: `expected +0 to be 5` |
| 소비자 H1 시험 3건(보관 복구 1–5, 보류 중 재연결, 세대 변경 뒤 gap) | — | 0 | 재설정 context를 재연결 cursor로 되돌림(A, H1 회귀) → 3건 모두 1(`expected [] to deeply equal [ 'eepoch-1-1', …]` 등) |

한계: 끝난 옛 `onReset`이 `applied`를 올리는 경로(재설정 중 **다른 원인의** 새 복구가 겹칠 때)만 따로 겨냥한 시험은 없다. 이 경로는 위 소비자 시험의 변이 C 조합으로만 덮인다.
회귀(각 1회, 종료 0): workbench-client 70 tests·통합 7, AW 629 tests·통합 1.

## T057 Codex 후속 집중 리뷰 6 (`--wait --base 4f4f939`, 대상 `a553aa0`)

판정: needs-attention. J1의 단일 재연결 경로는 방어한다고 확인했다. 후속 5절에서 따로 겨냥한 시험이 없다고 적은 `applied` 경로에서 High 2건:

| # | 문제(Codex 재현) | 조치 |
|---|---|---|
| K1 | 옛 세대 복구 재설정이 세대 변경 뒤에 끝나면, 세대 검사 전에 `applied`를 옛 세대 기준점(5)으로 올린다. `forgetEpochSequences`가 세운 0이 무효가 되어, 새 세대 재설정 context가 5가 되고 새 세대 1–5를 모두 걸러 버린다 | 재설정 시작 때 서버 세대를 잡아 두고, 끝난 재설정의 `applied` 갱신은 같은 세대일 때만 한다(복구 재설정·수신자 재동기 둘 다) |
| K2 | `applied`를 gap 기준점까지만 올렸다. 적재 중 6·7이 생겨 스냅샷이 7까지 반영해도 5로 기록되어, 겹친 복구가 6·7을 다시 내보낸다(`[1..7,6,7,8..12]`) | `SnapshotSource.position`(스냅샷이 반영한 마지막 순번)을 더했다. run 스냅샷은 `lastSequence`다. 끝난 재설정은 `applied`를 기준점·재동기 시점 최고 순번·`position` 중 큰 값까지 올린다 |

실제 `createNetworkEvents` run 소비자 시험(먼저 작성·실패 확인):
| 시험 | red | green | 변이 |
|---|---|---|---|
| K1 옛 세대 재설정 보류 → 서버 재기동·새 세대 보관 gap → 보류 해제 → 새 세대 출력 1–6 | 1: `expected [ 'eepoch-2-6' ] to deeply equal [ 'eepoch-2-1', …]` | 0 | 세대 검사 제거 → 같은 실패(1) |
| K2 스냅샷이 기준점(5)보다 앞선 7까지 반영 → 재설정 보류 중 끊김·보관 한도 초과 → 겹친 복구 → 출력 `[1..12]` 한 번씩 | 1: 출력 14개(6·7 중복) | 0 | run 스냅샷의 `position` 제거 → 같은 실패(1) |

후속 5절의 한계("끝난 옛 onReset이 applied를 올리는 경로만 따로 겨냥한 시험 없음")는 K1·K2 소비자 시험으로 겨냥했다. 교환·orchestration·worktree 스냅샷은 재설정 context를 쓰지 않아 `position`이 없다.
회귀(각 1회, 종료 0): workbench-client 70 tests·통합 7, AW 631 tests·통합 1.

## T057 Codex 후속 집중 리뷰 7 (`--wait --base 4f4f939`, 대상 `4d621f5`)

판정: needs-attention. K1·K2 시험이 각 방어를 직접 겨냥하고, 후속 6절의 변이 결과가 맞다고 확인했다(Codex는 메모리 실행으로 확인했고, 정식 Vitest는 그 환경의 파일시스템 제한으로 돌리지 못했다). High 1건:

| # | 문제(Codex 재현, 실제 서버 `decide_existing`도 같은 판정) | 조치 |
|---|---|---|
| L1 | 세대 변경 복구(기준점 0) 뒤 재연결 cursor는 0인데 `applied`는 5다. 여기서 서버가 다시 재기동하면 cursor 0 요청에는 `epochChanged`가 아니라 보관 gap(`retentionExceeded`)이 온다. 이 분기는 세대만 바꾸고 순번 상태를 잊지 않아, 새 세대 재설정에 5를 넘기고 새 세대 1–5를 잃는다(`a553aa0`에서는 모두 전달됐다) | gap의 세대가 스트림 세대와 다르면 **사유와 상관없이** 순번 상태를 잊는다(`forgetEpochSequences`). 알려진 세대에서 바뀐 경우에만 `onEpochChanged`를 알린다. 첫 연결 전 세대가 비어 있는 경우는 제외한다 |

실제 `createNetworkEvents` run 소비자 시험: "forgets the old epoch's applied position when a new epoch arrives as a retention gap on a cursor-0 reconnect".
- red(종료 1): `expected [ 'eepoch-3-6' ] to deeply equal [ 'eepoch-3-1', …]`. green 0.
- 변이 1(세대 확인을 `epochChanged`로 한정)은 **통과했다(종료 0). 이 변이는 옛 코드와 같지 않았다.** default 분기의 세대 갱신까지 빠뜨려, 복구가 옛 세대로 연결되고 서버가 따로 `epochChanged`를 보냈다.
- 옛 동작과 같은 변이 2(한정 + default 분기가 세대만 갱신)는 종료 1로, 위와 같은 실패였다.

회귀(각 1회, 종료 0): workbench-client 70 tests·통합 7, AW 632 tests·통합 1.
