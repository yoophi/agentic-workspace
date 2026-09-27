# Contract: Tauri 호환 어댑터 (041)

화면(`apps/agentic-workbench/src`)은 바뀌지 않는다. command 이름·인자·반환 타입·오류 문자열이 오늘과 같다.

## command → operation

| command | operation | 작업대 확보 |
|---|---|---|
| `bootstrap_orchestration_workspace(worktreePath, resumeWorkspaceId?)` | `orchestration.bootstrap` | `ensure(label, worktreePath)` |
| `recover_orchestration_workspace()` | `orchestration.recover` | 조회만(없으면 오늘 "작업 영역 없음" 오류) |
| `get_orchestration_workspace()` | `orchestration.get` | 조회만(없으면 `null`) |
| `list_recoverable_orchestration_workspaces(worktreePath)` | `orchestration.listRecoverable` | `ensure(label, worktreePath)`(오늘도 창과 무관하게 목록을 준다) |
| 나머지 13개 orchestration command | 대응 `orchestration.*` | 조회만(없으면 오늘 서비스 오류 JSON) |
| `replay_orchestration_runtime_events(runId, afterSequence)` | `run.replay` | 조회만. 결과는 오늘처럼 항상 `RunReplay`(실패 없음): 창에 작업대가 없거나 run이 다른 작업대 소유면 core를 부르지 않거나 `forbidden`을 받아 **모르는 run과 같은 빈 replay**(`events: []`, `lastSequence: 0`, `gapDetected: afterSequence > 0`)를 돌려준다 — 오늘은 다른 창의 run도 재생되던 누수를 막는다 |

- 오류: `serde_json::to_string(&OrchestrationError)`(fault `details.orchestrationError` 원본) — 오늘 `orchestration_error`와 같은 문자열. 작업대 계열 fault(`bench not found.` 등)는 창이 작업대를 가진 동안 생기지 않는다.
- **결과의 `boundWindowLabel`**: core DTO에는 없다. compat가 다시 채운다 — 작업 영역이 이 창의 작업대에 묶였으면 이 창 label, 아니면 `null`. core DTO의 `eventStreamId`는 결과에서 빼서 오늘 형태와 같게 둔다.
- 창을 언급하는 오류 문구 3개는 바이트 동일로 유지한다(core에 문자열로 남지만 창 식별자는 아니다). 4단계 화면 전환 때 재검토.

## 창 수명

| 이벤트 | 040 | 041 |
|---|---|---|
| 세션 창 `Destroyed` | `DesktopBenches::close` → orchestration `release_window(label)` | `DesktopBenches::close` → `bench.close`(소유 run 취소 → 작업 영역 복구 가능 전환·스트림 제거 hook). AW의 별도 해제 없음 |

## 이벤트 전달 (데스크톱)

`TauriDesktopBridge`가 작업대 → 창을 찾아 창 삽입 경로 하나로 보낸다. 네이티브 `emit` 제거.

| 종류 | CustomEvent 이름 | payload |
|---|---|---|
| 작업 영역 갱신 | `orchestration-workspace-updated-fallback` | `OrchestrationEvent`(오늘 형태) + `sequence`·`epoch`·`streamId`·`eventId` |
| 명령 상태(`reason`에 command) | `orchestration-command-updated-fallback` | 같은 payload |
| 알림 상태(`reason`에 notification) | `orchestration-coordinator-notification-updated-fallback` | 같은 payload |

화면 수신 코드(`listenWithFallback`)는 삽입 경로를 이미 듣는다.

## MCP 도구 (AW MCP 서버)

도구 16개는 `AuthenticatedPrincipal::agent(capability.run_id)`로 대응 agent operation을 부른다. 도구 쪽 `runId == principal.run_id` 검사와 문구 유지. 도구 결과·오류는 오늘 형태(`structuredContent`)로 되돌린다. capability registry는 **token → run id**만 한다(역할 주장 제거, research R7). 교대 시 이전 세대 토큰 폐기는 더 이상 권한 근거가 아니므로 제거해도 되지만, 토큰 수명 관리(run 종료 시 폐기)는 유지한다.
