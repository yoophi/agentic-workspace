# 044 구현 리뷰 기록

## 5단계 완료 기준 추적 (T051)

설계 문서 `docs/client-server-architecture-research.md` §5의 5단계 완료 기준이다. **044 완료는 5단계 완료가 아니다.** "후속 미완료"는 다음 증분의 범위이며, 이 PR로 완료된 것으로 세지 않는다.

| 기준 | 044에서 완료(근거) | 후속 미완료 |
|---|---|---|
| (a) 종료 뒤 지속 | 앱 종료(Cmd+Q·Dock·AppleScript·SIGTERM) 뒤 앱 PID 소멸 → 소유자 클라이언트로 같은 run 조회·live 출력·취소, 개발·배포 출처(`app-smoke.md` T045). 서버가 시작한 turn·대기열 prompt·orchestration 알림을 끝까지 실행(`wait_stop.rs`) | CLI(6단계)로 같은 흐름. 다시 연 데스크톱이 남은 작업대에 다시 붙는 화면. 교환 전달의 서버 소유(오늘은 데스크톱 UI가 라우팅·전송, 임대가 없으면 `undeliverableExchanges`로 보고만) |
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

## 구현 리뷰 범위 (T053)

OCR·Codex 구현 리뷰에 다음을 명시적으로 넣는다.

1. **대기 task 정책 변경**(구현 중 변경, research R7): 대기 task는 coordinator run이 살아 있고 바쁘거나 그에게 미전달 알림이 있을 때만 활동 작업이다. 그 밖은 `deferredTasks`로 보고만 한다. 증거와 한계는 `implementation-evidence.md`("대기 task 정책 변경과 실제 K 경로")에 있다.
2. **scheduler 복구 수정**(041부터 있던 결함): 복구가 대기열에 넣은 Ready task도 자리가 비면 `acquire`가 시작한다.
3. **관찰 → 수정(O1)**: MCP 도구로 받은 `draining` 거절이 `structuredContent.code = "internalError"`로 나가던 것을 fault 코드 그대로 싣게 고쳤다.
4. **Cmd+W (b2)** 위험(원인 가설 둘 기각, 미해결, 사람 키 입력 미검증)과 **플랫폼 미검증**(위 목록).
5. **재시도 알림 상한(O7)**: 진행 중 시도의 예약이 정지를 막는지, 성공할 수 있는 알림이 손실되지 않는지, 재시작 뒤 자동 재전달이 아니라 `superseded` + `collectReports` 보존인 한계가 타당한지.
6. 그 밖 044 전체: 단일 writer, identify, 소유자 우회 범위, 창 토큰 tombstone, 비우기 분류와 작업 관문, 종료 판정(`window_close_intent`), #207 tombstone, 앱 스모크 증거 범위.

## 최종 게이트 (T052)

(실행 뒤 기록)

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

- 재시도를 **기다리는** 재시도 가능 실패 알림은 `attemptCount < 3`일 때만 활동이다. 그 뒤에는 `stalledNotifications`로 보고만 한다. `pending`·`dispatching`(진행 중 시도)은 횟수와 무관하게 활동이다.
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

(실행 뒤 기록)
