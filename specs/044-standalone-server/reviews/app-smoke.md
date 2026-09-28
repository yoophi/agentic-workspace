# 044 앱 스모크 (T036 · T045 · T046 · T047)

2026-09-28, macOS Apple Silicon. 모든 실행은 **외부 서버 모드**(`AW_WORKBENCH_MODE` 없음)다. 앱 로그 `[workbench] mode: external`, 접근 기록은 서버 로그(`<data>/workbench/server/server.log`)에만 있고 앱 로그의 in-process `[workbench-http]` 줄은 0이다.

- 사용자 설치본과 데이터를 나누려고 별도 identifier를 썼다. 최종 Gate20은 배포 `…smoke044g20r`(`AW Gate20 Rel.app`)·개발 `…smoke044g20d`(`AW Gate20 Dev.app`)다. 앞선 T036·종료 시험의 identifier는 아래 과거 기록에 남긴다.
- 결과 파일은 `app-smoke/<실행>/`(probe·owner-check 보고, 실행 메타)다. 토큰·표 문자열은 없다. 복사 전에 모든 비밀 파일의 토큰 값이 이 파일들에 없음을 확인했다. 경로는 `<scratchpad>`·`~`로 줄였다.
- 종료는 각 실행이 띄운 **정확한 PID**만 했다(서버는 `server.json`의 pid + 명령줄의 실행 파일·데이터 디렉터리 확인 뒤). 삭제 명령은 쓰지 않았고, 실행마다 새 디렉터리 이름을 썼다. 사용자 설치본 프로세스는 건드리지 않았다.
- 가짜 agent: `fake_acp_permission_agent.py --echo`.

## 출처

| 출처 | 빌드 | Origin | 서버 실행 파일 |
|---|---|---|---|
| 개발(T036) | `tauri dev`(`VITE_AW_DEBUG_PROBE=1`) | `http://localhost:1420` | `AW_WORKBENCH_SERVER_PATH` → `target/debug` |
| 배포(T036) | `tauri build --debug --no-bundle` | `tauri://localhost` | **앱 실행 파일 옆**(경로 변수 없음, `server-path=neighbor`) |
| 배포 번들(T045–T047) | `tauri build --debug --bundles app` | `tauri://localhost` | 번들 `Contents/MacOS/`에 서버 실행 파일을 복사(externalBin 대용, (f) 배포는 미완료) |
| 개발 번들(T045·T046) | 같은 번들, `build.frontendDist = http://localhost:1420`, vite 따로 실행 | `http://localhost:1420` | 위와 같음 |

종료 경로 (c)(d)(e)는 번들 id가 있어야 자동화된다(Dock 항목, `tell application id … to quit`). 그래서 T045·T046은 두 출처 모두 번들로 실행했다. 개발 출처는 WebView가 개발 서버를 읽는 번들이다.

## r13 최종 앱 재실행 — Gate20 (`c4c8e0b`)

- 코드 출처: commit `c4c8e0bfd73bbea833ff9683fc449e3a8997ab01`, tree `4426dfd2cf41f8048448b4c11e9008d52588b8a0`.
- `VITE_AW_DEBUG_PROBE=1`로 새 번들을 만들었다. 배포 app SHA-256 `d5ae13b3…6fe15c`, 개발 app `d96b7a16…df194`, 두 번들의 server `4953570c…092c18`. 전체 값은 `app-smoke/gate20-source.txt`에 있다.
- 실행 직전마다 화면 잠금이 아님을 확인했다. 키 경로는 대상 PID가 frontmost인지 재확인했고, b2는 front window가 `Settings`인지 연속 두 번 확인한 경우에만 Cmd+W를 보냈다.
- `g19-rel-b2`의 첫 시도는 상대 bundle 경로 때문에 PID를 찾지 못해 키 미전송, `g19-rel-b2-2`는 probe 빌드 환경 누락으로 키 미전송이라 무효다. Gate19에서 front window 확인을 강화한 `g19-rel-b2-4`까지도 두 창 종료·retire 2회를 재현해 제품 결함을 확정했다. 이를 근거로 macOS predefined close를 focused Tauri window 하나만 닫는 custom 메뉴로 교체했다.

### T045 / SC-001

모든 경로는 `BUSY=1 TOKEN=1`이다. 앱 종료 전 turn을 gate로 붙잡고, 앱 PID 소멸 뒤 `busyRuns=1`과 새 prompt 없는 live 출력·완료를 owner로 관측한 뒤 취소했다.

