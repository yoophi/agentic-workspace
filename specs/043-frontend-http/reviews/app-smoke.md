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

## 아직 검증하지 않은 것(추적)

- **SC-004d 창 새로고침 1회 전달(실제 앱)**: 아직 증거가 없다. 교환 요청 → 라우팅·전송 뒤 확인 전 새로고침 → 원장 없이 재조정 → 같은 키 재전송 → agent 1회. T054 후속으로 추적한다.
- **Windows 출처**: 미검증(목록만).
- **호환 경로 창의 run 흐름(실제 앱)**: 위 기동 실패 실행에서는 조회까지만 봤다.
