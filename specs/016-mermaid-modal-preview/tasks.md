# Tasks: Mermaid Chart Expanded Modal for Markdown Preview

**Input**: Design documents from `specs/016-mermaid-modal-preview/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/mermaid-expanded-view-ui.md, quickstart.md

**Tests**: Required by the feature spec and constitution because this changes `packages/markdown-annotation-react` and two consuming apps.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the current shared Mermaid rendering and app adapter boundaries before changing code.

- [X] T001 Review current shared Mermaid rendering and render action behavior in `packages/markdown-annotation-react/src/MermaidDiagram.tsx`
- [X] T002 [P] Review current shared Markdown viewer Mermaid integration in `packages/markdown-annotation-react/src/MarkdownViewer.tsx`
- [X] T003 [P] Review current AW agent-run expanded Mermaid implementation in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-markdown.tsx`
- [X] T004 [P] Review current MA and AW MarkdownViewer adapter boundaries in `apps/markdown-annotator/src/shared/ui/markdown-viewer-components.tsx` and `apps/agentic-workbench/src/features/worktree-workspace/ui/markdown-viewer-components.tsx`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Add the shared expanded Mermaid contract and reusable implementation that all user stories consume.

**CRITICAL**: No user story wiring should begin until this phase is complete.

- [X] T005 [P] Add failing shared tests for rendered trigger, failed/empty exclusion, zoom bounds, and fit body in `packages/markdown-annotation-react/src/MermaidExpandedView.test.tsx`
- [X] T006 [P] Add failing shared MarkdownViewer tests for expanded Mermaid controls and ordinary code preservation in `packages/markdown-annotation-react/src/MarkdownViewer.test.tsx`
- [X] T007 Define expanded Mermaid component injection types in `packages/markdown-annotation-react/src/types.ts`
- [X] T008 Implement shared expanded Mermaid viewer, trigger, zoom controls, and fit body in `packages/markdown-annotation-react/src/MermaidExpandedView.tsx`
- [X] T009 Update `MermaidDiagram` to support shared expanded-view composition while keeping render actions visible only for rendered diagrams in `packages/markdown-annotation-react/src/MermaidDiagram.tsx`
- [X] T010 Wire optional expanded Mermaid behavior into shared `MarkdownViewer` without changing non-Mermaid code block rendering in `packages/markdown-annotation-react/src/MarkdownViewer.tsx`
- [X] T011 Export the new expanded Mermaid types and components from `packages/markdown-annotation-react/src/index.ts`
- [X] T012 Add shared expanded modal layout, toolbar, fit, and overflow styles in `packages/markdown-annotation-react/src/styles.css`

**Checkpoint**: Shared package exposes the reusable expanded Mermaid contract and keeps fallback/ordinary code behavior intact.

---

## Phase 3: User Story 1 - MA Mermaid diagrams open in a large modal (Priority: P1) MVP

**Goal**: MA users can open a rendered Mermaid diagram from document preview in a large modal and return to the same document workflow.

**Independent Test**: Open a MA document with one or more valid Mermaid diagrams, activate the expand control, verify the selected diagram opens in a viewport-sized modal, then close it and continue document review.

### Tests for User Story 1

- [X] T013 [P] [US1] Add MA adapter test coverage for Base UI dialog primitives used by expanded Mermaid viewer in `apps/markdown-annotator/src/shared/ui/markdown-viewer-components.test.tsx`
- [X] T014 [P] [US1] Add MA annotator regression test for Mermaid modal open/close preserving document workflow in `apps/markdown-annotator/src/pages/annotator/mermaid-expanded-view.test.tsx`

### Implementation for User Story 1

