# Contract: Session Info Metadata UI

## Input Event Contract

### Typed title update

```json
{
  "type": "sessionInfo",
  "title": "Fix session metadata",
  "updatedAt": null,
  "threadStatus": null
}
```

Expected behavior:

- AW standalone worktree window title becomes `Fix session metadata`.
- No raw JSON or `session info` timeline row is added.
- Existing active/idle status indicator remains unchanged.

### Typed freshness update

```json
{
  "type": "sessionInfo",
  "title": null,
  "updatedAt": "2026-07-09T03:20:00.000Z",
  "threadStatus": null
}
```

Expected behavior:

- Window title remains unchanged.
- Run/session header shows a readable freshness/update time label.
- No timeline row is added.

### Combined metadata and status update

```json
{
  "type": "sessionInfo",
  "title": "Review PR feedback",
  "updatedAt": "2026-07-09T03:25:00.000Z",
  "threadStatus": {
    "type": "active",
    "activeFlags": ["thinking"]
  }
}
```

Expected behavior:

- Window title becomes `Review PR feedback`.
- Header freshness label reflects `updatedAt`.
- Agent status indicator shows active.
- No timeline row is added.

### Raw fallback update

```json
{
  "type": "raw",
  "method": "session/update",
  "payload": {
    "sessionUpdate": "session_info_update",
    "title": "Fallback parsed title",
    "updatedAt": "2026-07-09T03:30:00.000Z",
    "_meta": {
      "codex": {
        "threadStatus": {
          "type": "idle"
        }
      }
    }
  }
}
```

Expected behavior:

- Fallback parser treats the payload as session metadata.
- Same UI behavior as typed `sessionInfo`.
- Raw payload is suppressed from the timeline.

## UI Rules

- `title` is the primary value for AW window title.
- Empty, whitespace-only, control-character, or over-limit titles are ignored.
- `updatedAt` is not included in the window title.
- Invalid `updatedAt` is not displayed and must not throw.
- Missing `threadStatus` must not clear the current active/idle indicator.
- Project/worktree route changes reset the live agent title to the default worktree title.

## Non-Goals

- Active/idle prefix or suffix in the window title. That remains #113 scope.
- Historical raw timeline migration.
- Persisting live metadata to session storage.
- Cross-app shared UI extraction.
