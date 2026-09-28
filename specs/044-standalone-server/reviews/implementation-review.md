# 044 구현 리뷰 기록

## 5단계 완료 기준 추적 (T051)

설계 문서 `docs/client-server-architecture-research.md` §5의 5단계 완료 기준이다. **044 완료는 5단계 완료가 아니다.** "후속 미완료"는 다음 증분의 범위이며, 이 PR로 완료된 것으로 세지 않는다.

| 기준 | 044에서 완료(근거) | 후속 미완료 |
|---|---|---|
| (a) 종료 뒤 지속 | **미완료(SC-001)** — 과거 개발·배포 스모크는 있으나 현재 fail-closed 판정 규칙과 최종 코드 tree에서 모든 종료 경로를 다시 실행하지 않았다. 최종 앱 스모크 전에는 완료로 세지 않는다 | CLI(6단계)로 같은 흐름. 다시 연 데스크톱이 남은 작업대에 다시 붙는 화면. 교환 전달의 서버 소유(오늘은 데스크톱 UI가 라우팅·전송, 임대가 없으면 `undeliverableExchanges`로 보고만) |
| (b) 독립 composition root | `crates/workbench-host` 조립 + `apps/agentic-workbench-server`, MCP·launch decorator의 Tauri 결합 제거, 네이티브 삽입 전달 제거 | — |
| (c) 단일 writer·생명주기 | 잠금·안내 파일·identify HMAC·ensure·시작 복구·서빙→비우기→정지·임대·유휴·정지 세 방식, 프로세스 시험(동시 10회, kill -9 복구, 권한), 연결 실패 화면(T047) | — |
| (d) 프로세스 트리 가두기 | 서버 종료 때 자식 정리는 오늘 수준 유지 | 공통 감독자, 플랫폼별 트리 가두기, 강제 종료 뒤 잔여 자식 회수 |
| (e) 데이터 이전·복원·호환 | 서버가 오늘의 앱 데이터 디렉터리를 같은 형식으로 연다 | 백업·복원, 단계적 이전, 이전 실패 되돌리기 |
| (f) 배포·업데이트 | 개발·디버그 빌드에서 서버 실행 파일 탐색 규칙(경로 변수 → 앱 옆 → 개발 산출물) | 설치본 포함(externalBin)·서명·공증, 버전별 실행 캐시, 업데이트 preflight와 활성 run 보존·quiesce |

추가 후속 미완료·미검증(완료로 세지 않음):

- 관측 불가 종료 경로(로그아웃·재시동), Windows·Linux의 종료·창 이벤트 순서와 모든 스모크.
- (b2) 자동화한 Cmd+W 한 번이 두 창을 닫는 원인, 그리고 사람이 누른 Cmd+W의 동작.
- OS 프로세스 재시작 뒤 보류 task 재배정(host 재조립 수준만 검증).
- embedded 모드와 compat command 제거(8단계).

현재 044 차단 항목: **SC-006/T046 미완료**. 자동화한 Cmd+W 한 번이 Settings뿐 아니라 main 창과 그 run까지 닫는 실제 개발·배포 스모크 실패가 남아 있다. 원인을 수정하고 두 창 회귀 시험 및 최종 앱 스모크를 통과하기 전에는 044를 merge하지 않는다.

## 구현 리뷰 범위 (T053)

OCR·Codex 구현 리뷰에 다음을 명시적으로 넣는다.

1. **대기 task 정책 변경**(구현 중 변경, research R7): 대기 task는 coordinator run이 살아 있고 바쁘거나 그에게 미전달 알림이 있을 때만 활동 작업이다. 그 밖은 `deferredTasks`로 보고만 한다. 증거와 한계는 `implementation-evidence.md`("대기 task 정책 변경과 실제 K 경로")에 있다.
2. **scheduler 복구 수정**(041부터 있던 결함): 복구가 대기열에 넣은 Ready task도 자리가 비면 `acquire`가 시작한다.
3. **관찰 → 수정(O1)**: MCP 도구로 받은 `draining` 거절이 `structuredContent.code = "internalError"`로 나가던 것을 fault 코드 그대로 싣게 고쳤다.
4. **Cmd+W (b2)** 위험(원인 가설 둘 기각, 미해결, 사람 키 입력 미검증)과 **플랫폼 미검증**(위 목록).
5. **재시도 알림 상한(O7)**: 진행 중 시도의 예약이 정지를 막는지, 성공할 수 있는 알림이 손실되지 않는지, 재시작 뒤 자동 재전달이 아니라 `superseded` + `collectReports` 보존인 한계가 타당한지.
6. 그 밖 044 전체: 단일 writer, identify, 소유자 우회 범위, 창 토큰 tombstone, 비우기 분류와 작업 관문, 종료 판정(`window_close_intent`), #207 tombstone, 앱 스모크 증거 범위.

## 최종 게이트 (T052)

각 명령을 한 번 실행하고, 원 명령의 종료 코드를 `status.txt`에 적었다(`CARGO_INCREMENTAL=0`). 세션 scratchpad `044/final-gate-*`.

| 실행 | HEAD | 결과 |
|---|---|---|
| gate-1 | `1d056bc` | cargo-test 101: **디스크 부족**(`No space left on device`, 환경 실패). `cargo clean`(31.9GiB) 뒤 다시 돌렸다 |
| gate-2 | `51eac2b` | 리뷰 수정 전 경합을 피하려고 중단(fmt·clippy 0까지). 무효 |
| gate-3 | `f7d4094` | 전부 0: fmt, clippy `-D warnings`, `cargo test --workspace --all-targets` 911 passed/87 targets/filtered 0, check-types, pnpm test, build, 두 itest(7, 2) |
| gate-4 | `12101f0` | 전부 0: cargo 924 passed/89 targets/filtered 0, TS 73·642, itest 7·2 |
| gate-5 | `283a0b8` | OCR 3차 수정 전에 중단(fmt·clippy 0까지). 무효 |
| **gate-6** | **`03db661`** | **전부 0**: fmt, clippy `-D warnings`, cargo 936 passed/0 failed/89 targets/filtered 0(ask-code·hushline 포함 workspace), check-types, pnpm test(workbench-client 73·agentic-workbench 642·기타 패키지 전부), build, workbench-client itest 7, AW itest 2 |

| gate-7 | `8865589` | OCR 4차 수정 전에 중단(fmt·clippy 0까지). 무효 |
| **gate-8** | **`d0f061f`** | **전부 0**: fmt, clippy `-D warnings`, cargo 946 passed/0 failed/90 targets/filtered 0, check-types, pnpm test(workbench-client 73·agentic-workbench 642·기타 전부), build, workbench-client itest 7, AW itest 2 |
| **gate-9** | **`a94372e`** | **전부 0**(Codex 6차 수정 뒤): fmt, clippy `-D warnings`, cargo 950 passed/0 failed/7 ignored/90 targets/filtered 0, check-types, pnpm test(workbench-client 73·agentic-workbench 642·기타 전부), build, workbench-client itest 7, AW itest 2 |
| **gate-10** | **`07d60c7`** | **전부 0**(Codex 7차 수정 뒤): fmt, clippy `-D warnings`, cargo 960 passed/0 failed/7 ignored/91 targets/filtered 0, check-types, pnpm test(workbench-client 73·agentic-workbench 646·기타 전부), build, workbench-client itest 7, AW itest 4(실제 host 패널 교환 2 포함) |
| **gate-11** | **`17bbd11`** | **전부 0**(Codex 8차 수정 뒤): fmt, clippy `-D warnings`, cargo 968 passed/0 failed/7 ignored/91 targets/filtered 0, check-types, pnpm test(workbench-client 73·agentic-workbench 655·기타 전부), build, workbench-client itest 7, AW itest 9 |
| **gate-12** | **`051c63a`** | **전부 0**(Codex 9차 수정 뒤): fmt, clippy `-D warnings`, cargo 976 passed/0 failed/7 ignored/92 targets/filtered 0, check-types, pnpm test(workbench-client 73·agentic-workbench 660·기타 전부), build, workbench-client itest 7, AW itest 11 |
| **gate-13** | **`656fea1`** | **전부 0**(Codex 10차 수정 뒤): fmt, clippy `-D warnings`, cargo 992 passed/0 failed/7 ignored/93 targets/filtered 0, check-types, pnpm test(workbench-client 73·agentic-workbench 664·기타 전부), build, workbench-client itest 7, AW itest 12 |
| **gate-14** | **`5bfa82c`** | **전부 0**(Codex 11차 수정 뒤): fmt, clippy `-D warnings`, cargo 997 passed/0 failed/7 ignored/94 targets/filtered 0, check-types, pnpm test(workbench-client 73·agentic-workbench 669·기타 전부), build, workbench-client itest 7, AW itest 13 |
| gate-15 | `8fcb8cb` | cargo test 도중 이전 세션이 끝나 중단. fmt·clippy 0만 기록됐고 cargo 종료 코드와 `gate-finished`가 없어 무효 |
| **gate-16** | **`8fcb8cb`** | **전부 0**(Codex 12차 수정 뒤, `CARGO_INCREMENTAL=0`): fmt, clippy `-D warnings`, cargo 1002 passed/0 failed/7 ignored/94 targets/filtered 0, check-types, pnpm test(workbench-client 73·agentic-workbench 676·기타 전부), build, workbench-client itest 7, AW itest 14. 원 로그·종료 코드는 `/private/tmp/aw-044-final-gate-16/` |
| gate-17 | `1a48e0c` | fmt 0, clippy 0 뒤 cargo workspace에서 `workbench-core --test orchestration_agent`의 `first_turn_result_and_input_request_update_the_task_and_notify` 1건 실패. 통합 process의 **실제 종료 코드 101**(tool 결과)이며, `set -e` 때문에 `status.txt`에는 cargo 종료 코드가 누락됐다. 후속 단계는 실행되지 않아 무효 |

- gate-8은 Codex 6차 수정(`02831fe`·`436b563`·`bf1d65d`) 전 코드다. gate-13은 Codex 11차 수정 전 코드다. gate-15는 중단돼 무효다. **최신 유효 게이트는 gate-16(`8fcb8cb`)이다.** (이전 기록) gate-6은 Codex 5차·OCR 4차 수정 전 코드다. gate-8 뒤 커밋은 스모크 스크립트 주석(`specs/`)만 바꿨고, 코드 트리는 `d0f061f`과 같다.
- (이전 기록) gate-6 뒤 커밋은 문서(`specs/`)만 바꿨다. 코드 트리는 `03db661`과 같다.
- contract_suite가 3개 결과 뒤 멈춘 것처럼 보인 구간은, 두 fixture 시험이 in-memory·HTTP 경로를 모두 도는 약 18초 동안이다. 교착이나 nested cargo가 아니다(`Harness::spawn`은 in-process loopback).

## OCR 구현 리뷰

`ocr delegate preview --from cb0bd4c --to 51eac2b`: 184개 검토 대상 파일(코드 129개). 규칙은 `ocr delegate rule`로 받았다. 네 조각(core src / host·server·protocol / AW 데스크톱·화면 / 시험)을 나눠 검토했다. 모든 지적은 메인 세션이 코드로 다시 확인했다.