- [X] T015 [US1] Extend MA MarkdownViewer component adapters with expanded Mermaid dialog primitives in `apps/markdown-annotator/src/shared/ui/markdown-viewer-components.tsx`
- [X] T016 [US1] Enable expanded Mermaid viewer props for MA document preview in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`
- [X] T017 [US1] Add MA Storybook states for rendered, expanded, large, and fallback-excluded Mermaid diagrams in `apps/markdown-annotator/src/stories/molecules/MarkdownViewer.stories.tsx`

**Checkpoint**: User Story 1 is independently functional and testable as the MVP.

---

## Phase 4: User Story 2 - AW markdown preview diagrams open in a large modal (Priority: P2)

**Goal**: AW worktree session markdown preview users get the same Mermaid expanded modal behavior without interfering with agent-run panel state.

**Independent Test**: Open a Mermaid markdown file in AW worktree markdown preview, activate the expand control, verify the modal opens and closes, and confirm preview panel state and agent-run panel state remain independent.

### Tests for User Story 2

- [X] T018 [P] [US2] Add AW worktree MarkdownViewer adapter test coverage for Radix dialog primitives in `apps/agentic-workbench/src/features/worktree-workspace/ui/markdown-viewer-components.test.tsx`
- [X] T019 [P] [US2] Add AW worktree preview regression test for Mermaid modal open/close and file-switch independence in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-mermaid-expanded.test.tsx`

### Implementation for User Story 2

- [X] T020 [US2] Extend AW worktree MarkdownViewer component adapters with expanded Mermaid dialog primitives in `apps/agentic-workbench/src/features/worktree-workspace/ui/markdown-viewer-components.tsx`
- [X] T021 [US2] Enable expanded Mermaid viewer props for AW worktree markdown preview in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T022 [US2] Add AW Storybook states for worktree preview Mermaid expanded behavior in `apps/agentic-workbench/src/stories/molecules.stories.tsx`

**Checkpoint**: User Story 2 works independently after the shared foundation, while User Story 1 remains functional.

---

## Phase 5: User Story 3 - Large, failed, and ordinary blocks remain stable (Priority: P3)

**Goal**: Large diagrams remain inspectable in the modal, failed/empty Mermaid blocks do not expose expand controls, ordinary code remains unchanged, and AW agent-run stays consistent with the shared behavior.

**Independent Test**: Use fixtures containing large Mermaid diagrams, failed Mermaid blocks, empty Mermaid blocks, and ordinary code blocks in MA and AW preview; verify expand controls appear only for rendered diagrams and large diagrams scroll/zoom inside the modal.

### Tests for User Story 3

- [X] T023 [P] [US3] Add shared large-diagram overflow and fallback-exclusion fixture tests in `packages/markdown-annotation-react/src/MermaidExpandedView.test.tsx`
- [X] T024 [P] [US3] Update AW agent-run expanded Mermaid tests for shared viewer consistency in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-markdown-expanded.test.tsx`
- [X] T025 [P] [US3] Update AW agent-run fallback exclusion tests for shared viewer consistency in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-markdown-expanded-fallback.test.tsx`

### Implementation for User Story 3

- [X] T026 [US3] Refactor AW agent-run Mermaid expanded view to consume the shared expanded viewer while preserving streaming debounce in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-markdown.tsx`
- [X] T027 [US3] Ensure shared CSS keeps large diagrams locally scrollable and prevents layout overlap in `packages/markdown-annotation-react/src/styles.css`
- [X] T028 [US3] Add or update AW agent-run Storybook state for shared Mermaid expanded controls in `apps/agentic-workbench/src/stories/organisms.stories.tsx`

**Checkpoint**: All three user stories are independently functional and the three Mermaid surfaces share consistent behavior.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final verification, cleanup, and governance checks across shared package and consumer apps.

- [X] T029 [P] Verify no app-to-app imports were introduced across `apps/markdown-annotator/src`, `apps/agentic-workbench/src`, and `packages/markdown-annotation-react/src`
- [X] T030 [P] Run shared package tests and type checks from `specs/016-mermaid-modal-preview/quickstart.md` for `packages/markdown-annotation-react`
- [X] T031 [P] Run MA tests and type checks from `specs/016-mermaid-modal-preview/quickstart.md` for `apps/markdown-annotator`
- [X] T032 [P] Run AW tests and type checks from `specs/016-mermaid-modal-preview/quickstart.md` for `apps/agentic-workbench`
- [X] T033 Run quickstart scenario validation and record any deviations in `specs/016-mermaid-modal-preview/quickstart.md`
- [X] T034 Update Korean architecture documentation in `docs/mermaid-expanded-view.md` if the implemented shared contract differs from `specs/016-mermaid-modal-preview/contracts/mermaid-expanded-view-ui.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; can start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion; blocks all user stories.
- **User Story 1 (Phase 3)**: Depends on Foundational completion; recommended MVP.
- **User Story 2 (Phase 4)**: Depends on Foundational completion; can run in parallel with US1 after shared contract exists.
- **User Story 3 (Phase 5)**: Depends on Foundational completion and should integrate with any completed US1/US2 wiring.
- **Polish (Phase 6)**: Depends on all desired user stories being complete.

