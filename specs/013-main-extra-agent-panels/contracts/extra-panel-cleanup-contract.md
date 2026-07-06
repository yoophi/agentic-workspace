# Contract: Extra Panel Close Cleanup

## Scope

This contract defines cleanup behavior when a user closes an extra agent panel. It covers frontend panel state and existing backend run cancellation.

## Idle Extra Close

**Given** an extra panel has no active run, **when** the user closes the tab, **then** the panel is removed immediately, its local prompt/timeline/queue state is discarded, and active tab fallback is applied.

## Running Extra Close

**Given** an extra panel has `isRunning = true` and an `activeRunId`, **when** the user closes the tab, **then** the UI shows confirmation with two choices:

- Cancel close: keep panel and run unchanged.
- Cancel run and close: cancel the run and remove the panel after cleanup.

## Confirmed Close Sequence

1. Mark the slot as `closing` so it no longer accepts routed prompts.
2. Call cancel for the captured `activeRunId`.
3. Treat cancel success, already-finished response, or observed settled lifecycle as valid cleanup completion.
4. Remove the slot after cleanup completion.
5. Select the next available extra panel, otherwise select main.
6. Ensure the closed panel has no actionable permission dialog, queued prompt, event listener, or active run state.

## Idempotency Rules

- Repeated close confirmation for the same panel must not call cancel repeatedly after the slot is already closing.
- If run completion arrives before cancel response, cleanup still removes the slot once.
- If cancel response arrives before lifecycle event, cleanup still removes the slot once.
- Late events for removed panel run ids must be ignored by the UI.
- Unknown or already-terminated run cancellation must still allow UI cleanup.

## Backend Expectations

- Canceling a run removes the run id from active registry state.
- Canceling a run clears permission state for that run.
- Canceling one run does not cancel other main or extra panel runs.
- Window close behavior continues to cancel all active runs owned by the window.

## Acceptance Checks

- Closing a running extra leaves backend active run count unchanged for other panels and decremented for the closed run.
- Pending permission for the closed run is no longer actionable after close.
- A queued prompt on the closed panel never dispatches after close.
- Starting a new extra run after close is not blocked by a stale closed run id.
- No user-visible error appears when close and run completion happen nearly simultaneously.