| # | 등급 | 지적 | 처리 | 근거 |
|---|---|---|---|---|
| O1 | High | MCP 도구의 입구 거절(`draining`/`unavailable`)이 `internalError`로 나감(`orchestration_tool.rs`, 제목 도구) | 수정 `82ecabd`: fault 코드를 그대로 싣는다 | `mcp_drain.rs` 실제 `/mcp` red(`internalError`) → green |
| O2 | Medium | ensure가 `server.status` 없이 비우는 서버를 돌려주고, 대기 중 소유 잠금을 다시 시도하지 않음(계약 §3) | 수정 `5366dd5` | 프로세스 시험 2건 red → green, 재시도 끄기 변이 red |
| O3 | Medium | 정지 때 `owner.lock`을 runtime 해체 전에 풂(두 writer 위험) | 수정 `8c24099`: host·runtime 해체(상한) → 안내 파일 → 잠금 | 코드 순서. 상한을 넘기면 잠금을 쥔 채 프로세스가 끝난다 |
| O4 | Medium | graceful 종료 중 SIGTERM이 무시되고 상한 없음, 신호 등록이 안내 파일 뒤 | 수정 `8c24099` | `finish_shutdown` 단위 시험(멈춘 시계), force 제거 변이 red. 실제 프로세스에서 graceful 종료를 멈추는 시험은 만들지 못함 |
| O5 | Medium | 입구 drain 판정과 C-call 예약이 다른 잠금(TOCTOU) | 수정 `a2ae902`: `WorkGate::admit` | 단위 시험 red(compile) → green, 예약 제거 변이 red. 강제 interleaving은 아님 |
| O6 | Medium | 비우기 시작 뒤 만든 대기 task가 보고에서 빠짐 | 수정 `b0bb7cf`: `deferredTasks`에 보고 | `server_stop.rs` 동작 red → green |
| O7 | Medium | 재시도 가능 실패 알림이 영원히 활동(재시도 무제한) → wait·유휴 정지가 안 끝남 | 수정 `b0bb7cf`: **정지 계약 변경**(아래) | 동작 red(시도 10회에도 pending) → green, 변이 2건 red |
| O8 | High | 임대 갱신 한 번 실패로 연결을 잊어, 이후 창 닫기가 `retireWindow`를 안 보냄 | 수정 `bee6411`: 재시도·재획득, 폐기 때 다시 붙음 | AW 단위 시험 red → green |
| O9 | High | 데스크톱 작업대 표가 서버 인스턴스와 무관(서버 재시작 뒤 모르는 작업대 id) | 수정 `bee6411`: 인스턴스·incarnation에 묶음 | AW 단위 시험 |
| O10 | Medium | 종료 경로 잠금 대기가 2초 상한 밖(부팅 중 Cmd+Q가 최대 ~40초 멈춤) | 수정 `bee6411`: 상한 안 잠금, `connect`가 ensure 동안 잠금을 쥐지 않음 | AW 단위 시험 red(5초 시간 초과) → green. "ensure 동안 잠금 안 쥠" 전용 시험은 없음 |
| O11 | Medium | 닫힌 창 표가 label 단위(같은 label로 다시 연 `settings`·`main`을 막음) | 수정 `bee6411`: (label, incarnation) | AW 단위 시험 |
| O12 | High | wait_stop 보류 시험이 멈추기 전 표본으로 판정(간헐 실패 위험) | 수정 `f82c625`: 멈춘 뒤 파생으로 판정 | 코드 |
| O13 | Medium | itest가 아무 거절이나 통과 | 수정 `f82c625`: 쉬는 run으로 보내고 drain 메시지 단정 | 코드 |
| O14 | Medium | `--idle-timeout 1` 시험이 준비 중 유휴 정지될 수 있음 | 수정 `5366dd5`: 3초, "멈추지 않음" 구간 6초 | 코드 |
| O15 | Medium | 가짜 agent 문 대기가 무한(고아 프로세스) | 수정 `f82c625`: 120초 상한·부모 소멸 시 종료 | 코드 |
| O16 | Medium | 프로세스 시험 정리가 이미 회수된 PID도 SIGKILL(PID 재사용) | 수정 `5366dd5`: 살아 있는 자식·명령줄 확인된 것만 | 코드 |
| O17 | Medium | 스모크 `run-probe.sh`가 확인 안 된 서버도 앱 체인으로 끔 | 수정 `f82c625` | 코드 |
| O18 | 관찰 | 자동 Cmd+W가 두 창을 닫음. 원인 가설: `Destroyed` 안의 메뉴 재구성 | **기각**: 지연 변경(`3f8ad2b`)으로 실제 앱에서 두 번 돌려도 두 창이 닫힘 → `068fa09`에서 되돌림 | `app-smoke.md` "Cmd+W (b2) 가설 시험" |

### 정지 계약 변경: 재시도 알림 상한 (O7, Codex 검토 대상)

- (최초 결정, OCR 2차 R1로 대체됨) 재시도를 **기다리는** 재시도 가능 실패 알림은 `attemptCount < 3`일 때만 활동이었다. **현재 조건은 `deliveryFailureCount < 3`(실제 전달 실패 수)이고, 마지막 실패가 바쁨 거절이면 횟수와 무관하게 활동이다**(아래 R1). 그 뒤에는 `stalledNotifications`로 보고만 한다. `pending`·`dispatching`(진행 중 시도)은 횟수와 무관하게 활동이다.
- 알림은 `failed`·재시도 가능으로 저장된 채 남는다. 시도 수는 "전달됨"이 아니다. 서빙 중 배경 재시도는 계속된다.
- 증명: 주입 실패 → 재시도 → 상한 전 정지 막음 → 상한 뒤 정지 + `stalledNotifications`. 상한 뒤 진행 중 시도는 정지를 막는다. 같은 runtime에서 coordinator가 회복되면 전달된다.
- **한계**:
  - 재시작 증거는 같은 시험 프로세스 안의 runtime 재조립(`restart_runtime`)이다. OS 프로세스 재시작으로는 보이지 않았다.
  - 재시작 뒤 그 알림은 새 coordinator에게 **자동으로 다시 전달되지 않는다**. 인계가 이전 세대 알림을 `superseded`로 바꾸고, 결과는 `orchestration.collectReports`로만 읽힌다. 시험이 증명한 것은 "보고 보존과 읽기"이지 "재전달"이 아니다.
  - N+1번째 시도에서 성공했을 coordinator라도 정지가 먼저 오면 그 알림을 받지 못한다.
- 대기 task 정책(`deferredTasks`)과 같이 정지를 막지 않는 대신 저장·보고한다. 이 교환이 타당한지는 Codex 검토에 맡긴다.

### 리뷰 수정 뒤 실제 앱 회귀

`app-smoke.md` "리뷰 수정 뒤 다시 실행": 배포 번들(`068fa09`) (c)(e)(g) run 지속, (a)(b1)(f) run 제거·토큰 401. 개발 출처는 다시 돌리지 않았다.

## Codex 구현 리뷰

### 실행 이력 (완료로 세지 않은 것 포함)

| 시도 | 대상 | 결과 | 리뷰로 셈 |
|---|---|---|---|
| 1 | `--base cb0bd4c`, HEAD `f7d4094` 전체 | `spawnSync git ENOBUFS`(diff 1.5MB가 companion의 git 출력 버퍼 1MB 초과) | 아니오 |
| 2 | 합성 base 두 개(8182ad8·34d62ec)를 HEAD에서 직접 | 두 번 모두 `ENOBUFS`. companion이 `merge-base HEAD base`로 범위를 정하는데 합성 base가 HEAD의 조상이 아니라 다시 `cb0bd4c`..HEAD 전체가 됨 | 아니오 |
| 3a | 코드: HEAD 트리와 같은 검토 커밋 `68074965`(부모 8182ad8), 144개 파일, 969,681B | turn이 `custom_tool_call` 출력 뒤 도구 실행 기록 없이 멈춤. app-server TCP 없음·CPU 0%, 66분 무진행. 그 job만 `/codex:cancel`로 중단(`turnInterrupted: true`). 공유 broker·app-server·다른 세션은 건드리지 않음 | 아니오 |
| 3b | 문서·증거: 검토 커밋 `52a4ef2d`(부모 34d62ec), 121개 파일, 535,263B | **needs-attention** 1건(아래 C1) | 예(`f7d4094` 문서) |

- 분할 범위 검증: 코드 144 + 문서 121 = 265 = `cb0bd4c..f7d4094` 전체. 겹침 0, 합집합 일치.
- 두 검토 커밋의 트리는 모두 `235454f` = HEAD `f7d4094`의 트리다. 검토 중 작업 트리 파일은 바뀌지 않았다(detached checkout, `git status` 0). 검토 뒤 브랜치로 돌아왔다.

### C1 (문서 리뷰, medium) — 쉬는 세션의 생존을 진행 중 작업 지속 증거로 씀

- 지적: T045는 시작 turn 완료(`busyRuns=0`) 뒤 앱을 끄고 새 prompt로 출력을 봤다. 진행 중 turn이 앱 종료를 넘어 계속되는지는 증명하지 않는다.
- 처리:
  - T045·SC-001을 먼저 부분 검증으로 표시했다.
  - debug probe `quit-busy`와 가짜 agent `--end-turn-gate … --after-gate-chunk`를 더했다. owner-check에는 `--observe-turn --release-file`을 더했다.
  - 흐름: 진행 중 turn을 붙잡음 → 종료 뒤 `busyRuns=1` → 앱 PID 소멸 → 구독 뒤 고유 표지로 문을 엶 → 새 prompt 없이 그 표지의 **새 출력**과 **완료**를 live로 받음.
  - 개발·배포 × (c)(d)(e)(g) 8건 모두 ok(`app-smoke.md` "진행 중 turn 지속"). 그 뒤 T045를 다시 완료로 표시했다.
  - 사용자 검토로 "완료 이벤트만"이 아니라 "종료 뒤 새 출력"까지 단정하게 보강했다.
- 단위 시험 `quit_busy_probe_reports_before_the_start_turn_completes`: red(`quit-busy-red-1.log` 종료 101, 기본 템플릿으로 떨어짐) → green(`quit-busy-green-1.log` 3 passed).

### 3차: 같은 HEAD `6d806c1` 분할 리뷰(코드 144 + 문서 157 = 301, 겹침 0)

- 검토 커밋의 트리는 모두 `7eb0506` = HEAD 트리다. 부모는 분할 base이고, merge-base가 의도한 base임을 확인했다.
- 코드 리뷰 1/2: needs-attention 3건. 문서 리뷰 2/2: needs-attention 2건.

| # | 등급 | 지적 | 처리 | 근거 |
|---|---|---|---|---|
| X1 | high | 임대 획득이 진행 중 유휴 정지를 되돌리지 못함(`try_stop_at`이 서빙 복귀를 보지 않음) | `3c71214`: 비우기 시작·유휴 취소에서 세대 증가, 서빙이면 판정 거절. OCR 2차로 `b967ff7`: 임대 삽입 전후 서빙 복귀, `stopping`이면 임대 없이 `unavailable` | 관문 수준 결정적 끼어들기 red → green, 변이 2건 red. 삽입~복귀 창은 강제할 수 없음(한계) |
| X2 | medium | 폐기 전에 인증한 요청이 폐기 뒤 작업대를 만들 수 있음 | `880379e`: 폐기 주체를 작업대 등록과 같은 잠금 아래 표시, 입구·등록이 거절 | 실제 HTTP delayed-body(`Expect: 100-continue`로 관찰 가능한 동기화) 401·작업대 없음. 등록 경합(probe로 등록 직전 멈춤) 거절. 변이 5건 red(등록만 뺀 변이는 HTTP 시험이 잡지 못함 → core (e)가 잡음, 한계 기록) |
| X3 | medium | 정상 Quit에서 열린 창 토큰을 폐기하지 않음 | `16a51d2`: 종료 경로가 살아 있는 창을 모두 `closeBench:false`로 폐기. OCR 2차 `c6a5fc8`: 연결을 잃은 뒤에도 종료 폐기 | 단위 시험 동작 red(200 ≠ 401) → green. 실제 앱 Dock·AppleScript × 개발·배포(최종 빌드 `b8da72f`): 옛 창 토큰 401 + 진행 중 turn 지속. **Cmd+Q는 세션 잠금으로 미검증** |
| X4 | medium | 계약의 identify 계산식(개행 없음·base64url)이 구현(`nonce\ninstanceId`, hex)과 다름 | `72badb6`: 계약·research 정정, 고정 벡터와 시험 | `identify_proof_matches_the_contract_vector` |
| X5 | medium | 스모크 정리가 신호 직전 PID 신원을 다시 확인하지 않음 | `72badb6`: 시작 시각 + 명령줄 신원 기록·재확인 | 자기 시험: 신원이 바뀐 PID·사라진 PID에 신호 없음 |

### OCR 2차 (`51eac2b..12101f0`, 코드·스크립트 38개 파일, 두 조각)

