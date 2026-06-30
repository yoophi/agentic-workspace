# Tasks: Worktree Auto Refresh

**Input**: Design documents from `specs/001-worktree-auto-refresh/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/worktree-auto-refresh.md](./contracts/worktree-auto-refresh.md), [quickstart.md](./quickstart.md)

**Tests**: Constitution-required pure logic tests are included for shared refresh policy and stale selection helpers.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing across `agentic-workbench`, `git-explorer`, and `markdown-annotator`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to
- Include exact file paths in descriptions

## Path Conventions

- **App frontend**: `apps/{agentic-workbench,git-explorer,markdown-annotator}/src`
- **App Tauri backend**: `apps/*/src-tauri/src`
- **Reusable TypeScript**: `packages/workspace-auto-refresh/src`
- **Documentation**: `docs/*.md`

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish the shared refresh package and inspect app-local integration points.

- [X] T001 Create `packages/workspace-auto-refresh/package.json` with package name, scripts, dependencies, and workspace-compatible exports.
- [X] T002 [P] Create `packages/workspace-auto-refresh/tsconfig.json` matching existing package TypeScript conventions.
- [X] T003 [P] Create `packages/workspace-auto-refresh/src/index.ts` export skeleton.
- [X] T004 [P] Inspect workbench integration points in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`.
- [X] T005 [P] Inspect git-explorer integration points in `apps/git-explorer/src/widgets/changes-panel/ui/ChangesPanel.tsx` and `apps/git-explorer/src/app/providers/query.tsx`.
- [X] T006 [P] Inspect markdown-annotator integration points in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx` and `apps/markdown-annotator/src/entities/document/api/documentApi.ts`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared pure helpers that all three apps depend on.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T007 [P] Add failing refresh option tests for 30-second fallback interval, focus refresh, and scope key comparison in `packages/workspace-auto-refresh/src/refresh-options.test.ts`.
- [X] T008 [P] Add failing stale selection tests for file, commit, and markdown document states in `packages/workspace-auto-refresh/src/selection-staleness.test.ts`.
- [X] T009 Implement refresh interval, focus refresh, and scope key helpers in `packages/workspace-auto-refresh/src/refresh-options.ts`.
- [X] T010 Implement stale file, stale commit, and stale markdown document helper functions in `packages/workspace-auto-refresh/src/selection-staleness.ts`.
- [X] T011 Export shared helpers from `packages/workspace-auto-refresh/src/index.ts`.
- [X] T012 Add `@yoophi/workspace-auto-refresh` dependency to `apps/agentic-workbench/package.json`.
- [X] T013 Add `@yoophi/workspace-auto-refresh` dependency to `apps/git-explorer/package.json`.
- [X] T014 Add `@yoophi/workspace-auto-refresh` dependency to `apps/markdown-annotator/package.json`.
- [X] T015 Run shared package tests with `pnpm --filter @yoophi/workspace-auto-refresh test` from `/Users/yoophi/project/agentic-workspace`.

**Checkpoint**: Shared refresh policy and stale state helpers are tested and consumable by all apps.

---

## Phase 3: User Story 1 - File tree and markdown document reflect file changes (Priority: P1) MVP

**Goal**: File tree, file preview, markdown tree, and active markdown document update automatically for active scope file additions, deletions, renames, and content changes.

**Independent Test**: In workbench Files/Markdown tabs and markdown-annotator, externally create/edit/delete a file and verify the visible tree/preview/document updates within 3 seconds without manual refresh or app reload.

### Tests for User Story 1

- [X] T016 [P] [US1] Add workbench file stale selection consumer tests or helper usage tests in `apps/agentic-workbench/src/features/worktree-workspace/model/workspace-auto-refresh.test.ts`.
- [X] T017 [P] [US1] Add markdown document stale selection consumer tests in `apps/markdown-annotator/src/pages/annotator/annotator-auto-reload.test.tsx`.

### Implementation for User Story 1

- [X] T018 [US1] Apply Tauri watcher event invalidation and shared fallback refresh options to workbench file list query in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`.
- [X] T019 [US1] Apply Tauri watcher event invalidation and shared fallback refresh options to workbench file preview query in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`.
- [X] T020 [US1] Apply shared stale file selection handling to workbench Files tab in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`.
- [X] T021 [US1] Apply shared refresh options and stale selection handling to workbench Markdown tab in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`.
- [X] T022 [US1] Add active markdown document watcher reload state using shared fallback refresh options in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`.
- [X] T023 [US1] Keep last successful markdown document text on reload failure in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`.
- [ ] T024 [US1] Validate US1 manually using Quickstart Scenarios 1, 2, and 4 from `specs/001-worktree-auto-refresh/quickstart.md`.

**Checkpoint**: File and markdown document reload works in workbench and markdown-annotator.

---

## Phase 4: User Story 2 - Git explorer reflects commit and branch changes (Priority: P1)

**Goal**: Git status, commit graph, commit list, branch/ref-derived display, and commit detail stay current in both workbench and git-explorer.

**Independent Test**: Open the same repository/worktree in workbench and git-explorer, create a commit or switch branch externally, and verify graph/list/status update within 3 seconds in both apps.

### Tests for User Story 2

- [X] T025 [P] [US2] Add shared stale commit selection cases for branch switch and rewritten history in `packages/workspace-auto-refresh/src/selection-staleness.test.ts`.
- [X] T026 [P] [US2] Add git-explorer Storybook/sample data coverage for auto refresh and stale commit states in `apps/git-explorer/src/shared/storybook/sample-data.ts`.

### Implementation for User Story 2

- [X] T027 [US2] Apply Tauri watcher event invalidation and shared fallback refresh options to workbench Git status query in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`.
- [X] T028 [US2] Apply Tauri watcher event invalidation and shared latest-page fallback refresh policy to workbench Git history and graph infinite queries in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`.
- [X] T029 [US2] Apply shared stale commit selection handling to workbench commit detail state in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`.
- [X] T030 [US2] Apply shared refresh options to git-explorer worktrees, branches, history, and graph queries in `apps/git-explorer/src/widgets/changes-panel/ui/ChangesPanel.tsx`.
- [X] T031 [US2] Preserve git-explorer existing repository watcher invalidation while adding shared refresh fallback in `apps/git-explorer/src/app/providers/query.tsx`.
- [X] T032 [US2] Apply shared stale commit selection handling to git-explorer selected commit and file diff state in `apps/git-explorer/src/widgets/changes-panel/ui/ChangesPanel.tsx`.
- [ ] T033 [US2] Validate US2 manually using Quickstart Scenario 3 from `specs/001-worktree-auto-refresh/quickstart.md`.

**Checkpoint**: Git auto refresh works in workbench and git-explorer.

---

## Phase 5: User Story 3 - Automatic refresh does not interrupt active review (Priority: P2)

**Goal**: Automatic refresh keeps current file, markdown preview, annotation context, commit selection, diff selection, and scroll context whenever selected items remain valid.

**Independent Test**: Select a file, markdown annotation target, or commit; trigger unrelated file/Git changes externally; verify the visible review context remains stable while lists update.

### Tests for User Story 3

- [X] T034 [P] [US3] Add shared valid-selection preservation tests in `packages/workspace-auto-refresh/src/selection-staleness.test.ts`.

### Implementation for User Story 3

- [X] T035 [US3] Preserve workbench file preview, markdown preview, and annotation state during background refresh in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`.
- [X] T036 [US3] Preserve workbench commit detail and diff scroll context during background refresh in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`.
- [X] T037 [US3] Preserve git-explorer commit list/detail/diff selection and scroll context during background refresh in `apps/git-explorer/src/widgets/changes-panel/ui/ChangesPanel.tsx`.
- [X] T038 [US3] Preserve markdown-annotator annotations and selection toolbar state when reloaded block anchors still resolve in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`.
- [ ] T039 [US3] Validate US3 manually using Quickstart Scenarios 2, 3, and 4 from `specs/001-worktree-auto-refresh/quickstart.md`.

**Checkpoint**: All apps preserve active review context across automatic refresh.

---

## Phase 6: User Story 4 - Refresh state is visible and recoverable (Priority: P3)

**Goal**: Users can tell when each app is refreshing, when refresh failed, and how to retry without losing last successful data.

**Independent Test**: Make the active worktree/repository/document temporarily unreadable or trigger a Git command failure, and verify recoverable error feedback while previous data remains visible where possible.

### Implementation for User Story 4

- [X] T040 [US4] Add compact refresh/error/stale state UI to workbench workspace headers in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`.
- [X] T041 [US4] Add compact refresh/error/stale state UI to git-explorer changes panel headers in `apps/git-explorer/src/widgets/changes-panel/ui/ChangesPanel.tsx`.
- [X] T042 [US4] Add compact refresh/error/stale state UI to markdown-annotator document header in `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`.
- [X] T043 [US4] Add retry handlers using each app's scoped refetch/reload function in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`, `apps/git-explorer/src/widgets/changes-panel/ui/ChangesPanel.tsx`, and `apps/markdown-annotator/src/pages/annotator/AnnotatorPage.tsx`.
- [ ] T044 [US4] Validate US4 manually using Quickstart Scenario 5 from `specs/001-worktree-auto-refresh/quickstart.md`.

**Checkpoint**: Refresh state and recovery are visible in all three apps.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, verification, and boundary checks across all stories.

- [X] T045 [P] Update cross-app automatic refresh behavior and validation steps in `docs/project-worktree-session-workspace-plan.md`.
- [X] T046 [P] Add Storybook coverage for reusable refresh state UI only if extracted in `apps/agentic-workbench/src/features/worktree-workspace/ui`, `apps/git-explorer/src/stories`, or `apps/markdown-annotator/src/stories`.
- [X] T047 Run `pnpm --filter @yoophi/workspace-auto-refresh test` using `packages/workspace-auto-refresh/package.json` from `/Users/yoophi/project/agentic-workspace`.
- [X] T048 Run `pnpm --filter @yoophi/agentic-workbench check-types && pnpm --filter @yoophi/agentic-workbench test` using `apps/agentic-workbench/package.json` from `/Users/yoophi/project/agentic-workspace`.
- [X] T049 Run `pnpm --filter @yoophi/git-explorer check-types && pnpm build-storybook:git` using `apps/git-explorer/package.json` from `/Users/yoophi/project/agentic-workspace`.
- [X] T050 Run `pnpm --filter @yoophi/markdown-annotator check-types && pnpm --filter @yoophi/markdown-annotator test` using `apps/markdown-annotator/package.json` from `/Users/yoophi/project/agentic-workspace`.
- [X] T051 Verify no app-to-app imports were introduced by reviewing `git diff --stat` and changed imports under `apps/agentic-workbench/src`, `apps/git-explorer/src`, and `apps/markdown-annotator/src`.
- [ ] T052 Execute all applicable scenarios in `specs/001-worktree-auto-refresh/quickstart.md` and record any skipped backend/Rust checks in the final implementation summary.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion - blocks all user stories.
- **User Stories (Phase 3+)**: Depend on Foundational phase completion.
- **Polish (Phase 7)**: Depends on all desired user stories being complete.

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational - MVP for file/document refresh.
- **User Story 2 (P1)**: Can start after Foundational - independent Git refresh path.
- **User Story 3 (P2)**: Depends on US1 and US2 refresh behavior.
- **User Story 4 (P3)**: Depends on US1 and US2 query/reload state and benefits from US3 stale states.

### Parallel Opportunities

- T002-T006 can run in parallel after T001.
- T007 and T008 can run in parallel.
- T016, T017, T025, T026, and T034 can run in parallel once shared helpers exist.
- US1 and US2 can proceed in parallel after Phase 2 because they target different app/data flows.
- T045 and T046 can run in parallel after all desired UI states exist.

---

## Parallel Example: User Story 1

```text
Task: "T016 [P] [US1] Add workbench file stale selection consumer tests in apps/agentic-workbench/src/features/worktree-workspace/model/workspace-auto-refresh.test.ts"
Task: "T017 [P] [US1] Add markdown document stale selection consumer tests in apps/markdown-annotator/src/pages/annotator/annotator-auto-reload.test.tsx"
```

---

## Parallel Example: User Story 2

```text
Task: "T025 [P] [US2] Add shared stale commit selection cases in packages/workspace-auto-refresh/src/selection-staleness.test.ts"
Task: "T026 [P] [US2] Add git-explorer auto refresh consumer tests in apps/git-explorer/src/widgets/changes-panel/ui/ChangesPanel.test.tsx"
```

---

## Implementation Strategy

### MVP First

1. Complete Phase 1 setup.
2. Complete Phase 2 shared package helpers and tests.
3. Complete Phase 3 User Story 1.
4. Stop and validate Quickstart Scenarios 1, 2, and 4.

### Incremental Delivery

1. Setup + Foundational gives tested shared refresh policy.
2. US1 delivers file/document auto reload.
3. US2 delivers Git auto refresh in workbench and git-explorer.
4. US3 hardens context preservation across all apps.
5. US4 adds visible refreshing/error/retry states.
6. Polish verifies docs, package tests, three app checks, and quickstart scenarios.

---

## Notes

- `[P]` tasks use different files or are safe to execute without depending on incomplete edits.
- `[US1]`, `[US2]`, `[US3]`, and `[US4]` map directly to the four user stories in `spec.md`.
- Backend changes are not planned. If implementation introduces backend commands, add Rust service/provider tests before wiring each app's `src-tauri/src/inbound` commands.
