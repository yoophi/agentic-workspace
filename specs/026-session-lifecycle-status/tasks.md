# Tasks: Session Lifecycle Status

**Input**: Design documents from `specs/026-session-lifecycle-status/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/session-lifecycle-status.md`, `quickstart.md`

**Tests**: Required for pure formatter/transition logic, run-panel state reducer behavior, raw suppression regressions, and visible UI/Storybook status states.

**Organization**: Tasks are grouped by user story so each story can be implemented and tested independently.

## Phase 1: Setup (Shared Context)

**Purpose**: Establish the existing code paths and fixtures before implementation.

- [X] T001 Review current session event parsing and lifecycle item formatting in `apps/agentic-workbench/src/entities/agent-run/model/format.ts`
- [X] T002 [P] Review existing formatter regression tests in `apps/agentic-workbench/src/entities/agent-run/model/format.test.ts`
- [X] T003 [P] Review current run-panel state handling for `session_info_update` in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T004 [P] Review run-panel state fixtures and assertions in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T005 [P] Review live event handling, header status badge, and lifecycle rendering in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T006 [P] Review agent run UI regression coverage in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`
- [X] T007 [P] Review existing organism-level Storybook patterns in `apps/agentic-workbench/src/stories/organisms.stories.tsx`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Define the shared status-message contract before any user story implementation.

**CRITICAL**: No user story work should begin until these tasks are complete.

- [X] T008 Add failing formatter tests for lifecycle status message labels, tones, and dedupe keys in `apps/agentic-workbench/src/entities/agent-run/model/format.test.ts`
- [X] T009 [P] Add failing run-panel state tests for run-local lifecycle status scope reset in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T010 [P] Add failing UI/source regression tests proving `session_info_update` raw JSON remains hidden in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`
- [X] T011 Add shared lifecycle status message types and helper exports in `apps/agentic-workbench/src/entities/agent-run/model/types.ts`
- [X] T012 Implement pure lifecycle status message formatter and dedupe-key helpers in `apps/agentic-workbench/src/entities/agent-run/model/format.ts`
- [X] T013 Export lifecycle status helpers from `apps/agentic-workbench/src/entities/agent-run/model/index.ts`

**Checkpoint**: Foundation ready - user story implementation can now proceed.

---

## Phase 3: User Story 1 - Show Session Start Status (Priority: P1) MVP

**Goal**: A user can immediately see a concise, low-emphasis status message when a new agent session starts.

**Independent Test**: Start a new run and verify that a short session-start message is visible without duplicating command summary content.

### Tests for User Story 1

- [X] T014 [P] [US1] Add formatter tests for deriving a session-start lifecycle message in `apps/agentic-workbench/src/entities/agent-run/model/format.test.ts`
- [X] T015 [P] [US1] Add run-panel state tests for appending exactly one start status message per new run in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T016 [P] [US1] Add UI regression test for low-emphasis session-start rendering in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`

### Implementation for User Story 1

- [X] T017 [US1] Map new run/session-start signals to a lifecycle status message in `apps/agentic-workbench/src/entities/agent-run/model/format.ts`
- [X] T018 [US1] Append the session-start status message in reducer-based event handling in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T019 [US1] Append the session-start status message in live panel event handling in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T020 [US1] Ensure lifecycle status rendering is visually lower-emphasis than prompt and assistant messages in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T021 [US1] Add a Storybook state showing a run with a session-start lifecycle message in `apps/agentic-workbench/src/stories/organisms.stories.tsx`

**Checkpoint**: User Story 1 is independently functional and testable as the MVP.

---

## Phase 4: User Story 2 - Show Idle Transition Status (Priority: P2)

**Goal**: A user can see when an active agent session enters idle state and can distinguish the timeline message from the header current-state badge.

**Independent Test**: Simulate an active-to-idle session update and verify that one concise idle status message appears while the header badge remains the current-state indicator.

### Tests for User Story 2

- [X] T022 [P] [US2] Add formatter tests for active-to-idle transition message generation in `apps/agentic-workbench/src/entities/agent-run/model/format.test.ts`
- [X] T023 [P] [US2] Add run-panel state tests for idle transition message append and awaiting-prompt reset in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T024 [P] [US2] Add UI regression test for idle status message and header badge coexistence in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`

### Implementation for User Story 2

- [X] T025 [US2] Implement active-to-idle transition detection in `apps/agentic-workbench/src/entities/agent-run/model/format.ts`
- [X] T026 [US2] Integrate idle transition message handling in reducer-based event handling in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T027 [US2] Integrate idle transition message handling in live panel event handling in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T028 [US2] Keep `AgentThreadStatusBadge` as the current-state indicator while showing timeline status messages in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T029 [US2] Add a Storybook state showing active-to-idle status transition output in `apps/agentic-workbench/src/stories/organisms.stories.tsx`

**Checkpoint**: User Stories 1 and 2 both work independently.

---

## Phase 5: User Story 3 - Dedupe Repeated Status Updates (Priority: P3)

**Goal**: Repeated identical session status updates do not pollute the timeline, while meaningful transitions still appear.

**Independent Test**: Send the same idle update repeatedly and verify that no duplicate idle messages are appended; send a real transition and verify it still appears.

### Tests for User Story 3

- [X] T030 [P] [US3] Add formatter tests for repeated status dedupe and malformed status suppression in `apps/agentic-workbench/src/entities/agent-run/model/format.test.ts`
- [X] T031 [P] [US3] Add run-panel state tests for repeated idle update dedupe and new-run scope reset in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T032 [P] [US3] Add UI regression test proving malformed or unknown status payloads do not render raw JSON in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`
- [X] T033 [P] [US3] Add UI/source regression test proving available command summary remains separate from lifecycle messages in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`

