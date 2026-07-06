# Quickstart: Main and Extra Agent Run Panels Validation

## Prerequisites

- Install dependencies with `pnpm install` if needed.
- Use branch `013-main-extra-agent-panels`.
- Feature pointer should be `specs/013-main-extra-agent-panels` in `.specify/feature.json`.

## Static Validation

```bash
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/agentic-workbench test
```

Expected outcome:

- TypeScript passes.
- Frontend model tests cover slot creation, active fallback, prompt routing, close confirmation, idempotent cleanup, main-only goal continuation flag, and extra settings persistence disabled.
- UI tests cover tab rendering, running badge, annotation target feedback, and close confirmation.

## Backend Validation

Run from the Tauri app directory if backend code changed:

```bash
cd apps/agentic-workbench/src-tauri
cargo test
```

Expected outcome:

- Multiple distinct run ids can coexist.
- Canceling one run does not cancel another run.
- Cancel clears run owner and permission state.
- Canceling an unknown or already-settled run still produces a settled result usable by frontend cleanup.

## Storybook Validation

```bash
pnpm --filter @yoophi/agentic-workbench build-storybook
```

Expected story states:

- Main-only agent area.
- Main plus one idle extra.
- Multiple extras with one running tab.
- Running extra close confirmation.
- Multiple running panels conflict warning.
- Annotation prompt target feedback.

## Manual End-to-End Scenario

1. Start the workbench app:

   ```bash
   pnpm tauri:dev:workbench
   ```

2. Open a Worktree Session.
3. Confirm `Main` is the first tab and has no close button.
4. Add `Extra 1`; confirm it becomes active.
5. Add `Extra 2`; switch between all tabs and confirm prompt/timeline state is independent.
6. Select `Extra 1`, send an annotation prompt from the workspace, and confirm the target feedback names `Extra 1`.
7. Start a run in `Extra 1`; while it is running, send another prompt and confirm it queues in `Extra 1`.
8. Start a run in `Main`; confirm both tabs show running state and the worktree conflict warning is visible.
9. Try closing running `Extra 1`; choose cancel close and confirm the run continues.
10. Try closing running `Extra 1` again; choose cancel run and close. Confirm the tab is removed, the run stops, pending permission state disappears, and late events do not re-open or mutate the closed panel.
11. Start a new extra run and confirm stale closed run state does not block it.

## Regression Checks

- Existing single-panel prompt workflow remains valid when no extra panel exists.
- Main goal continuation still works only in main.
- Extra panels do not persist worktree settings.
- Closing the whole Worktree Session still cancels active runs owned by that session.
