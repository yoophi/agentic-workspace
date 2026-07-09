# Quickstart: Speckit Files Panel

## Prerequisites

- Repository root: `agentic-workspace`
- Current feature pointer: `.specify/feature.json` points to `specs/021-speckit-files-panel`
- AW dependencies installed with the existing workspace setup

## Validation Commands

```bash
pnpm --filter agentic-workbench check-types
pnpm --filter agentic-workbench test
cargo test -p agentic-workbench
```

Run the Rust command only if the implementation changes `apps/agentic-workbench/src-tauri`.

## Manual Validation Scenarios

### Scenario 1: Speckit feature list

1. Open Agentic Workbench.
2. Open a worktree that contains `specs/021-speckit-files-panel`.
3. Open the Worktree Session workspace panel.
4. Select the `Speckit` tab.

**Expected**:
- The panel lists Speckit feature folders under `specs/*`.
- `021-speckit-files-panel` appears with `spec.md`, `plan.md`, and other existing documents grouped by type.

### Scenario 2: Markdown document open

1. In the Speckit tab, select `021-speckit-files-panel/spec.md`.
2. Select `021-speckit-files-panel/plan.md`.
3. Select `021-speckit-files-panel/tasks.md` if it exists.

**Expected**:
- The existing markdown viewer flow opens the selected document.
- The selected feature and document remain identifiable.
- Switching documents does not require using the generic file tree.

### Scenario 3: Tasks progress summary

1. Use a feature folder with a `tasks.md` that includes completed and incomplete checkbox tasks.
2. Open the Speckit tab.
3. Compare the displayed summary with the actual checkbox count.

**Expected**:
- Completed, total, and remaining counts match the document.
- A `tasks.md` without checkbox tasks shows a no-task state, not a misleading percentage.
- A feature without `tasks.md` shows no tasks summary while other documents remain usable.

### Scenario 4: Empty repository state

1. Open a worktree with no `specs` directory.
2. Open the Speckit tab.
3. Repeat with an empty `specs` directory.

**Expected**:
- The panel shows a clear empty state.
- No worktree session error is raised.
- Git, Files, and Markdown tabs continue to work.

### Scenario 5: File change refresh

1. Keep the Speckit tab open.
2. Add, edit, or delete a Speckit markdown document under `specs/*`.
3. Wait for watcher refresh or use the explicit refresh action.

**Expected**:
- The feature/document list and task summary update within the expected refresh behavior.
- Deleted selected documents show stale/error state instead of old content as a fresh result.

## Storybook Validation

Add or update AW Storybook stories for:

- Speckit panel with multiple features
- Empty Speckit state
- Long feature names and long document paths
- Document-level read error
- tasks progress states: no tasks, not started, in progress, complete

## Fixture Requirements

Use fixtures with:

- `specs/001-alpha/spec.md`
- `specs/001-alpha/plan.md`
- `specs/001-alpha/tasks.md` with mixed checkbox states
- `specs/001-alpha/contracts/frontend-state.md`
- `specs/001-alpha/checklists/requirements.md`
- `specs/002-empty/tasks.md` with no checkbox tasks
- `specs/not-a-feature/readme.txt`
- Korean or space-containing paths where supported by existing file viewer tests

## Validation Log

- 2026-07-09: `pnpm --filter agentic-workbench check-types` passed.
- 2026-07-09: `pnpm --filter agentic-workbench test` passed with 36 test files and 185 tests.
- 2026-07-09: `cargo test -p agentic-workbench` was not run because implementation did not change `apps/agentic-workbench/src-tauri/src`.
