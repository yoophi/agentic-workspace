# Tasks: About 메뉴 앱 정보 표시

**Input**: Design documents from `specs/004-about-menu-app-info/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/about-menu-ui-contract.md](./contracts/about-menu-ui-contract.md), [quickstart.md](./quickstart.md)

**Tests**: User-journey tests are optional for this native menu feature. Constitution-required verification is Rust formatting/checking for the affected Tauri app plus documented manual native menu validation.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Every task includes an exact file path

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the target app shell, metadata sources, and validation documents before user-story implementation.

- [X] T001 Inspect current Tauri app setup and existing plugin registrations in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T002 [P] Inspect official app version source in `apps/agentic-workbench/package.json`
- [X] T003 [P] Inspect build script and manifest dependencies in `apps/agentic-workbench/src-tauri/build.rs` and `apps/agentic-workbench/src-tauri/Cargo.toml`
- [X] T004 [P] Inspect validation expectations in `specs/004-about-menu-app-info/quickstart.md` and `specs/004-about-menu-app-info/contracts/about-menu-ui-contract.md`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Add shared app metadata plumbing required by all user stories.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T005 Add build-time extraction of `apps/agentic-workbench/package.json` version in `apps/agentic-workbench/src-tauri/build.rs`
- [X] T006 Add build-time commit hash resolution with CI environment variable priority and local Git fallback in `apps/agentic-workbench/src-tauri/build.rs`
- [X] T007 Add missing build script dependency required for package metadata parsing in `apps/agentic-workbench/src-tauri/Cargo.toml`
- [X] T008 Expose app name, version, commit hash, and commit fallback constants for app shell use in `apps/agentic-workbench/src-tauri/src/lib.rs`

**Checkpoint**: Build metadata is available to the app shell without adding persistence, shared packages, or frontend state.

---

## Phase 3: User Story 1 - 앱 정보 확인 경로 발견 (Priority: P1) MVP

**Goal**: 사용자가 `agentic-workbench` native menu에서 About 항목을 찾고 선택해 앱 정보 dialog를 열 수 있다.

**Independent Test**: 앱을 실행한 뒤 native menu에서 About 항목을 확인하고 선택했을 때 정보 dialog가 열리는지 확인한다.

### Implementation for User Story 1

- [X] T009 [US1] Add native menu builder wiring to Tauri app setup in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T010 [US1] Add `About Agentic Workbench` menu item with platform-appropriate placement in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T011 [US1] Register menu event handling that opens the About dialog when the About item is selected in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T012 [US1] Preserve existing menu behaviors for edit, window, close, fullscreen, hide, services, and quit items in `apps/agentic-workbench/src-tauri/src/lib.rs`

**Checkpoint**: User Story 1 is independently testable; About can be discovered and opened from the native menu.

---

## Phase 4: User Story 2 - 버전과 빌드 식별자 확인 (Priority: P1)

**Goal**: About dialog에 앱 이름, 공식 버전, 빌드 commit hash가 명확한 라벨과 일관된 순서로 표시된다.

**Independent Test**: About dialog를 열고 `Agentic Workbench`, `Version`, `Commit` 정보가 표시되며 version이 `package.json` 값과 일치하는지 확인한다.

### Implementation for User Story 2

- [X] T013 [US2] Format About dialog title and content according to the UI contract in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T014 [US2] Connect displayed version value to build-time `package.json` metadata in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T015 [US2] Connect displayed commit value to build-time commit metadata in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T016 [US2] Ensure dev and release builds use identical About labels and ordering in `apps/agentic-workbench/src-tauri/src/lib.rs`

**Checkpoint**: User Story 2 is independently testable; About displays the required app identity and build information.

---

## Phase 5: User Story 3 - 빌드 식별자 누락 환경에서도 안정적으로 확인 (Priority: P2)

**Goal**: commit hash를 확인할 수 없는 환경에서도 About dialog가 깨지지 않고 명확한 fallback 값을 표시한다.

**Independent Test**: commit hash resolution이 비어 있거나 실패하는 조건에서도 About dialog가 열리고 `Commit: unknown` 같은 fallback 값을 표시하는지 확인한다.

### Implementation for User Story 3

- [X] T017 [US3] Add blank or missing commit hash fallback handling in `apps/agentic-workbench/src-tauri/src/lib.rs`
- [X] T018 [US3] Add build script fallback for unavailable Git metadata in `apps/agentic-workbench/src-tauri/build.rs`
- [X] T019 [US3] Verify About dialog open and close behavior does not mutate project, worktree, run, prompt, permission, or session state in `apps/agentic-workbench/src-tauri/src/lib.rs`

**Checkpoint**: All user stories are independently functional, including no-commit fallback behavior.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, formatting, and validation that apply across all stories.

- [X] T020 [P] Document native About menu manual validation in Korean in `docs/native-about-menu-verification.md`
- [X] T021 [P] Run Rust formatting for `apps/agentic-workbench/src-tauri` with `cargo fmt --package agentic-workbench`
- [X] T022 [P] Run Rust compile validation for `apps/agentic-workbench/src-tauri` with `cargo check -p agentic-workbench`
- [X] T023 [P] Run frontend type validation for `apps/agentic-workbench` when dependencies are installed with `pnpm --filter @yoophi/agentic-workbench check-types`
- [X] T024 Validate manual scenarios from `specs/004-about-menu-app-info/quickstart.md`
- [X] T025 Verify no app-to-app imports, new shared packages, or persistence/session mutations were introduced in `apps/agentic-workbench/src-tauri/src/lib.rs`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup completion; blocks all user stories.
- **User Story 1 (Phase 3)**: Depends on Foundational completion; MVP scope.
- **User Story 2 (Phase 4)**: Depends on Foundational completion and can be validated after About can be opened.
- **User Story 3 (Phase 5)**: Depends on Foundational completion and can be validated after About display formatting exists.
- **Polish (Phase 6)**: Depends on the desired user stories being complete.

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational. Delivers the native menu entry point and dialog opening.
- **US2 (P1)**: Can start after Foundational, but final validation needs US1 dialog opening.
- **US3 (P2)**: Can start after Foundational, but final validation needs US2 content formatting.

### Within Each User Story

- Build metadata plumbing before dialog content display.
- Menu item creation before menu event handling validation.
- Dialog formatting before fallback behavior validation.
- Documentation and checks after implementation tasks.

### Parallel Opportunities

- T002, T003, and T004 can run in parallel during setup.
- T020, T021, T022, and T023 can run in parallel after implementation.
- US1 menu wiring and US2 metadata formatting touch the same `lib.rs`, so avoid editing that file in parallel unless changes are coordinated.

---

## Parallel Example: User Story 1

```bash
Task: "Add native menu builder wiring to Tauri app setup in apps/agentic-workbench/src-tauri/src/lib.rs"
Task: "Inspect validation expectations in specs/004-about-menu-app-info/contracts/about-menu-ui-contract.md"
```

## Parallel Example: User Story 2

```bash
Task: "Connect displayed version value to build-time package.json metadata in apps/agentic-workbench/src-tauri/src/lib.rs"
Task: "Run Rust compile validation for the affected Tauri app with cargo check -p agentic-workbench"
```

## Parallel Example: User Story 3

```bash
Task: "Add build script fallback for unavailable Git metadata in apps/agentic-workbench/src-tauri/build.rs"
Task: "Document native About menu manual validation in Korean in docs/native-about-menu-verification.md"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational metadata plumbing.
3. Complete Phase 3: User Story 1.
4. Stop and validate that the native About menu exists and opens a dialog.

### Incremental Delivery

1. Add US1 native menu entry and dialog opening.
2. Add US2 version and commit display.
3. Add US3 fallback stability.
4. Complete Polish validation and documentation.

### Single-Developer Strategy

Because the main implementation file is `apps/agentic-workbench/src-tauri/src/lib.rs`, prefer sequential implementation in task order to avoid conflicts. Parallelize only setup inspection, documentation, and verification tasks that touch different files.

## Notes

- [P] tasks use different files or are independent verification/documentation tasks.
- [US1], [US2], and [US3] labels map directly to the user stories in `spec.md`.
- This task list intentionally avoids shared package/crate work because the feature is app-local.
- Storybook tasks are omitted because native menu/dialog behavior is not a reusable React UI contract.
