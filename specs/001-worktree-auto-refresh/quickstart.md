# Quickstart: Worktree Auto Refresh Validation

## Prerequisites

- Repository root: `/Users/yoophi/project/agentic-workspace`
- A project/worktree registered in `agentic-workbench`
- A repository registered in `git-explorer`
- A markdown file opened in `markdown-annotator`
- Git installed and available to the Tauri app

## Run the App

```bash
pnpm install
pnpm dev:workbench
pnpm dev:git
pnpm dev:annotator
```

For full Tauri validation:

```bash
pnpm tauri:dev:workbench
pnpm tauri:dev:git
pnpm tauri:dev:annotator
```

## Validation Scenario 1: File Tree Auto Refresh

1. Open `ProjectWorktreeSessionPage` for a worktree.
2. Open the `Files` tab.
3. From a terminal, create a file inside the active worktree:

   ```bash
   printf 'hello\n' > path/to/worktree/tmp-auto-refresh.txt
   ```

4. Expected: within 3 seconds, `tmp-auto-refresh.txt` appears in the file tree without page reload.
5. Select the file, then change content:

   ```bash
   printf 'updated\n' > path/to/worktree/tmp-auto-refresh.txt
   ```

6. Expected: preview refreshes to `updated` without losing the file selection.
7. Delete the file:

   ```bash
   rm path/to/worktree/tmp-auto-refresh.txt
   ```

8. Expected: file tree removes it and the preview/selection shows a stale or cleared selection state.

## Validation Scenario 2: Markdown Tree Auto Refresh

1. Open the `Markdown` tab.
2. Create a markdown file:

   ```bash
   printf '# Auto Refresh\n\ncontent\n' > path/to/worktree/tmp-auto-refresh.md
   ```

3. Expected: within 3 seconds, the markdown tree includes the file.
4. Select the file and edit markdown content.
5. Expected: markdown preview updates while annotations/selection UI remain usable.

## Validation Scenario 3: Git Explorer Auto Refresh

1. Open the `Git` tab in `agentic-workbench` and the same repository in `git-explorer`.
2. Create and commit a file in the active worktree:

   ```bash
   printf 'commit test\n' > path/to/worktree/tmp-git-refresh.txt
   git -C path/to/worktree add tmp-git-refresh.txt
   git -C path/to/worktree commit -m "test auto refresh"
   ```

3. Expected: within 3 seconds, the new commit appears in graph/list in both apps.
4. Select an older commit and create another commit.
5. Expected: selected older commit remains selected if it still exists.
6. Switch branch:

   ```bash
   git -C path/to/worktree switch -
   ```

7. Expected: branch/ref/status display and commit list/graph update to the new branch in both apps. If selected commit is not present, detail pane marks it stale.

## Validation Scenario 4: Markdown Annotator Auto Reload

1. Open a markdown file in `markdown-annotator`.
2. Edit the same file externally:

   ```bash
   printf '# Auto Reload\n\nchanged\n' > path/to/document.md
   ```

3. Expected: within 3 seconds, markdown preview reflects the updated content without app reload.
4. Delete or temporarily make the file unreadable.
5. Expected: the app keeps last successful content where possible and marks the document stale or recoverable.

## Validation Scenario 5: Failure Recovery

1. Open `Files` or `Git` tab.
2. Temporarily move or chmod the active worktree to make it unreadable.
3. Expected: pane keeps last successful data when possible and displays a recoverable error.
4. Restore access.
5. Expected: next interval or manual refresh returns pane to current data.

## Automated Checks

Frontend checks:

```bash
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/agentic-workbench test
pnpm --filter @yoophi/git-explorer check-types
pnpm build-storybook:git
pnpm --filter @yoophi/markdown-annotator check-types
pnpm --filter @yoophi/markdown-annotator test
pnpm --filter @yoophi/workspace-auto-refresh test
```

Backend checks, only if Tauri backend code changes:

```bash
cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml
cargo test --manifest-path apps/git-explorer/src-tauri/Cargo.toml
cargo test --manifest-path apps/markdown-annotator/src-tauri/Cargo.toml
```

Shared package changes are planned for `packages/workspace-auto-refresh`; run the package tests and all three consuming app checks.