| 경로 | 배포 | 개발 | 창 토큰 |
|---|---|---|---|
| (c) Cmd+Q | `g20-rel-qc` 0 | `g20-dev-qc` 0 | 종료 뒤 401 |
| (d) Dock Quit | `g20-rel-qd` 0 | `g20-dev-qd` 0 | 종료 뒤 401 |
| (e) AppleScript quit | `g20-rel-qe` 0 | `g20-dev-qe` 0 | 종료 뒤 401 |
| (g) 검증된 PID SIGTERM 대조 | `g20-rel-qg` 0 | `g20-dev-qg` 0 | 앱 처리 없음, 계약대로 200 |

8개 실행 모두 `path-exercised=yes`, `owner-check-exit=0`, `smoke-result=ok`였다. 따라서 T045와 SC-001은 충족한다.

### T046 / SC-006

| 경로 | 기대 | 배포 | 개발 |
|---|---|---|---|
| (a) main 빨간 버튼 | Settings만 남음, run 제거, 토큰 401 | `g20-rel-a` 0 | `g20-dev-a` 0 |
| (b1) File > Close Window | Settings만 남음, run 제거, 토큰 401 | `g20-rel-b1` 0 | `g20-dev-b1` 0 |
| (b2) Settings 앞 Cmd+W | main·run 유지, 토큰 200, Settings retire 1회 | `g20-rel-b2` 0 | `g20-dev-b2` 0 |
| (f) 마지막 창 빨간 버튼 | 앱 종료, run 제거, 토큰 401 | `g20-rel-f` 0 | `g20-dev-f` 0 |

b2 두 출처 모두 `windows-after-close=Agentic Workbench`, `run-removed=no`, 토큰 200, `retireWindow-calls=1`이다. 나머지는 대상 main 작업대와 토큰만 폐기했다. T045의 정상 앱 종료에서는 작업대·run을 닫지 않았다. 따라서 T046과 SC-006은 충족한다.

아래 내용은 Gate20 전의 조사·재판정 기록이다. 미검증 또는 미해결이라는 표현은 당시 상태이며, 위 최종 표가 현재 판정이다.

## T036 — 043 스모크, 외부 서버 모드 (SC-010)

| 실행 | 출처 | 결과 | 근거 |
|---|---|---|---|
| `t036-dev-1` 출력 + 강제 재연결 | 개발 | ok | 시작·끊긴 뒤 에코 각 1회, 소켓 끊기 1, 순번 1–13 중복·빈 순번 없음. 서버 로그 `event-tickets` 2, agent prompt 2 |
| `t036-dev-refresh-1` 새로고침 1회 전달 | 개발 | ok | 새로고침 뒤 라우팅 1·확인 1, 앱 스트림 에코 1. agent 기록에서 메시지 prompt 1(전체 4) |
| `t036-rel-1` 출력 + 강제 재연결 | 배포(옆 서버) | ok | 개발과 같음(`event-tickets` 2, prompt 2) |
| `t036-rel-refresh-1` 새로고침 1회 전달 | 배포(옆 서버) | ok | 라우팅 1·확인 1·에코 1, 메시지 prompt 1(전체 4) |

범위: 043과 같다(패널 UI 라우팅은 probe가 대신함). 043에서 돌리지 않았던 배포 출처 새로고침을 이번에는 돌렸다.

## T045 — 앱 종료 뒤 run 지속 (SC-001)

증거는 두 가지이고 범위가 다르다.

1. **진행 중 turn의 실행·출력 지속**(아래 "진행 중 turn 지속", Codex 문서 리뷰 반영) — SC-001의 근거.
2. **쉬는 세션의 지속과 후속 호출**(이 절의 첫 표) — 시작 turn이 **끝난 뒤**(`promptCompleted`, 종료 직후 `busyRuns=0`) 앱을 끄고, 종료 뒤 소유자가 **새 prompt**로 출력을 본다. 세션이 남는다는 것만 증명하고, 진행 중 turn이 앱 종료를 넘어 계속되는지는 증명하지 않는다(Codex 문서 리뷰 지적). 이 증거만 있을 때 T045는 부분 검증이었다.

### 진행 중 turn 지속 (SC-001)

흐름(`quit-busy` probe, `BUSY=1 quit-run.sh`):

