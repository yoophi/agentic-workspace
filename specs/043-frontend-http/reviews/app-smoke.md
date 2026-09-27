# 043 앱 스모크 (T053 · T054)

앱 데이터 디렉터리를 사용자의 설치본과 분리하려고 별도 identifier로 실행했다: 개발 `com.yoophi.agentic-workbench.smoke043`, 배포 출처 `…smoke043r`. 결과 파일은 `app-smoke/`에 있고, 토큰·표 문자열은 들어 있지 않다.

## probe(debug 빌드 전용)

- **Rust**: `AW_APP_TRANSPORT_PROBE_FILE`이 있으면 메인 창 로드 뒤 probe를 넣는다(`infrastructure/http_probe.rs`, 모듈은 `#[cfg(debug_assertions)]`).
- **화면**: 빌드 플래그 `VITE_AW_DEBUG_PROBE=1`일 때만 `window.__awDebug`(앱 transport 핸들)를 둔다. 운영 빌드(`pnpm --filter @yoophi/agentic-workbench build`, 플래그 없음)의 `dist`에서 `__awDebug`·`report_app_probe`·`dropEventSockets` 문자열은 0개다.
- **가짜 agent**: `crates/workbench-core/tests/support/agents/fake_acp_permission_agent.py --echo`. 받은 prompt 본문을 `echo:<본문>` agent 메시지로 먼저 돌려보낸다.
- **흐름**:
  1. 경로 확인
  2. `list_projects`
  3. `start_agent_run`(일반 패널 `probe-panel`, 고유 문자열 목표)
  4. 시작 정착: 시작 prompt의 에코와 뒤이은 `lifecycle:promptCompleted`를 앱 transport로 받는다
  5. 이벤트 소켓 강제 끊기
  6. 고유 문자열 prompt를 `send_prompt_to_run`으로 보낸다
  7. 그 prompt의 에코와 뒤이은 완료를 받는다
  8. 받은 run 스트림의 중복·빈 순번을 검사한다
  - 순번이 커졌다는 것만으로 판정하지 않는다(사용자 검토 반영).

## 결과

| 실행 | 출처 | 경로 | 결과 | 근거 |
|---|---|---|---|---|
| 개발 네트워크 | `http://localhost:1420` | http | ok | 시작·끊긴 뒤 에코 각 1회와 완료. 소켓 끊기 1건. 스트림 순번 1–13, 중복·빈 순번 없음. 서버 접근 기록의 `event-tickets` 2건(처음 + 재연결). agent 기록의 prompt 2건 |
| 배포 출처 네트워크 | `tauri://localhost` | http | ok | 위와 같음(`tauri build --debug --no-bundle`, 화면은 번들 내장) |
| 개발 끝점 기동 실패 주입(`AW_WORKBENCH_HTTP_FAIL_START`) | `http://localhost:1420` | compat | compat | `[workbench-http] failed to start: injected start failure`. `list_projects`가 호환 경로로 성공. **범위: 조회까지만 확인** — run 흐름은 이 실행에서 보지 않았다 |

## 실행 중 겪은 일(재현 조건 기록)

- 첫 실행에서 `run.start`가 412(`Main Coordinator workspace is unavailable.`)였다. probe가 `panelId: "main-agent-run"`(orchestration Main Coordinator 패널)을 써서다. 일반 패널 id로 바꿨다.
- 에코가 목표 본문과 정확히 같지 않았다. runner가 목표 앞에 MCP 안내문을 붙이기 때문이다. 판정을 "`echo:`로 시작하고 고유 문자열로 끝나는 agent 메시지"로 바꿨다.
- 살아 있던 첫 `tauri dev`가 Rust 수정을 보고 앱을 자동으로 다시 띄웠다. 그 앱은 옛 환경 변수(`--echo` 없음)로 돌아 시간 초과 결과를 남겼다. 그 결과는 버렸고, 이후 실행 전에는 스모크 프로세스(vite·앱)를 확인해 끝냈다.

## SC-004d 창 새로고침 1회 전달(실제 앱, 개발 출처)

`AW_APP_PROBE_SCENARIO=refresh`. probe는 새로고침마다 다시 들어가고, 단계는 `sessionStorage`가 잇는다. 결과는 `app-smoke/dev-refresh-exactly-once.json`, agent 기록 요약은 `app-smoke/dev-refresh-agent-prompts.txt`에 있다.

1. **1단계**: run A·B(패널 pa·pb)를 시작하고 각각 정착을 기다린다. 교환 작업 영역을 동기화하고 교환을 보낸다(pa → pb).
   - 운영 코드의 `createExchangeReconciler`(debug 핸들로 노출)가 요청 이벤트를 받아 라우팅한다. 라우팅은 run B에 `exchangeDeliveryKey(requestId)` 키로 전송한다.
   - 원장이 확인을 보내는 순간(agent가 첫 전달을 끝낸 뒤) 창을 새로고침한다. **확인 전 새로고침**이다.
2. **2단계**: 새로 부팅한 창의 빈 원장이 구독 시작 재조정으로 같은 교환을 다시 받는다.
   - 결과: 라우팅 1회(같은 키로 재전송), 실제 `acknowledge_agent_exchange` 1회, 서버 상태 `delivered`.
3. **장벽과 판정**: 같은 run에 다른 고유 prompt(barrier)를 보내 완료까지 기다린다. 세션의 prompt는 차례로 처리되므로, 재전송이 agent에 갔다면 이 전에 도착했다. 그 뒤 센 결과:
   - 앱이 받은 run B 스트림에서 그 메시지 에코 **1회**
   - agent 기록에서 그 메시지 prompt **1회**(전체 4 = 시작 A, 시작 B, 메시지, barrier)

**범위**:
- 원장과 키 도출은 운영 코드를 그대로 썼다.
- **패널 UI의 라우팅·전송(`routePromptToPanel` → 패널의 대기열·즉시 전달)은 probe가 대신했다.** 패널이 교환 prompt에 키를 싣는 부분은 T036 화면 시험(`agent-run-panel.test.tsx`의 [http] run.start 키 단정과 변이)이 근거다.
- 서버의 키 중복 제거가 실제 agent 프로세스 경계에서 1회라는 근거는 core `exchange_delivery_acp`(대조 변이 포함)다.
- 배포 출처에서는 이 시나리오를 돌리지 않았다.

## 아직 검증하지 않은 것(추적)

- **Windows 출처**: 미검증(목록만).
- **호환 경로 창의 run 흐름(실제 앱)**: 위 기동 실패 실행에서는 조회까지만 봤다.
