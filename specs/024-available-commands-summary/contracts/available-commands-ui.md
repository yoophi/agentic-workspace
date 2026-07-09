# Contract: Available Commands UI

## Input Event Contract

### Valid command update

```json
{
  "type": "raw",
  "method": "session/update",
  "payload": {
    "sessionUpdate": "available_commands_update",
    "availableCommands": [
      {
        "name": "mcp",
        "description": "List configured Model Context Protocol (MCP) tools.",
        "input": null
      },
      {
        "name": "review",
        "description": "Review uncommitted changes, or review with custom instructions.",
        "input": {
          "hint": "optional review instructions"
        }
      },
      {
        "name": "$speckit-implement",
        "description": "Execute the implementation plan by processing and executing all tasks defined in tasks.md",
        "input": null
      }
    ]
  }
}
```

Expected behavior:

- Full raw JSON is not rendered as a timeline item.
- Header/status summary shows `3 commands available` or equivalent.
- Detail view lists `mcp`, `review`, and `$speckit-implement`.
- `review` shows `optional review instructions` as an input hint.

### Wrapped command update

```json
{
  "type": "raw",
  "method": "session/update",
  "payload": {
    "params": {
      "update": {
        "sessionUpdate": "available_commands_update",
        "availableCommands": [
          {
            "name": "status",
            "description": "Display status."
          }
        ]
      }
    }
  }
}
```

Expected behavior:

- Parser detects the wrapped update.
- Summary shows `1 command available` or equivalent.
- Detail view lists `status`.

### Empty command update

```json
{
  "type": "raw",
  "method": "session/update",
  "payload": {
    "sessionUpdate": "available_commands_update",
    "availableCommands": []
  }
}
```

Expected behavior:

- No raw timeline item is rendered.
- Summary shows empty state such as `No commands available`.
- Detail view is disabled or shows an empty-state message.

### Malformed command update

```json
{
  "type": "raw",
  "method": "session/update",
  "payload": {
    "sessionUpdate": "available_commands_update",
    "availableCommands": [
      null,
      {
        "description": "Missing name"
      },
      {
        "name": "valid",
        "input": {
          "hint": "optional text"
        }
      }
    ]
  }
}
```

Expected behavior:

- Invalid command entries are ignored.
- Summary reflects the valid command count.
- Detail view lists `valid` and its input hint.
- UI remains stable.

## UI Rules

- Summary is compact and should not push the primary timeline or prompt controls out of view.
- Detail view must be scrollable or otherwise bounded for long command lists.
- Description and input hint are optional display fields.
- `input` schema is not rendered as raw JSON.
- `$skill`, slash command, and plain command names are all valid names.
- Non-command raw events keep existing raw timeline behavior.

## Non-Goals

- Rendering full command input schema.
- Persisting command metadata across app restarts.
- Migrating historical raw timeline entries.
- Redesigning prompt autocomplete behavior beyond sharing the command metadata source.