| # | 등급 | 지적 | 처리 |
|---|---|---|---|
| R1 | Medium | 바쁜 coordinator의 거절이 시도 수 상한을 소진해 전달 가능한 알림이 stalled가 됨 | `b967ff7`: 상한은 **실제 전달 실패 수**(`deliveryFailureCount`, serde 기본 0)만 센다. 마지막 실패가 바쁨 거절이면 항상 활동. `attemptCount`는 전체 시도 수로 둔다. 시험 (a) 바쁨 거절이 상한을 넘어도 막음 → 뒤에 전달: red → green. (b) 바쁨 거절 3회 뒤 실제 실패 1회는 활동, 실제 실패가 상한에 닿으면 stalled: 누적 시도 수로 되돌리는 변이 red |
| R2 | Medium | 임대 삽입과 서빙 복귀 사이 경합 | `b967ff7`(X1 참조). 시험 `a_stopping_server_hands_out_no_lease`는 입구 거절 경로라 handler 변경과 무관하게 통과한다(한계 기록) |
| R3 | Low | 문서 주석 위치 | `b8da72f` |
| R4 | High | 데스크톱이 죽은·바뀐 서버 인스턴스를 잊지 않음 | `c6a5fc8`: 갱신 재시도 때 안내 파일을 다시 읽고 확인, 사라졌거나 다르면 잊음. 단위 시험 red → green, 변이 red |
| R5 | High | 스모크 Dock 경로가 메뉴 미확인·이름 일치만으로 누름(다른 앱을 끌 수 있음) | `c53dde6`: 표시 이름 → pid가 정확히 APID이고 메뉴가 열렸을 때만 누름 |
| R6 | Medium | 연결을 잃은 뒤 종료하면 폐기가 재부착을 건너뜀 | `c6a5fc8` |
| R7 | Medium | HTTP 폐기 시험이 목록 호출 실패에도 통과 | `b8da72f`: 200·배열 단정 |
| R8 | Medium | 가짜 agent 문 대기 상한이 긴 스모크를 깸 | `c53dde6`: `--gate-limit`(스모크 600초) |
| R9 | Medium | 닫힌 창 표가 한없이 자람 | `c6a5fc8`: 새 incarnation이 옛 닫힘 기록을 정리, 마지막 항목이 사라지면 잠금 제거. 세션 label은 재사용되지 않아 창마다 작은 기록 하나는 남는다(한계) |

- 참고: 실제 알림 전달기(`EngineAgentWorker`)는 바쁜 coordinator를 거절하지 않고 turn 뒤에 줄 서 기다린다(`dispatching`, 예약 유지). 바쁨 거절은 `accepted: false`를 돌려주는 포트의 계약이고, 시험은 probe(`DeclineAsBusy`)로 재현했다.
- 안전 사고 기록: 앞서 (c) Cmd+Q 시도 세 번이 대상 앱이 앞인지 확인하지 않고 전역 키 입력을 보냈다. 다른 앱 영향은 없음을 확인했다(`app-smoke.md`).

### OCR 3차 (`12101f0..283a0b8`, 17개 파일)

| # | 등급 | 지적 | 처리 |
|---|---|---|---|
| Q1 | Medium | 서빙 중 tick이 조용함을 본 뒤 임대가 완전히 들어가고, 그다음 유휴 비우기·정지가 임대를 보지 않고 멈춤 | 유휴 비우기의 정지 판정이 관문 잠금 아래에서 임대 수를 더한다(`server_control.rs` `try_stop`). 시험 `an_idle_stop_decision_counts_a_lease_inserted_before_the_resume`: 임대 표에 직접 넣어 결정적으로 재현, 동작 red(`lease-idle-red-1`) → green(`lease-idle-green-1`, `server_stop` 25 passed). wait 비우기는 임대와 무관(계약대로) |
| Q2 | Medium | 일시적 실패 한 번(identify 불가·NotReady)으로 살아 있는 서버를 잊고 갱신을 멈춤 | 확실한 증거(안내 파일 없음·다른 인스턴스·확인 실패 + 소유 잠금이 비어 있음)일 때만 잊는다. 시험: 살아 있는 서버(잠금 보유) 일시 불응 → 재시도·연결 유지: 동작 red(`liveness-red-2`) → green. 비정상 종료(잠금 빔) → 잊음(`a_renewal_after_a_crash_forgets_the_instance`). `liveness-red-1`은 필터 두 개를 준 명령 오류라 증거가 아니다 |
| Q3 | Medium | 기다리는 작업이 쥔 label 잠금을 거둬 직렬화가 깨짐 | 표와 이 호출자 둘뿐일 때만 거둔다(`Arc::strong_count == 2`, 표 잠금 아래). 시험 `a_label_lock_held_by_a_waiting_task_is_not_dropped`: 동작 red(`lock-red-1`) → green(`desktop_benches` 8 passed) |
| Q4 | 관찰 | Dock 경로는 `apps-named`가 빌드돼 있어야 동작(없으면 누르지 않음) | `app-smoke.md`에 빌드 절차 기록 |

### 5차: 같은 HEAD `299cb3a`(트리 `5821d25`) 세 파티션 리뷰

- 파티션: crates 101개 / apps·packages·루트 57개 / specs·docs 226개. rename 감지 없이 세면 합 385 = 전체 385, 겹침 0. 코드 한 파티션이 1,038,293B로 companion 버퍼 1MB에 가까워 둘로 나눴다.
- 세 검토 커밋의 트리는 모두 HEAD 트리와 같다. merge-base는 의도한 base다. 검토 뒤 브랜치로 돌아왔다(`status 0`).
- **세 파티션 모두 needs-attention.** companion의 `exit=0`은 실행 종료 코드이지 통과가 아니다.

| # | 등급 | 지적 | 처리 | 근거 |
|---|---|---|---|---|
| Y1 | high(crates) | 교환 소비·실패 기록이 `request_id`만 키로 써서 작업대끼리 간섭(같은 id면 다른 작업대 전달 거절, 정지 판정에서 누락) | `21cdbbb`: `(bench_id, request_id)` 키, 작업대 닫기 때 그 작업대 기록 제거, `failedExchangeDeliveries`는 `<benchId>/<requestId>` | `exchange_bench_scope.rs`: 동작 red → green(두 작업대 같은 id 각 1회 전달, A 소비가 B의 `pendingExchanges`·wait 정지에 영향 없음, 닫기 정리). 변이 2건 red. 한계: `undeliverableExchanges`는 아직 요청 id만 보고(보고용, 정지 판정 무관) |
| Y2 | high(apps) | 창 토큰 발급 전송 오류가 생존 확인 없이 연결을 잊음(비우는 서버라 재연결 불가 → 임대 만료 → 교환 전달 전 정지 가능) | `85cb7b6`: 오류 때 잊는 경로를 `renew_once`와 같은 확실한 증거 규칙으로 | 단위 시험 동작 red → green, 변이 red. **하나의 회귀 시나리오** `1c757e8`: 실제 host + 감시 루프. 데스크톱 임대 → 교환 대기 → `drainingWait`(`pendingExchanges=1`) → 토큰 응답 1회 유실 주입 → 같은 인스턴스 재발급·`renew_once` Renewed → turn 끝 → 재발급 토큰으로 continuation 전달 200 → 정지 완료. 5회 반복 green, 무조건 잊기 변이는 4단계에서 red. 주입 범위: 서버가 처리한 뒤 클라이언트가 응답을 버림(네트워크 유실 아님). 패널 라우팅·전송은 시험이 대신함 |
| Y3 | high(docs) | close-run.sh의 전역 Cmd+,·Cmd+W가 앞 앱 확인 없이 나감 | `77d5213`: `send_key_to`가 앞 프로세스가 APID임을 두 번 확인한 뒤에만 보냄, 아니면 무효·정리 | 스크립트 |
| Y4 | medium(docs) | stalled 상한 조건이 문서마다 `attemptCount`/시도 횟수로 남음 | `77d5213`: data-model·spec·ADR 0009를 `deliveryFailureCount < 3` + 바쁨 거절 예외로 통일(구현 리뷰의 옛 결정은 "대체됨"으로 표시) | 문서 |

### OCR 4차 (`299cb3a..8865589`)

| # | 등급 | 지적 | 처리 | 근거 |
|---|---|---|---|---|
| P1 | Medium | 진행 중 교환 전달(작업대 입장권을 쥐지 않음)이 작업대 닫기 뒤 완료되면서 닫힌 작업대의 기록을 되살림(`failedExchangeDeliveries`에 남음) | 관문에 닫힌 작업대 tombstone. `forget_bench_exchanges`가 세우고, `begin_exchange_delivery`는 `BenchClosed`(→ notFound)로 거절, `record_failed_delivery`는 적지 않음(모두 관문 잠금 아래) | 단위 시험 `a_delivery_arriving_after_the_bench_closed_leaves_no_record`: compile red(`closed-red-1`) → 동작 red(`closed-red-2`, 변형만 추가) → green(`closed-green-1`, `work_gate` 10 passed). **결정적 순서 시험**(`exchange_bench_scope.rs`): 가짜 엔진 `queue_prompt` 문(`queue_gate`·`queue_entered`)으로 전달을 엔진 안에서 붙잡음 → 작업대 A 닫기 완료 → 문 열기. 성공 완료·실패 완료(`fail_next_queue_prompt`) 두 경우 모두 A의 소비·실패 기록이 되살아나지 않고, B의 같은 id 교환은 그대로 1회 전달된다(`late-green-1`, 6 passed). 실패 기록 건너뛰기를 뺀 변이: 실패 완료 시험 red(`late-mut-record`). 순서 시험은 수정 뒤 작성했으므로 수정 전 실패는 변이로 보였다 |
| P2 | Medium | 스모크 `invalid_stop`이 번들 id로 quit(같은 번들의 다른 인스턴스를 끌 수 있음) | `invalid_stop`과 close-run 최종 정리가 기록한 신원(pid·시작 시각·명령줄) 확인을 거친 `kill_exact`만 쓴다. quit-run의 (e) AppleScript quit은 시험 대상 경로라 번들 id로 보내되, 그 번들의 실행 중 인스턴스가 정확히 APID 하나일 때만 보낸다(`apps-named --bundle`) | 스크립트. `kill_exact` 신원 확인은 앞선 자기 시험으로 검증됨 |

### 6차: 같은 HEAD `2d08d34` 세 파티션 리뷰

- 파티션과 커버리지 확인 방식은 5차와 같다(`--no-renames`로 합 = 전체, 겹침 0). 세 검토 커밋의 트리는 HEAD 트리와 같고, 검토가 끝난 뒤 브랜치로 돌아왔다.
- **세 파티션 모두 needs-attention**이었다. 지적은 4건이다. 수정은 파일별 병렬 fork로 했고, 공유 `verify.lock` 아래에서 cargo 실행·변이(옛 동작 복원) 구간을 직렬화했다. 변이는 잠금을 풀기 전에 모두 복원했고, 커밋 트리의 `MUTATION:` 표지는 0개다.

| # | 등급 | 지적 | 처리 | 근거 |
|---|---|---|---|---|
| Z1 | high(crates) | 서빙 중이나 wait 비우기 중 임대 획득이 세대를 올리지 않아, 임대 없이 파생한 낡은 판정(미소비 교환 0)으로 default/wait 정지가 성립 | `bf1d65d`: `WorkGate::note_lease_acquired`가 stopping이 아닌 모든 상태에서 G 잠금 아래 세대를 올린다(유휴 비우기는 서빙 복귀, stopping이면 임대 거절은 그대로). 임대 삽입 전후에 호출한다. default가 세대 불일치로 거절되면 activeWork를 다시 파생해 보고한다 | `server_stop.rs`: test-hooks `set_stop_probe`로 파생 뒤·판정 전에 멈추고 실제 `LeaseAcquire`. compile red → **동작 red**(wait는 `stopping`, default는 서버 정지) → green(27 passed). 세대 올림 제거 변이: 2개 red |
| Z2 | medium(crates) | `reserve_child_run` 저장소 예약 중 assign future가 abort되면 커밋된 예약과 scheduler 자리가 새어 재배정 불가 | `bf1d65d`: 예약을 소유 task(`tokio::spawn`)로 옮기고 `LaunchCleanup`이 handle을 쥔다. 결과 전 drop이면 커밋을 기다린 뒤, 노드의 현재 run이 예정 run id일 때만 조건부 해제하고 자리를 반납한다(`Existing`은 건드리지 않음) | `child_assign_atomic.rs`: `ReserveProbe(BeforeCommit)`에서 멈춤 → abort → 커밋 완료 → 해제·`active_count==0` → 재배정이 실제 run 기동. compile red → 동작 red → green(8 passed). 변이 b(rollback 무효화)·c(자리 반납 생략) red, 변이 a는 컴파일 오류라 증거 제외. 한계: 되돌리기 완료 전의 짧은 창에 다른 배정이 곧 해제될 예정 id를 받을 수 있음(기존 Reserved/Prepared abort 경로와 같은 동작, 이번 범위 밖) |
| Z3 | high(apps) | descriptor 읽기 오류(EACCES 등)를 부재로 보고 살아 있는 서버 연결을 잊음 | `436b563`: `descriptor_is_absent`(실제 NotFound만 부재). 읽기 오류는 불확실로 보고 연결 유지 | 실제 OS EACCES 주입. 잠금 없이 돈 red-1은 비증거, 잠금 아래 red-2 동작 red → green, 변이 red, AW lib 124 passed. 한계: 주입은 EACCES만, `Ok(None)` 전용 시험 없음 |
| Z4 | medium(docs) | quit-run (g) SIGTERM이 신원 확인 없는 `kill` | `02831fe`: (g)가 `kill_exact`(rc 반환)만 거치고, 불일치·소멸이면 `sigterm-not-sent`(시도 무효). 커밋본 lib.sh 작업 디렉터리는 `AW_SMOKE_DIR` | 모의 자기 시험 `selftest-signal.sh` 9/9. 실제 스모크·사용자 프로세스에는 신호를 보내지 않았다 |

