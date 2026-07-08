# Tasks: AW Worktree Session Files 하위 디렉터리 조회 수정

**Input**: Design documents from `specs/017-fix-filetree-subdirectories/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/worktree-files-ui.md](./contracts/worktree-files-ui.md), [quickstart.md](./quickstart.md)

**Tests**: User-journey tests are optional, but this feature includes constitution-required filesystem safety and state/fixture tests.

**Organization**: Tasks are grouped by user story so each story can be implemented and validated independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel because it touches different files or has no dependency on incomplete tasks
- **[Story]**: User story label for story phases only
- Every task includes an exact repository-relative file path

## Path Conventions

- **AW frontend**: `apps/agentic-workbench/src/{pages,features,entities,shared,stories}`
- **AW Tauri backend**: `apps/agentic-workbench/src-tauri/src/{domain,application,inbound,infrastructure}`
- **Feature docs**: `specs/017-fix-filetree-subdirectories/*`

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish fixtures and current behavior inspection needed by all story work.

- [X] T001 Inspect current Files tab lazy-loading and selected file flow in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T002 [P] Inspect current worktree file invoke contract and query keys in `apps/agentic-workbench/src/entities/worktree-file/api/worktree-file-repository.ts` and `apps/agentic-workbench/src/entities/worktree-file/api/query-keys.ts`
- [X] T003 [P] Inspect current Tauri worktree file safety/read behavior in `apps/agentic-workbench/src-tauri/src/application/worktree_file_service.rs` and `apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_file_provider.rs`
- [X] T004 [P] Extend nested file sample data with root, one-level, two-level, duplicate basename, Korean/space path, and read-failure fixtures in `apps/agentic-workbench/src/shared/storybook/sample-data.ts`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Add reusable verification anchors before story implementation.

**CRITICAL**: No user story implementation should begin until these checks can express the required behavior.

- [X] T005 [P] Add backend fixture coverage for nested relative path listing, nested file reads, duplicate basename reads, Korean/space path reads, and path escape rejection in `apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_file_provider.rs`
- [X] T006 [P] Add service-level coverage that preserves non-blank nested file paths and rejects blank preview paths in `apps/agentic-workbench/src-tauri/src/application/worktree_file_service.rs`
- [X] T007 [P] Add frontend Files tree contract tests for row visibility, full relativePath selection, duplicate basename distinction, and stale parent detection in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`
- [X] T008 [P] Add or update Storybook coverage for Files tree nested, duplicate basename, Korean/space path, loading, error, and stale states in `apps/agentic-workbench/src/stories/pages.stories.tsx`

**Checkpoint**: Backend and frontend tests now describe the target behavior and should fail where the release regression still exists.

---

## Phase 3: User Story 1 - Files 파일트리에서 하위 디렉터리 파일 열기 (Priority: P1) MVP

**Goal**: In the release-relevant Files tab flow, users can expand subdirectories and open nested files with matching preview content.

**Independent Test**: Open a worktree session fixture with `src/app.ts` and `src/deep/inner.ts`, expand `src` and `src/deep`, select each file, and verify the preview path/content matches the selected row.

### Tests for User Story 1

- [X] T009 [P] [US1] Add frontend test for expanding a folder, selecting `src/app.ts`, and sending the full `src/app.ts` relative path to preview in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`
- [X] T010 [P] [US1] Add frontend test for expanding nested folders and previewing `src/deep/inner.ts` after lazy directory loading in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`
- [X] T011 [P] [US1] Add Rust provider test that `read_text_file` returns `relative_path` as `src/deep/inner.ts` for nested files in `apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_file_provider.rs`

### Implementation for User Story 1

- [X] T012 [US1] Fix Files tab row selection so file rows pass the full `row.relativePath` to preview state in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T013 [US1] Fix lazy-loaded directory merge or visibility logic so nested child entries remain root-relative and visible under expanded parents in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T014 [US1] Verify and adjust read query key usage so selecting `src/app.ts` then `src/deep/inner.ts` creates distinct preview queries in `apps/agentic-workbench/src/entities/worktree-file/api/query-keys.ts`
- [X] T015 [US1] Verify and adjust Tauri file read handling for nested relative paths without weakening root/path validation in `apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_file_provider.rs`

**Checkpoint**: User Story 1 is complete when nested files can be opened independently from the Files tree and preview content/path matches the selected row.

---

## Phase 4: User Story 2 - 기존 루트 파일 조회 동작 유지하기 (Priority: P2)

**Goal**: Root file preview still works after the nested file fix, and root/nested switching does not leave stale preview content behind.

**Independent Test**: Select `README.md`, then `src/app.ts`, then `README.md` again five times and verify selected row, preview header, preview content, loading, and error states stay consistent.

### Tests for User Story 2

- [X] T016 [P] [US2] Add frontend regression test for switching `README.md` to `src/app.ts` and back without stale preview content in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`
- [X] T017 [P] [US2] Add frontend error-state test that a failed file preview does not keep previous file content as the current result in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`
- [X] T018 [P] [US2] Add Rust provider regression test that root-level `README.md` read behavior remains unchanged in `apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_file_provider.rs`

### Implementation for User Story 2

- [X] T019 [US2] Adjust preview rendering to clear or replace previous content while a newly selected file is loading or failing in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T020 [US2] Ensure stale selection detection only marks files stale when the selected file parent directory has been loaded in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T021 [US2] Preserve existing root file query and preview behavior while applying nested-path fixes in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`

**Checkpoint**: User Story 2 is complete when root and nested file selection can be alternated repeatedly without stale content, wrong selected state, or root-file regression.

---

## Phase 5: User Story 3 - 중첩 경로와 특수 경로 파일을 일관되게 처리하기 (Priority: P3)

**Goal**: Deep, duplicate-name, Korean, and space-containing paths are distinguished and previewed consistently.

**Independent Test**: Select `src/app.ts`, `docs/app.ts`, and `docs/한글 파일.md` from the Files tree and verify each preview maps to the selected full path.

### Tests for User Story 3

- [X] T022 [P] [US3] Add frontend test that duplicate basename files `src/app.ts` and `docs/app.ts` display distinct preview content in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`
- [X] T023 [P] [US3] Add frontend test for selecting a Korean/space path such as `docs/한글 파일.md` in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`
- [X] T024 [P] [US3] Add Rust provider test for reading `docs/한글 파일.md` and preserving its root-relative path in `apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_file_provider.rs`

### Implementation for User Story 3

- [X] T025 [US3] Fix any remaining basename-based comparison, keying, or preview matching so Files tab identity uses full `relativePath` in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T026 [US3] Verify and adjust frontend path display so Korean/space-containing paths are shown without truncation collisions or incorrect decoding in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- [X] T027 [US3] Verify and adjust repository invoke path forwarding for special relative paths in `apps/agentic-workbench/src/entities/worktree-file/api/worktree-file-repository.ts`

**Checkpoint**: All user stories are independently functional when duplicate basename and Korean/space paths preview the selected file only.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Finish verification, release-path confidence, and cleanup.

- [X] T028 [P] Update quickstart verification notes with any final command or release-path caveats discovered during implementation in `specs/017-fix-filetree-subdirectories/quickstart.md`
- [X] T029 [P] Run frontend type checking with `pnpm --dir apps/agentic-workbench check-types` and record results in `specs/017-fix-filetree-subdirectories/quickstart.md`
- [X] T030 [P] Run frontend tests with `pnpm --dir apps/agentic-workbench test` and record results in `specs/017-fix-filetree-subdirectories/quickstart.md`
- [X] T031 [P] Run Tauri backend tests with `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml worktree_file` and record results in `specs/017-fix-filetree-subdirectories/quickstart.md`
- [X] T032 Verify release or release-equivalent manual flow for root, nested, duplicate basename, Korean/space path, and read-failure cases in `specs/017-fix-filetree-subdirectories/quickstart.md`
- [X] T033 Verify no app-to-app imports, new shared package dependencies, or Tauri command business logic were introduced in `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx` and `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; can start immediately.
- **Foundational (Phase 2)**: Depends on Setup; blocks all user story implementation.
- **User Story 1 (Phase 3)**: Depends on Foundational; this is the MVP.
- **User Story 2 (Phase 4)**: Depends on Foundational; can be implemented after or alongside US1 but must preserve US1 behavior.
- **User Story 3 (Phase 5)**: Depends on Foundational; can be implemented after or alongside US1 but is easiest after full-path identity is fixed.
- **Polish (Phase 6)**: Depends on all implemented user stories.

### User Story Dependencies

- **US1 (P1)**: No dependency on US2/US3. Delivers nested file preview MVP.
- **US2 (P2)**: Builds on the same Files tab state but remains independently testable with root/nested switching.
- **US3 (P3)**: Builds on full `relativePath` identity and validates edge paths.

### Parallel Opportunities

- T002, T003, and T004 can run in parallel after T001 starts.
- T005, T006, T007, and T008 can run in parallel because they touch separate test/story files.
- Within US1, T009, T010, and T011 can run in parallel before T012-T015.
- Within US2, T016, T017, and T018 can run in parallel before T019-T021.
- Within US3, T022, T023, and T024 can run in parallel before T025-T027.
- T029, T030, and T031 can run in parallel if the environment supports concurrent test commands.

---

## Parallel Example: User Story 1

```bash
Task: "T009 [US1] Add frontend test for selecting src/app.ts in apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx"
Task: "T010 [US1] Add frontend test for selecting src/deep/inner.ts in apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx"
Task: "T011 [US1] Add Rust provider test for nested read_text_file in apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_file_provider.rs"
```

## Parallel Example: User Story 2

```bash
Task: "T016 [US2] Add frontend root/nested switching regression test in apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx"
Task: "T017 [US2] Add frontend failed preview stale-content test in apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx"
Task: "T018 [US2] Add Rust provider root read regression test in apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_file_provider.rs"
```

## Parallel Example: User Story 3

```bash
Task: "T022 [US3] Add frontend duplicate basename test in apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx"
Task: "T023 [US3] Add frontend Korean/space path test in apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx"
Task: "T024 [US3] Add Rust provider Korean/space path test in apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_file_provider.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 and Phase 2.
2. Implement Phase 3 only.
3. Validate `src/app.ts` and `src/deep/inner.ts` through the Files tree and backend read tests.
4. Stop and verify the release-regression path before adding broader edge cases.

### Incremental Delivery

1. Complete Setup and Foundational tests.
2. Deliver US1 nested file preview MVP.
3. Deliver US2 root regression and stale/error-state correctness.
4. Deliver US3 duplicate basename and Korean/space path handling.
5. Run Phase 6 validation and release-equivalent manual checks.

### Single-Agent Execution

1. Work sequentially in task ID order.
2. Keep backend safety tests and frontend state tests failing before implementation where practical.
3. After each story checkpoint, run the narrow tests for files touched by that story.
4. Run full quickstart verification after all selected stories are complete.

## Notes

- `[P]` tasks use separate files or can be prepared independently.
- Story labels map to the user stories in [spec.md](./spec.md).
- File access safety is not optional; do not weaken root/path validation to make nested files work.
