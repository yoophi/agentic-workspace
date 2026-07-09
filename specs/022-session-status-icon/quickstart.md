# Quickstart: Session Status Icon

## Prerequisites

- Install dependencies from the repository root:

```bash
pnpm install
```

## Automated Validation

Run focused tests for the agent-run model and UI:

```bash
pnpm --filter agentic-workbench exec vitest run \
  src/entities/agent-run/model/format.test.ts \
  src/features/agent-run/model/run-panel-state.test.ts \
  src/features/agent-run/ui/agent-run-panel.test.tsx
```

Expected outcomes:
- `session_info_update` events do not create timeline items.
- Active and idle status updates are extracted and reflected in run panel state.
- Metadata-only and unknown status updates do not break timeline rendering.
- Existing non-session-info raw event behavior remains covered.

Run the Agentic Workbench type check before implementation is considered complete:

```bash
pnpm --filter agentic-workbench check-types
```

Run the relevant app test suite when the focused tests pass:

```bash
pnpm --filter agentic-workbench test
```

## Manual Validation

Start the app:

```bash
pnpm --filter agentic-workbench tauri:dev
```

Validate these scenarios in an Agentic Workbench session:

1. Trigger or replay a `session_info_update` with `threadStatus.type = "active"`.
   - Expected: the session shows an active/work-in-progress status.
   - Expected: the timeline does not show the raw JSON payload.

2. Trigger or replay a `session_info_update` with `threadStatus.type = "idle"`.
   - Expected: the session shows an idle/ready status.
   - Expected: any waiting-for-response state is cleared when applicable.
   - Expected: the timeline does not show the raw JSON payload.

3. Trigger or replay a metadata-only `session_info_update` with `title` or `updatedAt`.
   - Expected: no raw JSON timeline item appears.
   - Expected: normal timeline messages before and after the update remain visible.

4. Trigger or replay a different raw event that is not `session_info_update`.
   - Expected: existing raw event behavior is unchanged.

## Completion Evidence

- Focused Vitest tests pass.
- `pnpm --filter agentic-workbench check-types` passes.
- Manual validation confirms 0 visible raw `session_info_update` payloads and distinct active/idle status states.