- 6차 수정으로 코드가 바뀌었으므로 gate-8은 최신 게이트가 아니다. T052는 gate-9 전까지 다시 미완료로 둔다.
- gate-9(`a94372e`)가 6차 수정을 모두 포함해 전부 통과했다.
- **OCR 6차 재리뷰**(`2d08d34..a94372e`, 코드·스크립트 11개 파일): High·Medium 없음. 확인한 점: 예약 task handle은 정상 경로에서 같은 poll 안에 비워져 Drop이 넘겨받지 않음(`Existing`·`Err`의 scheduler 반납은 기존대로 호출자 몫), abort 경로만 커밋 결과를 기다려 조건부 해제·반납, `note_lease_acquired`의 유휴 복귀 필드가 `resume_serving`과 같음, default 정지의 세대 불일치 재파생 보고, `descriptor_is_absent`는 `NotFound`만 부재로 봄, `kill_exact` rc 전파와 (g) 무효 표시. Low(보고만): 커밋본 lib.sh는 `AW_SMOKE_DIR`가 없으면 스크립트 디렉터리에 결과를 쓴다(실행 문서에서 환경 변수를 지정).

### 7차: 같은 HEAD `f35c58d`(트리 `8230775`) 세 파티션 리뷰

- 파티션: crates 103 / apps·packages·루트 57 / specs·docs 227. 합 387 = 전체, 겹침 0(`--no-renames`). 세 검토 커밋의 트리는 HEAD 트리와 같고, merge-base는 의도한 base다. 검토가 끝난 뒤 브랜치로 돌아왔다(status 0).
- **세 파티션 모두 needs-attention**이었다(companion `exit=0`은 통과가 아니다). Medium 4건.
- 수정은 core·apps 병렬 fork와 메인(docs)이 나눠 맡았다. cargo·pnpm 실행, 변이, 커밋은 공유 `verify.lock` 아래에서 직렬화했다.
  - protocol/core 공유 인터페이스 편집은 한 번에 완성해 컴파일을 확인한 뒤 바로 커밋했다.
  - 조율 사고 1: core의 첫 compile red는 apps의 protocol 편집 중에 돌아 비증거로 표시했다.
  - 조율 사고 2: core 변이 m1은 시험 안에 대기 상한이 없어 멈췄다. 메인이 신원 확인 뒤 그 시험 PID만 끝냈고 "hang, not counted"로 기록했다. 이후 시험에 `timeout`을 넣고 변이를 다시 돌렸다.
  - 조율 사고 3: 두 fork가 서로의 파일 커밋을 기다려 교착에 빠졌다. 대기 조건을 전역 dirty가 아닌 특정 파일로 좁혀 풀었다(`5d7c08c` → `2273b54` → `c551f8a`).

| # | 등급 | 지적 | 처리 | 근거 |
|---|---|---|---|---|
| W1 | medium(crates) | `bind_child_run` await 중 abort가 실행 없는 Running task·노드 불일치를 남김 | `2273b54`·`00ca830`: 기동 guard 하나가 모든 수명 단계를 소유한다. 예약·바인딩 커밋과 실패 되돌리기를 소유 task로 돌리고 guard가 handle을 쥔다. drop되면 커밋을 기다린 뒤 run 취소 → `revert_child_launch`(task·노드를 한 트랜잭션에서, 노드가 이 기동의 run일 때만) → scheduler 반납. 되돌리는 중 새 배정은 재시도 가능한 `launchRollingBack`을 받는다. R14에 수명 상태 × 소유 × drop 책임 표 | `child_assign_atomic.rs`: 결정적 저장소·엔진 취소 지점으로 (a) 바인딩 커밋 직전, (b) 직후·결과 수신 전 abort, (e) 되돌리는 중 새 배정. compile red → 동작 red((a)(b) task `running`) → green 13. 변이 m1-b·m3-b·m4-b 유한 시간 red(17–24초). 한계: m1-b는 (e)만 결정적으로 잡는다 |
| W2 | medium(crates) | `fail()`이 rollback 전에 Done으로 바뀌어 abort 때 정리·scheduler 반납이 빠짐 | 같은 커밋: 되돌리기가 끝난 뒤에만 Done이 된다. await 중 drop되면 guard가 끝까지 기다린 뒤 반납한다 | (c) 엔진 취소 대기 중 abort, (d) 예약 해제 대기 중 abort: 동작 red((c) 노드 run 미해제, (d) 자리 미반납) → green. 변이 m2-b·m5 red. 각 시험은 run 소멸·노드 해제·task 일치·관문 예약 0·`active_count==0`·`orchestration_tasks==0`·재배정 실제 기동을 확인 |
| W3 | medium(apps) | 패널에서 교환 대기 항목을 삭제하거나 steer하면 서버에 확인됐지만 미소비인 교환이 남아 wait-stop이 무기한 대기 | `6843997`: 새 연산 `exchange.discardDelivery`(C, `exchange:write`, epoch 멱등, 관문 잠금 아래 (작업대, 요청 id) 소비 표시, 닫힌 작업대는 기록 없음). `ec8e945`: 삭제는 항목을 먼저 빼고(자동 전송 방지) discard, 실패하면 되돌림. steer는 교환 항목에서 비활성(버튼·함수 모두). run 취소·거절된 steer 재시작 때 떨어지는 교환 항목도 discard | core: no-op handler 동작 red(`pendingExchanges` 1 유지) → green, 변이 red. 패널 vitest red(steer 가능) → green 177, 변이 a(삭제가 로컬만)·b(steer 허용) red. **실제 host + 실제 AgentRunPanel itest** `22338dc`(`agent-run-panel-exchange.itest.tsx`): 실제 패널 액션 → turn 완료 → `pendingExchanges` 0 → wait-stop 정지. 깨끗한 트리에서 green 2, 변이 a·b red. itest-green-1–4는 비통과(원인: 시험 host 허용 출처 없음 → WS 403, scripted engine에 prompt lifecycle 없음 → `5d7c08c`·`c551f8a` opt-in) |
| W4 | medium(docs) | (g) SIGTERM 미전송(`attempt invalid`)이 뒤의 PID 소멸 판정으로 `path-exercised=yes`가 됨 | `c0afc54`: `quit-action-sent`(보낸 명령 종료 코드 0)와 PID 소멸을 모두 요구한다(`quit_verdict`). 아니면 정리 뒤에도 무효이고 스크립트는 5로 끝난다. `0ef1621`: 과거 (g) 실행은 송신 결과 기록 전 스크립트였다는 한계를 app-smoke.md에 기록(실제 앱 재실행 안 함, 후속) | `selftest-signal.sh` 20개: 신호 전 소멸 → `path-exercised=no`·하위 셸 종료 코드 5, 송신·소멸 → yes·0, 송신·잔존 → no·5, quit-run 구조 확인. 옛 quit-run 사본으로 red(구조 4개), 수정본 green, "sent 무시" 변이 red(3개). 모두 scratch 사본으로 돌려 공유 트리는 바꾸지 않았다 |

- W3 한계:
  - 시험 host는 실제 ACP runner가 아니라 scripted engine이다. lifecycle은 opt-in으로 흉내만 낸다.
  - itest의 교환 라우팅은 043 원장 → 패널 `externalPromptRequest` 경로다. area 컴포넌트는 그리지 않았다.
  - steer 함수 안의 거부는 방어 코드이고 시험이 없다.
  - discard 뒤 교환 상태는 `delivered`로 남는다(도메인 종결 상태).
  - 전송이 계속 실패하는 대기 prompt의 무한 재시도는 기존 동작이다.
- W1/W2 한계:
  - 되돌리는 중 표시를 보기 전에 스냅샷을 읽은 배정은 직후 abort가 취소할 run id를 받을 수 있다(반환 직후 취소와 같은 일반 경우).
  - 소유 task의 join 오류(panic)는 조건부로 되돌린다.
  - drop 경로는 `scheduler.release`가 돌려준 다음 task를 기동하지 않는다(기존 동작).
- 7차 수정으로 코드가 바뀌었으므로 T052는 gate-10 전까지 다시 미완료로 둔다.
- gate-10(`07d60c7`)이 7차 수정을 모두 포함해 전부 통과했다.
- **OCR 7차 재리뷰**(`f35c58d..07d60c7`, 코드·스크립트 31개 파일): High·Medium 없음.
  - 기동 guard: 소유 task 결과 수신과 상태 전이가 같은 poll 안에서 일어난다(await 없음). `fail()`은 되돌리기가 끝난 뒤에만 Done이 된다. Drop은 상태별 소유 task(예약·바인딩·되돌리기)를 넘겨받아 기다린 뒤 반납한다. `revert_child_launch`는 노드가 이 기동의 run일 때만 task를 되돌린다. 되돌리는 중에는 `launchRollingBack`을 돌려준다.
  - `exchange.discardDelivery`: 작업대 범위의 교환인지 확인하고, 닫힌 작업대 tombstone을 따른다. 패널은 먼저 대기열에서 뺀 뒤 서버에 알린다(자동 전송 경합 없음). steer는 버튼과 함수 모두에서 막았다.
  - Low(보고만): discard가 서버에 반영된 뒤 응답만 유실되면 패널은 항목을 되돌린다. 그 항목을 다시 보내면 서버는 이미 소비된 교환이라 conflict로 거절한다(wait-stop에는 영향 없음, 화면에 오류 표시).

### 8차: 같은 HEAD `82eb3ff`(트리 `bb43eec`) 세 파티션 리뷰

- 파티션: crates 107 / apps·packages·루트 63 / specs·docs 228. 합 398 = 전체, 겹침 0(`--no-renames`). 세 검토 커밋의 트리는 모두 `bb43eec`다. 검토 뒤 브랜치로 돌아왔다(status 0).
- **세 파티션 모두 needs-attention**이었다. High 1, Medium 4.
- 담당 경계: core fork는 `crates/workbench-core`, apps fork는 `apps/`, 메인은 스모크 스크립트·문서. 프로토콜 변경은 없었다(기존 선택 필드 null = 모름을 씀).
- 조율:
  - core의 첫 `compile-red`는 zsh가 파일 목록을 나누지 않아 소스가 한 바이트도 바뀌지 않은 채 돌았다(종료 코드 0). **증거 아님**으로 정정했다.
  - apps의 `itest-dev-1…3`은 core 미커밋 파일로 빌드한 host라 비증거다.
  - 최종 itest·변이는 core 커밋 host(`2b233f4`/`65989ba`)에서, crates가 깨끗하고 표지 0인 것을 확인한 잠금 아래에서 돌렸다.

