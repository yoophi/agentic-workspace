# Tasks: Markdown Quick Prompt

**Input**: Design documents from `specs/005-markdown-quick-prompt/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/quick-prompt-ui.md](./contracts/quick-prompt-ui.md), [quickstart.md](./quickstart.md)

**Tests**: Required for shared pure logic, shared React viewer contracts, app integration states, Storybook states, and quickstart regression coverage.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the feature file structure and make exports ready for story work.

- [X] T001 Create quick prompt core directory in `packages/markdown-annotation-core/src/quick-prompt/`
- [X] T002 Create quick prompt core type file in `packages/markdown-annotation-core/src/types/quick-prompt.ts`
- [X] T003 Create quick prompt feature UI directory in `apps/markdown-annotator/src/features/quick-prompt/ui/`
- [X] T004 Create quick prompt Storybook file in `apps/markdown-annotator/src/stories/organisms/QuickPrompt.stories.tsx`
- [X] T005 Update public exports for quick prompt types and helpers in `packages/markdown-annotation-core/src/index.ts`
- [X] T006 Update public exports for quick prompt React contracts in `packages/markdown-annotation-react/src/index.ts`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared model, formatter, viewer action, and dialog contracts required before any story can be independently implemented.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T007 [P] Add failing fixture tests for QuickPromptScope and QuickPromptContext validation in `packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.test.ts`
- [X] T008 [P] Add failing formatter tests for complete and reduced quick prompt output in `packages/markdown-annotation-core/src/quick-prompt/format-quick-prompt-for-agent.test.ts`
- [X] T009 [P] Add failing React contract tests for optional viewer quick prompt block action rendering in `packages/markdown-annotation-react/src/MarkdownViewer.test.tsx`
- [X] T010 Define QuickPromptScope, QuickPromptContext, PromptDraft, AgentTarget, and QuickPromptActionState types in `packages/markdown-annotation-core/src/types/quick-prompt.ts`
- [X] T011 Implement shared context validation and length-state helpers in `packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.ts`
- [X] T012 Implement shared agent prompt formatter in `packages/markdown-annotation-core/src/quick-prompt/format-quick-prompt-for-agent.ts`
- [X] T013 Extend MarkdownViewer quick prompt action prop types in `packages/markdown-annotation-react/src/types.ts`
- [X] T014 Add optional block quick prompt action rendering without changing existing annotation actions in `packages/markdown-annotation-react/src/MarkdownViewer.tsx`
- [X] T015 Create app-local QuickPromptDialog component with context preview, prompt textarea, target state, and send controls in `apps/markdown-annotator/src/features/quick-prompt/ui/QuickPromptDialog.tsx`
- [X] T016 Create app-local quick prompt target model and unavailable standalone target helper in `apps/markdown-annotator/src/features/quick-prompt/model/quick-prompt-target.ts`
- [X] T017 Run foundational package tests for quick prompt core and viewer contracts using `packages/markdown-annotation-core/package.json` and `packages/markdown-annotation-react/package.json`

**Checkpoint**: Shared core, formatter, viewer action slot, and MA prompt dialog foundation are ready.

---

## Phase 3: User Story 1 - 선택 영역을 agent 질문으로 전환 (Priority: P1) MVP

**Goal**: A user can create a quick prompt from a non-empty selected text range, review the selected context, edit prompt text, and send or see a target-unavailable state.

**Independent Test**: Select text in MA, trigger selection quick prompt, confirm the context is labeled `선택 영역`, edit prompt text, and verify send is delivered to the configured target or blocked with a clear unavailable-target reason.

### Tests for User Story 1

- [X] T018 [P] [US1] Add failing core tests for selection context with single-block and multi-block anchors in `packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.test.ts`
- [X] T019 [P] [US1] Add failing MA integration test for opening selection quick prompt from selected text in `apps/markdown-annotator/src/pages/annotator/quick-prompt-selection.test.tsx`
- [X] T020 [P] [US1] Add failing dialog test for editing prompt text while preserving selected context in `apps/markdown-annotator/src/features/quick-prompt/ui/QuickPromptDialog.test.tsx`

### Implementation for User Story 1

- [X] T021 [US1] Implement selection context builder from AnnotationAnchor arrays in `packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.ts`
- [X] T022 [US1] Add selection quick prompt state and draft lifecycle to `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`
- [X] T023 [US1] Add lightning icon button to the existing selection toolbar in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`
- [X] T024 [US1] Wire QuickPromptDialog to selection drafts and unavailable standalone target in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`
- [X] T025 [US1] Add selected-context formatting and send callback handling in `apps/markdown-annotator/src/features/quick-prompt/ui/QuickPromptDialog.tsx`
- [X] T026 [US1] Add selection quick prompt Storybook state in `apps/markdown-annotator/src/stories/organisms/QuickPrompt.stories.tsx`

**Checkpoint**: User Story 1 is functional and testable as the MVP.

---

## Phase 4: User Story 2 - Markdown 블럭 단위로 요청 작성 (Priority: P2)

**Goal**: A user can trigger a lightning action on a Markdown block and compose a prompt with that block's source context attached.

**Independent Test**: Open a document with paragraph, list, table, code, and Mermaid blocks; trigger block quick prompt on each; confirm context label is `Markdown 블럭` and the block source/line metadata is visible before send.

### Tests for User Story 2

- [X] T027 [P] [US2] Add failing core tests for block context from paragraph, table, code, and Mermaid MarkdownBlock values in `packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.test.ts`
- [X] T028 [P] [US2] Add failing viewer test for block quick prompt action coexisting with delete/comment/note actions in `packages/markdown-annotation-react/src/MarkdownViewer.test.tsx`
- [X] T029 [P] [US2] Add failing MA integration test for opening block quick prompt from MarkdownViewer block toolbar in `apps/markdown-annotator/src/pages/annotator/quick-prompt-block.test.tsx`

### Implementation for User Story 2

- [X] T030 [US2] Implement block context builder using MarkdownBlock.rawContent and line metadata in `packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.ts`
- [X] T031 [US2] Add onRequestBlockQuickPrompt prop plumbing through BlockShell and MarkdownBlockRenderer in `packages/markdown-annotation-react/src/MarkdownViewer.tsx`
- [X] T032 [US2] Add lightning icon tooltip and accessible name for block quick prompt action in `packages/markdown-annotation-react/src/MarkdownViewer.tsx`
- [X] T033 [US2] Wire block quick prompt handler from MarkdownViewer into MA page drafts in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`
- [X] T034 [US2] Add block context preview details for block type, line range, and reduced state in `apps/markdown-annotator/src/features/quick-prompt/ui/QuickPromptDialog.tsx`
- [X] T035 [US2] Add block quick prompt Storybook state with annotation actions present in `apps/markdown-annotator/src/stories/molecules/MarkdownViewer.stories.tsx`

