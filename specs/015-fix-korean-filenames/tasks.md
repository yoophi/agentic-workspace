# Tasks: AW Git Commit 상세 한글 파일명 표시 수정

**Input**: Design documents from `/specs/015-fix-korean-filenames/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: User-journey tests are optional unless requested by the spec.
Constitution-required tests are NOT optional: pure logic, parsers, formatters,
graph layout, reducers, shared packages/crates, and safety boundaries require
unit or fixture tests.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **App frontend**: `apps/agentic-workbench/src/{app,pages,features,entities,shared,components/ui}`
- **App Tauri backend**: `apps/agentic-workbench/src-tauri/src/{domain,application,inbound,infrastructure,ports}`
- **Reusable TypeScript**: `packages/git-ui/src`, `packages/git-graph/src`
- **Reusable Rust**: `crates/git-core/src`
- Paths match the structure selected in plan.md.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the existing shared Git data path and capture the implementation target before changing behavior.

- [X] T001 Review the current commit-detail CLI command and parser flow in `crates/git-core/src/git_cli.rs`
- [X] T002 [P] Review the AW Tauri command-service-provider flow for commit detail in `apps/agentic-workbench/src-tauri/src/application/worktree_git_service.rs`
- [X] T003 [P] Review the shared commit detail UI path usage in `packages/git-ui/src/ui/commit-detail-view.tsx`
- [X] T004 [P] Review the file tree path splitting and sorting behavior in `packages/git-ui/src/model/file-tree.ts`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Add failing shared-core fixtures and define the core path-display behavior that all stories depend on.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T005 Add failing unit tests for quoted octal, normal UTF-8 Korean, mixed ASCII/Korean, and ASCII commit file paths in `crates/git-core/src/git_cli.rs`
- [X] T006 Add a failing fixture test for a real temporary repository commit containing a Korean filename in `crates/git-core/src/git_cli.rs`
- [X] T007 Implement a pure helper that converts Git quoted octal path output into displayable UTF-8 paths without changing already-normal paths in `crates/git-core/src/git_cli.rs`
- [X] T008 Update commit file parsing to use the displayable path helper for commit detail file paths in `crates/git-core/src/git_cli.rs`
- [X] T009 Run the focused failing/passing core test command documented in `specs/015-fix-korean-filenames/quickstart.md`

**Checkpoint**: Foundation ready - `GitCommitFileChange.path` can be made readable before AW or shared UI consumes it.

---

## Phase 3: User Story 1 - 커밋 상세에서 한글 파일명 확인하기 (Priority: P1) MVP

**Goal**: AW commit detail changed-file list shows Korean file names as readable Korean text and never exposes `\\355\\202\\244` style octal byte escapes.

**Independent Test**: Open or fixture-test a commit containing a Korean file path and verify changed files show readable Korean text with no octal byte escapes.

### Tests for User Story 1

- [X] T010 [P] [US1] Add a contract-style regression test for `GitCommitDetail.files[*].path` readability in `crates/git-core/src/git_cli.rs`
- [X] T011 [P] [US1] Add file-tree tests for Korean directory and filename rows in `packages/git-ui/src/model/file-tree.test.ts`
- [X] T012 [P] [US1] Add Korean commit detail sample data for Storybook/UI validation in `apps/agentic-workbench/src/shared/storybook/sample-data.ts`

### Implementation for User Story 1

- [X] T013 [US1] Update the `git diff-tree` commit-detail invocation to avoid Git path quoting for non-ASCII names in `crates/git-core/src/git_cli.rs`
- [X] T014 [US1] Ensure parsed commit file paths from `parse_commit_files` are displayable for normal and quoted input in `crates/git-core/src/git_cli.rs`
- [X] T015 [US1] Verify the shared TypeScript `GitCommitFileChange` shape still represents readable display paths in `packages/git-graph/src/types.ts`
- [X] T016 [US1] Wire the Korean commit detail sample into an organism or page story state in `apps/agentic-workbench/src/stories/organisms.stories.tsx`
- [X] T017 [US1] Run `cargo test -p git-core korean` and `cargo test -p git-core commit_detail` as documented in `specs/015-fix-korean-filenames/quickstart.md`

**Checkpoint**: User Story 1 is independently functional when a Korean filename commit detail displays readable names in the changed-files list.

---

## Phase 4: User Story 2 - 파일명 표시 일관성 유지하기 (Priority: P2)

**Goal**: The same Korean path is used consistently across the changed-files list, selected file state, and diff display request/response flow.

**Independent Test**: Select a Korean file path from commit detail and verify the selected row, diff request path, and rendered diff area all refer to the same readable Korean path.

### Tests for User Story 2

- [X] T018 [P] [US2] Add a selection consistency regression test or fixture for Korean file paths in `packages/git-ui/src/model/file-tree.test.ts`
- [X] T019 [P] [US2] Add AW integration test coverage for Korean selected diff path state in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`

