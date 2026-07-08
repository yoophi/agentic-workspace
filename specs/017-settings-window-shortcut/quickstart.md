# Quickstart: 설정 별도 창과 단축어 실행

## Prerequisites

- Install dependencies with `pnpm install` if needed.
- Use the active feature artifacts under `specs/017-settings-window-shortcut`.
- Run from the repository root unless a command specifies `--dir`.

## Validation Commands

Run frontend verification:

```bash
pnpm --dir apps/agentic-workbench check-types
pnpm --dir apps/agentic-workbench test
```

Run Tauri backend verification:

```bash
cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml
```

Run targeted formatting checks for touched Rust files:

```bash
rustfmt --edition 2024 --check apps/agentic-workbench/src-tauri/src/lib.rs apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs
```

Optional visual verification:

```bash
pnpm --dir apps/agentic-workbench storybook
pnpm --dir apps/agentic-workbench tauri:dev
```

## Scenario 1: toolbar opens settings window

1. Start Agentic Workbench.
2. From the main project dashboard, click the Settings button.
3. Expected: a separate Settings window opens.
4. Expected: the main dashboard remains visible in the original window.

## Scenario 2: `Cmd+,` opens or focuses settings

1. Focus the main window.
2. Press `Cmd+,`.
3. Expected: the Settings window opens within 1 second.
4. Press `Cmd+,` five more times.
5. Expected: the existing Settings window is focused and no duplicate settings windows are created.

## Scenario 3: settings from worktree session preserves work state

1. Open a worktree session page.
2. Type a prompt draft or select a non-default panel.
3. Trigger a settings action from the session flow.
4. Expected: the Settings window opens separately.
5. Close the Settings window.
6. Expected: the worktree session, selected panel, and prompt draft remain as they were.

## Scenario 4: minimized settings window is reused

1. Open the Settings window.
2. Minimize it or place it behind another window.
3. Press `Cmd+,` or click Settings again.
4. Expected: the existing Settings window is restored/focused instead of creating a new one.

## Scenario 5: settings save behavior remains unchanged

1. Open Settings in the dedicated window.
2. Change an agent command override or environment variable setting.
3. Save.
4. Expected: success toast or saved state appears in the Settings window.
5. Start or configure a new agent run that consumes the setting.
6. Expected: the saved setting is reflected without restarting the main work flow.

## Scenario 6: Storybook page states render

1. Start Storybook.
2. Open the page-level Settings stories.
3. Expected: default, loading/error, and long-content states fit within the intended settings-window width.
4. Expected: no text overlaps or clipped controls are visible.

## Expected Artifacts After Implementation

- Tauri command and window manager tests for fixed settings label, route URL, and duplicate prevention.
- Frontend API wrapper and consumer tests for settings open actions.
- `SettingsPage` Storybook page states.
- No migration or new persistent data files.

## Validation Log

- 2026-07-08: `npm exec --yes -- pnpm --dir apps/agentic-workbench test` passed.
- 2026-07-08: `npm exec --yes -- pnpm --dir apps/agentic-workbench check-types` passed.
- 2026-07-08: `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml` passed.
- 2026-07-08: `rustfmt --edition 2024 --check apps/agentic-workbench/src-tauri/src/lib.rs apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs` passed.
- 2026-07-08: Manual Tauri GUI validation was not run in this session; verify toolbar Settings, `Cmd+,`, duplicate prevention, minimized window restore, and session state preservation in a local GUI run.
