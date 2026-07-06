# Research: Main and Extra Agent Run Panels

## R1. Panel orchestration location

**Decision**: Add `WorktreeAgentRunArea` under `features/agent-run/ui` and keep `ProjectWorktreeSessionPage` responsible only for the outer Worktree Session layout and workspace handoff.

**Rationale**: The new behavior is a user action/business interaction around agent panels: adding extra panels, switching active panel, routing annotation prompts, closing panels, and tracking run state. Keeping this in `features/agent-run` follows the FSD boundary while preventing the page from becoming the owner of detailed agent behavior.

**Alternatives considered**:

- Keep all state in `ProjectWorktreeSessionPage`: rejected because page-level UI would accumulate prompt routing, close confirmation, and panel lifecycle logic.
- Move to `app` routing state: rejected because the panel list is not global application routing state and is scoped to a single Worktree Session instance.

## R2. Panel state model

**Decision**: Use an app-local pure model for `AgentRunPanelSlot` and state transitions: create extra, select panel, update run state, request close, confirm close, cancel close, and remove closed panel.

**Rationale**: Slot transitions are easy to test without rendering or starting real agent runs. This also makes active-tab fallback and idempotent close behavior explicit.

**Alternatives considered**:

- Store slot state only in React component state: rejected because close/race behavior is high-risk and should have deterministic unit tests.
- Persist slots immediately: rejected because MVP explicitly excludes layout/session restore.

## R3. Main-only goal continuation

**Decision**: Add an `enableGoalContinuation` capability flag to `AgentRunPanel`, defaulting to `false`, and pass `true` only for the main panel.

**Rationale**: Goal state is worktree-level. Multiple panels observing and automatically continuing the same goal can duplicate work and token accounting. The flag makes the policy explicit and testable.

**Alternatives considered**:

- Let every panel keep current goal behavior: rejected because it can trigger duplicated continuation runs.
- Move goal UI completely out of `AgentRunPanel`: deferred because it is a larger refactor than needed for MVP.

## R4. Settings persistence policy

**Decision**: Add a `persistSettings` capability flag to `AgentRunPanel`, defaulting to `true` for backward compatibility, and pass `false` for extra panels. Extra panels keep settings panel-local for their lifetime.

**Rationale**: Existing settings are keyed by working directory. Multiple panels writing the same key would make the last extra panel change overwrite main defaults unexpectedly.

**Alternatives considered**:

- Add `workingDirectory + panelProfileId` persistence now: rejected as out of MVP scope and requiring a settings migration/design.
- Make extra panels read-only with no configurable settings: rejected because manual review/experiment panels still need temporary agent/profile/permission choices.

## R5. Annotation prompt routing

**Decision**: Route workspace annotation prompts to the currently active agent panel and show short target-panel feedback.

**Rationale**: This matches the spec assumption and lets the user intentionally direct workspace prompts by selecting the tab first. Queue behavior remains panel-local.

**Alternatives considered**:

- Always route to main: rejected because it makes extra panels less useful for review/experiment workflows.
- Add a persistent default-target setting now: deferred because it is a follow-up policy feature.

## R6. Extra close cleanup

**Decision**: Parent `WorktreeAgentRunArea` tracks each panel's `isRunning` and `activeRunId`. Closing an active extra panel opens confirmation. Confirmed close calls cancel for the active run, marks the slot closing, removes the panel after cancellation is acknowledged or known settled, clears panel-local queue/permission state by unmounting, and ignores late events because listeners are panel-scoped and filtered by active run.

**Rationale**: Existing backend `cancel_agent_run` already removes registry entries and clears permission state when a run is canceled. The frontend still needs an idempotent slot lifecycle so close, cancel, run completion, and late events cannot leave orphan UI state.

**Alternatives considered**:

- Put close button inside `AgentRunPanel`: rejected for MVP because the parent owns tabs and active fallback.
- Remove active extra panel immediately without cancel: rejected because the backend run could continue without visible ownership.

## R7. Backend cleanup surface

**Decision**: Use existing `cancelAgentRun(runId)` and backend registry cleanup unless tests reveal gaps. Add or preserve tests proving cancel removes only the target run, clears permission state, and unknown/already-finished runs produce a settled lifecycle response.

**Rationale**: Current `AppState` supports multiple distinct run ids and `cancel_run` removes run owner and permission state. The planning risk is mostly race handling and user-visible cleanup, not a new backend abstraction.

**Alternatives considered**:

- Add a dedicated `close_agent_panel` Tauri command: rejected because panels are frontend UI concepts and backend already operates on run ids.
- Add panel ids to backend runs: deferred until persistent panel profiles or panel-specific backend ownership is required.

## R8. Resizable panel identity

**Decision**: Add `panelId` to `AgentRunPanel` and use it in any internal resizable panel ids.

**Rationale**: Multiple `AgentRunPanel` instances in one React tree cannot share fixed resizable ids without collision risk.

**Alternatives considered**:

- Render only one `AgentRunPanel` instance and swap state manually: rejected because each panel must keep independent prompt, timeline, queue, and permission state.

## R9. Storybook and validation

**Decision**: Add Storybook coverage for the agent panel tab/area shell with main-only, multiple extras, running badges, and close confirmation states. Use model tests for lifecycle and UI tests where command calls can be mocked.

**Rationale**: The tabs and close states are reusable UI within the app and carry meaningful interaction risk. Storybook satisfies the constitution requirement for reusable UI components.

**Alternatives considered**:

- Rely only on manual Tauri validation: rejected because close/routing regressions should be caught without starting real agent processes.
