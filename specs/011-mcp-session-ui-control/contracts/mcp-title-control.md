# Contract: MCP Session Title Control

**Feature**: 011-mcp-session-ui-control | **Date**: 2026-07-06

## Scope

이 계약은 AW가 agent에게 노출하는 초기 MCP HTTP service의 user-facing capability를 정의한다. 초기 구현은 Worktree Session window title 변경만 지원한다.

Out of scope:

- unstaged change display
- source file display
- Markdown preview control
- workspace refresh
- file read/write/stage/commit
- permission approval

## Transport

| Item | Contract |
|------|----------|
| Endpoint | `http://127.0.0.1:{port}/mcp` |
| Binding | localhost only |
| Request format | MCP-compatible JSON-RPC over HTTP POST |
| Response format | JSON-RPC response with MCP tool result content |
| Authentication | Required. Agent receives a runtime token during run launch. |
| Origin protection | Requests with invalid browser `Origin` are rejected. |

The implementation targets the stable MCP Streamable HTTP behavior for request/response JSON. SSE streaming is not required for title changes.

## Agent Runtime Inputs

The app provides the active agent run with runtime connection information.

| Name | Description |
|------|-------------|
| `AW_MCP_URL` | MCP endpoint URL for the local app instance |
| `AW_MCP_TOKEN` | Runtime secret required for MCP requests |
| `AW_MCP_RUN_ID` | Agent run id that owns the Worktree Session window |

These env key names are final for this implementation and are app-reserved.

## tools/list

The MCP service MUST expose exactly one tool for this feature.

```json
{
  "tools": [
    {
      "name": "set_window_title",
      "description": "Change the current Worktree Session window title for the active agent run.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "runId": {
            "type": "string",
            "description": "The active agent run id provided by AW_MCP_RUN_ID."
          },
          "title": {
            "type": "string",
            "description": "Readable window title to apply to the owning Worktree Session window."
          }
        },
        "required": ["runId", "title"],
        "additionalProperties": false
      }
    }
  ]
}
```

No other tool names may be listed in the initial implementation.

## tools/call: set_window_title

### Request arguments

```json
{
  "runId": "9c1822b7-6f0e-4f3f-90e2-1d44e52b2c42",
  "title": "Fix login retry state"
}
```

### Success result

```json
{
  "ok": true,
  "appliedTitle": "Fix login retry state"
}
```

Expected user-visible effect:

- The Worktree Session window that owns `runId` changes title to `appliedTitle`.
- Other Worktree Session windows remain unchanged.
- Agent timeline, permission state, workspace tab, selected file, and prompt input remain unchanged.

### Failure result

```json
{
  "ok": false,
  "code": "invalidTitle",
  "reason": "Window title must be non-empty and at most 80 characters."
}
```

Required failure codes:

| Code | Meaning |
|------|---------|
| `unauthorized` | Token/header validation failed |
| `unknownRun` | `runId` is not known to the app |
| `inactiveRun` | run exists no longer or cannot accept control requests |
| `invalidTitle` | title is blank, unreadable, or violates length/readability rules |
| `windowUnavailable` | owner window is no longer available |
| `unsupportedTool` | caller requested any tool other than `set_window_title` |
| `internalError` | unexpected failure |

## Security Rules

- The server MUST bind to localhost, not all interfaces.
- The server MUST require the runtime token for every request after initialization.
- The server MUST reject browser-origin requests whose `Origin` is not trusted for this app.
- The server MUST derive the target window from `runId` ownership state, never from caller-provided window labels.
- The server MUST NOT include prompt text, file contents, or token values in diagnostic logs.

## Compatibility Notes

- The initial contract does not require server-to-client MCP notifications or streaming responses.
- The available capability list is a testable boundary: adding any user-facing tool beyond `set_window_title` requires a new spec update.