**Checkpoint**: User Stories 1 and 2 both work independently.

---

## Phase 5: User Story 3 - 전체 문서에 대해 질문 (Priority: P3)

**Goal**: A user can create a quick prompt for the entire open Markdown document and see complete or reduced context state before sending.

**Independent Test**: Trigger document quick prompt on a normal document and on a long document fixture; confirm context label is `전체 문서`, prompt text is editable, and long context clearly shows whether content is complete or reduced.

### Tests for User Story 3

- [X] T036 [P] [US3] Add failing core tests for document context and long-context reduction in `packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.test.ts`
- [X] T037 [P] [US3] Add failing formatter tests for document-wide line labels and reduced context notice in `packages/markdown-annotation-core/src/quick-prompt/format-quick-prompt-for-agent.test.ts`
- [X] T038 [P] [US3] Add failing MA integration test for document quick prompt and long-context preview in `apps/markdown-annotator/src/pages/annotator/quick-prompt-document.test.tsx`

### Implementation for User Story 3

- [X] T039 [US3] Implement document context builder with source label, document path, revision label, and length state in `packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.ts`
- [X] T040 [US3] Add document-level quick prompt action near document header actions in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`
- [X] T041 [US3] Add long-context reduced/complete badge and included/original length display in `apps/markdown-annotator/src/features/quick-prompt/ui/QuickPromptDialog.tsx`
- [X] T042 [US3] Preserve document revision label when auto reload changes document while dialog is open in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`
- [X] T043 [US3] Add document and long-context Storybook states in `apps/markdown-annotator/src/stories/organisms/QuickPrompt.stories.tsx`