| # | 등급 | 지적 | 처리 | 근거 |
|---|---|---|---|---|
| V1 | high(crates) | 활동 파생 때 orchestration 저장소·ledger 읽기 실패를 0으로 취급해 Running task가 있어도 default/wait 정지가 성립(`host.shutdown`이 살아 있는 run 취소) | `2b233f4`: 파생 실패를 unknown 플래그로 전달한다. `active_total`에 +1(정지 차단), activeWork의 해당 필드와 `unresolvedOperations`는 null(모름). default·wait·유휴 모두 멈추지 않고, 복구되면 정상 판정한다 | `server_stop.rs` `a_store_read_failure_blocks_default_wait_and_idle_stops_until_it_recovers`: 보고 전 Running 자식 + 읽기 오류 주입 → default conflict, wait는 tick 반복에도 안 멈춤, 해제 뒤 실제 보고로 정지. compile red(`compile-red-2`) 뒤 수정만 되돌린 단일 변이 red: orchestration(`red-h1-orch`)·ledger(`red-h1-ledger`) |
| V2 | medium(crates) | 같은 task가 active면 `acquire`가 Acquired를 돌려줘, 되돌리는 중인 A의 정리가 재배정 B의 scheduler 자리를 지움(동시 실행 한도 초과 가능) | 같은 커밋: task 자리 안에 시도별 보유(hold) 집합을 둔다. 기동 성공이면 실행 중으로 넘기고, 그 밖의 끝은 자기 보유만 놓는다. 보유는 기동 guard가 끝까지 책임진다(호출자 반납 제거). `65989ba`: 시험이 대기가 아니라 소유권 단정에서 실패하게 조정 | `child_assign_atomic.rs` `a_reassign_holding_the_slot_across_a_rollback_keeps_its_slot`: A를 바인딩 커밋 뒤 붙잡고 abort → B는 보유 획득 뒤·snapshot 전에 멈춤 → A 되돌리기 완료 → B 재개. B 실제 기동, 보유 1, 한도 유지. 변이 red `red-m2`·`red-m2-b` |
| V3 | medium(crates) | `revert_child_launch` 저장 실패를 버리고 완료 처리(노드가 취소된 run id를 계속 가리키고, 되돌리는 중 표시가 지워져 이후 `alreadyAssigned`로 고착, Running 잔존) | 같은 커밋·`48a5086`: rollback이 결과를 돌려준다. 실패면 `pending_reverts`로 넘겨 되돌리는 중 표시와 보유를 유지한다(새 배정은 `launchRollingBack`). 재시도는 서버 감시 tick과 같은 task의 새 배정 시도가 맡는다. 미완료 정리는 활동(`orchestrationTasks`)으로 세고 drain 중에도 돈다(C 성격). **재시작 복구**: 시작 시 노드가 가리키는 run이 엔진에 없으면 되돌린다 | `a_rollback_that_cannot_be_stored_is_kept_and_retried_until_it_succeeds`: 일시적 저장 실패 주입 → 유지·관측·재시도 → 회복 뒤 정리 완료·재배정 실제 기동. 변이 red `red-m3`·`red-m3-count`. force stop·재시작 경로: 저장 실패가 남은 채 런타임 재조립 → 회복 → 같은 작업대 재배정이 `alreadyAssigned` 없이 실제 기동. 변이 red `red-restart` |
| V4 | medium(apps) | 거절된 steer 재시작의 `cancelAgentRun`이 적용 전에 실패(notApplied)하면 catch가 교환 항목을 뺀 대기열로 덮어써, delivered 미소비 교환이 고착(wait-stop 무기한) | `bb74f12`: `unsettledCall()`(notApplied·unknown). 성공한 취소만 교환 제거·discard·새 run을 한다. 취소 실패·notApplied면 원래 대기열·거절된 steer·응답 대기 상태를 복원한다. unknown이면 재시작을 보류하고, 복구된 run 이벤트로 판단한다(취소 끝 이벤트면 1회 재개, 새 turn이면 run이 살아 있으므로 보류 폐기·turn 끝에 교환 전달). 일반 취소 버튼도 같은 규칙 | 패널 vitest: red(4) → 옛 패널 red(7) → green 39, 변이 a–e red. **실제 host + 실제 AgentRunPanel itest**(화면 호출 클라이언트에 1회 결정적 주입): (A) 미적용(notApplied, unknown이지만 미적용) → 교환 보존·새 run 없음·turn 끝 1회 전달 → wait-stop 정지. (B) 적용 + 응답 유실, Serving → 취소 끝 복구 뒤 대체 run 정확히 1개·대기열 정리·discard·`pendingExchanges=0`·wait-stop 완료. 옛 패널 red(4), 변이 a·b·c red. (B) Draining(대체 run이 새 작업으로 거절되는 정상 동작, 우회 안 함)은 옛 패널에서도 통과해 수정의 증거가 아니다 |
| V5 | medium(docs) | `quit-run.sh`가 owner-check 실패를 기록만 하고 종료 송신·PID 소멸만으로 0을 반환 | `218b060`: `OC=$?` 보존, `smoke_final`: 경로 무효 5, 유효 경로 + owner-check 실패 6, 둘 다 통과 0 | 자기 시험이 실제 `quit-run.sh`의 판정 이후 구간(`gone=no`부터 끝)을 그대로 떼어 모의 입력으로 돌린다. 옛 스크립트 red → green 30 → owner-check 무시 변이 red(owner-check 검사 2개로만 분리). 변이는 scratch 사본에만 적용했다. 과거 실제 앱 실행 31개는 원자료 `owner-check-exit=`로만 재판정했다(모두 0, app-smoke.md) |

- 한계:
  - V1·V3: 재시도는 서버 tick·새 배정 시도에 기대며, 저장소가 계속 실패하면 정리 미완료로 정지가 계속 막힌다(의도, status에 보임). OS 프로세스 재시작 뒤 재할당은 기존 후속 항목이다.
  - V4: 실패는 화면 호출 클라이언트에서 주입했고 실제 네트워크 단절이 아니다. 시험 host는 scripted engine이다. unknown이지만 미적용인 취소 뒤 run이 새 turn도 끝 이벤트도 없이 쉬면 보류가 그대로 남는다. 실제 앱 스모크는 7·8차 수정 뒤 돌리지 않았다.
  - OCR 7차 Low(discard 응답 유실 뒤 복원 항목의 conflict 거절)는 그대로 유지한다.
  - V1–V3 추가: 실행까지 간(`Active`) 노드가 재시작 뒤 run이 없으면 기존대로 task를 `Blocked(runtimeLost, 재시도 가능)`로 둔다(복구는 재시도 명령, `alreadyAssigned` 고착 없음). 이 경우의 전용 시험은 없다. 되돌리기 재시도에는 backoff가 없다. 오류 주입은 런타임·ledger test-hook으로 했고 실제 SQLite 장애가 아니다.
- 8차 수정으로 코드가 바뀌었으므로 T052는 gate-11 전까지 다시 미완료로 둔다.
- gate-11(`17bbd11`)이 8차 수정을 모두 포함해 전부 통과했다.
- **OCR 8차 재리뷰**(`82eb3ff..17bbd11`): High·Medium 없음. 확인: scheduler 시도별 보유(자기 보유만 놓음, 실행 중 자리 보존, 옛 보유가 새 시도에 영향 없음 — 단위 시험 3), unknown 활동의 합계 +1과 `null` 표시, tick 재시도, 패널의 unsettled 취소 분기(notApplied·unknown이면 상태 보존, unknown은 복구된 run 이벤트로 판단, 끝 이벤트가 먼저 온 경우 포함). Low(보고만): `unsettledCall`은 transport의 공유 메시지 상수를 문자열로 비교해 분류한다(같은 상수를 쓰므로 일치하지만 결합이 느슨하다); 대기열 승격으로 생긴 보유 없는 자리가 배정 없이 남는 것은 기존 동작.

### 9차: 같은 HEAD `4a0b6a6`(트리 `22c4fbc`) 네 파티션 리뷰

- crates 파티션이 984KB로 companion 버퍼(1MB)에 가까워 `crates/workbench-core/src`와 나머지 crates로 나눴다. 파티션: core-src 31 / crates 나머지 76 / apps·packages·루트 66 / specs·docs 228. 합 401 = 전체, 누락 0(`--no-renames`).
  - 첫 분할은 경로 목록 누락으로 8개 파일이 빠졌다. `:(exclude)` pathspec으로 다시 만든 뒤 합집합 일치를 확인하고 실행했다.
  - 네 검토 커밋 모두 트리 `22c4fbc`다. 검토 뒤 브랜치로 돌아왔다(status 0).
- **네 파티션 모두 needs-attention**이었다. Medium 6건.
- 담당: core fork는 `crates/workbench-core`, host fork는 `crates/workbench-host`, apps fork는 `apps/`, 메인은 스모크 스크립트·문서. 프로토콜 변경은 없었다.
- 조율: host 증거 실행은 core 커밋 뒤에만 했다(그 전 실행은 비증거 표시). apps 최종 itest는 core·host 커밋 host에서 crates가 깨끗하고 표지 0일 때만 돌렸다. 스크립트는 bash로 실행했다(zsh 목록 분할 사고 방지).

| # | 등급 | 지적 | 처리 | 근거 |
|---|---|---|---|---|
| U1 | medium(core-src) | 되돌리기 재시도가 항목을 목록에서 빼고 예약 없이 기다려, 동시 정지 판정이 활동 0으로 읽고 낡은 파생 값으로 stopping에 들어갈 수 있다. 재시도 future를 취소하면 정리 책임도 유실된다 | `3598fc1`: 재시도 중에도 항목을 목록에 두고 `in_flight`로 표시한다(동시 재시도는 하나로 묶음). 등록과 재시도의 시작·끝에서 관문 세대를 올린다(`note_activity_change`). 커밋·결과 반영은 소유 task가 끝까지 한다 | `child_assign_atomic.rs` (a) 재시도가 커밋 직전에 멈춘 사이 default 거절·wait 유지 → 다시 실패해도 활동 → 회복 뒤에야 정지, (b) 재시도 abort 뒤에도 정리 완료·재배정 실제 기동. 동작 red → green 19. 변이 m1(진행 중 항목 숨김)·m2(소유 task 없음) red |
| U2 | medium(core-src) | 되돌리기가 작업대 id로만 작업 영역을 찾아, 작업대를 닫으면 영원히 NotFound → 영구 재시도·보유 잔존·정지 영구 차단 | `3598fc1`·`6d6ec3f`: `revert_child_launch`가 작업 영역 id로 찾는다(노드가 이 기동의 run일 때만 바꿈, 작업 영역이 없으면 바꿀 것 없음). R14 수명 표 | (c) 저장 실패 → 작업대 닫기 → 회복 → 목록·보유 해제, 활동 0 → 같은 작업 영역 재개 뒤 재배정 실제 기동. 변이 m3(작업대 id 조회) red |
| U3 | medium(crates-rest) | identify HTTP 요청에 전체 시간·응답 크기 상한이 없다. 조금씩 보내는 끝점이면 `ensure`가 `startup.lock`을 쥔 채 멈춘다 | `0a800c7`: 요청 하나에 절대 deadline(`min(지금+5s, 호출자 deadline)`, 읽을 때마다 남은 시간 계산)과 응답 16MiB 상한을 둔다. `content-length`·chunked 끝에서 읽기를 끝낸다(헤더 해석 선형). `verify_by`·`require_serving_by`·`call_by`에 같은 deadline을 준다. `ensure`는 최초 확인 전에 `ready_timeout`을 시작한다 | `bounded_requests.rs`(시험 안 대기 상한 30초): 끝없이 조금씩 보냄 → deadline 안 오류, 초과 본문·헤더 없는 초과 → `too large`, 완결 응답은 열린 연결에서도 바로 반환, `ensure` → 상한 안 `Timeout` 뒤 `startup.lock` 재획득. 동작 red 5건(끝없는 읽기 2건은 시험 상한에서 실패), 변이 a·b·c red. 증거 실행은 모두 core 미커밋 0 |
| U4 | medium(apps) | Full restart 취소 대기 중 도착해 확인된 교환을, 취소가 unknown·fault로 끝날 때 호출 전 스냅샷으로 덮어써 삭제 | `dcae39a`: 실패·거절·unknown이면 현재 대기열을 보존한다(취소 대기 중 들어온 항목 포함). 성공이면 취소된 run의 현재 대기열(끝 이벤트가 먼저 왔으면 그때 비운 대기열)로 재시작한다. 교환은 discard하고, 일반 prompt는 새 run으로 옮긴다. Cancel 버튼도 같은 규칙 | 패널 vitest: 옛 패널에서 새 시험 5개가 모두 동작 red → green 44, 변이 a·c red. **실제 host + 실제 AgentRunPanel itest**: 취소 응답을 붙잡은 사이 `x-2` 도착·확인 → unknown → `x-2` 보존(`pendingExchanges=2`) → turn 끝 `x-1`·`x-2` 각 1회 전달 → wait-stop 정지, 대체 run 없음. 옛 패널 red, 변이 a red |
| U5 | medium(apps) | 첫 재시작이 unknown(미적용)이면 보류가 남고, 재시도의 cancelled 이벤트가 응답보다 먼저 오면 보류 callback과 성공 경로가 각각 run을 시작(중복) | `dcae39a`: Full restart마다 시도 id를 주고 재시작은 시도당 한 번만 소비한다. 새 Full restart·Cancel은 이전 보류를 대체하고, 대체된 시도의 늦은 결과는 패널을 바꾸지 않는다 | 패널 변이 d(두 guard 제거, 재시도)·e(Cancel) red. itest: 첫 재시작 unknown(미적용) → 재시도 취소가 서버에 닿고 응답은 끝 이벤트 뒤까지 붙잡음 → 대체 run 정확히 1개(서버 `bench.list`에 원래 run과 새 run만), `pendingExchanges=0`, wait-stop 완료. 옛 패널 red(run 3개), 변이 d red. **변이 b(단일 소비 검사만 제거)는 green으로 남았다**: 단일 소비 검사와 보류 해제가 각각 혼자서도 중복 기동을 막기 때문이다(방어 중복). 두 검사를 모두 뺀 d·e가 red다 |
| U6 | medium(docs) | `close-run.sh`가 run 잔존·토큰 200을 기록만 하고 0으로 끝나고, `quit-run.sh`도 TOKEN=1의 토큰 결과를 무시 | `9519f42`: `close_final`(run 잔존 7, 토큰 미폐기·검사 오류 8), `token_verdict`. quit은 정상 종료(c·d·e) 401, SIGTERM(g) 대조 200을 요구하고 어긋나면 8. 판정은 정리 전에 보존하고 정리 뒤 종료 코드로 돌려준다 | 자기 시험이 두 스크립트의 판정 구간(`ok=no`·`gone=no`부터 끝)을 그대로 떼어 모의 입력으로 돌린다. 옛 스크립트 red 10 → green 46 → 변이 2건 red(각각 토큰·run 잔존 판정으로만 분리). 변이는 scratch 사본에만 적용했다. 과거 실제 앱 실행은 원자료 `run-removed=`·`token-after-*=`로 재판정했다(창 닫기 21개 모두 제거·401, quit 유효 경로 401, (g) 200; app-smoke.md) |