### Implementation for User Story 2

- [X] T020 [US2] Ensure `CommitDetailView` passes the same readable Korean `file.path` to `onSelectFile` in `packages/git-ui/src/ui/commit-detail-view.tsx`
- [X] T021 [US2] Ensure AW passes the readable selected file path unchanged to `getWorktreeCommitFileDiff` in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T022 [US2] Ensure the Tauri application service preserves the readable path when delegating file diff requests in `apps/agentic-workbench/src-tauri/src/application/worktree_git_service.rs`
- [X] T023 [US2] Verify the internal contract examples from the design artifact against implementation behavior in `specs/015-fix-korean-filenames/contracts/commit-detail-path-display.md`

**Checkpoint**: User Stories 1 and 2 both work independently: readable Korean file names display and the selected diff flow keeps the same path.

---

## Phase 5: User Story 3 - 한글 파일명 변경 이력 이해하기 (Priority: P3)

**Goal**: Rename/path-change commit entries involving Korean filenames expose readable Korean names for the path information represented by the current data shape.

**Independent Test**: Parse or open a commit where a file is renamed from or to a Korean name and verify the exposed changed-file path is readable Korean, not octal byte text.

### Tests for User Story 3

- [X] T024 [P] [US3] Add failing rename status fixtures with quoted Korean old and new paths in `crates/git-core/src/git_cli.rs`
- [X] T025 [P] [US3] Add a file-tree regression case for renamed Korean path display in `packages/git-ui/src/model/file-tree.test.ts`

### Implementation for User Story 3

- [X] T026 [US3] Update commit file parsing for rename status records so the exposed current path is decoded and displayable in `crates/git-core/src/git_cli.rs`
- [X] T027 [US3] Document any current-domain limitation around previous rename paths near `GitCommitFileChange` in `crates/git-core/src/domain.rs`
- [X] T028 [US3] Confirm `packages/git-graph/src/types.ts` remains aligned with the Rust `GitCommitFileChange` rename-path behavior in `packages/git-graph/src/types.ts`

**Checkpoint**: All user stories are independently functional, including Korean rename/path-change display for the path information currently exposed.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Verify shared-package consumers and clean up documentation/test artifacts.