**Checkpoint**: User Stories 1, 2, and 3 are independently functional.

---

## Phase 6: User Story 4 - 사용 불가 상태와 접근성 유지 (Priority: P4)

**Goal**: Empty/disabled states and keyboard-accessible icon controls work without breaking existing annotation, rendering, and auto reload workflows.

**Independent Test**: Exercise missing selection, empty block, empty document, unavailable/busy agent target, keyboard focus, tooltip, annotation operations, Mermaid rendering, code block rendering, and auto reload regression flows.

### Tests for User Story 4

- [X] T044 [P] [US4] Add failing action-state tests for empty selection, empty block, empty document, and unavailable target in `packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.test.ts`
- [X] T045 [P] [US4] Add failing accessibility tests for quick prompt buttons and dialog keyboard flow in `apps/markdown-annotator/src/features/quick-prompt/ui/QuickPromptDialog.test.tsx`
- [X] T046 [P] [US4] Add failing regression test for annotation actions and quick prompt controls coexisting in `packages/markdown-annotation-react/src/MarkdownViewer.test.tsx`
- [X] T047 [P] [US4] Add failing MA regression test for auto reload stale context display while quick prompt dialog is open in `apps/markdown-annotator/src/pages/annotator/annotator-auto-reload.test.tsx`

### Implementation for User Story 4

- [X] T048 [US4] Implement QuickPromptActionState disabled reason helpers in `packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.ts`
- [X] T049 [US4] Add disabled, blocked, sending, failed, and sent UI states to `apps/markdown-annotator/src/features/quick-prompt/ui/QuickPromptDialog.tsx`
- [X] T050 [US4] Add keyboard focus handling, accessible names, and hover/focus tooltip copy for selection and document actions in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`
- [X] T051 [US4] Add keyboard focus handling, accessible names, and hover/focus tooltip copy for block action in `packages/markdown-annotation-react/src/MarkdownViewer.tsx`
- [X] T052 [US4] Integrate quick prompt formatter with AW Markdown workspace send path without app-to-app imports in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T053 [US4] Add unavailable, empty, keyboard-visible, and blocked-target Storybook states in `apps/markdown-annotator/src/stories/organisms/QuickPrompt.stories.tsx`

**Checkpoint**: All user stories are functional with disabled states, accessibility, and regression coverage.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Final verification, cleanup, and delivery checks across all stories.

- [X] T054 [P] Update quickstart verification notes if implementation-specific commands changed in `specs/005-markdown-quick-prompt/quickstart.md`
- [X] T055 [P] Run core typecheck and tests using `packages/markdown-annotation-core/package.json`
- [X] T056 [P] Run React package typecheck and tests using `packages/markdown-annotation-react/package.json`
- [X] T057 [P] Run MA typecheck and tests using `apps/markdown-annotator/package.json`
- [X] T058 [P] Run AW typecheck and tests using `apps/agentic-workbench/package.json`
- [X] T059 [P] Build or inspect MA Storybook quick prompt stories using `apps/markdown-annotator/src/stories/organisms/QuickPrompt.stories.tsx`
- [X] T060 Verify no app-to-app imports were introduced in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`
- [X] T061 Verify no app-to-app imports were introduced in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T062 Run quickstart manual validation scenarios from `specs/005-markdown-quick-prompt/quickstart.md`

---

## Phase 8: Amendment - Selection Toolbar Quick Annotate Dialog & Agent-Run Delivery