1. 가짜 agent(`--end-turn-gate <파일> --after-gate-chunk`)가 시작 turn을 문 파일로 붙잡는다. probe는 시작 에코만 받고 **완료 전에** `ready-to-quit-busy`를 보고한다(`completedBeforeQuit: false`).
2. 그 경로로 앱을 끈다. **앱 PID 소멸**을 확인한다. 직후 `server.status.busyRuns = 1`이다(turn이 서버에서 진행 중). 이때 문 파일은 없다.
3. `owner-check.py --observe-turn --release-file`:
   - replay에 완료가 없음을 확인한다(`replayHasCompletion: false`).
   - 구독을 연다.
   - 고유 표지를 문 파일에 원자적으로 쓴다(열기).
   - **새 prompt 없이**(`promptSent: false`) live로 기존 turn의 **새 출력** `after-gate:<표지>`(`liveOutputAfterRelease`)와 **완료**(`liveCompletion`)를 모두 받아야 통과한다.
   - 그 뒤 취소한다.
4. 표지는 앱 종료 뒤에 만들어진다. 그래서 그 출력은 앱이 없을 때 서버가 이어 간 turn이 만든 것이다. agent 기록의 prompt는 1개(시작 prompt)이고, `after-gate` 기록도 1개다.

| 경로 | 배포 | 개발 |
|---|---|---|
| (c) Cmd+Q | ok `t045-rel-c-busyout-1` | ok `t045-dev-c-busyout-1` |
| (d) Dock Quit | ok `t045-rel-d-busyout-1` | ok `t045-dev-d-busyout-1` |
| (e) AppleScript `quit` | ok `t045-rel-e-busyout-1` | ok `t045-dev-e-busyout-1` |
| (g) `SIGTERM` | ok `t045-rel-g-busyout-1` | ok `t045-dev-g-busyout-1` |

- **run 조회 실패 분리(Codex r12 docs)**: 옛 `close-run.sh`는 `bench-check.py`의 종료 코드를 버리고, 출력에 `"runListed": false`가 없으면 "제거 안 됨"으로 봤다. 그래서 b2(run 유지 기대)에서 조회가 모두 실패해도 통과할 수 있었다. 지금 스크립트는 조회 결과를 있음·없음·검사 실패로 나누고(`run_state_of`), 한 번도 성공한 조회가 없으면 7로 끝난다(`run-state-unknown`). 과거 원자료 재판정: 옛 루프의 `run-removed=yes`는 출력에 `"runListed": false`가 있을 때만 기록되므로 모두 성공한 조회다. b2 과거 실행 7개는 모두 `run-removed=yes`(제거)라 조회 실패를 run 유지로 잘못 통과한 경우는 없다. 자기 시험(75개)에 조회 실패(`identity-failed`, 빈 출력, 잘못된 JSON)를 넣었다.
- **b2 기대값 정정(Codex r11 docs)**: 10차 판정은 b2를 (a)(b1)과 같이 "Settings만 남음"으로 봤는데, 이는 실제 닫기 대상과 반대였다. `close-run.sh`는 b2에서 Settings를 연 뒤 앞에 있는 Settings에 Cmd+W를 보낸다(원자료 `front=Settings`). 그래서 올바른 b2 결과는 Settings만 닫히고, 메인 창(`Agentic Workbench`)·앱·run이 남고, 메인 창 토큰이 200인 것이다. 지금 스크립트는 경로별로 기대를 나눈다: (a)(b1)(f)는 run 제거·토큰 401, (b2)는 run 유지·토큰 200. 창 판정은 (b2)가 남은 창 `Agentic Workbench`다. 과거 b2 7개 실행은 모두 앱 전체가 종료돼(원자료 `app-alive-after-close=no`) 이 기대로도 9이고, b2는 여전히 미해결이다. 자기 시험(66개)에 정상 b2(Settings만 닫힘 → 0), 배경 메인 창만 닫힘(9), run 제거(7), 토큰 401(8)을 넣었다.
- **닫기 동작·창 상태 재판정(Codex r10 docs)**: 옛 `close-run.sh`는 클릭 명령의 실패 코드를 버리고, 앱 생존·남은 창을 기록만 했다. 그래서 각 실행의 원자료(`action:` 줄, `app-alive-after-close=`, `windows-after-close=`)로만 다시 판정했다.
  - (a)(b1) 14개: 모두 앱이 살아 있고 남은 창은 `Settings` 하나다(대상 창만 닫힘) → 통과.
  - (f) 5개: 모두 앱이 종료됐다 → 통과.
  - (b2) 7개: 모두 앱 전체가 종료됐다(두 창 모두 닫힘) → **새 판정으로 9(예상 밖 창 상태)**(11차에 b2 기대를 "메인 창 유지"로 정정한 뒤에도 9). 이미 적힌 "(b2) 두 창 모두 닫히고 종료, 미해결" 기록과 같다. b2는 통과로 세지 않는다.
  - 지금 스크립트: 닫기 동작 실패는 무효 6, 창 상태가 경로 기대와 다르면 9(`window_verdict`; b2 기대는 11차에 정정), 이어서 run 제거 여부가 경로 기대와 다르면 7, 토큰이 경로 기대와 다르면 8(`close_final`)이다. 닫기 동작부터 끝까지를 그대로 떼어 모의 입력으로 돌리는 자기 시험(59개)으로 확인했다.
