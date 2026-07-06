# Contract: Agent Panel UI and Prompt Routing

## Scope

This contract defines the user-visible and component-level behavior for the Worktree Session agent area. It is not a public network API.

## WorktreeAgentRunArea Inputs

- `workingDirectory`: Current worktree path.
- `worktreeBranch`: Current branch label or fallback display value.
- `worktreeStatus`: Current worktree status for the header.
- `onOpenSettings`: Optional callback for app settings.
- `onSendAnnotationPrompt`: Existing workspace callback must be adapted so prompts are routed to active panel.

## Required UI Behavior

1. The tab list always starts with a non-closable `Main` tab.
2. The add-extra action creates a new `Extra N` tab, activates it, and mounts an independent `AgentRunPanel`.
3. Extra tabs expose a close action. Main does not.
4. Running panels expose a visible running state on their tab or equivalent selector.
5. Multiple running panels in the same worktree show a conflict-risk warning.
6. The visible `AgentRunPanel` is the active tab's panel; inactive panels preserve local state while mounted.
7. Annotation prompts from the workspace route to the active panel and produce short target feedback.
8. If the active panel is running, routed prompt follows that panel's existing queue behavior.

## AgentRunPanel Required Props

- `panelId`: Stable unique id used for run-state reporting and internal layout ids.
- `workingDirectory`: Current worktree path.
- `scrollHeader`: Optional header content.
- `onRunSettled`: Existing callback when a run ends.
- `onRunStateChange`: Callback with `{ isRunning, activeRunId }`.
- `initialInputMode`: Existing input mode behavior.
- `externalPromptRequest`: Prompt request targeted to this panel.
- `enableGoalContinuation`: `true` only for main panel.
- `persistSettings`: `true` only for main panel in MVP.
- `onOpenSettings`: Optional settings callback.

## Prompt Routing Rules

1. Trim prompt text before routing.
2. Empty text is ignored with no new prompt request.
3. Generate a new request id for each routed annotation prompt.
4. Set the request only on the target slot.
5. Do not route to a closing or removed slot.
6. Display target feedback containing the panel title.

## Acceptance Checks

- Creating two extras produces `Main`, `Extra 1`, `Extra 2` with unique panel ids.
- Sending an annotation prompt while `Extra 1` is active only updates `Extra 1`.
- Sending an annotation prompt while active panel is running queues the prompt in that panel.
- Inactive panels do not display events from another panel's active run.
- Extra panel settings changes do not call persisted settings save for the worktree key.
- Extra panels do not start goal continuation when the worktree goal qualifies for continuation.