- 한계:
  - U1·U2: 닫기와 되돌리기가 함께 실패하면 정리가 목록에 남고 정지가 계속 막힌다(의도, status에 보임). 재시도에 backoff가 없다. 오류 주입은 test-hook이다.
  - U3: 16MiB 상한은 lifecycle 응답 기준이다(그보다 큰 operation 출력은 거절). `ensure` 전체는 `startup_lock_timeout + ready_timeout`까지 걸릴 수 있다(기존 계약). 가짜 끝점은 시험 안 `TcpListener`이고, 실제로 다른 프로세스가 포트를 차지한 상황은 재현하지 않았다.
  - U4·U5: 실패는 화면 호출 클라이언트에서 주입했다(실제 네트워크 단절 아님). 시험 host는 scripted engine이다.
  - 실제 앱 스모크는 7·8·9차 수정 뒤 돌리지 않았다.
- 9차 수정으로 코드가 바뀌었으므로 T052는 gate-12 전까지 다시 미완료로 둔다.
- gate-12(`051c63a`)가 9차 수정을 모두 포함해 전부 통과했다.
- **OCR 9차 재리뷰**(`4a0b6a6..051c63a`): High·Medium 없음.
  - 확인: 되돌리기 재시도는 항목을 목록에 남긴 채 `in_flight`로 표시하고, 시작·끝에서 세대를 올리며, 결과는 소유 task가 반영한다. 작업 영역 id로 되돌린다. identify·lifecycle 요청은 읽기마다 남은 시간을 다시 계산하는 전체 deadline과 크기 상한을 가진다. 패널은 시도 id로 재시작을 한 번만 소비한다.
  - Low(보고만): chunked 응답은 빈 줄로 끝나는 읽기마다 본문 전체를 다시 풀어 보므로 큰 chunked 응답에서 비용이 제곱으로 늘 수 있다(lifecycle 응답은 작다).

### 10차: 같은 HEAD `b849283`(트리 `4e42103`) 네 파티션 리뷰

- 파티션: core-src 31 / crates 나머지 77 / apps·packages·루트 66 / specs·docs 228. 합 402 = 전체, 누락 0(`--no-renames`). 네 검토 커밋 모두 트리 `4e42103`이다. 검토 뒤 브랜치로 돌아왔다(status 0).
- **네 파티션 모두 needs-attention**이었다. Medium 4건.
- 담당·조율은 9차와 같다(core·host·apps fork + 메인 docs, 공유 잠금, bash 실행, 프로토콜 변경 없음).
  - host 증거 실행은 core 미커밋이 0일 때만 셌다(`green-1`은 비증거).
  - apps 최종 itest는 core·host 커밋 뒤 crates가 깨끗하고 표지 0일 때 돌렸다.
  - apps의 첫 itest 묶음(`itest-green-1` 등)은 시험 자체의 순서 결함으로 비증거다(고친 뒤 다시 실행).

| # | 등급 | 지적 | 처리 | 근거 |
|---|---|---|---|---|
| T1 | medium(core-src) | 바인딩 커밋 직후 결과 수신 전에 복구가 돌면, 자리를 보유 없는 실행 중 자리로 바꿔 abort 뒤 `release_hold`가 자리를 못 지운다(capacity 1이면 영구 대기) | `fb91db9`·`7836952`: `reconcile_preserving(active, ready, launching)`. 이전 자리의 시도별 보유를 옮기고, 이 프로세스에서 기동 중·되돌리는 중인 task는 성공 인계(`transfer`) 전에 실행 중으로 확정하지 않는다. 보유가 있으면 자리를 남긴다. R14 복구 시점 표 | `child_assign_atomic.rs`: 바인딩 커밋 직후 복구 → abort(자리 반납, 새 과제가 자리를 얻음), 예약 커밋 직후 복구 → abort·성공, 저장되지 않은 되돌리기 중 복구(보유 유지 → 재시도 저장 → 재배정 기동). 동작 red 4 → green 23, 변이 m1·m2·m3 red |
| T2 | medium(crates-rest) | 인증 전 응답의 chunk 길이 `ffffffffffffffff`가 `size + 2` overflow panic(release에서는 잘못된 슬라이스 panic)을 일으킨다 | `d204599`·`2c324c5`: chunk·`content-length`를 검사한 산술로만 다룬다(`checked_add`·`get`). 상한 초과 선언은 본문 전에 곧바로 거절한다. 16진이 아닌 값, 충돌하는 `content-length`, CRLF 누락, 1KiB 넘는 줄, 짧은 본문은 오류로 끝낸다. chunked 끝 검사는 증분 스캐너로 선형이다(OCR 9차 Low 해결) | `untrusted_framing.rs` 10개(시험 안 `TcpListener`, 별도 스레드 + 상한 30초 join으로 panic을 단정 실패로 드러냄): 동작 red 8(`usize::MAX`·`MAX-1`은 실제 overflow panic) → green, 변이 a·b·c red, release 빌드 시험 10 passed. 비UTF8 상태 줄·확장자 붙은 정상 chunk 시험은 수정 전에도 통과해 회귀 방지용이다(증거 아님) |
| T3 | medium(apps) | Full restart 취소 대기 중 turn이 끝났는데 취소가 unknown·fault로 끝나면, 호출 전 응답 대기 true를 되살려 자동 전송이 막힌다(교환 미전달, wait-stop 미완료) | `0fa1a5a`·`fd95a36`: 취소 진행은 `cancelsInFlight`로 따로 센다(대기 중에는 취소 중인 run에 대기열을 보내지 않음). 응답 대기는 lifecycle만 정하고 바꿀 때마다 순번을 올린다. Full restart는 응답 대기를 건드리지 않는다. 자동 전송·직접 전송·Cancel & send 실패는 그 사이 lifecycle이 안 바뀌었을 때만 되돌린다 | 패널 vitest: 옛 패널 red 4 → green 48, 변이 a·b red. **실제 host + 실제 AgentRunPanel itest**: 취소 응답 보류 → turn 끝 → `x-2` 도착·확인 → 전송 없음 → wait-stop(`pendingExchanges=2`) → unknown 해제 → `x-1`·`x-2` 각 1회 전달 → 정지. 옛 패널 red, 변이 a는 지적한 wait-stop 미완료를 재현, 변이 b red |
| T4 | medium(docs) | `close-run.sh`가 클릭 명령 실패 코드를 버리고, 앱 생존·남은 창을 기록만 해 (a)(b1)에서 Settings까지 닫히는 회귀도 통과시킬 수 있다 | `54a208d`: 닫기 동작 종료 코드 보존(실패 = 무효 6). `window_verdict`: (a)(b1)은 앱 생존 + 남은 창이 정확히 `Settings`, (f)는 앱 종료. 어긋나면 9. (b2 기대를 `Settings`로 둔 것은 **틀렸다** — 11차 S5에서 정정) | 자기 시험이 닫기 동작부터 스크립트 끝까지를 그대로 떼어 모의 입력(sx·키 입력 모의)으로 돌린다. 옛 스크립트 red 9 → green 59 → 변이 2건 red(클릭 실패 판정·창 상태 판정으로 분리). 과거 원자료 재판정: (a)(b1) 14개 통과, (f) 5개 통과, **(b2) 7개는 앱 전체 종료로 9** — 기존 "(b2) 미해결" 기록과 같다(app-smoke.md) |

- 한계:
  - T1: 복구가 기동 중 목록을 읽은 직후 새로 시작한 시도는 목록에 없지만, 보유가 있으면 자리를 보존한다.
  - T2: 가짜 끝점은 시험 안 소켓이다. 상한(16MiB, 줄 1KiB)은 lifecycle 응답 기준이다.
  - T3: 취소 대기 중에도 수동 직접 전송은 막지 않는다(자동 전송만 막음). 직접 전송·Cancel & send·자동 전송 실패 경로의 순번 규칙에는 전용 시험이 없다. 실패는 화면 호출 클라이언트에서 주입했고, 시험 host는 scripted engine이다.
  - 실제 앱 스모크는 7–10차 수정 뒤 돌리지 않았다.
- 10차 수정으로 코드가 바뀌었으므로 T052는 gate-13 전까지 다시 미완료로 둔다.
- gate-13(`656fea1`)이 10차 수정을 모두 포함해 전부 통과했다.
- **OCR 10차 재리뷰**(`b849283..656fea1`): High·Medium 없음.
  - 확인: `reconcile_preserving`은 이전 자리의 보유를 옮기고, 기동 중 task를 실행 중으로 확정하지 않으며, 보유가 있으면 자리를 남기고 대기열에 넣지 않는다(단위 시험 2 + 순서 시험 4). 패널은 취소 진행(`cancelsInFlight`)을 응답 대기와 분리하고 lifecycle 순번으로 되돌리기를 판정한다. 틀 해석은 검사한 산술만 쓴다.
  - Low(보고만): 기동 중 목록에 있으나 이전 자리가 없는 task는 보유·실행 중 표시가 없는 자리로 남는다(기동 중이면 보유가 있어야 하므로 실제로는 생기지 않을 것으로 본다).

### 11차: 같은 HEAD `827cc84`(트리 `2671ae6`) 네 파티션 리뷰

- 파티션: core-src 31 / crates 나머지 78 / apps·packages·루트 66 / specs·docs 228. 합 403 = 전체, 누락 0(`--no-renames`). 네 검토 커밋 모두 트리 `2671ae6`이다. 검토 뒤 브랜치로 돌아왔다(status 0).
- **네 파티션 모두 needs-attention**이었다. Medium 5건.
- 담당·조율은 9·10차와 같다.
  - core·apps fork가 **API rate limit(429)으로 중간 종료**됐다. 메인이 잠금 비어 있음, 표지 0, 실행 중 프로세스 없음을 확인하고, 두 fork를 각자의 맥락 그대로 다시 이어 가게 했다(새 fork로 중복 시작하지 않음).
  - host `red-1`은 읽기 속도 때문에 결함을 재현하지 못해 비증거다(오류 문구 차이만).
  - host `green-1`, apps `panel-red-1`(환경: 전역 vitest)·`itest-dev-1`(시험 순서 결함)도 비증거다.

