# Tasks: Speckit Files Panel

**Input**: Design documents from `specs/021-speckit-files-panel/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/speckit-panel-ui.md](./contracts/speckit-panel-ui.md), [quickstart.md](./quickstart.md)

**Tests**: Required for pure Speckit classification/progress logic, UI states, and watcher/query behavior by the Agentic Workspace Constitution and FR-013.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

- **App frontend**: `apps/agentic-workbench/src/{app,pages,features,entities,shared,components/ui}`
- **App Tauri backend**: `apps/agentic-workbench/src-tauri/src/{domain,application,inbound,infrastructure,ports}`
- **Documentation**: `specs/021-speckit-files-panel/*.md`

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Prepare app-local files and fixtures without changing behavior yet.

- [X] T001 Review existing workspace tab, markdown viewer selection, and file watcher paths in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T002 [P] Create Speckit model/test file skeletons in `apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.ts` and `apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.test.ts`
- [X] T003 [P] Add Speckit fixture sample data placeholders in `apps/agentic-workbench/src/shared/storybook/sample-data.ts`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core model, query, and UI shell dependencies that all user stories share.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 Define `SpeckitFeature`, `SpeckitDocument`, `TaskProgressSummary`, and `SpeckitPanelState` types in `apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.ts`
- [X] T005 [P] Add Speckit query key helpers using existing worktree-file query prefixes in `apps/agentic-workbench/src/entities/worktree-file/api/query-keys.ts`
- [X] T006 [P] Add `listSpeckitMarkdownFiles` repository helper that calls `listWorktreeFiles` with `scope.dir = "specs"` and markdown scope in `apps/agentic-workbench/src/entities/worktree-file/api/worktree-file-repository.ts`
- [X] T007 Create Speckit panel component shell with loading, refresh action slot, and typed props in `apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.tsx`
- [X] T008 [P] Add Speckit panel test harness helpers and fixture builders in `apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.test.tsx`

**Checkpoint**: Foundation ready. Speckit model/types, repository/query access, and panel shell exist.

---

## Phase 3: User Story 1 - Speckit 기능 목록을 한 곳에서 탐색하기 (Priority: P1) MVP

**Goal**: User can open a Speckit tab in Worktree Session and see grouped `specs/*` feature folders and existing Speckit documents.

**Independent Test**: Open a worktree with `specs/*`, select the Speckit tab, and verify feature folders and existing `spec.md`, `plan.md`, `tasks.md`, `contracts/*`, and `checklists/*` documents are grouped without using the generic file tree.

### Tests for User Story 1

- [X] T009 [P] [US1] Add fixture tests for grouping root-relative `specs/*` entries into `SpeckitFeature[]` in `apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.test.ts`
- [X] T010 [P] [US1] Add fixture tests for document type/group classification including `contracts/*` and `checklists/*` in `apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.test.ts`
- [X] T011 [P] [US1] Add UI test that renders feature rows and grouped document rows in `apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.test.tsx`

### Implementation for User Story 1

- [X] T012 [US1] Implement feature grouping and document classification helpers in `apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.ts`
- [X] T013 [US1] Implement feature list and grouped document list rendering in `apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.tsx`
- [X] T014 [US1] Add Speckit tab metadata to workspace tabs in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T015 [US1] Wire Speckit tab data loading through `listSpeckitMarkdownFiles` in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T016 [US1] Include Speckit query invalidation in the existing worktree watcher flow in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`

**Checkpoint**: US1 MVP complete. Speckit tab lists feature folders and known document types.

---

## Phase 4: User Story 2 - Speckit 문서를 빠르게 열어 검토하기 (Priority: P2)

**Goal**: User can select a Speckit document and open it through the existing markdown review flow.

**Independent Test**: From the Speckit tab, select `spec.md`, `plan.md`, and `tasks.md` for a feature and verify the markdown viewer opens the selected document while keeping feature/document identity visible.

### Tests for User Story 2

- [X] T017 [P] [US2] Add UI test for document row click emitting root-relative path in `apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.test.tsx`
- [X] T018 [P] [US2] Add integration-style UI test for Speckit selection updating markdown viewer state in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`
- [X] T019 [P] [US2] Add stale/deleted selected document test coverage in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`

### Implementation for User Story 2

- [X] T020 [US2] Add selected document callback and selected path styling to `apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.tsx`
- [X] T021 [US2] Connect Speckit document selection to existing markdown selected file state in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T022 [US2] Preserve selected feature/document identity while rendering markdown content in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T023 [US2] Handle read failure and stale selected Speckit document states without showing previous content as fresh in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`

**Checkpoint**: US2 complete. Speckit document rows open through the existing markdown viewer flow.

---

## Phase 5: User Story 3 - tasks 진행 상태를 요약해서 파악하기 (Priority: P3)

**Goal**: User can see completed/total/remaining task progress for features with `tasks.md`, including no-task and no-file states.

**Independent Test**: Show features with mixed checkbox tasks, no checkbox tasks, and no `tasks.md`; verify each progress summary matches the source document and does not mislead users.

### Tests for User Story 3

- [X] T024 [P] [US3] Add unit tests for checkbox task parsing states `noTasks`, `notStarted`, `inProgress`, and `complete` in `apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.test.ts`
- [X] T025 [P] [US3] Add unit tests for assigning `TaskProgressSummary` to the matching feature `tasks.md` in `apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.test.ts`
- [X] T026 [P] [US3] Add UI tests for tasks progress, no-task, and missing-tasks states in `apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.test.tsx`

### Implementation for User Story 3

- [X] T027 [US3] Implement markdown checkbox task progress parser in `apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.ts`
- [X] T028 [US3] Load `tasks.md` contents for listed Speckit features via existing text file queries in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T029 [US3] Merge parsed `TaskProgressSummary` into feature view models in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T030 [US3] Render compact tasks progress, no-task, and missing-tasks states in `apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.tsx`

**Checkpoint**: US3 complete. tasks progress summaries are accurate and visible.

---

## Phase 6: User Story 4 - Speckit 구조가 없는 저장소에서도 안전하게 동작하기 (Priority: P4)

**Goal**: User sees a clear empty state for worktrees without Speckit documents and panel-level/document-level errors stay contained.

**Independent Test**: Open worktrees with no `specs`, empty `specs`, unreadable documents, and long lists; verify no Worktree Session crash and other tabs remain usable.

### Tests for User Story 4

- [X] T031 [P] [US4] Add model tests that ignore non-Speckit directories and non-markdown files under `specs` in `apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.test.ts`
- [X] T032 [P] [US4] Add UI tests for no `specs`, empty `specs`, panel error, document error, and long-list states in `apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.test.tsx`
- [X] T033 [P] [US4] Add watcher refresh test for Speckit query invalidation behavior in `apps/agentic-workbench/src/features/worktree-workspace/model/workspace-auto-refresh.test.ts`

### Implementation for User Story 4

- [X] T034 [US4] Implement empty-state and non-Speckit filtering helpers in `apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.ts`
- [X] T035 [US4] Render empty, panel-level error, document-level error, and long-list-safe layout states in `apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.tsx`
- [X] T036 [US4] Ensure Speckit tab query errors do not block Git, Files, or Markdown tabs in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T037 [US4] Add explicit refresh behavior for Speckit list and task summaries in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`

**Checkpoint**: US4 complete. Missing or invalid Speckit structures are safe and understandable.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Storybook, validation, and final consistency checks across all stories.

- [X] T038 [P] Add Speckit panel fixture data for multiple features, long paths, empty state, document error, and task progress variants in `apps/agentic-workbench/src/shared/storybook/sample-data.ts`
- [X] T039 [P] Register Speckit panel Storybook stories under organisms/pages categories in `apps/agentic-workbench/src/stories/organisms.stories.tsx`
- [X] T040 [P] Update Worktree Workspace Storybook coverage for the Speckit tab in `apps/agentic-workbench/src/stories/pages.stories.tsx`
- [X] T041 Run `pnpm --filter agentic-workbench check-types` from `/Users/yoophi/project/agentic-workspace`
- [X] T042 Run `pnpm --filter agentic-workbench test` from `/Users/yoophi/project/agentic-workspace`
- [X] T043 Run `cargo test -p agentic-workbench` from `/Users/yoophi/project/agentic-workspace` if files under `apps/agentic-workbench/src-tauri/src` changed
- [X] T044 Validate quickstart scenarios and record any deviations in `specs/021-speckit-files-panel/quickstart.md`
- [X] T045 Verify no app-to-app imports or unintended `packages/*`/`crates/*` changes were introduced using `git diff --stat` from `/Users/yoophi/project/agentic-workspace`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - blocks all user stories
- **US1 (Phase 3)**: Depends on Foundational - MVP
- **US2 (Phase 4)**: Depends on US1 document list and tab integration
- **US3 (Phase 5)**: Depends on US1 feature/document grouping; can run in parallel with US2 after US1
- **US4 (Phase 6)**: Depends on US1 panel shell and query flow; can run in parallel with US2/US3 after US1
- **Polish (Phase 7)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational. Delivers MVP feature list and document grouping.
- **US2 (P2)**: Requires US1 document rows. Adds markdown viewer selection.
- **US3 (P3)**: Requires US1 feature grouping. Adds tasks progress summaries.
- **US4 (P4)**: Requires US1 panel/query shell. Adds safe empty/error/refresh hardening.

### Within Each User Story

- Tests for pure logic and UI states should be written before implementation and fail first.
- Model helpers before UI rendering.
- Repository/query keys before panel data loading.
- Panel rendering before workspace tab integration.
- Story checkpoint should pass before moving to the next priority story.

## Parallel Opportunities

- T002 and T003 can run in parallel after T001.
- T005, T006, and T008 can run in parallel after T004 is started.
- T009, T010, and T011 can run in parallel for US1.
- T017, T018, and T019 can run in parallel for US2.
- T024, T025, and T026 can run in parallel for US3.
- T031, T032, and T033 can run in parallel for US4.
- After US1, US2, US3, and US4 can be staffed in parallel with coordination on `worktree-workspace-panel.tsx`.
- T038, T039, and T040 can run in parallel once UI props and sample data shapes are stable.

## Parallel Example: User Story 1

```bash
Task: "T009 [US1] Add fixture tests for grouping root-relative specs/* entries in apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.test.ts"
Task: "T010 [US1] Add fixture tests for document type/group classification in apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.test.ts"
Task: "T011 [US1] Add UI test that renders feature rows and grouped document rows in apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.test.tsx"
```

## Parallel Example: User Story 2

```bash
Task: "T017 [US2] Add UI test for document row click emitting root-relative path in apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.test.tsx"
Task: "T018 [US2] Add integration-style UI test for Speckit selection updating markdown viewer state in apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx"
Task: "T019 [US2] Add stale/deleted selected document test coverage in apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx"
```

## Parallel Example: User Story 3

```bash
Task: "T024 [US3] Add unit tests for checkbox task parsing states in apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.test.ts"
Task: "T025 [US3] Add unit tests for assigning TaskProgressSummary to matching feature tasks.md in apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.test.ts"
Task: "T026 [US3] Add UI tests for tasks progress states in apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.test.tsx"
```

## Parallel Example: User Story 4

```bash
Task: "T031 [US4] Add model tests for non-Speckit filtering in apps/agentic-workbench/src/features/worktree-workspace/model/speckit-files.test.ts"
Task: "T032 [US4] Add UI tests for empty/error/long-list states in apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.test.tsx"
Task: "T033 [US4] Add watcher refresh test for Speckit query invalidation behavior in apps/agentic-workbench/src/features/worktree-workspace/model/workspace-auto-refresh.test.ts"
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: US1
4. Stop and validate that the Speckit tab lists `specs/*` features and grouped documents
5. Demo MVP before adding markdown opening, progress summaries, and edge-state hardening

### Incremental Delivery

1. Add US1 feature list and grouping
2. Add US2 markdown document opening
3. Add US3 tasks progress summaries
4. Add US4 empty/error/refresh hardening
5. Finish Storybook and quickstart validation

### Parallel Team Strategy

1. Complete Setup and Foundational together
2. Implement US1 as the shared base
3. Split US2, US3, and US4 between developers after US1
4. Coordinate edits to `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`

## Notes

- [P] tasks use different files or isolated test sections and can run in parallel.
- US2, US3, and US4 all touch `worktree-workspace-panel.tsx`; serialize those specific implementation edits if multiple agents are working.
- Keep backend changes out of scope unless existing `listWorktreeFiles` scope behavior is insufficient during implementation.