### Implementation for User Story 3

- [X] T034 [US3] Implement run-local displayed status key tracking in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T035 [US3] Apply repeated status dedupe to live panel event handling in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T036 [US3] Ensure malformed, missing, or unknown `threadStatus` values produce no user-facing raw payload in `apps/agentic-workbench/src/entities/agent-run/model/format.ts`
- [X] T037 [US3] Preserve available command summary and detail rendering paths while adding lifecycle status messages in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T038 [US3] Add a Storybook state showing repeated status updates collapsed to a single lifecycle message in `apps/agentic-workbench/src/stories/organisms.stories.tsx`

**Checkpoint**: All user stories are independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final verification, cleanup, and completion checks across all stories.

- [X] T039 [P] Run focused Vitest command from `specs/026-session-lifecycle-status/quickstart.md`
- [X] T040 [P] Run Agentic Workbench type checks from `specs/026-session-lifecycle-status/quickstart.md`
- [X] T041 [P] Run full Agentic Workbench test suite from `specs/026-session-lifecycle-status/quickstart.md`
- [X] T042 Verify Storybook status states cover start, idle, and dedupe examples in `apps/agentic-workbench/src/stories/organisms.stories.tsx`
- [X] T043 Verify no backend, persistence, shared package, or app-to-app import changes were introduced in `apps/agentic-workbench/src`
- [X] T044 Run manual validation checklist from `specs/026-session-lifecycle-status/quickstart.md`
- [X] T045 Update completed task checkboxes in `specs/026-session-lifecycle-status/tasks.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 Setup**: No dependencies.
- **Phase 2 Foundational**: Depends on Phase 1 and blocks all user stories.
- **Phase 3 US1**: Depends on Phase 2. This is the MVP.
- **Phase 4 US2**: Depends on Phase 2 and can be implemented independently, but should preserve US1 behavior.
- **Phase 5 US3**: Depends on Phase 2 and should validate behavior across US1/US2 transitions.
- **Phase 6 Polish**: Depends on all selected user stories being complete.

### User Story Dependencies

- **US1 (P1)**: Can start after foundational helpers and tests are in place.
- **US2 (P2)**: Can start after foundational helpers and tests are in place.
- **US3 (P3)**: Can start after foundational helpers and tests are in place; validates dedupe across repeated updates and new run scope.

### Within Each User Story

- Write the story-specific tests first and confirm they fail for the missing behavior.
- Implement pure formatter/transition logic before reducer or UI integration.
- Integrate reducer-based tests before live UI event handling when possible.
- Add or update Storybook state after UI rendering exists.

---

## Parallel Opportunities

- Setup review tasks T002-T007 can run in parallel after T001.
- Foundational tests T009-T010 can run in parallel with T008 because they touch different test files.
- Story-specific tests in each user story are marked `[P]` and can be written in parallel.
- US1, US2, and US3 can start in parallel after Phase 2 if different contributors own distinct files and coordinate changes to `format.ts`, `run-panel-state.ts`, and `agent-run-panel.tsx`.
- Final verification commands T039-T041 can run independently once implementation is complete.

## Parallel Example: User Story 1

```bash
# Write US1 tests in parallel:
Task: "T014 [P] [US1] Add formatter tests for deriving a session-start lifecycle message in apps/agentic-workbench/src/entities/agent-run/model/format.test.ts"
Task: "T015 [P] [US1] Add run-panel state tests for appending exactly one start status message per new run in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "T016 [P] [US1] Add UI regression test for low-emphasis session-start rendering in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
```

## Parallel Example: User Story 2

```bash
# Write US2 tests in parallel:
Task: "T022 [P] [US2] Add formatter tests for active-to-idle transition message generation in apps/agentic-workbench/src/entities/agent-run/model/format.test.ts"
Task: "T023 [P] [US2] Add run-panel state tests for idle transition message append and awaiting-prompt reset in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "T024 [P] [US2] Add UI regression test for idle status message and header badge coexistence in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
```

## Parallel Example: User Story 3

```bash
# Write US3 regressions in parallel:
Task: "T030 [P] [US3] Add formatter tests for repeated status dedupe and malformed status suppression in apps/agentic-workbench/src/entities/agent-run/model/format.test.ts"
Task: "T031 [P] [US3] Add run-panel state tests for repeated idle update dedupe and new-run scope reset in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "T032 [P] [US3] Add UI regression test proving malformed or unknown status payloads do not render raw JSON in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
Task: "T033 [P] [US3] Add UI/source regression test proving available command summary remains separate from lifecycle messages in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
```

---

## Implementation Strategy

### MVP First (US1 Only)

1. Complete Phase 1 and Phase 2.
2. Complete Phase 3 for session-start status messages.
3. Run the focused tests from `specs/026-session-lifecycle-status/quickstart.md`.
4. Validate that command summary and raw session update suppression are unaffected.

### Incremental Delivery

1. Deliver US1 session-start status.
2. Add US2 active-to-idle transition status.
3. Add US3 repeated update dedupe and malformed status hardening.
4. Finish with type checks, full tests, Storybook validation, and manual quickstart checks.

### Notes

- `[P]` tasks touch different files or can be prepared independently.
- User story labels map directly to `specs/026-session-lifecycle-status/spec.md`.
- No Tauri backend, persistence, shared package, or cross-app import work is planned.
