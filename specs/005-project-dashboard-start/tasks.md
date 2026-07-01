# Tasks: 프로젝트 대시보드 시작화면

**Input**: Design documents from `specs/005-project-dashboard-start/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/dashboard-ui-contract.md](./contracts/dashboard-ui-contract.md), [quickstart.md](./quickstart.md)

**Tests**: User-journey tests are optional unless requested by the spec. Constitution-required tests are NOT optional: pure dashboard summary/ranking helpers require Vitest coverage, and dashboard UI states require Storybook examples.

**Organization**: Tasks are grouped by user story so each story can be implemented, tested, and reviewed independently after the shared foundation is complete.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish the AW-local dashboard file locations and reusable fixtures without changing runtime behavior.

- [X] T001 Create the dashboard page directory and placeholder page component in `apps/agentic-workbench/src/pages/project-dashboard/ui/project-dashboard-page.tsx`
- [X] T002 [P] Create the dashboard feature UI directory and placeholder actions component in `apps/agentic-workbench/src/features/project-dashboard/ui/project-dashboard-actions.tsx`
- [X] T003 [P] Add dashboard fixture data placeholders to `apps/agentic-workbench/src/shared/storybook/sample-data.ts`
- [X] T004 [P] Add dashboard story placeholders to `apps/agentic-workbench/src/stories/pages.stories.tsx`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Define the shared dashboard model and pure derivation logic used by all user stories.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T005 [P] Define `ProjectDashboard`, `ProjectDashboardItem`, `DashboardAction`, `SessionSummary`, `WorktreeSummary`, and `ChangeSummary` types in `apps/agentic-workbench/src/entities/project/model/dashboard.ts`
- [X] T006 [P] Add failing Vitest cases for dashboard status mapping, summary degradation, and default action availability in `apps/agentic-workbench/src/entities/project/lib/dashboard-summary.test.ts`
- [X] T007 Implement dashboard summary builders and status mapping helpers in `apps/agentic-workbench/src/entities/project/lib/dashboard-summary.ts`
- [X] T008 Export dashboard model and helper modules from `apps/agentic-workbench/src/entities/project/model/index.ts`
- [X] T009 Wire dashboard fixture builders to the new model types in `apps/agentic-workbench/src/shared/storybook/sample-data.ts`

**Checkpoint**: Foundation ready. Dashboard user stories can now be implemented independently.

---

## Phase 3: User Story 1 - 최근 프로젝트로 즉시 복귀 (Priority: P1) MVP

**Goal**: 앱 시작화면에서 최근 또는 관련성 높은 프로젝트와 주요 상태를 확인하고 프로젝트, worktree, 또는 재개 가능한 세션으로 바로 이동한다.

**Independent Test**: 최근 프로젝트와 세션/worktree/change fixture가 있는 상태에서 `/` dashboard를 렌더링하고, 프로젝트 상태를 확인한 뒤 primary action으로 기존 project detail 또는 session/worktree route에 진입할 수 있으면 통과한다.

### Tests for User Story 1

- [X] T010 [P] [US1] Add failing Vitest cases for recent/relevant project ordering and fallback ordering in `apps/agentic-workbench/src/entities/project/lib/dashboard-summary.test.ts`
- [X] T011 [P] [US1] Add ready-state dashboard Storybook story with recent projects and resumable status in `apps/agentic-workbench/src/stories/pages.stories.tsx`

### Implementation for User Story 1

- [X] T012 [US1] Implement recent/relevant project ordering in `apps/agentic-workbench/src/entities/project/lib/dashboard-summary.ts`
- [X] T013 [P] [US1] Implement the recent project item UI with status badges and non-overlapping long text handling in `apps/agentic-workbench/src/pages/project-dashboard/ui/project-dashboard-page.tsx`
- [X] T014 [P] [US1] Implement resume/open action controls for recent project items in `apps/agentic-workbench/src/features/project-dashboard/ui/project-dashboard-actions.tsx`
- [X] T015 [US1] Compose the ready dashboard layout using project, session, worktree, and change summaries in `apps/agentic-workbench/src/pages/project-dashboard/ui/project-dashboard-page.tsx`
- [X] T016 [US1] Replace the root `/` route from `ProjectListPage` to `ProjectDashboardPage` while preserving existing project detail and session/worktree routes in `apps/agentic-workbench/src/app/App.tsx`
- [X] T017 [US1] Pass existing project selection and worktree/session navigation callbacks from `apps/agentic-workbench/src/app/App.tsx` into `apps/agentic-workbench/src/pages/project-dashboard/ui/project-dashboard-page.tsx`

**Checkpoint**: User Story 1 is fully functional and independently testable as the MVP.

---

## Phase 4: User Story 2 - 주요 프로젝트 작업 시작 (Priority: P2)

**Goal**: 시작화면에서 새 프로젝트 생성, 기존 프로젝트 열기, 최근 세션 재개 같은 주요 action을 별도 탐색 없이 실행한다.

**Independent Test**: 최근 프로젝트 데이터 유무와 관계없이 dashboard 상단 action 영역에서 새 프로젝트 생성, 기존 프로젝트 열기, 재개 가능한 세션 action을 선택했을 때 각 목적지 또는 다음 단계가 명확히 실행되면 통과한다.

### Tests for User Story 2

- [X] T018 [P] [US2] Add failing Vitest cases for quick action availability without optional summaries in `apps/agentic-workbench/src/entities/project/lib/dashboard-summary.test.ts`
- [X] T019 [P] [US2] Add Storybook story that shows dashboard quick actions with and without recent sessions in `apps/agentic-workbench/src/stories/pages.stories.tsx`

### Implementation for User Story 2

- [X] T020 [US2] Add quick action derivation for create project, open existing project, open project, resume session, open worktree, and retry actions in `apps/agentic-workbench/src/entities/project/lib/dashboard-summary.ts`
- [X] T021 [US2] Implement the compact quick action toolbar in `apps/agentic-workbench/src/features/project-dashboard/ui/project-dashboard-actions.tsx`
- [X] T022 [US2] Wire create project and open existing project actions to the existing project form/open flows in `apps/agentic-workbench/src/app/App.tsx`
- [X] T023 [US2] Render quick actions in the dashboard page without duplicating the full project list management UI in `apps/agentic-workbench/src/pages/project-dashboard/ui/project-dashboard-page.tsx`

**Checkpoint**: User Story 2 works independently with or without recent project data.

---

## Phase 5: User Story 3 - 상태별 시작화면 이해 (Priority: P3)

**Goal**: 프로젝트 없음, 로딩, 오류, 부분 요약 실패, 긴 콘텐츠 상태에서도 사용자가 현재 상황과 다음 action을 이해할 수 있다.

**Independent Test**: Storybook에서 empty, loading, error, partial summary, long-content 상태를 각각 렌더링했을 때 다음 action이 보이고, loading/empty/error가 서로 오해되지 않으며, 긴 텍스트가 action을 침범하지 않으면 통과한다.

### Tests for User Story 3

- [X] T024 [P] [US3] Add failing Vitest cases for empty, loading, error, partial summary, and long-content-safe state mapping in `apps/agentic-workbench/src/entities/project/lib/dashboard-summary.test.ts`
- [X] T025 [P] [US3] Add Storybook stories for empty, loading, error, partial summary, and long-content dashboard states in `apps/agentic-workbench/src/stories/pages.stories.tsx`

### Implementation for User Story 3

- [X] T026 [US3] Implement empty, loading, error, partial summary, and unavailable summary presentation in `apps/agentic-workbench/src/pages/project-dashboard/ui/project-dashboard-page.tsx`
- [X] T027 [US3] Add retry action handling for project list load failures in `apps/agentic-workbench/src/app/App.tsx`
- [X] T028 [US3] Add long project name, long path, and long session label fixtures in `apps/agentic-workbench/src/shared/storybook/sample-data.ts`
- [X] T029 [US3] Verify dashboard overflow and full-value reveal behavior using `apps/agentic-workbench/src/shared/ui/ellipsis-popover-text.tsx` where long labels appear in `apps/agentic-workbench/src/pages/project-dashboard/ui/project-dashboard-page.tsx`

**Checkpoint**: All required dashboard states are independently functional and visible in Storybook.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final verification, cleanup, and contract alignment across all user stories.

- [X] T030 [P] Review the implemented dashboard against `specs/005-project-dashboard-start/contracts/dashboard-ui-contract.md` and update `apps/agentic-workbench/src/pages/project-dashboard/ui/project-dashboard-page.tsx` for any missing observable behavior
- [X] T031 [P] Run `pnpm --filter @yoophi/agentic-workbench check-types` and fix TypeScript issues in `apps/agentic-workbench/src`
- [X] T032 [P] Run `pnpm --filter @yoophi/agentic-workbench test` and fix failing dashboard tests in `apps/agentic-workbench/src/entities/project/lib/dashboard-summary.test.ts`
- [X] T033 Build or run Storybook validation for dashboard page states and fix story regressions in `apps/agentic-workbench/src/stories/pages.stories.tsx`
- [X] T034 Run the validation scenarios from `specs/005-project-dashboard-start/quickstart.md` and record any remaining implementation gaps in `specs/005-project-dashboard-start/tasks.md`
- [X] T035 Verify no cross-app imports or shared UI package changes were introduced by reviewing imports in `apps/agentic-workbench/src/pages/project-dashboard/ui/project-dashboard-page.tsx`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies. Can start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion. Blocks all user stories.
- **User Stories (Phase 3+)**: Depend on Foundational completion.
- **Polish (Phase 6)**: Depends on all desired user stories being complete.

### User Story Dependencies

- **User Story 1 (P1)**: Starts after Foundational. No dependency on US2 or US3. This is the MVP.
- **User Story 2 (P2)**: Starts after Foundational. Can be implemented independently, but final app wiring should coexist with US1's route integration.
- **User Story 3 (P3)**: Starts after Foundational. Can be implemented independently against dashboard state fixtures, then integrated with US1/US2 UI.

### Within Each User Story

- Constitution-required tests for pure helper logic must be written before helper implementation.
- Model/types before summary helpers.
- Summary helpers before page composition.
- Page UI before `App.tsx` route wiring where possible.
- Storybook states must be added before final quickstart validation.

---

## Parallel Opportunities

- T002, T003, and T004 can run in parallel after T001 starts.
- T005 and T006 can run in parallel because they touch different files.
- T010 and T011 can run in parallel for US1.
- T013 and T014 can run in parallel after T012 defines the data shape.
- T018 and T019 can run in parallel for US2.
- T024 and T025 can run in parallel for US3.
- T030, T031, and T032 can run in parallel during final verification.

## Parallel Example: User Story 1

```text
Task: "T010 [P] [US1] Add failing Vitest cases for recent/relevant project ordering and fallback ordering in apps/agentic-workbench/src/entities/project/lib/dashboard-summary.test.ts"
Task: "T011 [P] [US1] Add ready-state dashboard Storybook story with recent projects and resumable status in apps/agentic-workbench/src/stories/pages.stories.tsx"
Task: "T013 [P] [US1] Implement the recent project item UI with status badges and non-overlapping long text handling in apps/agentic-workbench/src/pages/project-dashboard/ui/project-dashboard-page.tsx"
Task: "T014 [P] [US1] Implement resume/open action controls for recent project items in apps/agentic-workbench/src/features/project-dashboard/ui/project-dashboard-actions.tsx"
```

## Parallel Example: User Story 2

```text
Task: "T018 [P] [US2] Add failing Vitest cases for quick action availability without optional summaries in apps/agentic-workbench/src/entities/project/lib/dashboard-summary.test.ts"
Task: "T019 [P] [US2] Add Storybook story that shows dashboard quick actions with and without recent sessions in apps/agentic-workbench/src/stories/pages.stories.tsx"
```

## Parallel Example: User Story 3

```text
Task: "T024 [P] [US3] Add failing Vitest cases for empty, loading, error, partial summary, and long-content-safe state mapping in apps/agentic-workbench/src/entities/project/lib/dashboard-summary.test.ts"
Task: "T025 [P] [US3] Add Storybook stories for empty, loading, error, partial summary, and long-content dashboard states in apps/agentic-workbench/src/stories/pages.stories.tsx"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 setup.
2. Complete Phase 2 foundation.
3. Complete Phase 3 US1.
4. Validate `/` dashboard with recent projects and primary project/session/worktree actions.
5. Stop and demo the MVP before adding secondary actions or extended states.

### Incremental Delivery

1. Add US1 for recent project resume.
2. Add US2 for quick start actions.
3. Add US3 for empty/loading/error/partial/long-content states.
4. Run quickstart validation and app checks after each increment.

### Parallel Team Strategy

1. Complete Setup and Foundational phases together.
2. Assign US1, US2, and US3 to separate implementers if needed.
3. Keep changes isolated by file ownership: helpers in `entities/project/lib`, actions in `features/project-dashboard/ui`, page rendering in `pages/project-dashboard/ui`, and route composition in `app/App.tsx`.

## Notes

- `[P]` tasks touch different files or can be implemented without waiting for another incomplete task in the same phase.
- `[US#]` labels map directly to the prioritized user stories in `spec.md`.
- Backend command work is intentionally omitted because the plan chooses existing frontend repositories and Tauri commands for v1. If implementation discovers missing backend data, create a follow-up task set that preserves `domain` → `application` → `inbound` → `infrastructure` boundaries.
