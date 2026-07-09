# Contract: Session Status UI

## Purpose

Define the observable UI and state behavior for handling `session_info_update` events in Agentic Workbench.

## Inputs

### Session info update with idle status

```json
{
  "type": "raw",
  "method": "session/update",
  "payload": {
    "sessionUpdate": "session_info_update",
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

**Expected behavior**:
- No timeline item is added.
- Latest agent thread status becomes `idle`.
- The visible status indicator communicates idle/ready.
- Awaiting prompt response state is cleared when applicable.

### Session info update with active status

```json
{
  "type": "raw",
  "method": "session/update",
  "payload": {
    "sessionUpdate": "session_info_update",
    "_meta": {
      "codex": {
        "threadStatus": {
          "type": "active",
          "activeFlags": []
        }
      }
    }
  }
}
```

**Expected behavior**:
- No timeline item is added.
- Latest agent thread status becomes `active`.
- The visible status indicator communicates active/work-in-progress.

### Session info update with metadata only

```json
{
  "type": "raw",
  "method": "session/update",
  "payload": {
    "sessionUpdate": "session_info_update",
    "title": "test",
    "updatedAt": "2026-07-02T11:11:12.255Z"
  }
}
```

**Expected behavior**:
- No timeline item is added.
- Existing visible status is preserved.
- Supported session title/freshness fields may update where the product already displays them.

### Other raw events

Any raw event that is not `session_info_update`.

**Expected behavior**:
- Existing raw event behavior is preserved.
- The event may still appear in the Raw group if that is the current app behavior.

## UI Requirements

- The indicator must be compact enough to sit near the run/session header or agent identity area.
- Active and idle states must be distinguishable by icon, label, tone, or accessible name.
- The indicator must have an accessible text equivalent for active, idle, and unknown/neutral states.
- Repeated status updates must not cause visible timeline growth.

## Non-Goals

- Migrating historical raw metadata timeline items.
- Persisting thread status across app restarts.
- Changing backend session APIs.
- Hiding all raw events.
