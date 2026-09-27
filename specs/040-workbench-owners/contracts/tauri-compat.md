# Contract: Tauri 호환 어댑터 (040)

화면(`apps/agentic-workbench/src`)은 바뀌지 않는다. command 이름·인자·반환 타입·오류 문자열이 오늘과 같다.

## command → operation

| command | operation | 작업대 확보 | 반환 변환 |
|---|---|---|---|
| `list_agent_tool_command_candidates(query)` | `run.listToolCandidates` | `ensure(label, query.workingDirectory)` | 그대로 |
| `start_agent_run(request, panelId?)` | `run.start` | `ensure(label, request.cwd)` | `AgentRun` |
| `send_prompt_to_run(runId, prompt)` | `run.sendPrompt` | 조회만(없으면 `"agent run is not active"`) | `()` |
| `steer_prompt_to_run` · `cancel_current_prompt_and_send_to_run` · `set_run_permission_mode` | `run.steer` · `run.cancelAndSend` · `run.setPermissionMode` | 조회만 | `()` |
| `cancel_agent_run(runId)` | `run.cancel` | 조회만(없으면 `Ok(())` — 오늘 항상 성공) | `()` |
| `respond_agent_permission(runId, permissionId, optionId)` | `run.respondPermission` | 조회만(없으면 `"unknown or finished run: …"`) | `()` |
| `sync_agent_workspace(request)` | `exchange.syncWorkspace` | `ensure(label, request.worktreePath)` | 그대로 |
| `send_agent_exchange(request)` | `exchange.send` | 조회만(없으면 `unknownWorkspace` JSON) | `AgentExchange` |
| `acknowledge_agent_exchange(request)` | `exchange.acknowledge` | 조회만 | `AgentExchange` |
| `list_agent_exchanges()` | `exchange.list` | 조회만(없으면 `[]`) | `[AgentExchange]` |

- run command의 오류: fault `message` 그대로(037 규칙).
- 교환 command의 오류: `serde_json::to_string(&{"code": details.exchangeCode, "message": message})` — 오늘 `exchange_error`와 같은 문자열.
- "조회만"은 창에 작업대가 없을 때 새로 열지 않는다는 뜻이다(작업대가 없으면 그 창이 소유한 run·교환도 없다).

## 창 수명

| 이벤트 | 오늘 | 040 |
|---|---|---|
| 세션 창 `Destroyed` | `cancel_runs_owned_by(label)` → `remove_window(label)` → orchestration `release_window(label)` | `DesktopBenches::close(label)` → `bench.close`(run 취소·교환 삭제) → orchestration `release_window(label)`(041까지 그대로) |
| 작업대 열기 | — | 창이 처음 작업대가 필요한 command를 부를 때(창 생성 코드 불변) |

## 이벤트 전달 (데스크톱)

`TauriDesktopBridge`가 작업대 → 창을 찾아 **창 삽입 경로 하나로만** 보낸다. 네이티브 `emit`은 제거한다.

| 종류 | CustomEvent 이름 | payload |
|---|---|---|
| run | `agent-run-event-fallback` | 039와 같음(`{runId, event, sequence, epoch, streamId, eventId}`) |
| 교환 요청 | `agent-exchange-requested-fallback` | `AgentExchangeRequestedEvent`(오늘 형태) + `sequence`·`epoch`·`streamId`·`eventId` |
| 교환 상태 | `agent-exchange-status-fallback` | `AgentExchange`(`windowLabel` 없음) + 같은 추가 필드 |
| 창 제목 | `mcp-window-title-fallback` | `{title}` — async task에서 `set_title` + 메뉴 동기화 뒤 dispatch |

화면 수신 코드(`listenWithFallback`, `App.tsx`의 제목 리스너)는 삽입 경로를 이미 듣는다. 네이티브 리스너는 남아 있어도 더 이상 이벤트를 받지 않는다.

## MCP 도구 (AW MCP 서버)

| 도구 | operation | 결과 변환 |
|---|---|---|
| `list_peer_agents{runId}` | `exchange.listPeers` | `{peers}` |
| `send_message_to_agent{runId, …}` | `exchange.sendFromRun` | `AgentExchange` JSON |
| `get_agent_exchange_status{runId, requestId}` | `exchange.getForRun` | `AgentExchange` JSON |
| `set_window_title{runId, title}` | `bench.requestTitle` | `TitleChangeResult{ok, appliedTitle}` / 실패 `{ok: false, reason, code}` |

principal: `AuthenticatedPrincipal::agent(capability.run_id)`. 도구 쪽 `runId == principal.run_id` 검사와 문구 유지. 교환 오류는 `structuredContent`에 `{code, message}`(오늘 형태). orchestration 도구 16개는 변경 없음(041).