### User Story Dependencies

- **US1 (P1)**: Requires shared expanded viewer foundation only.
- **US2 (P2)**: Requires shared expanded viewer foundation only; no dependency on MA implementation.
- **US3 (P3)**: Requires shared expanded viewer foundation and validates cross-surface consistency; best after at least one consumer surface is wired.

### Within Each User Story

- Tests should be written first and fail before implementation.
- App adapter tests precede app adapter implementation.
- Shared package changes precede MA, AW preview, and AW agent-run wiring.
- Storybook updates follow functional implementation for that story.

### Parallel Opportunities

- T002, T003, and T004 can run in parallel during setup.
- T005 and T006 can run in parallel during foundation.
- T013 and T014 can run in parallel for US1.
- T018 and T019 can run in parallel for US2.
- T023, T024, and T025 can run in parallel for US3.
- T029, T030, T031, and T032 can run in parallel during polish.

---

## Parallel Example: User Story 1

```bash
Task: "T013 [US1] Add MA adapter test coverage in apps/markdown-annotator/src/shared/ui/markdown-viewer-components.test.tsx"
Task: "T014 [US1] Add MA annotator regression test in apps/markdown-annotator/src/pages/annotator/mermaid-expanded-view.test.tsx"
```

---

## Parallel Example: User Story 2

```bash
Task: "T018 [US2] Add AW adapter test coverage in apps/agentic-workbench/src/features/worktree-workspace/ui/markdown-viewer-components.test.tsx"
Task: "T019 [US2] Add AW preview regression test in apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-mermaid-expanded.test.tsx"
```

---

## Parallel Example: User Story 3

```bash
Task: "T023 [US3] Add shared large-diagram tests in packages/markdown-annotation-react/src/MermaidExpandedView.test.tsx"
Task: "T024 [US3] Update AW agent-run expanded tests in apps/agentic-workbench/src/features/agent-run/ui/agent-run-markdown-expanded.test.tsx"
Task: "T025 [US3] Update AW agent-run fallback tests in apps/agentic-workbench/src/features/agent-run/ui/agent-run-markdown-expanded-fallback.test.tsx"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 setup review.
2. Complete Phase 2 shared expanded viewer foundation.
3. Complete Phase 3 MA wiring and MA verification.
4. Stop and validate MA independently against User Story 1.

### Incremental Delivery

1. Shared foundation enables expand controls only for rendered diagrams.
2. Add MA support first for MVP.
3. Add AW worktree markdown preview support without changing agent-run state.
4. Refactor/align AW agent-run and large/fallback stability behavior.
5. Run package and consumer app verification from quickstart.

### Parallel Team Strategy

1. One developer owns shared package foundation.
2. After foundation, one developer wires MA while another wires AW worktree preview.
3. A third developer can update AW agent-run consistency tests and refactor after the shared viewer API is stable.

## Notes

- `[P]` tasks use different files or can be performed without depending on incomplete sibling tasks.
- `[US1]`, `[US2]`, and `[US3]` labels map to the user stories in `specs/016-mermaid-modal-preview/spec.md`.
- No backend, persistence, filesystem, Tauri command, or Rust tasks are required for this feature.