- **run 제거·토큰 판정 재판정(Codex r9 docs)**: 옛 `close-run.sh`는 run 잔존·토큰 200을 기록만 하고 0으로 끝났고, `quit-run.sh`도 TOKEN=1의 토큰 결과를 종료 코드에 넣지 않았다. 그래서 각 실행의 원자료 `meta.txt`의 `run-removed=`와 `token-after-close=`/`token-after-quit=`로만 다시 판정했다.
  - 창 닫기 실행 21개(`t046-*`): 모두 `run-removed=yes`. 토큰을 확인한 실행은 모두 `401 unauthenticated`다.
  - quit TOKEN 실행(`t045-*-busytok-*`): 유효 경로 (d)(e)는 모두 401, (g) SIGTERM 대조는 200(기대값). (c) Cmd+Q 실행은 모두 `path-exercised=no`(무효, 이미 기록)이고, 그 200은 판정 대상이 아니다.
  - 지금 스크립트: `close-run.sh`는 run 잔존이면 7, 토큰 미폐기·검사 오류면 8, 통과하면 0으로 끝난다(`close_final`). `quit-run.sh`는 정상 종료(c·d·e)에 401, (g)에 200을 요구하고, 어긋나면 8로 끝난다(`token_verdict`, `smoke_final`). 두 스크립트의 판정 구간을 그대로 떼어 모의 입력으로 돌리는 자기 시험(46개)으로 확인했다.
- **owner-check 종료 코드 재판정(Codex r8 docs)**: 옛 `quit-run.sh`는 owner-check 실패를 기록만 하고 최종 종료 코드에 넣지 않았다. 그래서 이 문서의 실행을 스크립트 종료 코드가 아니라 각 실행의 원자료 `meta.txt`의 `owner-check-exit=`로만 다시 판정했다.
  - 이 문서가 인용한 실행 31개(`t045-*`) 모두 `owner-check-exit=0`이다.
  - 경로가 무효인 실행(`path-exercised=no` 또는 `app-gone=no`: `t045-{dev,rel}-c-busytok-2`, `t045-rel-c-busytok-3`, `t045-rel-d-busy-1`)은 owner-check와 관계없이 이미 무효로 기록돼 있다.
  - 옛 실행(`t045-*-{c,d,e,g}-1`)에는 `path-exercised` 줄이 없다(그 기록 전 스크립트). 판정 근거는 `app-gone=yes`와 owner-check다.
  - 지금 스크립트는 경로 무효면 5, 유효한 종료 뒤 owner-check 실패면 6, 둘 다 통과해야 0으로 끝난다(`smoke_final`). 실제 스크립트의 판정 이후 구간을 그대로 떼어 모의 입력으로 돌리는 자기 시험으로 확인했다.
- **(g) 송신 기록 한계(Codex r7 docs)**: 표의 모든 (g) 실행(`t045-*-g-*`)은 신원 확인·송신 결과 기록(`02831fe`·`c0afc54`) 전 스크립트로 돌았다. 앱 PID에 맨 `kill -TERM`을 보냈고 그 종료 코드는 기록하지 않았다(`kills.txt`에는 서버 정리만 있다). 앱은 신호 직전 probe 완료(`probe-status=done`) 상태였다. 따라서 이 실행들의 `path-exercised=yes`는 PID 소멸만 근거이며, 지금 규칙(`quit-action-sent=yes`와 PID 소멸을 모두 요구, 아니면 무효·종료 코드 5)으로 다시 판정한 증거가 아니다. 새 규칙은 모의 자기 시험(`selftest-signal.sh`, 최종 판정·종료 코드 포함 20개)으로만 검증했고, 실제 앱 (g) 재실행은 하지 않았다(후속).
- 모든 실행: 종료 뒤 `busyRuns=1`. live 순번 `[9, 10]` = 9 `after-gate:<표지>` 출력, 10 완료. 취소 완료.
- 무효 실행 기록: `t045-rel-d-busy-1`은 Dock 메뉴가 열리기 전에 누름이 가 `Invalid index`가 났다. 앱이 끝나지 않아 스크립트가 정확한 PID에 TERM을 보냈다. 그래서 (d) 증거가 아니다.
  - 이후 스크립트는 메뉴가 열릴 때까지 조건 대기한다(`dock-menu-open`).
  - 경로로 끝나지 않은 실행은 `path-exercised=no`로 무효를 적는다.
