# UI Contract: Session Lifecycle Status

## Purpose

Defines how Agentic Workbench should expose session start and idle lifecycle/status changes without reintroducing raw session update noise.

## Scope

In scope:

- New session start status message.
- Idle transition status message.
- Repeated status update dedupe.
- Coexistence with current header thread status badge.
- Coexistence with available command summary UI.

Out of scope:

- Available command summary/detail implementation.
- New backend session update contract.
- Persistent status history across app restarts.
- Full session inspector UI.

## Status Message Contract

| Event or State | User-Facing Result |
|----------------|--------------------|
| New run/session starts | A concise low-emphasis start/status message is visible. |
| Thread status becomes active | Current header badge may show active; timeline/status message is added only when meaningful and non-duplicative. |
| Thread status transitions from active to idle | A concise low-emphasis idle message is visible. |
| Same thread status repeats | No duplicate user-facing status message is appended for the same run. |
| Status payload is malformed or unknown | UI remains stable and no raw JSON message is shown. |

## Visual Contract

- Status messages must be visually lighter than prompt and assistant messages.
- Status messages must remain readable in the lifecycle/status portion of the run UI.
- Status messages must not duplicate command summary text such as command counts.
- Header current-state badge remains the current status indicator.

## Run Scope Contract

- Status messages belong to one run.
- Events for old run ids must not update the current run status message state.
- Starting a new run resets status dedupe for that run.

## Regression Requirements

- Tests must prove session start and idle status messages can be produced.
- Tests must prove repeated idle updates do not append duplicate messages.
- Tests must prove malformed or unknown session updates do not create raw timeline JSON.
- Tests must prove command summary handling remains separate from lifecycle/status messaging.
- Storybook or rendering tests must show representative status message states.