| # | 등급 | 지적 | 처리 | 근거 |
|---|---|---|---|---|
| S1 | medium(core-src) | 복구가 task를 Ready로 읽은 뒤 scheduler 적용 전에 그 기동이 bind·transfer를 끝내면, 보유 없는 실행 중 자리를 버려 살아 있는 run이 용량에서 사라진다 | `e8a2518`·`1a2efdf`: scheduler가 task마다 변경 세대를 남긴다(보유 획득·놓기, transfer, release, 대기열, 승격). `recover`는 스냅샷 읽기 **전** 세대를 받아 `reconcile_since`에 넘긴다. 그보다 늦게 바뀐 task는 지금 자리·대기 상태를 그대로 쓴다 | `child_assign_atomic.rs`: 스냅샷 `Ready` 사이 기동 성공·transfer → 복구 적용(실행 중 자리 유지, 한도 1), 스냅샷 `Running` 사이 release → 복구(자리 되살아나지 않음). 새 지점 `RecoverBeforeSchedulerApply`. compile red → 동작 red 4 → green 27. 변이 m2(스냅샷이 덮음)·m3(세대를 스냅샷 뒤에 읽음) red |
| S2 | medium(core-src) | `launch_existing_task`·`launch_task_for_ui`가 SlotHold를 쥔 채 작업 영역을 읽는 동안(기동 guard 전) 취소되면 유령 보유가 영구히 남는다(복구도 보존) | 같은 커밋: `SlotHold`를 RAII로 바꿨다(`transfer`·`release_hold` 없이 버려지면 drop이 자기 보유를 놓음, 두 번 놓지 않음). 모든 `acquire_hold` 지점이 같은 타입을 거친다 | 배정과 UI 기동을 각각 `BeforeAssignSnapshot`에서 abort → 복구 → 보유 0, 새 과제가 자리를 얻음. 변이 m1(drop 놓기 제거) red |
| S3 | medium(crates-rest) | 쓰기 대기 상한을 한 번 잡고 `write_all`을 불러, 큰 요청을 천천히 읽는 끝점이면 부분 쓰기마다 같은 상한이 반복돼 요청 전체 deadline을 넘는다 | `c828572`: `write_message`가 부분 쓰기마다 `remaining(deadline)`으로 상한을 다시 잡는다(0바이트 = 닫힘, `WouldBlock`·`TimedOut` = 시간 초과). 연결·쓰기·읽기가 한 deadline을 공유한다 | `slow_request_writes.rs`: `call_by`로 8MiB 입력, 끝점은 10ms마다 4KiB, 호출자 deadline 1초. `red-2` 동작 red(19.9초) → green(core 미커밋 0) → 변이 red(18.2초). 시험 자체 상한 30초 |
| S4 | medium(apps) | 교환 삭제의 discard 응답을 기다리는 동안 Full restart가 run을 바꾼 뒤 discard가 실패하면, 옛 항목을 새 run 대기열에 되살린다(서버 거절 → 자동 전송이 선두에 반복 재삽입 → 뒤 prompt 막힘) | `6e9be39`·`062cbbb`: 대기열 교환 항목에 `exchangeRunId`를 둔다. discard 실패 복원은 같은 run이 활성일 때만, 자동 전송 실패 재삽입은 notApplied·unknown이고 같은 run일 때만 한다. 서버가 거절했거나 run이 바뀐 교환은 오류만 보이고 버리며, 다른 run에 묶인 교환은 보내지 않고 뺀다 | 패널 vitest: 옛 패널 동작 red 3 → green 223(agent-run), 변이 a(항상 되돌림)·b(항상 재삽입) red. **실제 host + 실제 AgentRunPanel itest**: discard 보류 → Full restart·새 run → discard notApplied → 새 대기열에 옛 교환 없음 → 후속 prompt 새 run에 1회 → `pendingExchanges=0` → wait-stop 정지. 옛 패널 red, 변이 a red |
| S5 | medium(docs) | b2 판정이 실제 닫기 대상과 반대였다(Cmd+W는 앞의 Settings에 가는데 "Settings만 남음"을 성공으로 봄 → 배경 메인 창만 닫히는 오동작이 통과할 수 있음) | `2eff197`: 경로별 기대를 나눴다. (a)(b1)(f)는 메인 창을 닫으므로 run 제거·토큰 401, (b2)는 Settings를 닫으므로 남은 창 `Agentic Workbench`·run 유지·토큰 200. 10차 T4 설명도 정정했다 | 자기 시험 66개: 정상 b2(Settings만 닫힘 → 0), 배경 메인 창만 닫힘(9), run 제거(7), 토큰 401(8). 옛 스크립트 red 7 → green → 변이 2건 red. 과거 b2 7개 실행은 모두 앱 전체 종료라 정정된 기대로도 9이다. **b2는 미해결**이다(app-smoke.md) |

- 한계:
  - S1: 세대를 읽은 뒤·스냅샷 전 사이에 바뀐 task도 늦게 바뀐 것으로 보아 지금 상태를 쓴다(결과 같음).
  - S3: 읽기 속도는 시험 안 sleep으로 흉내 냈다. 결함이 드러나는지는 끝점 속도에 달렸다. 요청 쪽 크기 상한은 두지 않았다.
  - S4: 활성 run 없이 들어온 교환은 묶인 run이 없다(기존). 일반 prompt의 재삽입은 제한이 없다(영구 거절이면 반복될 수 있음). 실패는 화면 호출 클라이언트에서 주입했다.
  - 실제 앱 스모크는 7–11차 수정 뒤 돌리지 않았다.
- 11차 수정으로 코드가 바뀌었으므로 T052는 gate-14 전까지 다시 미완료로 둔다.
- 조율 추가: apps fork의 커밋 시도 한 번이 core 커밋의 `git index.lock`에 걸려 exit 128로 실패했다. 잠금 파일은 건드리지 않고, 풀린 뒤 잠금 아래에서 다시 커밋했다.
- gate-14(`5bfa82c`)가 11차 수정을 모두 포함해 전부 통과했다.
- **OCR 11차 재리뷰**(`827cc84..5bfa82c`): High·Medium 없음.
  - 확인: `SlotHold` drop은 scheduler 잠금을 잡지만, `transfer`·`release_hold`는 소유를 해제한 뒤 잠근다(이중 잠금 없음). `reconcile_since`는 스냅샷 세대보다 늦게 바뀐 task의 자리·대기를 그대로 둔다. `write_message`는 부분 쓰기마다 남은 시간으로 상한을 다시 잡는다. 패널 교환 항목은 `exchangeRunId`로 run에 묶인다.
  - Low(보고만): scheduler의 `touched` 맵은 다음 재구성 때까지 task id마다 한 항목씩 쌓인다(task 수만큼, 이벤트 수가 아님).

### 12차: 같은 HEAD `7ba92ba`(트리 `133de34`) 네 파티션 리뷰

- 파티션: core-src 31 / crates 나머지 79 / apps·packages·루트 66 / specs·docs 228. 합 404 = 전체, 누락 0. 네 검토 커밋 모두 트리 `133de34`, 검토 뒤 브랜치로 돌아왔다(status 0).
- **네 파티션 모두 needs-attention**이었다. 서로 다른 지적 4건(High 1, Medium 3). crates-rest 파티션의 high는 core-src의 첫 medium과 같은 문제라 high로 처리했다.
- 담당: core fork(`crates/workbench-core`), apps fork(`apps/`), 메인(스모크 스크립트·문서). host 지적은 없었다. git commit도 잠금 아래에서 했다(11차 `index.lock` 충돌 방지).

| # | 등급 | 지적 | 처리 | 근거 |
|---|---|---|---|---|
| R1 | high(crates-rest·core-src) | 겹치는 복구: A가 Ready 스냅샷을 얻은 뒤 기동이 성공·transfer하고, B가 먼저 적용되면 B가 A에 필요한 변경 기록을 지워, A가 살아 있는 run의 자리를 버린다(한도 초과) | `80598c3`·`a1b01df`: 복구는 스냅샷 전에 `begin_reconcile`로 자기 세대를 창으로 등록하고 `reconcile_window`로 적용한다. 변경 기록은 진행 중 창 중 가장 오래된 세대보다 늦은 것을 모두 남긴다. 재구성이 바꾼 task도 변경으로 기록해, 뒤늦게 적용되는 낡은 복구가 새 결과를 덮지 않는다. 창이 모두 닫히면 기록을 지운다(OCR 11차 Low 해결) | `child_assign_atomic.rs` `overlapping_recoveries…`: A 스냅샷 → 기동 성공 → B 복구 완료 → A 적용 → 자리 유지, 추가 task 대기. 수정 전 소스에서 동작 red(`active 0≠1`) → green 28(scheduler 13). 변이 m1(다른 창 무시) 통합·단위 red, m3(재구성 기록 안 함) 단위만 red |
| R2 | medium(core-src) | `transfer`가 자기 보유 제거 실패를 무시하고 실행 중으로 만들어, 옛 기동의 transfer가 새 시도의 자리를 실행 중으로 고정한다(용량 점유) | 같은 커밋: 자기 보유 id를 실제로 뺀 경우에만 실행 중으로 확정한다. R14에 scheduler 상태 변경 표(모든 변경이 자기 보유·자기 세대일 때만 효과) | scheduler 단위 시험 `a_stale_transfer…`(옛 보유 → release → 새 보유 → 옛 transfer → 새 보유 해제 → `active_count=0`). 변이 m2 단위 red(통합 시험은 이 순서를 만들지 않아 0) |
| R3 | medium(apps) | 교환 전송이 서버에 적용되고 답만 unknown이면, turn을 이미 봤는데도 같은 키로 재전송하고 그 멱등 재생을 새 turn으로 보아 응답 대기가 풀리지 않는다(뒤 prompt·교환 고착, wait-stop 미완료) | `bbf82e6`·`7e741f6`·`ad3f69d`: 패널이 run별 관측 `promptSent` 수를 센다. unknown인데 그 사이 같은 run의 turn을 봤으면 적용된 것으로 보고 다시 넣지 않는다. 아직 못 봤으면 처음 기준을 남긴 채 같은 키로 다시 넣고, 다시 보내기 직전에 그 turn이 관측되면 보내지 않는다. 직접 전송도 turn을 봤으면 입력창에 되돌리지 않는다 | 패널 vitest: 옛 패널 동작 red 2 → green 229, 두 판정을 모두 뺀 변이 c red(각 판정만 뺀 a·b는 green — 같은 순서를 서로 막는 방어 중복), 직접 전송 red·변이 a2 red. **실제 host + 실제 AgentRunPanel itest**: x-1 서버 적용·답 보류 → turn 끝 관측 → unknown → x-2 도착·확인 → wait-stop 정지, x-1 1회(재생 없음)·x-2 1회. 옛 패널 red(wait-stop 미완료), 변이 c red |
| R4 | medium(docs) | `close-run.sh`가 `bench-check.py` 종료 코드를 버려, b2(run 유지 기대)에서 조회가 모두 실패해도 통과할 수 있다 | `f181e92`: 조회 결과를 있음·없음·검사 실패로 나눈다(`run_state_of`: 종료 0 + `result=ok` + `runListed` 참/거짓만 성공). 성공한 조회가 없으면 7(`run-state-unknown`) | 자기 시험 75개: `identity-failed`·빈 출력·잘못된 JSON → 7, `run_state_of` 단위. 옛 스크립트 red 4 → green → 변이 red. 과거 b2 실행은 모두 성공한 조회의 `run-removed=yes`라 잘못 통과한 경우 없음(app-smoke.md) |

- 한계:
  - R1·R2: 옛 인계 순서와 재구성의 변경 기록은 단위 시험으로만 확인했다(변이 m2·m3은 통합 시험에서 red가 아님). 이미 결과를 보고한 task에 바인딩이 성공하면 그 run은 scheduler 자리 없이 끝까지 돈다(task는 이미 끝남).
  - R3: 관측된 `promptSent`가 결과를 모르는 바로 그 전송의 것인지는 확인하지 않는다(그 사이 같은 run에 직접 전송·steer하면 오판 가능). 멱등 키 없는 일반 대기 prompt를 turn을 보기 전에 다시 보내면 중복 turn이 생길 수 있다(기존). 실패는 화면 호출 클라이언트에서 주입했다.
  - 실제 앱 스모크는 7–12차 수정 뒤 돌리지 않았다.
- 12차 수정으로 코드가 바뀌었으므로 T052는 gate-15 전까지 다시 미완료로 둔다.

### 최종 HEAD 재검토

위 수정으로 HEAD가 바뀌었으므로, 최종 게이트 뒤 코드·문서 분할 리뷰를 **같은 최종 HEAD**에서 다시 실행한다(아래에 기록).

#### OCR 12차 (`7ba92ba..fd4776f`)

