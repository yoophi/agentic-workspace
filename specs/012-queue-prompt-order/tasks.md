# Tasks: Queue Prompt Order

**Input**: Design documents from `specs/012-queue-prompt-order/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: User-journey tests are optional unless requested by the spec. Constitution-required tests are NOT optional: pure logic, parsers, formatters, graph layout, reducers, shared packages/crates, and safety boundaries require unit or fixture tests.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **App frontend**: `apps/agentic-workbench/src/{app,pages,features,entities,shared,components/ui}`
- **Documentation**: `docs/[english-file-name].md`
- Paths match the structure selected in plan.md.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the existing AgentRunPanel and prompt queue files that will be changed, without introducing new app/package boundaries.

- [X] T001 Review current first-run, saved prompt, external prompt, and queued prompt handlers in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T002 Review existing queued prompt helpers and tests in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts` and `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T003 [P] Review current AgentRun Storybook coverage in `apps/agentic-workbench/src/stories/organisms.stories.tsx`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish testable prompt queue state primitives that all user stories depend on.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 Add first-run prompt queue state types or helper signatures in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T005 [P] Add failing unit tests for blank prompt rejection and duplicate prompt prevention in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T006 Implement blank prompt rejection and duplicate prompt prevention helpers in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T007 Verify existing exported agent-run model types still cover run events without new public API changes in `apps/agentic-workbench/src/entities/agent-run/model/types.ts`

**Checkpoint**: Foundation ready - user story implementation can now begin.

---

## Phase 3: User Story 1 - 최초 입력 프롬프트를 대기 상태로 표시한다 (Priority: P1) MVP

**Goal**: A first prompt submitted before the agent session is ready appears as a queued prompt, not as an already executed user message.

**Independent Test**: Open a new Worktree Session, submit the first prompt with Return, and confirm the agent-run area shows the prompt in queued prompt format before prompt-specific output appears.

### Tests for User Story 1

- [X] T008 [P] [US1] Add failing first-run queued prompt state test in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T009 [P] [US1] Add failing run-start failure restoration test for queued first prompt in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`

### Implementation for User Story 1

- [X] T010 [US1] Implement helper to create a queued first-run prompt without appending a userMessage timeline item in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T011 [US1] Update `run()` to clear the textarea and start the first run through the queued prompt path in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T012 [US1] Update `startRun()` initialization to set queued first prompt state instead of calling `addUserMessage([], runId, displayPrompt)` in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T013 [US1] Update start failure handling to remove queued first prompt state and restore retry text without leaving an executed user message in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T014 [US1] Ensure `QueuedPromptTimeline` renders when the run is starting and `activeRunId` is present in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`

**Checkpoint**: User Story 1 is independently functional and testable.

---

## Phase 4: User Story 2 - agent 준비 완료 후 대기 프롬프트를 실행한다 (Priority: P2)

**Goal**: Once the agent is ready, the queued first prompt transitions into execution and output appears after that prompt.

**Independent Test**: Submit the first prompt, wait for agent readiness and response, and confirm the visible order is agent execution/preparation, queued prompt execution, then prompt output.

### Tests for User Story 2

- [X] T015 [P] [US2] Add failing lifecycle transition test for queued first prompt becoming active on promptSent in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T016 [P] [US2] Add failing test that prompt output is ordered after first prompt activation in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`

### Implementation for User Story 2

- [X] T017 [US2] Update `applyRunEvent` or a dedicated helper to transition the queued first prompt on `promptSent` and `promptCompleted` lifecycle events in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T018 [US2] Wire first prompt activation into the agent event subscription state update path in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T019 [US2] Preserve prompt history recording while avoiding duplicate userMessage timeline items in successful first-run startup in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T020 [US2] Verify cancellation and terminal lifecycle events clear queued first prompt state through existing finish handling in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`

**Checkpoint**: User Stories 1 and 2 both work independently.

---

## Phase 5: User Story 3 - 기존 실행 중 프롬프트 큐 동작과 일관성을 유지한다 (Priority: P3)

**Goal**: First-run prompts and prompts queued during an active run use consistent queued prompt presentation and ordering rules.

**Independent Test**: Compare first-run queued prompt display with prompts queued while the agent is running, then add multiple prompts before completion and confirm order is clear and stable.

### Tests for User Story 3

- [X] T021 [P] [US3] Add failing multi-prompt first-run ordering test in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T022 [P] [US3] Add failing saved prompt or external prompt first-run queue path test in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`

