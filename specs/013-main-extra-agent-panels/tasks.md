# Tasks: Main and Extra Agent Run Panels

**Input**: Design documents from `specs/013-main-extra-agent-panels/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: User-journey tests are optional unless requested by the spec. Constitution-required tests are included for pure panel state logic and agent session cleanup safety boundaries.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4, US5)
- Include exact file paths in descriptions

## Path Conventions

- **App frontend**: `apps/agentic-workbench/src/{app,pages,features,entities,shared,components/ui}`
- **App Tauri backend**: `apps/agentic-workbench/src-tauri/src/{domain,application,inbound,infrastructure,ports}`
- **Documentation**: `docs/*.md`
- **Feature docs**: `specs/013-main-extra-agent-panels/*.md`

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the feature pointer, branch, and existing code boundaries before implementation.

- [X] T001 Verify `.specify/feature.json` points to `specs/013-main-extra-agent-panels` and record any mismatch in `specs/013-main-extra-agent-panels/tasks.md`
- [X] T002 Inspect current single-panel integration points in `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`
- [X] T003 [P] Inspect current agent run settings, goal continuation, event filtering, and cancel paths in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T004 [P] Inspect backend run cancel and registry cleanup behavior in `apps/agentic-workbench/src-tauri/src/application/cancel_agent_run.rs` and `apps/agentic-workbench/src-tauri/src/infrastructure/agent_session_registry.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Add shared panel slot state and component contracts required by every user story.

**Critical**: No user story implementation should begin until this phase is complete.

- [X] T005 [P] Create `AgentRunPanelKind`, `AgentRunPanelSlot`, `AgentPanelRunState`, and close-state types in `apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.ts`
- [X] T006 [P] Add failing unit tests for main slot invariants, extra creation, active fallback, run-state reports, prompt routing, and idempotent close cleanup in `apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.test.ts`
- [X] T007 Implement pure panel slot reducers and helpers for create/select/updateRunState/routePrompt/requestClose/cancelClose/confirmClose/removeClosedPanel in `apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.ts`
- [X] T008 Add `panelId`, `onRunStateChange`, `enableGoalContinuation`, and `persistSettings` props to the exported prop contract in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T009 Update internal resizable panel ids to include `panelId` in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T010 Gate settings hydration/save behavior behind `persistSettings` while preserving main-panel defaults in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T011 Gate goal query UI, goal mutations, goal progress recording, and automatic continuation behind `enableGoalContinuation` in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T012 Emit `onRunStateChange` whenever `isRunning` or `activeRunId` changes in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`

**Checkpoint**: Panel slots and `AgentRunPanel` multi-instance prerequisites are ready.

---

## Phase 3: User Story 1 - main agent 패널을 항상 유지한다 (Priority: P1) MVP

**Goal**: Worktree Session always shows one non-removable main agent panel.

**Independent Test**: Open Worktree Session and confirm `Main` is the first panel, has no close action, and remains after all extra panels are gone.

### Tests for User Story 1

- [X] T013 [P] [US1] Add unit tests for required main slot creation and blocked main close in `apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.test.ts`
- [X] T014 [P] [US1] Add UI test or Storybook fixture for main-only agent area in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.test.tsx`

### Implementation for User Story 1

- [X] T015 [US1] Create `WorktreeAgentRunArea` with a fixed main slot and main-only `AgentRunPanel` rendering in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T016 [US1] Create `AgentRunPanelTabs` with non-closable main tab rendering in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel-tabs.tsx`
- [X] T017 [US1] Move the existing worktree header content into `WorktreeAgentRunArea` without changing visible branch/status display in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T018 [US1] Replace direct `AgentRunPanel` usage with `WorktreeAgentRunArea` in `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`
- [X] T019 [US1] Pass `enableGoalContinuation={true}` and `persistSettings={true}` only to the main panel in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T020 [US1] Add a main-only story state to `apps/agentic-workbench/src/stories/organisms.stories.tsx`

**Checkpoint**: User Story 1 is independently functional and testable.

---

## Phase 4: User Story 2 - extra agent 패널을 추가하고 전환한다 (Priority: P2)

**Goal**: Users can add extra agent panels and switch between main and extras while each panel keeps local UI state.

**Independent Test**: Add `Extra 1` and `Extra 2`, confirm each has a unique title/id, new extras become active, and switching tabs restores the selected panel view.

### Tests for User Story 2

- [X] T021 [P] [US2] Add unit tests for extra sequence, unique ids, new-extra activation, and active fallback after idle close in `apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.test.ts`
- [X] T022 [P] [US2] Add UI test for add-extra, tab switching, and idle extra close behavior in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.test.tsx`

### Implementation for User Story 2

- [X] T023 [US2] Add the add-extra tab action and extra close buttons to `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel-tabs.tsx`
- [X] T024 [US2] Manage slot list, active panel id, and extra creation in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T025 [US2] Render one mounted `AgentRunPanel` per slot and hide inactive panels without losing local state in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T026 [US2] Pass `panelId`, `enableGoalContinuation={false}`, and `persistSettings={false}` for extra panels in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T027 [US2] Ensure idle extra close removes only that slot and selects the next extra or main in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T028 [US2] Add Storybook states for one extra and multiple extras in `apps/agentic-workbench/src/stories/organisms.stories.tsx`

**Checkpoint**: User Stories 1 and 2 both work independently.

---

## Phase 5: User Story 3 - 패널별 실행 상태와 prompt 흐름을 분리한다 (Priority: P3)

**Goal**: Each panel owns independent prompt input, timeline, active run id, queue, running state, and permission state.

**Independent Test**: Run prompts in main and extra panels and verify each panel displays only its own timeline, queue, permission request, and running state.

### Tests for User Story 3

- [X] T029 [P] [US3] Add unit tests for run-state report isolation and unknown-panel report ignore behavior in `apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.test.ts`
- [X] T030 [P] [US3] Add UI test proving two mounted panels show independent running badges and do not share external prompt state in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.test.tsx`
- [X] T031 [P] [US3] Add regression test for `enableGoalContinuation=false` preventing extra-panel continuation in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`
- [X] T032 [P] [US3] Add regression test for `persistSettings=false` preventing extra-panel settings save in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`

### Implementation for User Story 3

- [X] T033 [US3] Store each panel's reported `isRunning` and `activeRunId` in slot state from `onRunStateChange` in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T034 [US3] Show running state on each tab based on slot state in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel-tabs.tsx`
- [X] T035 [US3] Show a same-worktree concurrent run warning when more than one slot is running in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T036 [US3] Keep permission dialog response handling scoped to each panel's `activeRunId` in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T037 [US3] Keep `listenRunEvents` filtering scoped to each panel's `activeRunId` and clear panel state only for matching run events in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T038 [US3] Improve concurrent run limit error display for extra-panel start failures in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`

**Checkpoint**: User Stories 1, 2, and 3 all work independently.

---

## Phase 6: User Story 4 - workspace annotation prompt를 active 패널로 보낸다 (Priority: P4)

**Goal**: Workspace annotation prompts route to the current active agent panel with target feedback.

**Independent Test**: Activate main or an extra panel, send a workspace annotation prompt, and confirm only the active panel receives it; if active panel is running, the prompt queues there.

### Tests for User Story 4

- [X] T039 [P] [US4] Add unit tests for prompt routing to active panel, empty prompt ignore, and closing-panel routing rejection in `apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.test.ts`
- [X] T040 [P] [US4] Add UI test for annotation prompt target feedback and active-panel-only delivery in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.test.tsx`

### Implementation for User Story 4

- [X] T041 [US4] Move `workspacePromptRequest` state from the page into `WorktreeAgentRunArea` slot-level prompt routing in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T042 [US4] Update `ProjectWorktreeSessionPage` to pass workspace annotation prompt text into `WorktreeAgentRunArea` instead of directly into `AgentRunPanel` in `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`
- [X] T043 [US4] Generate a unique routed request id and assign `externalPromptRequest` only to the active slot in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T044 [US4] Add short target-panel feedback for routed annotation prompts in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T045 [US4] Ensure existing running-panel queue behavior handles routed annotation prompts without cross-panel queue mutation in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`

**Checkpoint**: User Stories 1 through 4 all work independently.

---

## Phase 7: User Story 5 - 실행 중인 extra 패널 종료를 안전하게 처리한다 (Priority: P5)

**Goal**: Closing an active extra panel requires explicit confirmation and cleans up the associated agent session process, queue, permission state, and late events.

**Independent Test**: Close a running extra panel, choose cancel close and confirm the run continues; close again, confirm cancel run and close, then verify no run, permission request, queued prompt, or late event remains for the closed panel.

### Tests for User Story 5

- [X] T046 [P] [US5] Add unit tests for close confirmation, closing state, idempotent cleanup, late run-state reports, and active fallback in `apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.test.ts`
- [X] T047 [P] [US5] Add UI test for running extra close dialog cancel-close and cancel-run-and-close paths in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.test.tsx`
- [X] T048 [P] [US5] Add backend test that cancel clears run owner and permission state in `apps/agentic-workbench/src-tauri/src/infrastructure/agent_session_registry.rs`
- [X] T049 [P] [US5] Add backend test that canceling an unknown or already-finished run emits a settled lifecycle response in `apps/agentic-workbench/src-tauri/src/application/cancel_agent_run.rs`

### Implementation for User Story 5

- [X] T050 [US5] Add close confirmation dialog state and copy for running extra panels in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T051 [US5] Call `cancelAgentRun` for the captured extra `activeRunId` before removing a confirmed running extra panel in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T052 [US5] Mark confirmed closing extras so they reject new routed prompts and duplicate close confirmations in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T053 [US5] Remove closed extra panels only once after cancel success, already-settled response, or matching settled run state in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T054 [US5] Ignore late run-state callbacks and events for removed extra panel ids in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [X] T055 [US5] Verify backend `cancel_run` clears permission state and owner state without affecting other runs in `apps/agentic-workbench/src-tauri/src/infrastructure/agent_session_registry.rs`
- [X] T056 [US5] Keep session window close behavior canceling all owner-window runs in `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`

**Checkpoint**: All user stories are independently functional.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Final verification, documentation alignment, and regression checks.

- [X] T057 [P] Add or update reusable UI Storybook examples for main-only, one extra, multiple extras, running badge, close confirmation, and prompt target feedback in `apps/agentic-workbench/src/stories/organisms.stories.tsx`
- [X] T058 [P] Update Korean design documentation if final behavior differs from the original design in `docs/main-extra-agent-run-panels-design.md`
- [X] T059 Run frontend type checking and tests from `specs/013-main-extra-agent-panels/quickstart.md`
- [X] T060 Run Storybook build validation from `specs/013-main-extra-agent-panels/quickstart.md`
- [X] T061 Run Rust tests from `specs/013-main-extra-agent-panels/quickstart.md` if backend files changed
- [X] T062 Verify no app-to-app imports or cross-app shared package changes were introduced in `specs/013-main-extra-agent-panels/plan.md`
- [X] T063 Perform the manual end-to-end scenario and record any deviations in `specs/013-main-extra-agent-panels/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; blocks all user stories.
- **US1 (Phase 3)**: Depends on Foundational and delivers the MVP main panel.
- **US2 (Phase 4)**: Depends on Foundational; integrates naturally after US1 because tabs need the main area shell.
- **US3 (Phase 5)**: Depends on US2 for multiple mounted panels.
- **US4 (Phase 6)**: Depends on US2 for active panel routing and benefits from US3 state isolation.
- **US5 (Phase 7)**: Depends on US2 and US3 for extra panel run-state tracking.
- **Polish (Phase 8)**: Depends on desired story phases being complete.

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational; MVP scope.
- **User Story 2 (P2)**: Can start after Foundational, but should land after US1 for a coherent agent area.
- **User Story 3 (P3)**: Depends on US2's multi-panel rendering.
- **User Story 4 (P4)**: Depends on active panel state from US2.
- **User Story 5 (P5)**: Depends on run-state tracking from US3.

### Within Each User Story

- Write constitution-required pure logic and safety tests before implementation.
- Implement model/state helpers before UI orchestration.
- Wire page composition after reusable feature UI exists.
- Keep backend application/infrastructure tests before backend behavior changes.
- Complete each story checkpoint before moving to the next priority.

## Parallel Opportunities

- T003 and T004 can run in parallel during setup.
- T005 and T006 can run in parallel before T007.
- US1 tests T013 and T014 can run in parallel.
- US2 tests T021 and T022 can run in parallel.
- US3 tests T029, T030, T031, and T032 can run in parallel.
- US4 tests T039 and T040 can run in parallel.
- US5 tests T046, T047, T048, and T049 can run in parallel.
- Polish tasks T057 and T058 can run in parallel with validation tasks after story completion.

## Parallel Example: User Story 3

```text
Task: "T029 Add unit tests for run-state report isolation in apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.test.ts"
Task: "T030 Add UI test for independent running badges in apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.test.tsx"
Task: "T031 Add regression test for enableGoalContinuation=false in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
Task: "T032 Add regression test for persistSettings=false in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
```

## Parallel Example: User Story 5

```text
Task: "T046 Add close cleanup unit tests in apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.test.ts"
Task: "T047 Add running extra close dialog UI test in apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.test.tsx"
Task: "T048 Add registry cleanup backend test in apps/agentic-workbench/src-tauri/src/infrastructure/agent_session_registry.rs"
Task: "T049 Add unknown-run cancel lifecycle backend test in apps/agentic-workbench/src-tauri/src/application/cancel_agent_run.rs"
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 setup.
2. Complete Phase 2 foundational `AgentRunPanel` props and slot model.
3. Complete Phase 3 so Worktree Session renders a non-removable main panel through `WorktreeAgentRunArea`.
4. Stop and validate US1 independently before adding extra panel behavior.

### Incremental Delivery

1. US1: Main panel remains stable through the new agent area shell.
2. US2: Add extra panel creation, switching, and idle close.
3. US3: Add run-state isolation, running indicators, main-only goal/settings policy, and conflict warning.
4. US4: Route workspace annotation prompts to active panel.
5. US5: Add running extra close confirmation and cleanup guarantees.
6. Polish: Storybook, docs alignment, quickstart validation.

### Parallel Team Strategy

After Phase 2, separate contributors can work on tests and UI for later stories, but merging should preserve dependency order: US1 → US2 → US3 → US4/US5.

## Notes

- `[P]` tasks use different files or can be executed before dependent implementation tasks.
- `[US#]` labels map directly to prioritized user stories in `spec.md`.
- All task descriptions include exact file paths and are intended to be executable without additional context.