- `ocr delegate preview --format json`이 17개 변경 파일 중 9개를 reviewable로, 8개를 excluded로 분류했다. reviewable 9개 전부에 `ocr delegate rule`을 적용하고 diff와 호출 경로를 검토했다(9/9, coverage 100%, skipped 0). verdict는 **needs-attention**이다.
- 최초 지적은 queued exchange와 직접 prompt를 함께 High로 분류했다. queued exchange의 **같은 키 재시도 계약 자체**는 기존 시험 `an exchange delivery answered unknown before any turn was seen is retried with the same key`로 유효했다. 다만 당시 적용한 run-wide `promptSent` 관측 상관은 r13에서 별도의 유효한 High 결함으로 판정됐다. 최종 구현은 lifecycle 횟수 추정을 제거하고 모든 prompt에 stable idempotency key를 부여하며, HTTP replay 표지로 확정한다.
- **Medium — 직접 prompt의 unknown 응답 뒤 늦은 `promptSent`를 조정하지 않아 중복 전송 가능**(`agent-run-panel.tsx`): 직접 prompt에는 멱등 키가 없다. HTTP unknown이 먼저 오면 패널은 입력을 복원하고 낙관적 transcript를 제거했지만, 뒤늦은 `promptSent`는 그 상태를 고치지 않았다. 사용자가 복원된 입력을 다시 보내면 같은 prompt가 두 번 적용될 수 있다.
- **당시 반영 후 r13에서 교체**: OCR12 직후에는 run·prompt text·전송 전 `promptSent` 수를 unsettled 상태로 남겼고 패널 60/60을 통과했다. 사용자가 늦은 이벤트 전에 편집한 새 초안과 replacement run 격리 시험도 통과했다. 그러나 같은 run의 다른 송신을 구별하지 못하므로 이 방식은 최종 근거가 아니다.
- **최종 복구 계약**: 직접 prompt도 `prompt-send:<uuid>` stable key를 가진다. unknown이면 같은 원문만 같은 키로 재시도할 수 있고, 편집한 새 초안은 보존하되 중복 위험 때문에 전송하지 않는다. 사용자는 `Cancel`로 복구할 수 있다. 시간 경과나 uncorrelated lifecycle로 적용 여부를 추정하지 않는다. 응답의 `replayed`가 확정 적용을 구별하고, old run의 늦은 이벤트는 새 run composer에 닿지 않는다. 최종 패널 시험 60/60은 r13 표 C12에 기록한다.
- OCR excluded 8개는 `run-panel-state.test.ts`, `agent-run-panel.test.tsx`, 044 계약·research·review·tasks 문서다. 이 분류를 reviewable coverage에 섞지 않았고, 최종 Codex 네 파티션에는 모두 포함한다.

### Codex r13 네 파티션과 18건 정본

- 순서: OCR12 결과와 반영을 먼저 기록한 뒤, 같은 코드 tree `b87f356`에서 Codex r13을 실행했다. core-src, crates-rest, apps·packages·root, specs·docs의 **4/4 모두 `needs-attention`**이었다. 보고는 High 12건·Medium 6건, 합계 18건이며 문서 파티션 5건을 포함한다.
- 범위: 임시 ref의 실제 비교는 `refs/review/044-r13-baseN..refs/review/044-r13-headN`이며 파티션별 31 / 78 / 57 / 228개다. 합집합 394개, 중복 0이고 이는 044 전체 구현 baseline `cb0bd4c`(043 stage 4 merge)..`b87f356`의 394개와 정확히 같다. `7ba92ba..b87f356` 직접 delta는 17개이므로 전체 범위 근거로 쓰지 않는다. `.specify/feature.json`은 3번 apps·packages·root 파티션에 포함했다. 네 head의 tree는 모두 `0eaf5835bf3780d64f4d583e2cd9a918832e1f58`로 `b87f356` tree와 같고, 리뷰 중 checkout한 코드 tree를 바꾸지 않았다.
- 아래의 “통과”는 실제 종료 코드 0과 1개 이상 실행된 시험만 적는다. “반영”은 코드·문서 편집 상태이며 최종 전체 gate나 실제 앱 스모크를 뜻하지 않는다.

| # | 등급 | 유효성·지적 | 반영 파일(요약) | 실제 통과한 회귀시험 | 남은 검증 |
|---|---|---|---|---|---|
| C1 | High | **유효** — retire 뒤 window principal이 새 event 구독 가능 | `workbench_runtime.rs` | `events_reject_a_window_retired_after_authentication` 1/1; core lib 260/260(후속 scheduler 시점) | 최종 core/workspace gate |
| C2 | High | **유효** — ACP 비동기 queue 실패가 handler 성공 뒤 유실 | `run_engine.rs`, `acp_run_engine.rs`, `run_service.rs`, exchange handler·시험 | production `AcpRunEngine` 경로 + fake Python ACP peer 시험: 장시간/queued turn에서 HTTP 호출은 turn 종료 전 반환, 뒤 RPC 실패 기록·같은 키 무중복·다음 교환 소비·wait-stop 해제 1/1 | 최종 exchange 전체 suite |
| C3 | High | **유효** — reject와 delivery consume 경합, 첫 수정에는 stale Accepted snapshot 경합 잔존 | exchange handler, WorkGate rejection claim·성공 tombstone, test hook, `exchange_delivery_drain.rs` | suite 10/10: delivery 선점, **snapshot 뒤 pause→reject 저장 완료→delivery 재개**, 저장 실패 claim 해제; future abort claim 해제 단위 1/1. rejected는 모두 `exchange_consumed=false` | 최종 workspace gate |
| C4 | High | **유효** — terminal child가 bind 전에 끝나 scheduler 용량 밖 worker가 생김 | orchestration service·scheduler·engine worker·agent tool, `child_assign_atomic.rs`, `orchestration_agent.rs` | terminal-before-bind 1/1; stale transfer 새 보유 뒤 회귀 1/1. 빠른 완료 create는 worker를 bind하지 않고 `completed/idle` 성공을 반환하며 same-key는 같은 task·무기동, new-key만 새 task인 결정적 시험 1/1; `orchestration_agent` 9/9; core all-targets **566 passed/0 failed/7 ignored/61 targets/filtered 0, 종료 코드 0**(lib 262 포함) | 최종 workspace gate |
| C5 | High | **유효** — detached bench cleanup이 drain reservation보다 오래 생존 | `bench_service.rs`, `bench_close_race.rs` | caller 취소 뒤 close 완료·reservation 유지 1/1 | 최종 bench integration 전체 suite |
| C6 | Medium | **유효** — window token·lease·retire epoch command가 키를 무시 | epoch idempotency, intent-first, server handler, protocol replay | owner epoch replay 3/3; client replay metadata 74/74 | 최종 protocol/core/client gate |
| C7 | High | **유효** — drain 중 ready=true, handshake 상태 누락 | server info, handshake, health, lifecycle client, process 시험 | `new_work_is_refused_while_draining` 1/1. strict ready와 동일-instance liveness를 분리한 뒤 AW draining 재시도 2/2·AW lib 125/125 | 최종 server/process suite |
| C8 | High | **유효** — run terminal·launch 실패·발급 직후 취소에서 MCP bearer 잔존 | terminal hook, launch decorator, 두 launch 경로의 `PendingLaunchRevocation`, orchestration 시험 | failed main launch·normal terminal targeted 1/1; **발급 뒤 future 취소 롤백** 1/1 | host/core 전체 suite에서 재확인 |
| C9 | Medium | **유효** — server host가 all-scope owner bearer 원문을 장기 보관 | `identity.rs`, descriptor/server, embedded ownership, identify client·계약·smoke helper | digest HMAC 고정 벡터 포함 identity 4/4; real/impostor identify 2/2 | host 전체 suite·실제 owner smoke |
| C10 | Medium | **유효** — custom `--log`가 0600을 강제하지 않음 | lifecycle server log, process 시험 | 기존 permissive custom log를 0600으로 고침 1/1 | default log 포함 process 전체 suite |
| C11 | High | **유효** — retire 1회 실패 뒤 local mapping·재시도 정보 유실 | server client lifecycle call/idempotency, retire 시험 | (a) 서버 적용·응답 유실 후 같은 키 replay 1/1, (b) **최초 요청이 서버 미도달** 후 mapping 보존·같은 검증 instance 재시도 1/1; 둘 다 bench 종료·tombstone·pending 0 | 실제 앱 창 폐기 smoke |
| C12 | High | **유효** — 같은 run의 unrelated `promptSent`로 unknown 송신 오판 | panel, HTTP transport, client/protocol `replayed`, UI 시험 | 패널 60/60: 늦은 이벤트 전 편집 초안 보존, unrelated same-run·old-run 이벤트 격리, 같은 stable key 재시도/Cancel 복구 | 최종 frontend gate·실제 앱 prompt smoke |
| C13 | Medium | **유효** — `status` CLI가 `server.status`를 호출하지 않음 | server CLI, process 시험 | status가 실제 status payload를 출력 1/1 | process 전체 suite |
| C14 | High | **유효, 미입증** — Cmd+W Settings 한 번이 main/run까지 닫음 | 중복 native Close menu 제거(`src-tauri/lib.rs`), 기존 이중-close 단위 가정 제거 | 코드 반영만 완료 | **development·release 실제 앱 b2 smoke 필수** |
| C15 | High | **유효** — gate-16은 최종 tree 이전이라 ship 근거 아님 | `tasks.md`, 이 문서 | 해당 없음 | final tree에서 `CARGO_INCREMENTAL=0` 전체 gate |
| C16 | Medium | **유효** — T045/SC-001 등 완료 표지가 실제 smoke보다 앞섬 | `spec.md`, `tasks.md`, app-smoke 문서 | 완료 표지를 미완료로 되돌림 | T045/T046/SC-001/SC-006/T052는 실제 앱 smoke·최종 gate 뒤에만 완료 |
| C17 | Medium | **유효** — FR-014가 구현과 다른 server-side incarnation 등록을 요구 | `spec.md`, research/contract | desktop 생성 incarnation + server token/tombstone 계약으로 정합화 | 최종 문서 교차검사 |
| C18 | Medium | **유효** — 계약이 공개 `server:read` scope를 누락 | lifecycle contract, plan, data-model, spec | `server.status` 요구 scope와 owner 할당을 문서화 | 최종 문서 교차검사 |

현재 결론은 **18건 모두 유효**다. C14–C16은 검증이 끝나지 않았고, 나머지도 표의 targeted 통과를 최종 전체 gate로 대체하지 않는다. 특히 owner 원문 제거는 proof key를 bearer digest로 바꾸고 안내 파일 작성 뒤 원문과 token-bearing descriptor를 drop했으며, MCP는 terminal·실패뿐 아니라 발급 직후 future 취소에서도 guard가 회수한다. retire는 응답 유실과 서버 미도달을 별도 시험으로 구분한다.

#### 수정 중 전체 suite에서 발견한 회귀

- `cargo test -p agentic-workbench --lib` 첫 실행은 123 passed / 2 failed였다. 실패는 C11 retire 시험이 아니라 `a_lost_token_response_from_a_draining_server_keeps_the_connection`, `a_lost_token_response_during_a_wait_stop_still_delivers_the_exchange_and_stops`였다.
- 원인: C7에서 draining `/health/ready`를 올바르게 503으로 바꾼 뒤, 응답 유실 재시도의 `instance_is_live`도 신규 연결용 strict `verify()`를 써 동일한 살아 있는 draining instance를 죽은 것으로 오판했다. 시험의 server executable은 의도적으로 nonexistent라 잘못된 ensure가 `ENOENT`로 끝났다.
- 수정: `verify_instance`는 identify proof + handshake의 instance/protocol/storage만 확인하고, 신규 연결용 `verify`는 이어서 ready=true를 요구한다. 기존 연결의 동일-instance 재시도만 전자를 쓴다. 두 실패를 각각 1/1 재실행한 뒤 AW lib 전체 **125/125, filtered 0**을 통과했다. 따라서 개별 retire 통과만을 C11 근거로 삼지 않는다.
- gate-17의 core 병렬 suite에서는 `first_turn_result_and_input_request_update_the_task_and_notify`가 `A terminal task cannot be bound to a worker run`으로 실패했다. C4가 terminal task의 늦은 bind를 거절하고 준비된 worker를 취소하도록 바뀌었지만, 상위 create tool이 이미 적용된 task·result·알림을 `Conflict/NotApplied`로 바꿔 반환한 API 회귀였다. 수정 뒤 최초 호출은 저장된 `completed/idle`을 성공으로 반환하고 run을 노출하지 않는다. 같은 key 재시도는 같은 task를 무기동으로 반환하며, 새 key만 별도 task를 만든다. 결정적 시험 1/1, C4 용량 시험 1/1, `orchestration_agent` 9/9, `cargo test -p workbench-core --all-targets` **566 passed/0 failed/7 ignored/61 targets/filtered 0, 실제 종료 코드 0**으로 통과했다(`/private/tmp/aw-044-core-c4/`).