### Implementation for User Story 3

- [X] T023 [US3] Update `sendSavedPrompt()` to use the first-run queued prompt path when no agent run is active in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T024 [US3] Update external prompt request handling to use the first-run queued prompt path when no agent run is active in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T025 [US3] Preserve existing queue edit, move, remove, and steer behavior for non-initial queued prompts in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T026 [US3] Add or update an AgentRun queued first-run Storybook story in `apps/agentic-workbench/src/stories/organisms.stories.tsx`

**Checkpoint**: All user stories are independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Validate the feature end-to-end and clean up cross-story concerns.

- [X] T027 [P] Run `pnpm --dir apps/agentic-workbench test -- run-panel-state` and record the result in `specs/012-queue-prompt-order/quickstart.md`
- [X] T028 [P] Run `pnpm --dir apps/agentic-workbench check-types` and record the result in `specs/012-queue-prompt-order/quickstart.md`
- [X] T029 Execute the manual first-run, fast-startup, multi-prompt, and start-failure scenarios from `specs/012-queue-prompt-order/quickstart.md`
- [X] T030 Review `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx` for accidental prompt duplication, stale `activeRunIdRef`, or incorrect `isAwaitingPromptResponse` transitions
- [X] T031 [P] Update Korean documentation with a Mermaid flow in `docs/agent-run-prompt-queue-order.md` if the implementation changes the documented agent startup/prompt queue flow

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion - blocks all user stories.
- **User Story 1 (Phase 3)**: Depends on Foundational completion and is the MVP.
- **User Story 2 (Phase 4)**: Depends on Foundational completion; integrates naturally after US1 because it completes the same first prompt lifecycle.
- **User Story 3 (Phase 5)**: Depends on Foundational completion; can begin after the first-run queue path exists.
- **Polish (Phase 6)**: Depends on all desired user stories being complete.

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational - no dependency on other stories.
- **User Story 2 (P2)**: Can start after Foundational, but should be integrated after US1 for a complete first-run lifecycle.
- **User Story 3 (P3)**: Can start after Foundational, but depends on the first-run queue path from US1 for saved/external prompt reuse.

### Within Each User Story

- Constitution-required tests must be written and fail before implementation.
- Model/state helpers before UI event handler wiring.
- Queue state creation before lifecycle transition handling.
- UI integration before Storybook and manual validation.

### Parallel Opportunities

- T003 can run in parallel with T001-T002.
- T005 can run while T004 helper signatures are being drafted if test names and intended behavior are agreed.
- T008 and T009 can run in parallel for US1.
- T015 and T016 can run in parallel for US2.
- T021 and T022 can run in parallel for US3.
- T027 and T028 can run in parallel after implementation.
- T031 can run in parallel with final verification if documentation is required.

---

## Parallel Example: User Story 1

```bash
Task: "Add failing first-run queued prompt state test in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "Add failing run-start failure restoration test for queued first prompt in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
```

## Parallel Example: User Story 2

```bash
Task: "Add failing lifecycle transition test for queued first prompt becoming active on promptSent in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "Add failing test that prompt output is ordered after first prompt activation in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
```

## Parallel Example: User Story 3

```bash
Task: "Add failing multi-prompt first-run ordering test in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "Add failing saved prompt or external prompt first-run queue path test in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational.
3. Complete Phase 3: User Story 1.
4. Stop and validate that first prompt submission shows queued prompt before any prompt output.

### Incremental Delivery

1. Complete Setup + Foundational.
2. Add User Story 1 and validate first-run queued display.
3. Add User Story 2 and validate execution/output order.
4. Add User Story 3 and validate saved/external/multi-prompt consistency.
5. Run static and manual checks from quickstart.

### Parallel Team Strategy

1. Complete Setup and Foundational together.
2. Implement US1 first to establish the first-run queue path.
3. In parallel after US1 model shape is stable, one developer can work on US2 lifecycle transitions while another updates US3 saved/external prompt entry points.

---

## Notes

- [P] tasks use different files or independent test cases and have no dependency on incomplete implementation tasks.
- [US1], [US2], and [US3] labels map directly to the user stories in `specs/012-queue-prompt-order/spec.md`.
- `docs/agent-run-prompt-queue-order.md` is conditional because the plan treats project documentation as optional for this narrow UI flow unless implementation expands documented behavior.