**Purpose**: Bring the completed implementation into alignment with the updated selection-toolbar contract: the quick annotate button appears in the toolbar shown after region selection, opens a user-input dialog, and sends the dialog-entered prompt text plus selected context to agent-run.

**Independent Test**: Select a non-empty Markdown region in AW, confirm the selection toolbar shows quick annotate, click it, enter a prompt in the dialog, send it, and verify the active agent-run receives the exact entered prompt text with the selected Markdown context.

### Tests for Selection Toolbar Amendment

- [ ] T063 [P] [US1] Add MA integration coverage for selection toolbar quick annotate visibility after non-empty text selection in `apps/markdown-annotator/src/pages/annotator/quick-prompt-selection.test.tsx`
- [ ] T064 [P] [US1] Add AW integration coverage for selection toolbar quick annotate opening a prompt dialog before send in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`
- [ ] T065 [P] [US1] Add dialog regression coverage that submitted payload uses dialog-entered prompt text instead of a default prompt in `apps/markdown-annotator/src/features/quick-prompt/ui/QuickPromptDialog.test.tsx`

### Implementation for Selection Toolbar Amendment

- [ ] T066 [US1] Extend quick prompt action state usage to identify the `selection-toolbar` surface in `packages/markdown-annotation-core/src/types/quick-prompt.ts`
- [ ] T067 [US1] Ensure the selection toolbar quick annotate lightning button is rendered only for current non-empty selections in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`
- [ ] T068 [US1] Add AW selection quick annotate draft state that opens a prompt dialog instead of immediately sending a default prompt in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [ ] T069 [US1] Wire AW dialog send to deliver the exact user-entered prompt text plus selected Markdown context to the active agent-run in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [ ] T070 [US1] Add Storybook coverage for selection toolbar quick annotate and selection prompt dialog states in `apps/markdown-annotator/src/stories/organisms/QuickPrompt.stories.tsx`

### Verification for Selection Toolbar Amendment

- [ ] T071 [P] Run MA tests and typecheck for selection toolbar quick annotate using `apps/markdown-annotator/package.json`
- [ ] T072 [P] Run AW tests and typecheck for agent-run quick annotate delivery using `apps/agentic-workbench/package.json`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup and blocks every user story.
- **US1 (Phase 3)**: Depends on Foundational. This is the MVP.
- **US2 (Phase 4)**: Depends on Foundational; can run after or alongside US1, but final UI conflict checks should account for US1 selection toolbar changes.
- **US3 (Phase 5)**: Depends on Foundational; can run alongside US1/US2 after shared formatter support exists.
- **US4 (Phase 6)**: Depends on all story surfaces being present enough to harden disabled/a11y/regression states.
- **Polish (Phase 7)**: Depends on desired user stories being complete.
- **Amendment (Phase 8)**: Depends on the existing US1 foundation and AW send integration; should complete before any final PR merge.

### User Story Dependencies

- **US1**: No dependency on other stories after Foundational.
- **US2**: No dependency on US1 for core block context; UI conflict review should happen before final checkpoint.
- **US3**: No dependency on US1/US2 except shared formatter foundation.
- **US4**: Depends on US1-US3 UI surfaces for full regression and accessibility coverage.
- **US1 Amendment**: Depends on existing US1 selection context/dialog work and AW delivery path from US4.

### Parallel Opportunities

- T001-T004 can be done in parallel if separate files/directories are assigned.
- T007-T009 can be written in parallel before implementation.
- US1 tests T018-T020 can be written in parallel.
- US2 tests T027-T029 can be written in parallel.
- US3 tests T036-T038 can be written in parallel.
- US4 tests T044-T047 can be written in parallel.
- Polish verification T055-T059 can run in parallel after implementation.
- Amendment tests T063-T065 can be written in parallel before T066-T070 implementation.
- Amendment verification T071-T072 can run in parallel after T066-T070 are complete.

---

## Parallel Example: User Story 1

