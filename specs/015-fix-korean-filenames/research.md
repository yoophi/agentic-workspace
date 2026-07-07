# Research: AW Git Commit 상세 한글 파일명 표시 수정

## Decision 1: Fix path readability at the Git core boundary

**Decision**: Normalize commit-detail file paths in `crates/git-core` before they are returned through AW Tauri commands.

**Rationale**: The observed value, such as `\\355\\202\\244\\354\\230\\244`, is generated before React rendering. Current AW uses `GitCliHistoryReader.get_commit_detail()` through `GitCliWorktreeGitProvider`, so fixing the shared core prevents every consumer from receiving unreadable paths and preserves UI simplicity.

**Alternatives considered**:

- Decode in `packages/git-ui`: rejected because it treats a data contract problem as presentation logic and would not help file selection or Tauri/API consumers.
- Decode only in `apps/agentic-workbench`: rejected because `git-core` is already the shared commit-detail source for AW and Git Explorer.

## Decision 2: Prefer unquoted/NUL-safe Git path output and keep parser fixtures for quoted octal

**Decision**: Plan implementation around Git output that avoids C-style path quoting for commit file lists, with regression tests that also cover quoted octal input.

**Rationale**: Git can quote non-ASCII paths depending on `core.quotePath`, producing backslash+octal byte sequences. The most robust contract is that `GitCommitFileChange.path` returned by `git-core` is already a displayable UTF-8 path. NUL-safe parsing also avoids tab/newline ambiguity in file names.

**Alternatives considered**:

- Rely on user/global Git configuration: rejected because app behavior must not depend on each repository or user setting.
- Keep line/tab parsing only: rejected because it is fragile for unusual path characters and does not directly address quoted octal output.

## Decision 3: Keep diff requests keyed by the normalized file path

**Decision**: The selected file path passed from UI to `get_worktree_commit_file_diff` should remain the same displayable path emitted by commit detail.

**Rationale**: AW currently selects a file by `row.file.path` and passes that path back to the backend for file diff. If commit detail emits readable paths and the backend accepts those paths, the list, selection, and diff request contract stays coherent.

**Alternatives considered**:

- Add separate raw path and display path fields immediately: rejected for this bug because it changes the public data shape and requires broader frontend migration. It can be reconsidered only if a real repository fixture proves displayable paths cannot safely address file diff requests.

## Decision 4: Validate rename paths explicitly

**Decision**: Include rename/path-change fixtures in Git core tests even though the current TypeScript type only exposes `path` and `status`.

**Rationale**: The spec requires rename readability. Git `--name-status` rename output has multiple path fields, so the parser behavior must not silently keep octal output in either old or new path. If the current domain cannot represent both paths, tasks must call out the limitation and at least ensure the exposed path is readable.

**Alternatives considered**:

- Exclude rename from implementation: rejected because it is in the feature spec and is a common path display edge case.

## Decision 5: Verification should cover shared consumers

**Decision**: Run Rust core tests/checks, shared TypeScript package checks, and AW checks.

**Rationale**: `crates/git-core` and `packages/git-ui` are shared by multiple apps. The constitution requires cross-app verification for shared core changes.

**Alternatives considered**:

- Verify only AW manually: rejected because it would miss regressions in shared contracts and Git Explorer consumers.