- `*-busy-1`(출력 표지 전 1차, 완료만 확인)은 scratchpad에 남기고, 이 표는 출력까지 확인한 `*-busyout-1`로 대체한다.

### 정상 Quit 뒤 옛 창 토큰 거절 + 진행 중 turn 지속 (Codex 코드 리뷰)

- 지적: 정상 Quit(Cmd+Q·Dock·AppleScript)은 `Exit`만 온다(R8). 그래서 창이 폐기되지 않아 종료한 앱의 창 토큰이 최대 15분 유효했다.
- 수정: 종료 경로가 살아 있는 창을 모두 `retireWindow{closeBench:false}`로 폐기하고, 2초 상한 안에서 기다린다. 단위 시험 `an_exit_retires_the_open_windows_without_closing_their_benches`: compile red → 동작 red(200 ≠ 401) → green.
- 실제 앱(`quit-busy-token` probe, `BUSY=1 TOKEN=1 quit-run.sh`)으로 한 실행에서 둘 다 본다.
  - 시작 turn이 진행 중인 채로 그 창 토큰을 비밀 파일(0600)에 넘긴다. 종료 전 같은 토큰으로 handshake하면 200이다.
  - 그 경로로 종료한다. 앱 PID가 사라진 뒤 같은 토큰(그 창 Origin)으로 handshake한다.
  - `busyRuns=1`을 확인한다. owner-check가 새 prompt 없이 종료 뒤 출력과 완료를 live로 받는다. 그 뒤 취소한다.

| 경로 | 배포 | 개발 | 종료 뒤 옛 창 토큰 |
|---|---|---|---|
| (d) Dock Quit | ok `t045-rel-d-busytok-1` | ok `t045-dev-d-busytok-1` | 401 `unauthenticated` |
| (e) AppleScript `quit` | ok `t045-rel-e-busytok-1` | ok `t045-dev-e-busytok-1` | 401 |
| (g) `SIGTERM`(대조) | `t045-rel-g-busytok-1` | `t045-dev-g-busytok-1` | **200**. 앱 처리가 없어 폐기되지 않는다. 토큰 TTL(15분)과 임대 TTL로만 거둔다(계약대로, 한계) |
| (c) Cmd+Q | **미검증**(무효 2회 + 1회) | **미검증**(무효 1회) | — |

