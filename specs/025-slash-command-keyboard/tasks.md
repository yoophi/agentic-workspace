# Tasks: Slash Command Keyboard Navigation

**Input**: Design documents from `/specs/025-slash-command-keyboard/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Required by the feature contract and constitution verification guidance for UI interaction regressions.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **App frontend**: `apps/agentic-workbench/src/{app,pages,features,entities,shared,components/ui}`
- **Feature docs**: `specs/025-slash-command-keyboard`
- Paths match the structure selected in [plan.md](./plan.md).

## Phase 1: Setup (Shared Context)

**Purpose**: Confirm the existing autocomplete and prompt key handling boundaries before changing behavior.

- [X] T001 Review existing autocomplete rendering in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.tsx`
- [X] T002 [P] Review existing autocomplete rendering tests in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx`
- [X] T003 [P] Review existing prompt key handling in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T004 [P] Review existing source regression tests in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`
- [X] T005 [P] Review existing prompt autocomplete model helper tests in `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Add shared regression coverage for the existing single-command autocomplete contract before user-story implementation.

**Critical**: No user story implementation should start until these tests define the intended boundaries.

- [X] T006 [P] Add regression test that `PromptCommandAutocomplete` renders a listbox surface with option items and selected state in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx`
- [X] T007 [P] Add regression test that long candidate names and descriptions are constrained within suggestion items in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx`
- [X] T008 [P] Add source regression test that `AgentRunPanel` still routes autocomplete keys through `handleAutocompleteKeyDown` before prompt history navigation in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`
- [X] T009 [P] Add regression test that single candidate replacement remains covered by model helpers in `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts`

**Checkpoint**: Existing autocomplete contract is protected and user-story work can begin.

---

## Phase 3: User Story 1 - 키보드로 후보를 이동하며 항상 현재 항목을 확인한다 (Priority: P1) MVP

**Goal**: Arrow key navigation keeps the highlighted command candidate visible in long scrollable suggestion lists.

**Independent Test**: Open a long autocomplete list, move from first to last candidate and back using only `ArrowDown`/`ArrowUp`, and confirm the highlighted candidate always stays inside the visible list area.

### Tests for User Story 1

- [X] T010 [P] [US1] Add component regression test or source assertion for highlighted candidate element refs and nearest scroll behavior in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx`
- [X] T011 [P] [US1] Add source regression test for `ArrowDown` and `ArrowUp` highlight changes in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`

### Implementation for User Story 1

- [X] T012 [US1] Add highlighted option ref tracking in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.tsx`
- [X] T013 [US1] Scroll the highlighted option into view with nearest alignment when `highlightedIndex` changes in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.tsx`
- [X] T014 [US1] Add stable option identifiers or data attributes for highlighted candidates in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.tsx`
- [X] T015 [US1] Ensure `ArrowDown` and `ArrowUp` continue to clamp highlighted index against current candidates in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T016 [US1] Verify autocomplete list item layout keeps command name, description, and source label within the container in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.tsx`

**Checkpoint**: User Story 1 is independently functional and manually verifiable as the MVP.

---

## Phase 4: User Story 2 - 마우스 없이 후보를 선택하고 닫는다 (Priority: P2)

**Goal**: Users can apply or dismiss autocomplete candidates using `Enter`, `Tab`, and `Escape` without mouse interaction.

**Independent Test**: Open autocomplete, move to a candidate, apply it with `Enter` and `Tab`, then reopen and close with `Escape` without changing prompt text.

### Tests for User Story 2

- [X] T017 [P] [US2] Add source regression test for `Enter`, `Tab`, and `Escape` handling in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`
- [X] T018 [P] [US2] Add component regression test that pointer selection and keyboard selection use the same `onSelect` candidate contract in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx`

### Implementation for User Story 2

- [X] T019 [US2] Ensure `Escape` suppresses autocomplete without mutating prompt text in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T020 [US2] Ensure `Enter` applies the currently clamped highlighted candidate and restores prompt focus in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T021 [US2] Ensure `Tab` follows the same valid candidate application path without introducing multi-select state in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T022 [US2] Preserve pointer hover and pointer selection behavior while keeping a single highlighted candidate in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.tsx`

**Checkpoint**: User Stories 1 and 2 both work independently with keyboard-only command selection.

---

## Phase 5: User Story 3 - 후보 목록 변경 후에도 안전하게 탐색한다 (Priority: P3)

**Goal**: Filtering, empty states, and candidate count changes cannot leave autocomplete with an invalid highlighted candidate or broken keyboard handling.

**Independent Test**: Open autocomplete, highlight a later candidate, change the query so candidate count shrinks or becomes empty, and confirm keyboard actions remain safe.

### Tests for User Story 3

- [X] T023 [P] [US3] Add or extend model test for highlighted index clamping across zero, one, and many candidates in `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts`
- [X] T024 [P] [US3] Add source regression test that candidate length changes continue to call `clampHighlightedIndex` in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`
- [X] T025 [P] [US3] Add component regression test for loading, empty, no-match, and error states remaining non-selectable messages in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx`

### Implementation for User Story 3

- [X] T026 [US3] Ensure candidate count changes keep `autocompleteHighlightedIndex` within valid bounds in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T027 [US3] Ensure empty, loading, no-match, and error states do not render selectable option items in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.tsx`
- [X] T028 [US3] Ensure keyboard events with no candidates do not throw or select invalid candidates in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T029 [US3] Confirm no multi-select chip, selected-command accumulation, or command palette replacement state is introduced in `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.tsx`

