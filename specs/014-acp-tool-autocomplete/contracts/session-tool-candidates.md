# Contract: Session Tool/Command Candidates

## Purpose

Define the candidate source contract used by prompt autocomplete. This contract is intentionally independent of a specific transport so implementation can use existing frontend state, a Tauri command, or future session metadata without changing UI behavior.

## Candidate Query Input

```json
{
  "runId": "optional-current-run-id",
  "agentId": "selected-agent-id",
  "workingDirectory": "absolute-worktree-path",
  "sessionMode": "new-or-reuse"
}
```

## Candidate Query Output

```json
{
  "status": "ready",
  "candidates": [
    {
      "id": "session:set_window_title",
      "name": "set_window_title",
      "description": "Change the current Worktree Session window title.",
      "insertText": "$set_window_title",
      "source": "sessionTool",
      "scope": {
        "runId": "current-run-id",
        "agentId": "codex",
        "workingDirectory": "/path/to/worktree"
      }
    }
  ]
}
```

## Status Values

- `loading`: candidate source is not ready yet.
- `ready`: candidates are available and may be filtered by the UI.
- `empty`: source is ready but no candidates exist.
- `error`: source lookup failed; prompt typing must remain available.

## Rules

- Candidates must be scoped to the current prompt composer session/run context when a run exists.
- Candidates from another run, another worktree, or another owner window must not be shown.
- Candidate lookup must not start an agent run.
- Candidate lookup must not call tools.
- Candidate lookup must not persist candidate data.
- Candidate lookup failure must degrade to normal prompt editing.
- `insertText` must be a text token only; execution happens only after user submits a prompt through the normal prompt flow.

## Backend Boundary Requirements

If implemented through Tauri:

- The inbound command validates input shape and delegates to application service.
- The application service validates run/session owner scope.
- The infrastructure adapter reads available ACP/session metadata without exposing raw protocol objects directly to the UI.
- Rust tests cover owner scope mismatch, missing session, empty candidate source, and successful candidate normalization.