- 모든 유효 실행에서 진행 중 turn은 이어졌다: `busyRuns=1`, 새 prompt 없이 `after-gate` 출력과 완료가 live로 왔고, 취소를 마쳤다. prompt는 1개다.
- (c) 무효 이유:
  - `t045-{rel,dev}-c-busytok-1`에서는 Cmd+Q 키 입력이 앱에 닿지 않았다. 앱 로그에 `exit:` 줄이 없어 `Exit` 자체가 없었다. 스크립트가 `path-exercised=no`로 적고 정확한 PID로 정리했다.
  - 다시 시도한 `t045-rel-c-busytok-2`에서 앞 프로세스를 확인하게 했다. 앞 프로세스가 `loginwindow`, 즉 **Mac 세션이 잠긴 상태**였다(`frontmost-before-cmd-q=no`). 잠긴 동안에는 키 입력 자동화가 불가능하다. Dock·AppleScript 경로는 접근성·Apple Event라 동작했다.
  - **안전 사고 기록**: 이 세 번의 (c) 시도는 대상 앱이 앞인지 확인하지 않은 채(`-2`는 확인이 실패했는데도) 전역 Cmd+Q 키 입력을 보냈다. 키 입력은 앞 프로세스로 가므로 다른 사용자 앱을 끌 수 있었다.
    - 직후 확인: 보이는 프로세스 목록이 시도 전과 같고(ghostty·Aside·Finder·handy·agentic-workbench(설치본, 1일 이상 실행 중)·Chrome·mermaid-live·Safari·ChatGPT·Slack), 설치본 AW PID도 그대로다. 앞 프로세스가 `loginwindow`(잠금)라 키 입력이 앱에 닿지 않은 것으로 보인다.
    - 스크립트 수정: 대상 APID가 앞 프로세스임을 조건 대기로 확인하고, 보내기 직전에 다시 확인한다. 둘 중 하나라도 실패하면 키 입력을 **보내지 않고** 그 시도를 무효로 기록한다(`cmd-q-not-sent`).
  - 원인 진단(읽기 전용, UI 재시도 반복 전):
    - `CGSessionCopyCurrentDictionary`: `onConsole=1`, `CGSSessionScreenIsLocked=1`, NSWorkspace 전면 앱 `loginwindow`(pid 179).
    - 대상: `NSRunningApplication(pid=APID)` = `AW Quit 044`(bundle `…smoke044qr`), activationPolicy Regular, 숨김 아님, 기동 완료, `active=false`. System Events 프로세스(같은 unix id)는 `agentic-workbench`, visible, frontmost false, background only false.
    - 따라서 APID↔GUI 프로세스는 일치하고, 숨김·백그라운드 전용도 아니다. 세션 잠금이 활성화를 막는다. 화면 잠금 우회나 다른 앱 키 입력은 하지 않았다.
  - 안전 분기 뒤 재시도 `t045-rel-c-busytok-3·-4`, `t045-dev-c-busytok-2·-3`: 모두 `frontmost=no` → 키 미전송 → 무효.
    - `-4`는 잠금 중 느린 osascript로 전면 대기가 2분을 넘었다. 가짜 agent의 문 대기 상한(120초)이 먼저 와 agent가 끝났고(`gate-abandoned`), 그래서 owner-check가 run을 찾지 못했다(`owner-check-exit=1`). 제품 실패가 아니라 무효 시도의 부작용이다.
  - Cmd+Q도 (d)(e)와 같은 `Exit` 처리 경로를 쓴다. 하지만 이것을 검증으로 세지 않는다. **세션 잠금 해제 뒤 (c)를 다시 실행해야 한다.**
- 이 수정 전의 (c) 증거(`t045-*-c-busyout-1`)는 진행 중 turn 지속만 보인다. 그때 토큰은 폐기되지 않았다.

### 최종 빌드 재실행 (`b8da72f`, OCR 2차·Codex 3차 수정 뒤)

OCR 2차 수정이 종료 폐기 경로(연결을 잃은 뒤에도 종료 폐기, 사라진 서버 인스턴스 잊기)와 스모크 스크립트(Dock 안전)를 바꿨다. 그래서 정상 Quit 조합을 최종 빌드로 다시 돌렸다(`BUSY=1 TOKEN=1`).

| 실행 | 결과 |
|---|---|
| `t045-rel-d-final-1`, `t045-dev-d-final-1` (Dock) | Dock 이름 → pid가 정확히 APID(`dock-name-pids`), 메뉴 열림 확인 뒤 누름. 종료 뒤 옛 창 토큰 401, `busyRuns=1`, 새 prompt 없이 종료 뒤 출력·완료 live, 취소. 앱 로그 `exit:` 1줄 |
| `t045-rel-e-final-1`, `t045-dev-e-final-1` (AppleScript) | 같음 |

- **(c) Cmd+Q는 여전히 미검증이다.** 세션이 잠긴 상태(`screenLocked=1`)라 다시 시도하지 않았다. 안전 분기 뒤 시도들(`t045-{rel,dev}-c-busytok-{3,4}`/`-{2,3}`)은 모두 키를 보내지 않은 무효 시도다. 잠금 해제 뒤 실행해야 한다.
- 스모크 안전 조건:
  - Cmd+Q는 대상 APID가 앞 프로세스임을 두 번 확인한 뒤에만 보낸다.
  - Dock은 표시 이름(`NSRunningApplication.localizedName`)이 정확히 이 APID 하나일 때, 메뉴가 열린 것을 확인한 뒤에만 누른다. 설치본 `Agentic Workbench`는 다른 표시 이름이라 겹치지 않고, System Events 프로세스 이름(`agentic-workbench`)은 쓰지 않는다.
  - 신호는 기록한 신원(시작 시각 + 명령줄)이 같을 때만 보낸다.
  - 실행마다 보이는 앱 목록이 그대로임을 확인했다.

- 스크립트 준비: `scripts/apps-named.swift`와 `winid.swift`, 진단용 `session-state`를 `swiftc -O <file>.swift -o $SMOKE/<name>`으로 `$SMOKE`에 빌드한다. `apps-named`가 없으면 Dock 경로는 누르지 않고 무효로 끝난다(닫힌 쪽으로 실패).

