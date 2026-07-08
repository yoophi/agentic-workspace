# Tasks: 설정 별도 창과 단축어 실행

**Input**: Design documents from `specs/017-settings-window-shortcut/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/settings-window-ui.md, quickstart.md

**Tests**: User-journey tests are included because this feature changes a Tauri app-shell safety boundary and the constitution requires Rust tests for affected Tauri backend behavior.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **App frontend**: `apps/agentic-workbench/src/{app,pages,features,entities,shared,components/ui}`
- **App Tauri backend**: `apps/agentic-workbench/src-tauri/src/{domain,application,inbound,infrastructure,ports}`
- **Tauri capability**: `apps/agentic-workbench/src-tauri/capabilities/default.json`
- **Storybook**: `apps/agentic-workbench/src/stories`
- **Documentation**: `docs/*.md` with English file names and Korean content

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm current settings routing, command permissions, and story coverage before changing behavior.

- [X] T001 Inspect current settings route, settings entrypoints, and Tauri command capability entries in `apps/agentic-workbench/src/app/App.tsx`, `apps/agentic-workbench/src-tauri/capabilities/default.json`, and `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T002 [P] Inspect existing Tauri window manager and native menu patterns in `apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs` and `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T003 [P] Inspect existing SettingsPage data loading and Storybook page registration in `apps/agentic-workbench/src/pages/settings/ui/settings-page.tsx` and `apps/agentic-workbench/src/stories/pages.stories.tsx`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Add shared names and frontend command boundary that all user stories use.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 Add settings window label and route constants for reuse in `apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs`
- [X] T005 [P] Create the frontend settings window API wrapper `openSettingsWindow()` in `apps/agentic-workbench/src/entities/settings-window/api/settings-window-repository.ts`
- [X] T006 [P] Export a settings window query key or model type only if needed by tests in `apps/agentic-workbench/src/entities/settings-window/model/types.ts`
- [X] T007 Register `open_settings_window` in the Tauri capability allowlist in `apps/agentic-workbench/src-tauri/capabilities/default.json`

**Checkpoint**: Foundation ready - user story implementation can now begin.

---

## Phase 3: User Story 1 - 설정을 별도 창에서 열기 (Priority: P1) MVP

**Goal**: Settings opens in a separate dedicated window without changing the current main/session work state.

**Independent Test**: From the main window, invoke the settings open action and verify a dedicated Settings window appears while the original window route and session state remain unchanged.

### Tests for User Story 1

- [X] T008 [P] [US1] Add Rust unit tests for settings window route, fixed label, and non-`session-` label invariant in `apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs`
- [X] T009 [P] [US1] Add frontend tests for `openSettingsWindow()` invoking `open_settings_window` without payload in `apps/agentic-workbench/src/entities/settings-window/api/settings-window-repository.test.ts`
- [X] T010 [P] [US1] Add SettingsPage window-mode rendering tests for no back navigation requirement and save-error visibility in `apps/agentic-workbench/src/pages/settings/ui/settings-page.test.tsx`

### Implementation for User Story 1

- [X] T011 [US1] Implement `settings_url()` and `open_settings_window()` with fixed `settings` label, restore/show/focus behavior, and `index.html#/settings-window` route in `apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs`
- [X] T012 [US1] Add inbound `open_settings_window` Tauri command that delegates to `window_manager::open_settings_window` in `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`
- [X] T013 [US1] Register `open_settings_window` in the Tauri invoke handler imports and `generate_handler!` list in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T014 [US1] Add `/settings-window` route rendering `SettingsPage` in window mode in `apps/agentic-workbench/src/app/App.tsx`
- [X] T015 [US1] Update `SettingsPage` props to support dedicated window mode without `returnTo` back navigation in `apps/agentic-workbench/src/pages/settings/ui/settings-page.tsx`
- [X] T016 [US1] Wire the top-level Settings button to `openSettingsWindow()` and preserve existing error display behavior in `apps/agentic-workbench/src/app/App.tsx`
- [X] T017 [US1] Verify the settings window close path does not trigger session cleanup by keeping cleanup scoped to `session-` labels in `apps/agentic-workbench/src-tauri/src/lib.rs`

**Checkpoint**: User Story 1 should be fully functional and testable independently.

---

## Phase 4: User Story 2 - `Cmd+,` 단축어로 설정 열기 (Priority: P2)

**Goal**: The macOS native Preferences shortcut opens or focuses the single Settings window from active app windows.

**Independent Test**: Focus the app, press `Cmd+,`, and verify the Settings window opens or the existing Settings window comes forward without inserting text into focused inputs.

### Tests for User Story 2

- [X] T018 [P] [US2] Add Rust tests for Preferences menu id and accelerator construction in `apps/agentic-workbench/src-tauri/src/lib.rs`

### Implementation for User Story 2

- [X] T019 [US2] Add a `Preferences...` menu item id with `CmdOrCtrl+,` accelerator to the macOS app menu in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T020 [US2] Handle the Preferences menu event by calling `window_manager::open_settings_window` in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T021 [US2] Ensure menu open failures are surfaced consistently with existing app-shell error patterns in `apps/agentic-workbench/src-tauri/src/lib.rs`

**Checkpoint**: User Stories 1 and 2 should both work independently.

---

## Phase 5: User Story 3 - 기존 설정 진입점의 일관성 유지 (Priority: P3)

**Goal**: Existing settings entrypoints all use the same dedicated settings window behavior and do not navigate the current work surface away.

**Independent Test**: Open settings from existing toolbar/session/error actions and verify each path opens or focuses the same Settings window with no duplicate windows and no main/session route reset.

### Tests for User Story 3

- [X] T022 [P] [US3] Add App routing/entrypoint tests that existing settings actions call `openSettingsWindow()` instead of navigating to `/settings` in `apps/agentic-workbench/src/app/App.test.tsx`
- [X] T023 [P] [US3] Add agent run settings error action test coverage for `onOpenSettings` preserving the current panel state in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`

### Implementation for User Story 3

- [X] T024 [US3] Replace worktree session `onOpenSettings` route navigation with `openSettingsWindow()` in `apps/agentic-workbench/src/app/App.tsx`
- [X] T025 [US3] Remove obsolete `/settings?returnTo=...` usage and embedded settings route behavior from `apps/agentic-workbench/src/app/App.tsx`
- [X] T026 [US3] Ensure `ProjectWorktreeSessionPage` passes settings actions through without changing session route or prompt state in `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`
- [X] T027 [US3] Ensure `WorktreeAgentRunArea` and `AgentRunPanel` keep existing `onOpenSettings` behavior but receive the new dedicated window opener in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx` and `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T028 [US3] Confirm settings save invalidation still uses `APP_COMMAND_OVERRIDE_SETTINGS_KEY` and existing query keys in `apps/agentic-workbench/src/pages/settings/ui/settings-page.tsx`

**Checkpoint**: All user stories should now be independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Finish visual coverage, documentation checks, and implementation verification across all stories.

- [X] T029 [P] Add SettingsPage Storybook page stories for default, loading/error, and long-content window states in `apps/agentic-workbench/src/stories/pages.stories.tsx`
- [X] T030 [P] Check existing docs for settings navigation references and update Korean documentation if needed in `docs/settings-window.md`
- [X] T031 Run frontend verification commands from quickstart in `specs/017-settings-window-shortcut/quickstart.md`
- [X] T032 Run Tauri backend tests and targeted rustfmt commands from quickstart in `specs/017-settings-window-shortcut/quickstart.md`
- [X] T033 Run manual Tauri validation for toolbar, `Cmd+,`, duplicate prevention, minimized window restore, and session state preservation from `specs/017-settings-window-shortcut/quickstart.md`
- [X] T034 Update the quickstart validation log with executed commands, pass/fail results, and deviations in `specs/017-settings-window-shortcut/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion - blocks all user stories.
- **User Story 1 (Phase 3)**: Depends on Foundational completion and is the MVP.
- **User Story 2 (Phase 4)**: Depends on US1 because the menu shortcut must call the same settings window opener.
- **User Story 3 (Phase 5)**: Depends on US1 because existing entrypoints must reuse the same settings window opener.
- **Polish (Phase 6)**: Depends on all desired user stories being complete.

### User Story Dependencies

- **US1 (P1)**: Required MVP. No dependency on US2 or US3.
- **US2 (P2)**: Requires US1's backend settings window opener.
- **US3 (P3)**: Requires US1's frontend `openSettingsWindow()` wrapper and route separation.

### Within Each User Story

- Tests listed for a story should be written before implementation.
- Backend window manager behavior before inbound command wiring.
- Inbound command registration before frontend invoke wrapper consumers.
- Page route/mode before replacing existing entrypoints.
- Story complete before moving to the next priority unless staffed separately.

### Parallel Opportunities

- T002 and T003 can run in parallel with T001 after repository context is available.
- T005 and T006 can run in parallel with T004 because they touch frontend files.
- T008, T009, and T010 can run in parallel because they cover different files.
- US2 menu tests can be prepared while US1 implementation is being reviewed, but US2 implementation depends on US1 opener.
- T022 and T023 can run in parallel because they cover App and agent-run UI separately.
- T029 and T030 can run in parallel after stories are implemented.

---

## Parallel Example: User Story 1

```bash
Task: "T008 [US1] Add Rust unit tests for settings window route, fixed label, and non-session label invariant in apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs"
Task: "T009 [US1] Add frontend tests for openSettingsWindow() in apps/agentic-workbench/src/entities/settings-window/api/settings-window-repository.test.ts"
Task: "T010 [US1] Add SettingsPage window-mode rendering tests in apps/agentic-workbench/src/pages/settings/ui/settings-page.test.tsx"
```

## Parallel Example: User Story 3

```bash
Task: "T022 [US3] Add App routing/entrypoint tests in apps/agentic-workbench/src/app/App.test.tsx"
Task: "T023 [US3] Add agent run settings error action test coverage in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational.
3. Complete Phase 3: User Story 1.
4. Stop and validate: toolbar Settings opens a separate window and the current work surface remains intact.

### Incremental Delivery

1. Add US1 to create and render the dedicated Settings window.
2. Add US2 to expose native macOS `Cmd+,` behavior.
3. Add US3 to migrate all existing settings entrypoints to the same opener.
4. Finish Storybook, docs check, quickstart verification, and validation log.

### Parallel Team Strategy

1. One engineer handles Tauri window manager/inbound command work.
2. One engineer handles frontend route/page/API wrapper work.
3. One engineer handles Storybook/tests once the public props and command wrapper are stable.

---

## Notes

- [P] tasks touch different files and can run in parallel.
- [US1], [US2], and [US3] labels map to spec.md user stories.
- Avoid adding shared packages or crates for this app-local feature.
- Keep generated shadcn/ui components under `apps/agentic-workbench/src/components/ui` if any are needed.
- Do not change settings persistence semantics or introduce migration for this feature.
