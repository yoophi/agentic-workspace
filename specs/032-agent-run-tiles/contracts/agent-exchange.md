# Interface Contract: Agent Run Exchange

## Frontend → Tauri commands

명령 이름은 snake_case Tauri command, frontend wrapper는 camelCase를 사용한다.

### `sync_agent_workspace`

Request:

```json
{
  "worktreePath": "/canonical/worktree",
  "revision": 3,
  "focusedPanelId": "extra-agent-run-1",
  "panels": [
    {
      "panelId": "main-agent-run",
      "title": "Main",
      "runId": "run-main",
      "status": "running"
    },
    {
      "panelId": "extra-agent-run-1",
      "title": "Reviewer",
      "runId": null,
      "status": "idle"
    }
  ]
}
```

Response:

```json
{
  "revision": 3,
  "acceptedPanels": 2
}
```

Rules:

- `windowLabel`은 invoking window에서 주입한다.
- stale revision은 기존 snapshot을 덮지 않고 현재 accepted revision을 반환한다.
- active `runId` owner가 invoking window와 다르면 해당 snapshot 전체를 거부한다.
- worktree path는 canonicalize 및 session worktree scope 검증을 거친다.

### `send_agent_exchange`

Request:

```json
{
  "requestId": "uuid",
  "sourcePanelId": "main-agent-run",
  "sourceRunId": "run-main",
  "targetPanelId": "extra-agent-run-1",
  "targetRunId": null,
  "message": "Review the current diff and report risks.",
  "delivery": "draft"
}
```

Response is an `AgentExchange` summary. User command actor는 invoking window로 인증한다. Agent actor는 MCP adapter만 생성한다.

### `acknowledge_agent_exchange`

Request:

```json
{
  "requestId": "uuid",
  "targetPanelId": "extra-agent-run-1",
  "outcome": "delivered",
  "reason": null
}
```

`outcome`은 `delivered | rejected | failed | cancelled`다. Application service는 current target snapshot과 terminal-state invariant를 검증한다.

### `list_agent_exchanges`

현재 invoking window의 최근 exchange summary를 반환한다. 다른 창의 기록은 반환하지 않는다.

## Backend → frontend events

### `agent-exchange-requested`

Payload:

```json
{
  "requestId": "uuid",
  "source": {
    "panelId": "main-agent-run",
    "runId": "run-main",
    "title": "Main"
  },
  "target": {
    "panelId": "extra-agent-run-1",
    "runId": null,
    "title": "Reviewer"
  },
  "message": "Review the current diff and report risks.",
  "delivery": "draft",
  "createdAt": "2026-07-27T12:00:00Z"
}
```

Frontend handler:

1. request ID가 이미 처리됐으면 prompt를 다시 적용하지 않고 기존 outcome을 재-ack한다.
2. target panel ID와 current run ID를 확인한다.
3. `externalPromptRequest`에 같은 request ID, text, delivery를 설정한다.
4. panel이 request를 소비하거나 거부한 결과를 acknowledge한다.

### `agent-exchange-status`

모든 상태 변경을 source/target 소유 창 하나에 emit한다. Payload는 request ID, status, source/target refs, failure code/reason, updatedAt을 포함한다.

## MCP tools

모든 tool은 기존 bearer token/origin 검증을 통과해야 한다.

### `list_peer_agents`

Arguments:

```json
{
  "runId": "run-main"
}
```

Result:

```json
{
  "peers": [
    {
      "panelId": "extra-agent-run-1",
      "title": "Reviewer",
      "runId": "run-review",
      "status": "running"
    }
  ]
}
```

Rules:

- source run의 active owner window를 확인한다.
- source와 closing endpoint를 제외한다.
- 같은 window snapshot의 endpoint만 반환한다.

### `send_message_to_agent`

Arguments:

```json
{
  "runId": "run-main",
  "requestId": "uuid",
  "targetPanelId": "extra-agent-run-1",
  "targetRunId": "run-review",
  "message": "Inspect the failing tests.",
  "delivery": "queue"
}
```

Result:

```json
{
  "requestId": "uuid",
  "status": "accepted",
  "targetPanelId": "extra-agent-run-1"
}
```

Rules:

- `runId`는 agent source이자 owner 검증 기준이다.
- target의 current run ID가 전달된 값과 다르면 `stale_target_run`이다.
- 동일 request ID/same payload는 기존 exchange를 반환하고 재전달하지 않는다.
- 동일 request ID/different payload는 `duplicate_conflict`다.

### `get_agent_exchange_status`

Arguments:

```json
{
  "runId": "run-main",
  "requestId": "uuid"
}
```

source run이 소유한 exchange만 조회한다.

## Limits and timeout

- message: trim 후 1..16,384 UTF-8 bytes
- panel snapshot: 최대 8 endpoints
- accepted exchange acknowledgement: 30초 후 `ack_timeout`
- retained exchange summaries: window당 최근 500개
- window destroy: pending/accepted는 `cancelled`, snapshot과 retained data는 제거

## Error mapping

Tauri command는 typed error code와 user-readable message를 반환한다. MCP는 `isError: true` tool result 안에 동일 code/message를 넣고 JSON-RPC transport error로 ownership/domain failure를 숨기지 않는다.