### 쉬는 세션 지속 (보조 증거)

흐름(`quit` probe): 에코 run 시작 → 시작 에코·완료 → `ready-to-quit`(run 살려 둠) → 그 경로로 종료 → **앱 PID 소멸** → 서버 PID 생존 → `server.status` → `owner-check.py`(identify 증명 → handshake → `bench.list`에서 같은 run → replay → 구독으로 소유자 prompt 에코를 live로 받음 → `run.cancel` → 목록에서 사라짐).

| 경로 | 배포 | 개발 |
|---|---|---|
| (c) 앱 메뉴 Quit(Cmd+Q) | ok (`t045-rel-c-1`, `-c-2`) | ok (`t045-dev-c-1`) |
| (d) Dock 메뉴 Quit | ok (`t045-rel-d-1`, `-d-2`) | ok (`t045-dev-d-1`) |
| (e) AppleScript `quit` | ok (`t045-rel-e-1`, `-e-2`) | ok (`t045-dev-e-1`) |
| (g) `SIGTERM` | ok (`t045-rel-g-1`, `-g-2`) | ok (`t045-dev-g-1`) |

- 모든 실행에서 앱 PID가 사라진 뒤 서버는 살아 있었다. owner-check 결과는 모두 같다: replay 1–9 연속, live 10·11(에코), `liveAfterReplay`, 취소 완료.
- 임대: 종료 직후 `server.status.leases`는 (c)(d)(e)에서 0(앱이 `lease.release`), (g)에서 1(앱 처리 없음, TTL로 거둠). 두 출처 모두 같다(`*-2`, `t045-dev-*`의 `status-after-quit.json`).
- 기록 정정: 첫 배포 실행(`*-1`)에서 "종료 뒤 서버 로그 조각"으로 (d)(e)의 `lease.release`를 못 봤다. 두 번째 실행에서는 (c)도 비어 있어, 로그 조각 방식이 믿을 수 없다고 판단했다. 판정은 `server.status`의 임대 수로 한다(위).
- (h) 로그아웃·재시동은 관측 불가·미검증이다(T048).

## T046 — 창 닫기 대조 (SC-006)

흐름(`close-token` probe = `quit` 흐름 + 이 창 토큰 넘기기): 에코 run 시작 → 그 창의 연결 토큰을 **0600 비밀 파일에만** 쓴다(보고서 없음) → 창 안에서 그 토큰으로 handshake한 상태 코드만 보고 → 창 닫기 경로 → `bench.list`에서 run이 사라질 때까지 조건 대기 → 같은 토큰 + 그 창 Origin으로 handshake(상태 코드만 출력, `token-check.py`).

| 경로 | 앱 | run·작업대 | 같은 창 토큰 닫기 전 → 뒤 | 배포 | 개발 |
|---|---|---|---|---|---|
| (a) 빨간 버튼(main, Settings 남음) | 살아 있음, Settings만 남음 | 제거(작업대 0) | 200 → 401 `unauthenticated` | ok | ok |
| (b1) 메뉴 `File > Close Window`(main 앞) | 살아 있음, Settings만 남음 | 제거 | 200 → 401 | ok | ok |
| (f) 마지막 창 빨간 버튼 | 종료 | 제거 | 200 → 401 | ok | ok |
| (b2) Cmd+W 키 입력(System Events, Settings 앞) | **두 창 모두 닫히고 종료** | 제거 | 200 → 401 | 관측대로 재현 | 관측대로 재현 |

- 실행: `t046-{rel,dev}-{a,b1,f,b2}-tok-1`. 창 안 probe의 닫기 전 handshake도 200이었다(`probe.json` `steps.tokenBeforeClose`). 비밀 파일은 0600이며 저장소에 넣지 않았다.
- 401이 만료 때문이 아님: 창 토큰 TTL은 15분(`DESKTOP_TOKEN_TTL`)이다. 각 실행은 토큰 발급부터 끝까지 6–11초였다.
- (a)(b1)에서 서버가 떠 있고 다른 창(Settings)이 붙어 있는 채로 401이 나온다. 거절은 그 창의 폐기(`desktop.retireWindow{closeBench:true}`) 때문이다.
- 토큰 없는 앞선 실행(`t046-*-{a,b1,f,b2}-1`, scratchpad)도 같은 run 제거를 보였다. 이 표는 토큰 확인 실행으로 대체한다.
- (b2) 위험: 자동화한 Cmd+W 한 번이 두 창을 닫는다(spike와 같음). 원인은 확인하지 못했다. 사람이 누른 Cmd+W도 같은지는 확인하지 않았다(미확인 위험, T048).