```text
Task: "T018 [P] [US1] Add failing core tests for selection context with single-block and multi-block anchors in packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.test.ts"
Task: "T019 [P] [US1] Add failing MA integration test for opening selection quick prompt from selected text in apps/markdown-annotator/src/pages/annotator/quick-prompt-selection.test.tsx"
Task: "T020 [P] [US1] Add failing dialog test for editing prompt text while preserving selected context in apps/markdown-annotator/src/features/quick-prompt/ui/QuickPromptDialog.test.tsx"
```

## Parallel Example: User Story 2

```text
Task: "T027 [P] [US2] Add failing core tests for block context from paragraph, table, code, and Mermaid MarkdownBlock values in packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.test.ts"
Task: "T028 [P] [US2] Add failing viewer test for block quick prompt action coexisting with delete/comment/note actions in packages/markdown-annotation-react/src/MarkdownViewer.test.tsx"
Task: "T029 [P] [US2] Add failing MA integration test for opening block quick prompt from MarkdownViewer block toolbar in apps/markdown-annotator/src/pages/annotator/quick-prompt-block.test.tsx"
```

## Parallel Example: User Story 3

```text
Task: "T036 [P] [US3] Add failing core tests for document context and long-context reduction in packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.test.ts"
Task: "T037 [P] [US3] Add failing formatter tests for document-wide line labels and reduced context notice in packages/markdown-annotation-core/src/quick-prompt/format-quick-prompt-for-agent.test.ts"
Task: "T038 [P] [US3] Add failing MA integration test for document quick prompt and long-context preview in apps/markdown-annotator/src/pages/annotator/quick-prompt-document.test.tsx"
```

## Parallel Example: User Story 4

```text
Task: "T044 [P] [US4] Add failing action-state tests for empty selection, empty block, empty document, and unavailable target in packages/markdown-annotation-core/src/quick-prompt/build-quick-prompt-context.test.ts"
Task: "T045 [P] [US4] Add failing accessibility tests for quick prompt buttons and dialog keyboard flow in apps/markdown-annotator/src/features/quick-prompt/ui/QuickPromptDialog.test.tsx"
Task: "T046 [P] [US4] Add failing regression test for annotation actions and quick prompt controls coexisting in packages/markdown-annotation-react/src/MarkdownViewer.test.tsx"
Task: "T047 [P] [US4] Add failing MA regression test for auto reload stale context display while quick prompt dialog is open in apps/markdown-annotator/src/pages/annotator/annotator-auto-reload.test.tsx"
```

## Parallel Example: Selection Toolbar Amendment

```text
Task: "T063 [P] [US1] Add MA integration coverage for selection toolbar quick annotate visibility after non-empty text selection in apps/markdown-annotator/src/pages/annotator/quick-prompt-selection.test.tsx"
Task: "T064 [P] [US1] Add AW integration coverage for selection toolbar quick annotate opening a prompt dialog before send in apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx"
Task: "T065 [P] [US1] Add dialog regression coverage that submitted payload uses dialog-entered prompt text instead of a default prompt in apps/markdown-annotator/src/features/quick-prompt/ui/QuickPromptDialog.test.tsx"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 setup.
2. Complete Phase 2 shared foundations.
3. Complete Phase 3 selection quick prompt.
4. Validate US1 independently with tests and manual MA selection flow.
5. Stop for review before adding block/document scopes.

### Incremental Delivery

1. Deliver US1 selection quick prompt as MVP.
2. Add US2 block quick prompt using the same dialog and formatter.
3. Add US3 document quick prompt and long-context display.
4. Add US4 disabled, accessibility, AW integration, and regression hardening.
5. Complete Phase 8 selection toolbar quick annotate amendment.
6. Run all quickstart checks.

### Notes

- Tasks marked `[P]` touch separate files or are test-writing tasks that can be prepared independently.
- Story labels map directly to spec user stories: US1 selection, US2 block, US3 document, US4 disabled/accessibility.
- No Tauri backend tasks are listed because the plan keeps v1 delivery app-local and callback-based.
- Phase 8 tasks are intentionally unchecked because they were added after the initial implementation pass to reflect the updated selection toolbar and agent-run dialog-send contract.