**Checkpoint**: All user stories are independently functional and edge cases are protected.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Validate the whole feature and keep the implementation within the planned scope.

- [X] T030 [P] Run focused Vitest command from `specs/025-slash-command-keyboard/quickstart.md`
- [X] T031 [P] Run `pnpm --filter agentic-workbench check-types` for `apps/agentic-workbench`
- [X] T032 [P] Run `pnpm --filter agentic-workbench test` for `apps/agentic-workbench`
- [X] T033 Verify no backend, persistence, shared package, or app-to-app import changes were introduced in `apps/agentic-workbench/src`
- [X] T034 Run manual keyboard validation scenarios from `specs/025-slash-command-keyboard/quickstart.md`
- [X] T035 Update `specs/025-slash-command-keyboard/tasks.md` checkboxes to `[X]` after implementation and validation

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; can start immediately.
- **Foundational (Phase 2)**: Depends on Setup; blocks user story implementation.
- **User Story 1 (Phase 3)**: Depends on Foundational; MVP.
- **User Story 2 (Phase 4)**: Depends on Foundational and can be implemented after or alongside US1, but final verification should include US1 behavior.
- **User Story 3 (Phase 5)**: Depends on Foundational and can be implemented after or alongside US1/US2, but final verification should include all stories.
- **Polish (Phase 6)**: Depends on selected user stories being complete.

### User Story Dependencies

- **User Story 1 (P1)**: No dependency on other stories after Foundation. Delivers MVP.
- **User Story 2 (P2)**: Can start after Foundation; selection behavior should be validated with US1 visibility behavior.
- **User Story 3 (P3)**: Can start after Foundation; edge-case behavior should be validated with US1 and US2.

### Within Each User Story

- Tests first, then implementation.
- Component rendering changes before panel-level integration when both are needed.
- Model helper changes before UI code if highlighted index behavior changes.
- Story checkpoint validation before moving to the next story.

### Parallel Opportunities

- Setup review tasks T002-T005 can run in parallel.
- Foundational regression tests T006-T009 can run in parallel because they target different contracts or files.
- US1 tests T010-T011 can run in parallel before implementation.
- US2 tests T017-T018 can run in parallel before implementation.
- US3 tests T023-T025 can run in parallel before implementation.
- Polish validation commands T030-T032 can run in parallel if resources allow.

---

## Parallel Example: User Story 1

```bash
# Parallel test preparation for User Story 1:
Task: "T010 [US1] Add component regression test or source assertion for highlighted candidate element refs and nearest scroll behavior in apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx"
Task: "T011 [US1] Add source regression test for ArrowDown and ArrowUp highlight changes in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
```

## Parallel Example: User Story 2

```bash
# Parallel test preparation for User Story 2:
Task: "T017 [US2] Add source regression test for Enter, Tab, and Escape handling in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
Task: "T018 [US2] Add component regression test that pointer selection and keyboard selection use the same onSelect candidate contract in apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx"
```

## Parallel Example: User Story 3

```bash
# Parallel test preparation for User Story 3:
Task: "T023 [US3] Add or extend model test for highlighted index clamping across zero, one, and many candidates in apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts"
Task: "T024 [US3] Add source regression test that candidate length changes continue to call clampHighlightedIndex in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
Task: "T025 [US3] Add component regression test for loading, empty, no-match, and error states remaining non-selectable messages in apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational tests.
3. Complete Phase 3: User Story 1.
4. Stop and validate: highlighted candidate remains visible through long-list `ArrowDown` and `ArrowUp` movement.

### Incremental Delivery

1. Setup + Foundational tests define the autocomplete contract.
2. US1 adds visible highlighted item behavior.
3. US2 adds complete keyboard-only select/dismiss flow.
4. US3 hardens candidate filtering, empty states, and invalid highlight edge cases.
5. Polish phase runs automated and manual validation.

### Scope Guardrails

- Do not replace autocomplete with multi-select UI.
- Do not introduce selected command chips or persistent selected candidate state.
- Do not move prompt editing focus into the suggestion list.
- Do not change backend command candidate discovery.
- Do not change command filtering or insertion semantics beyond what is required for safe keyboard navigation.

## Notes

- [P] tasks affect different files or can be prepared without depending on incomplete implementation.
- [US1], [US2], and [US3] labels map directly to `spec.md` user stories.
- Each story should be independently demonstrable before moving to the next.
- If a task proves unnecessary after reading current code, mark it complete only after documenting why in the implementation summary.