## T047 — 연결 실패 화면과 서버 기동 (SC-005)

배포 번들, `AW_WORKBENCH_SERVER_PATH`에 없는 경로를 줬다(`t047-rel-3`, 창만 캡처):

1. 앱이 연결 실패 화면을 보였다: "Workbench 서버에 연결하지 못했습니다", 이유 `No such file or directory (os error 2)`, "다시 시도" 버튼(`app-smoke/t047-rel-3/failure-window.png`). 이때 서버 프로세스와 `server.json`은 없었다.
2. 그 경로에 서버 실행 파일 링크를 만든 뒤 "다시 시도"를 눌렀다. 서버가 한 번 떠(프로세스 1개) 작업 대시보드가 보였다(`after-retry-window.png`). `server.status`: 임대 1.
3. 서버가 떠 있을 때 새로 띄우지 않음(`t047-rel-2`): AppleScript `quit` 뒤 서버는 남았다(임대 0). 앱을 다시 띄우자 같은 서버 PID(87805)에 붙었다. 임대는 1로 돌아왔고 서버 프로세스는 여전히 1개였다.

- 캡처: 창 id로 AW 창만 찍었다(`screencapture -l`). 처음 실행(`t047-rel-2`)의 화면 영역 캡처에는 다른 앱의 떠 있는 창이 함께 찍혀 저장소에 넣지 않았다.
- 첫 시도(`t047-rel-1`)는 WebView 내용을 접근성 API로 읽으려다 실패했다(글자 0). 판정에서 제외했다. 그 실행에서도 실패 동안 서버 프로세스는 0이었다.
- 동시 기동 1회(동시 10회 ensure)는 프로세스 시험(T021 계열) 몫이다. 이 스모크는 앱 한 번의 기동만 본다.
- 개발 출처 T047은 돌리지 않았다(연결 실패는 WebView Origin과 무관한 기동 경로).

## 리뷰 수정 뒤 다시 실행 (배포 번들, `068fa09`)

OCR 구현 리뷰 수정(임대 갱신 실패 유지·재획득, 창 작업대 표의 서버 인스턴스·incarnation 묶음, 종료 잠금 상한, MCP drain 코드, ensure 상태 확인)이 종료·창 닫기 경로를 바꿨으므로 배포 번들로 핵심 경로를 다시 돌렸다.

| 실행 | 결과 |
|---|---|
| `t045-rel-{c,e,g}-postfix-1` | 세 경로 모두 앱 PID 소멸 뒤 owner-check ok(같은 run replay·live 에코·취소). 임대 (c)(e) 0, (g) 1 |
| `t046-rel-{a,b1,f}-postfix-1` | (a)(b1) 앱 생존·run 제거, (f) 앱 종료·run 제거. 같은 창 토큰 200 → 401 |

개발 출처는 리뷰 수정 뒤 다시 돌리지 않았다(위 수정은 WebView Origin과 무관한 Rust 경로다).

### Cmd+W (b2) 가설 시험 — 기각, 미해결

- OCR이 원인 가설로 "`Destroyed` 처리 안의 창 메뉴 동기 재구성이 AppKit 키 동작 처리 중 메뉴를 바꿔 두 번째 Close Window가 다음 앞 창에 맞는다"를 냈다.
- 메뉴 재구성을 callback 뒤로 미루는 변경(`3f8ad2b`)으로 자동 Cmd+W를 두 번 돌렸다(`t046-rel-b2-deferred-menu-{1,2}`, 그 변경이 든 빌드).
- 두 번 모두 앱이 종료됐고 `retireWindow` 2회, 두 창이 닫혔다. 가설을 기각했다. 근거 없는 시점 변경이라 `068fa09`에서 되돌렸다.
- spike에서 기각한 "Close Window 메뉴 중복" 가설과 합쳐, 원인은 여전히 모른다. 사람이 누른 Cmd+W의 동작도 확인하지 않았다(미확인 위험).

## 아직 검증하지 않은 것 (T048)

- (h) 로그아웃·재시동 종료: 관측 불가·미검증.
- Windows·Linux: 미검증.
- OS 프로세스 재시작 뒤 보류 task 재배정: host 재조립 수준만 검증했다(`implementation-evidence.md` 대기 task 정책 변경).
- 서명·notarization된 배포 번들의 externalBin 서버: (f) 미완료. 스모크는 번들 안에 실행 파일을 복사해 대신했다.
