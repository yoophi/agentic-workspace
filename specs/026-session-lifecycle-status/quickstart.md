# Quickstart: Session Lifecycle Status

## Prerequisites

- From repository root: `/Users/yoophi/project/agentic-workspace`
- Node dependencies installed with pnpm.
- Current feature directory: `specs/026-session-lifecycle-status`

## Automated Validation

Run focused model and UI tests:

```sh
pnpm --filter agentic-workbench exec vitest run \
  src/entities/agent-run/model/format.test.ts \
  src/features/agent-run/model/run-panel-state.test.ts \
  src/features/agent-run/ui/agent-run-panel.test.tsx
```

Run type checks:

```sh
pnpm --filter agentic-workbench check-types
```

Run full Agentic Workbench tests before completion:

```sh
pnpm --filter agentic-workbench test
```

## Manual Validation

1. Start Agentic Workbench.
2. Start a new agent run.
3. Confirm a concise session start/status message is visible within 1 second.
4. Confirm the message is visually lower-emphasis than prompt and assistant messages.
5. Wait for or simulate a transition from active to idle.
6. Confirm a concise idle status message is visible within 1 second.
7. Trigger repeated identical idle updates or observe repeated updates.
8. Confirm identical idle messages do not keep accumulating.
9. Confirm the header thread status badge still shows current active/idle state.
10. Confirm command summary/detail UI remains separate and is not duplicated by lifecycle messages.
11. Confirm malformed or unknown session status data does not appear as raw JSON in the timeline.

## Expected Outcomes

- Users can identify session start and idle transition without reading raw JSON.
- Repeated status updates do not pollute the timeline.
- Header status badge and lifecycle/status messages have distinct roles.
- Existing command summary behavior remains unchanged.
- No backend, persistence, or shared package changes are required.