- [X] T029 [P] Run `cargo test -p git-core` and record the result in `specs/015-fix-korean-filenames/quickstart.md`
- [X] T030 [P] Run `cargo check -p agentic-workbench` and record the result in `specs/015-fix-korean-filenames/quickstart.md`
- [X] T031 [P] Run `cargo check -p git-explorer` and record the result in `specs/015-fix-korean-filenames/quickstart.md`
- [X] T032 [P] Run `pnpm --filter @yoophi/git-ui test` and `pnpm --filter @yoophi/git-ui check-types` and record the result in `specs/015-fix-korean-filenames/quickstart.md`
- [X] T033 [P] Run `pnpm --filter @yoophi/agentic-workbench test` and `pnpm --filter @yoophi/agentic-workbench check-types` and record the result in `specs/015-fix-korean-filenames/quickstart.md`
- [X] T034 [P] Run `pnpm --filter @yoophi/git-explorer check-types` and record the result in `specs/015-fix-korean-filenames/quickstart.md`
- [X] T035 Verify no app-to-app imports or Tauri boundary violations were introduced in `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`
- [X] T036 Verify the manual smoke scenario against a repository with Korean filenames and record pass/fail notes in `specs/015-fix-korean-filenames/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User stories can proceed in priority order (P1 -> P2 -> P3)
  - US2 and US3 can start after Phase 2 if separate contributors coordinate shared-file edits
- **Polish (Final Phase)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational - no dependency on other stories
- **User Story 2 (P2)**: Can start after Foundational but is most useful after US1 confirms readable paths
- **User Story 3 (P3)**: Can start after Foundational and parser helper availability; independent of UI selection work

### Within Each User Story

- Constitution-required tests MUST be written and fail before implementation
- Shared Rust parser behavior before Tauri/AW integration
- Shared core before shared UI
- Shared UI fixture/story coverage before final app verification
- Story complete before moving to the next priority when working sequentially

### Parallel Opportunities

- T002, T003, and T004 can run in parallel during setup
- T010, T011, and T012 can run in parallel after foundational parser work is ready
- T018 and T019 can run in parallel because they target different files
- T024 and T025 can run in parallel because they target Rust parser and TypeScript file-tree tests separately
- T029 through T034 can run in parallel after implementation is complete

---

## Parallel Example: User Story 1

```bash
Task: "Add a contract-style regression test for GitCommitDetail.files[*].path readability in crates/git-core/src/git_cli.rs"
Task: "Add file-tree tests for Korean directory and filename rows in packages/git-ui/src/model/file-tree.test.ts"
Task: "Add Korean commit detail sample data for Storybook/UI validation in apps/agentic-workbench/src/shared/storybook/sample-data.ts"
```

## Parallel Example: User Story 2

```bash
Task: "Add a selection consistency regression test or fixture for Korean file paths in packages/git-ui/src/model/file-tree.test.ts"
Task: "Add AW integration test coverage for Korean selected diff path state in apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx"
```

## Parallel Example: User Story 3

```bash
Task: "Add failing rename status fixtures with quoted Korean old and new paths in crates/git-core/src/git_cli.rs"
Task: "Add a file-tree regression case for renamed Korean path display in packages/git-ui/src/model/file-tree.test.ts"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational parser tests and helper
3. Complete Phase 3: User Story 1
4. Stop and validate with `cargo test -p git-core korean`, `cargo test -p git-core commit_detail`, and a Korean filename commit detail sample
5. Demo AW changed-files list with no octal byte escapes

### Incremental Delivery

1. Foundation: shared parser handles displayable paths
2. US1: changed-files list shows readable Korean filenames
3. US2: selected file and diff flow use the same readable path
4. US3: rename/path-change entries expose readable Korean paths
5. Polish: run all shared consumer verification commands

### Parallel Team Strategy

With multiple developers:

1. Team completes setup and foundational parser helper together
2. Developer A: US1 core path readability and sample data
3. Developer B: US2 selection/diff path consistency
4. Developer C: US3 rename fixtures and parser behavior
5. Team runs shared Rust, shared TypeScript, AW, and Git Explorer verification

---

## Notes

- `[P]` tasks target different files or independent verification commands.
- `[US1]`, `[US2]`, and `[US3]` labels map to the prioritized user stories in spec.md.
- No project-level `docs/*.md` update is required for this narrow bug fix.
- If implementation proves the current `GitCommitFileChange` shape cannot represent rename old/new paths adequately, create a follow-up spec rather than expanding this task set without review.
