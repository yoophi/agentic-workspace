# Tasks: 긴 Permission 다이얼로그 레이아웃 개선

**Input**: Design documents from `/specs/019-improve-permission-dialog/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/permission-dialog-ui.md](./contracts/permission-dialog-ui.md), [quickstart.md](./quickstart.md)

**Tests**: Required by plan.md/quickstart.md and constitution guidance for pure display logic. Write tests before implementation where a task explicitly says "Add failing".

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel because it touches different files and has no dependency on incomplete tasks.
- **[Story]**: User story label (`US1`, `US2`, `US3`) for user story phases only.
- Every task includes an exact repository-relative file path.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the file boundaries selected in plan.md without changing runtime behavior.

- [X] T001 Create `apps/agentic-workbench/src/entities/agent-run/model/permission-display.ts` with exported placeholder types/functions for permission display mapping.
- [X] T002 [P] Create `apps/agentic-workbench/src/entities/agent-run/model/permission-display.test.ts` with an empty `describe("permission display")` suite ready for story tests.
- [X] T003 [P] Create `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.tsx` by moving the existing `PermissionRequestDialog` implementation out of `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`.
- [X] T004 Update `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx` to import and render `PermissionRequestDialog` from the new file without behavior changes.
- [X] T005 [P] Create `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.test.tsx` with shared fixtures for long permission requests.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish the shared display model that all user stories depend on.

**Critical**: No user story implementation should begin until the permission display helper contract is stable.

- [X] T006 Add exported `PermissionDisplayModel`, `CommandDetailDisplay`, and `ApprovalOptionDisplay` types in `apps/agentic-workbench/src/entities/agent-run/model/permission-display.ts`.
- [X] T007 Add `createPermissionDisplayModel(permission)` skeleton and safe fallback helpers in `apps/agentic-workbench/src/entities/agent-run/model/permission-display.ts`.
- [X] T008 Export the permission display helpers from `apps/agentic-workbench/src/entities/agent-run/model/index.ts`.
- [X] T009 Update `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.tsx` to depend on `createPermissionDisplayModel` while preserving current visible behavior.
- [X] T010 [P] Add shared long command, long JSON payload, and long approval option fixtures in `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.test.tsx`.

**Checkpoint**: Foundation ready. User story implementation can now begin.

---

## Phase 3: User Story 1 - 긴 권한 요청을 안정적으로 검토한다 (Priority: P1) MVP

**Goal**: 긴 command, 여러 줄 payload, JSON-like input을 표시해도 dialog가 화면 안에 유지되고 상세 내용을 검토할 수 있다.

**Independent Test**: 매우 긴 command와 여러 줄 payload가 포함된 permission request를 표시했을 때 header, detail region, action region이 모두 렌더링되고 full detail text에 접근 가능하다.

### Tests for User Story 1

- [X] T011 [P] [US1] Add failing tests for JSON-like input stringification, title fallback, and full detail preservation in `apps/agentic-workbench/src/entities/agent-run/model/permission-display.test.ts`.
- [X] T012 [P] [US1] Add failing render tests for long command/detail visibility and separated action region in `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.test.tsx`.

### Implementation for User Story 1

- [X] T013 [US1] Implement robust permission input formatting for strings, objects, arrays, null, and serialization failures in `apps/agentic-workbench/src/entities/agent-run/model/permission-display.ts`.
- [X] T014 [US1] Implement command detail metadata including `isMultiline` and `isLong` in `apps/agentic-workbench/src/entities/agent-run/model/permission-display.ts`.
- [X] T015 [US1] Update `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.tsx` to render title/summary separately from a bounded scrollable detail region.
- [X] T016 [US1] Update `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.tsx` so long single-line text wraps or breaks without expanding dialog width.
- [X] T017 [US1] Verify US1 by running `pnpm --dir apps/agentic-workbench test -- permission-display permission-request-dialog` and record any command limitation in `specs/019-improve-permission-dialog/quickstart.md` if needed.

**Checkpoint**: User Story 1 is independently complete when long permission details are reviewable without displacing action controls.

---

## Phase 4: User Story 2 - 승인 옵션을 빠르게 이해하고 선택한다 (Priority: P2)

**Goal**: 긴 승인 option label은 짧은 button label로 요약하고, 원문 option text와 승인 범위는 상세 영역에서 확인할 수 있다.

**Independent Test**: 긴 approval prefix가 포함된 permission request에서 button label은 짧고 클릭 가능하며, 선택 시 원본 `optionId`가 제출된다.

### Tests for User Story 2

- [X] T018 [P] [US2] Add failing tests for approval option summary, reject detection, command-like label shortening, and empty label fallback in `apps/agentic-workbench/src/entities/agent-run/model/permission-display.test.ts`.
- [X] T019 [P] [US2] Add failing render tests for concise button labels, full option text visibility, pending state, and original `optionId` submission in `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.test.tsx`.

### Implementation for User Story 2

- [X] T020 [US2] Implement approval option summary generation in `apps/agentic-workbench/src/entities/agent-run/model/permission-display.ts`.
- [X] T021 [US2] Implement reject/destructive option detection based on option kind/name in `apps/agentic-workbench/src/entities/agent-run/model/permission-display.ts`.
- [X] T022 [US2] Update `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.tsx` to use summarized button labels and expose full option text in the dialog.
- [X] T023 [US2] Update `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.tsx` to keep pending labels short and preserve stable button sizing while submitting.
- [X] T024 [US2] Verify US2 by running `pnpm --dir apps/agentic-workbench test -- permission-display permission-request-dialog`.

**Checkpoint**: User Story 2 is independently complete when long approval labels no longer break the button row and original option semantics are preserved.

---

## Phase 5: User Story 3 - 좁은 화면에서도 승인 흐름을 완료한다 (Priority: P3)

**Goal**: 360px 수준의 좁은 창에서도 permission request를 읽고 승인 또는 거절을 완료할 수 있다.

**Independent Test**: narrow container에서 긴 permission request를 렌더링했을 때 detail/action 영역이 겹치지 않고 keyboard-accessible controls가 유지된다.

### Tests for User Story 3

- [X] T025 [P] [US3] Add failing narrow-layout render tests for action wrapping/stacking and accessible controls in `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.test.tsx`.
- [X] T026 [P] [US3] Add failing keyboard interaction test for focusing and activating permission actions in `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.test.tsx`.

### Implementation for User Story 3

- [X] T027 [US3] Update `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.tsx` dialog content classes to constrain max width, max height, and viewport-relative sizing.
- [X] T028 [US3] Update `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.tsx` action region classes so buttons wrap or stack cleanly in narrow windows.
- [X] T029 [US3] Update `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.tsx` to preserve keyboard focus order across header, detail region, and action controls.
- [X] T030 [US3] Verify US3 by running `pnpm --dir apps/agentic-workbench test -- permission-request-dialog`.

**Checkpoint**: User Story 3 is independently complete when the dialog remains usable in a 360px-wide story/test container.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Storybook coverage, regression verification, and final cleanup across all stories.

- [X] T031 [P] Add `LongCommand`, `LongJsonPayload`, `LongApprovalLabels`, and `NarrowWindow` PermissionRequestDialog stories in `apps/agentic-workbench/src/stories/organisms.stories.tsx`.
- [X] T032 [P] Update story fixtures or local helpers for permission dialog states in `apps/agentic-workbench/src/stories/organisms.stories.tsx`.
- [X] T033 Run `pnpm --dir apps/agentic-workbench check-types` and fix any type errors in `apps/agentic-workbench/src/entities/agent-run/model/permission-display.ts`, `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.tsx`, or related imports.
- [X] T034 Run `pnpm --dir apps/agentic-workbench test` and fix regressions in `apps/agentic-workbench/src/entities/agent-run/model/permission-display.test.ts` and `apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.test.tsx`.
- [X] T035 Run the manual validation scenarios from `specs/019-improve-permission-dialog/quickstart.md` and update that file only if commands or expected validation steps changed.
- [X] T036 Review `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx` to confirm only dialog wiring remains and no app-to-app imports or backend permission contract changes were introduced.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup completion and blocks all user stories.
- **US1 (Phase 3)**: Depends on Foundational and is the MVP.
- **US2 (Phase 4)**: Depends on Foundational; can run after US1 tests exist, but final UX should be checked with US1 detail region.
- **US3 (Phase 5)**: Depends on Foundational; can run after US1 structure exists.
- **Polish (Phase 6)**: Depends on all selected user stories.

### User Story Dependencies

- **User Story 1 (P1)**: No dependency on other user stories after Foundation.
- **User Story 2 (P2)**: No functional dependency on US3; integrates with US1 detail region for full option text.
- **User Story 3 (P3)**: No functional dependency on US2; relies on the extracted dialog structure from Foundation/US1.

### Parallel Opportunities

- T002, T003, and T005 can run in parallel after T001 is started because they touch different files.
- T011 and T012 can run in parallel for US1.
- T018 and T019 can run in parallel for US2.
- T025 and T026 can run in parallel for US3.
- T031 and T032 can run in parallel with final verification tasks after story implementation is complete.

## Parallel Example: User Story 1

```bash
Task: "Add failing tests for JSON-like input stringification, title fallback, and full detail preservation in apps/agentic-workbench/src/entities/agent-run/model/permission-display.test.ts"
Task: "Add failing render tests for long command/detail visibility and separated action region in apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.test.tsx"
```

## Parallel Example: User Story 2

```bash
Task: "Add failing tests for approval option summary, reject detection, command-like label shortening, and empty label fallback in apps/agentic-workbench/src/entities/agent-run/model/permission-display.test.ts"
Task: "Add failing render tests for concise button labels, full option text visibility, pending state, and original optionId submission in apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.test.tsx"
```

## Parallel Example: User Story 3

```bash
Task: "Add failing narrow-layout render tests for action wrapping/stacking and accessible controls in apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.test.tsx"
Task: "Add failing keyboard interaction test for focusing and activating permission actions in apps/agentic-workbench/src/features/agent-run/ui/permission-request-dialog.test.tsx"
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 setup.
2. Complete Phase 2 display model foundation.
3. Complete Phase 3 US1.
4. Stop and validate long command/detail review with targeted tests.

### Incremental Delivery

1. US1 delivers stable long detail review.
2. US2 improves approval option readability while preserving original option semantics.
3. US3 hardens narrow-window and keyboard approval flow.
4. Polish adds Storybook states and full quickstart verification.

### Notes

- `[P]` tasks touch different files or independent test sections.
- Story labels map directly to prioritized user stories in `spec.md`.
- Backend permission policy, `respond_agent_permission`, and agent protocol changes are intentionally out of scope.
