# Tasks: AW Window Menu List

**Input**: Design documents from `/specs/018-aw-window-menu-list/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/native-window-menu.md](./contracts/native-window-menu.md), [quickstart.md](./quickstart.md)

**Tests**: User-journey tests are optional, but constitution-required Rust unit tests for pure menu ID, fallback title, state mapping, and safety handling are required.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel because it touches different files and has no dependency on incomplete tasks in the same phase.
- **[Story]**: User story label (`US1`, `US2`, `US3`) for story phases only.
- Every task includes an exact repository-relative file path.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the module boundaries selected in the implementation plan.

- [x] T001 Create `pub mod window_menu;` module declarations in `apps/agentic-workbench/src-tauri/src/domain/mod.rs` and `apps/agentic-workbench/src-tauri/src/application/mod.rs`
- [x] T002 [P] Create empty native menu adapter module and export it from `apps/agentic-workbench/src-tauri/src/infrastructure/mod.rs`
- [x] T003 [P] Add `apps/agentic-workbench/src-tauri/src/domain/window_menu.rs` skeleton with public types named in `specs/018-aw-window-menu-list/data-model.md`
- [x] T004 [P] Add `apps/agentic-workbench/src-tauri/src/application/window_menu_service.rs` skeleton for menu state and focus command orchestration
- [x] T005 [P] Add `apps/agentic-workbench/src-tauri/src/infrastructure/native_window_menu.rs` skeleton for Tauri native `Window` submenu synchronization

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Implement testable core contracts that every user story depends on.

**Critical**: No user story work should begin until menu IDs, title fallback, and window snapshot mapping are stable.

- [x] T006 [P] Add failing Rust unit tests for `window-focus:<label>` menu ID creation/parsing in `apps/agentic-workbench/src-tauri/src/domain/window_menu.rs`
- [x] T007 [P] Add failing Rust unit tests for blank/control-character title fallback behavior in `apps/agentic-workbench/src-tauri/src/domain/window_menu.rs`
- [x] T008 [P] Add failing Rust unit tests for mapping AW window snapshots to stable menu items in `apps/agentic-workbench/src-tauri/src/application/window_menu_service.rs`
- [x] T009 Implement `WindowKind`, `WindowMenuEntry`, `WindowMenuState`, and menu ID helpers in `apps/agentic-workbench/src-tauri/src/domain/window_menu.rs`
- [x] T010 Implement `WindowMenuService` snapshot-to-state mapping and deterministic item ordering in `apps/agentic-workbench/src-tauri/src/application/window_menu_service.rs`
- [x] T011 Wire new modules into `apps/agentic-workbench/src-tauri/src/lib.rs` imports without changing runtime behavior yet

**Checkpoint**: Foundation ready. User story implementation can now begin.

---

## Phase 3: User Story 1 - 열린 AW 창을 메뉴에서 확인한다 (Priority: P1) MVP

**Goal**: Native `Window` 메뉴에서 현재 열린 AW 창 목록을 확인할 수 있다.

**Independent Test**: AW 창을 2개 이상 연 뒤 native `Window` 메뉴에 각 창이 최신 제목으로 표시되는지 확인한다.

### Tests for User Story 1

- [x] T012 [P] [US1] Add failing unit tests for native menu item model conversion in `apps/agentic-workbench/src-tauri/src/infrastructure/native_window_menu.rs`
- [x] T013 [P] [US1] Add failing unit tests for including main, settings, and session window labels in `apps/agentic-workbench/src-tauri/src/application/window_menu_service.rs`

### Implementation for User Story 1

- [x] T014 [US1] Implement Tauri menu item construction for `WindowMenuState` in `apps/agentic-workbench/src-tauri/src/infrastructure/native_window_menu.rs`
- [x] T015 [US1] Refactor `build_native_menu` to expose a stable `Window` submenu target in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [x] T016 [US1] Add `sync_window_menu` function that collects `app.webview_windows()` and rebuilds AW window items in `apps/agentic-workbench/src-tauri/src/infrastructure/native_window_menu.rs`
- [x] T017 [US1] Call initial native window menu synchronization during Tauri setup in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [x] T018 [US1] Call native window menu synchronization after session window creation in `apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs`
- [x] T019 [US1] Call native window menu synchronization after settings window creation or reuse in `apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs`

**Checkpoint**: US1 is complete when opened AW windows appear in native `Window` menu without enabling menu-based focusing yet.

---

## Phase 4: User Story 2 - 메뉴에서 원하는 창으로 전환한다 (Priority: P2)

**Goal**: Native `Window` 메뉴 항목 선택으로 대상 AW 창을 전면 이동하거나 포커스할 수 있다.

**Independent Test**: 배경 또는 최소화된 AW 창을 native `Window` 메뉴에서 선택하면 해당 창이 활성화된다.

### Tests for User Story 2

- [x] T020 [P] [US2] Add failing unit tests for valid and invalid focus command parsing in `apps/agentic-workbench/src-tauri/src/application/window_menu_service.rs`
- [x] T021 [P] [US2] Add failing unit tests for stale target labels returning safe no-op focus results in `apps/agentic-workbench/src-tauri/src/infrastructure/native_window_menu.rs`

### Implementation for User Story 2

- [x] T022 [US2] Implement focus command creation from `window-focus:<label>` menu item IDs in `apps/agentic-workbench/src-tauri/src/application/window_menu_service.rs`
- [x] T023 [US2] Add `focus_window_by_label` helper that unminimizes, shows, and focuses a target window in `apps/agentic-workbench/src-tauri/src/infrastructure/window_manager.rs`
- [x] T024 [US2] Route `window-focus:` menu events to the focus helper in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [x] T025 [US2] Re-synchronize native window menu after every handled focus menu event in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [x] T026 [US2] Ensure stale or unknown window menu item IDs are ignored without user-visible error dialogs in `apps/agentic-workbench/src-tauri/src/lib.rs`

**Checkpoint**: US2 is complete when a user can select a background or minimized AW window from the native menu and activate it.

---

## Phase 5: User Story 3 - 창 목록이 현재 상태와 동기화된다 (Priority: P3)

**Goal**: 창 생성, 닫힘, 제목 변경 후 native `Window` 메뉴가 최신 창 목록과 제목을 표시한다.

**Independent Test**: 창 생성, 창 닫힘, title control로 제목 변경을 각각 수행한 뒤 native `Window` 메뉴 상태가 현재 창 상태와 일치하는지 확인한다.

### Tests for User Story 3

- [x] T027 [P] [US3] Add failing unit tests for destroyed-window cleanup sync behavior in `apps/agentic-workbench/src-tauri/src/infrastructure/native_window_menu.rs`
- [x] T028 [P] [US3] Add failing unit tests for title-change sync invocation behavior in `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/mod.rs`
- [x] T029 [P] [US3] Add or update frontend title helper tests for duplicate and changed titles in `apps/agentic-workbench/src/entities/project/lib/worktree-window-title.test.ts`

### Implementation for User Story 3

- [x] T030 [US3] Re-synchronize native window menu on `WindowEvent::Destroyed` after session cleanup in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [x] T031 [US3] Re-synchronize native window menu after MCP `set_window_title` successfully calls `window.set_title` in `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/mod.rs`
- [x] T032 [US3] Add a narrow frontend-to-backend title sync command only if `getCurrentWindow().setTitle(windowTitle)` changes are not observable by backend menu sync in `apps/agentic-workbench/src/app/App.tsx`
- [x] T033 [US3] Register any new title sync command in `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs` only if T032 requires a command boundary
- [x] T034 [US3] Keep menu entries unique for duplicate titles by preserving label-based target IDs in `apps/agentic-workbench/src-tauri/src/domain/window_menu.rs`

**Checkpoint**: US3 is complete when opened, closed, and renamed AW windows are reflected in the native `Window` menu.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Verification, cleanup, and final native behavior checks across all stories.

- [x] T035 [P] Run Rust unit tests for window menu logic with `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml window_menu`
- [x] T036 [P] Run full AW Tauri Rust tests with `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml`
- [x] T037 [P] Run AW frontend tests with `pnpm --dir apps/agentic-workbench test`
- [x] T038 [P] Run AW frontend typecheck with `pnpm --dir apps/agentic-workbench check-types`
- [x] T039 Run rustfmt check for changed backend files listed in `specs/018-aw-window-menu-list/quickstart.md`
- [x] T040 Verify no `packages/*`, `crates/*`, or cross-app imports were introduced for this AW-only feature in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [x] T041 Run manual native menu validation scenarios 1 through 4 from `specs/018-aw-window-menu-list/quickstart.md`
- [x] T042 Update implementation notes in `specs/018-aw-window-menu-list/quickstart.md` with the commands run and pass/fail results

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 Setup**: No dependencies.
- **Phase 2 Foundational**: Depends on Phase 1 and blocks all user stories.
- **Phase 3 US1**: Depends on Phase 2 and is the MVP.
- **Phase 4 US2**: Depends on Phase 2, can begin before US1 is complete if native menu item IDs and sync adapter contracts are stable.
- **Phase 5 US3**: Depends on Phase 2, but title/close wiring is safest after US1 menu sync exists.
- **Phase 6 Polish**: Depends on all implemented stories targeted for delivery.

### User Story Dependencies

- **US1 (P1)**: Independent after Foundation. Delivers visible window list.
- **US2 (P2)**: Independent after Foundation for focus command parsing, integrates with US1-created menu items for full UX.
- **US3 (P3)**: Independent after Foundation for lifecycle sync rules, integrates with US1 sync adapter and US2 stale selection safety.

### Within Each User Story

- Write constitution-required failing tests before implementation.
- Domain helpers before application service logic.
- Application focus/sync commands before Tauri `lib.rs` menu event wiring.
- Infrastructure adapter before lifecycle hook integration.
- Each checkpoint must be manually verifiable before moving to the next priority story.

## Parallel Opportunities

- T002, T003, T004, and T005 can run in parallel after T001 is understood.
- T006, T007, and T008 can run in parallel because they touch different test scopes.
- US1 tests T012 and T013 can run in parallel.
- US2 tests T020 and T021 can run in parallel.
- US3 tests T027, T028, and T029 can run in parallel.
- Final verification tasks T035 through T038 can run in parallel.

## Parallel Example: User Story 1

```bash
Task: "T012 [P] [US1] Add failing unit tests for native menu item model conversion in apps/agentic-workbench/src-tauri/src/infrastructure/native_window_menu.rs"
Task: "T013 [P] [US1] Add failing unit tests for including main, settings, and session window labels in apps/agentic-workbench/src-tauri/src/application/window_menu_service.rs"
```

## Parallel Example: User Story 2

```bash
Task: "T020 [P] [US2] Add failing unit tests for valid and invalid focus command parsing in apps/agentic-workbench/src-tauri/src/application/window_menu_service.rs"
Task: "T021 [P] [US2] Add failing unit tests for stale target labels returning safe no-op focus results in apps/agentic-workbench/src-tauri/src/infrastructure/native_window_menu.rs"
```

## Parallel Example: User Story 3

```bash
Task: "T027 [P] [US3] Add failing unit tests for destroyed-window cleanup sync behavior in apps/agentic-workbench/src-tauri/src/infrastructure/native_window_menu.rs"
Task: "T028 [P] [US3] Add failing unit tests for title-change sync invocation behavior in apps/agentic-workbench/src-tauri/src/infrastructure/mcp/mod.rs"
Task: "T029 [P] [US3] Add or update frontend title helper tests for duplicate and changed titles in apps/agentic-workbench/src/entities/project/lib/worktree-window-title.test.ts"
```

## Implementation Strategy

### MVP First (US1 Only)

1. Complete Phase 1 setup.
2. Complete Phase 2 foundational domain/application contracts.
3. Complete Phase 3 US1.
4. Stop and validate that multiple AW windows appear in native `Window` menu.

### Incremental Delivery

1. Add US1 to make the menu list visible.
2. Add US2 to make listed windows selectable and focusable.
3. Add US3 to keep the list current across create/close/title changes.
4. Run Phase 6 verification and manual native menu validation.

### Parallel Team Strategy

1. Complete Phase 1 and Phase 2 together.
2. Assign US1 adapter/list rendering, US2 focus behavior, and US3 lifecycle/title sync to separate implementers after foundation stabilizes.
3. Merge in priority order to keep MVP reviewable.

## Notes

- `[P]` tasks use different files or independent test scopes.
- Story labels map to the prioritized user stories in [spec.md](./spec.md).
- No Storybook tasks are included because this feature adds no reusable frontend UI component.
