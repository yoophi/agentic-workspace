# Tasks: SDD 워크플로 단계 표시 및 제어

**Input**: Design documents from `/specs/031-sdd-workflow-controls/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [UI contract](./contracts/sdd-workflow-controls-ui.md), [quickstart.md](./quickstart.md)

**Tests**: 순수 단계/경로/프롬프트 모델은 컨스티튜션에 따라 구현 전에 unit test를 작성한다. UI와 prompt routing은 focused component/model test로 검증한다.

**Organization**: 작업은 사용자 스토리별로 분리하여 P1 단계 표시만 또는 P1 작업 실행만 각각 검증 가능한 증분으로 구현한다.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 선행 작업에 의존하지 않고 다른 파일에서 병렬로 수행 가능
- **[Story]**: 사용자 스토리 작업의 소속 (`US1`, `US2`, `US3`)
- 모든 작업은 정확한 파일 경로를 포함한다.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 현재 Speckit panel, agent prompt routing, Storybook fixture의 기준선을 고정한다.

- [X] T001 기존 query·watcher·prompt routing 결합 지점을 확인하고 구현 파일 목록을 `specs/031-sdd-workflow-controls/plan.md`와 대조한다.
- [X] T002 [P] SDD 단계 및 active pointer fixture를 `apps/agentic-workbench/src/shared/storybook/sample-data.ts`에 추가한다.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 모든 사용자 스토리가 공유하는 안전한 active pointer/단계 모델과 draft delivery contract를 만든다.

**⚠️ CRITICAL**: 이 단계가 완료되기 전에는 Speckit controls나 prompt delivery UI를 연결하지 않는다.

- [X] T003 `ActiveFeaturePointer`, `SddStage`, `SddStageState`, `SddActionRequest`의 타입과 순수 함수 test를 `apps/agentic-workbench/src/features/worktree-workspace/model/sdd-workflow.test.ts`에 작성한다.
- [X] T004 `feature_directory` JSON parsing, relative `specs/<feature>` validation, malformed/absolute/traversal/stale pointer 거부, artifact 기반 단계 계산을 `apps/agentic-workbench/src/features/worktree-workspace/model/sdd-workflow.ts`에 구현한다.
- [X] T005 `send | draft` delivery와 active-panel routing의 호환성 test를 `apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.test.ts`에 추가한다.
- [X] T006 `AgentPromptRequest`의 delivery mode를 backward-compatible하게 확장하고 active-panel routing에서 보존하도록 `apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.ts`를 수정한다.
- [X] T007 draft request가 run/queue/history를 만들지 않고 textarea만 갱신하며 send request의 기존 동작을 보존하는 test를 `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`에 추가한다.
- [X] T008 `draft` delivery를 prompt textarea 주입으로 처리하고 send/queue의 현재 흐름을 유지하도록 `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`를 수정한다.
- [X] T009 `.specify/feature.json` text-file query와 watcher/refresh invalidation key를 `apps/agentic-workbench/src/entities/worktree-file/api/query-keys.ts`에 추가한다.

**Checkpoint**: pointer가 유효하지 않으면 안전하게 unavailable이 되고, draft와 즉시 전송의 실행 의미가 분리된다.

---

## Phase 3: User Story 1 - 현재 SDD 진행 단계를 즉시 파악하기 (Priority: P1) 🎯 MVP

**Goal**: 사용자가 Speckit 탭에서 `.specify/feature.json`이 가리키는 활성 기능을 하이라이트로 식별하고, 4단계 현재/완료/대기 상태를 즉시 확인한다.

**Independent Test**: spec-only, plan 포함, tasks 포함 feature와 여러 Speckit feature를 준비하고, valid pointer가 가리키는 하나의 row만 `현재 작업 중`으로 표시되며 단계 상태가 artifact에 따라 달라지는지 확인한다.

### Tests for User Story 1

- [X] T010 [P] [US1] active feature highlight, accessible label, loading/unavailable/error pointer state의 component test를 `apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.test.tsx`에 작성한다.
- [X] T011 [P] [US1] spec-only/plan/tasks 상태와 stale pointer를 포함한 SDD controls rendering test를 `apps/agentic-workbench/src/features/worktree-workspace/ui/sdd-workflow-controls.test.tsx`에 작성한다.

### Implementation for User Story 1

- [X] T012 [US1] `activeFeaturePath`를 받아 하이라이트와 `현재 작업 중` 접근성 상태를 표시하도록 `apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.tsx`를 확장한다.
- [X] T013 [US1] 4단계 상태와 pointer load/unavailable/error 안내를 표시하는 read-only SDD controls organism을 `apps/agentic-workbench/src/features/worktree-workspace/ui/sdd-workflow-controls.tsx`에 구현한다.
- [X] T014 [US1] `.specify/feature.json` query, Speckit feature 목록, tasks progress, SDD state를 결합하고 refresh/watcher와 함께 갱신하도록 `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`의 `SpeckitWorkspaceTab`을 확장한다.
- [X] T015 [US1] valid, loading, unavailable, stale pointer 및 highlighted feature 상태를 `apps/agentic-workbench/src/stories/organisms.stories.tsx`에 등록한다.

**Checkpoint**: 활성 feature가 유일하게 하이라이트되고 artifact 변화 뒤 다시 조회하면 단계 표시가 갱신된다.

---

## Phase 4: User Story 2 - 다음 SDD 작업을 버튼으로 시작하기 (Priority: P1)

**Goal**: 사용자가 현재 활성 기능의 Specify, Plan, Tasks, Implement 작업을 버튼으로 요청하고, active pointer가 unavailable이면 편집 가능한 초기 SDD prompt를 주입받는다.

**Independent Test**: 실행 가능한 각 stage button을 선택해 대상 feature가 포함된 send request가 active agent panel로 전달되는지 확인한다. missing/invalid pointer에서는 request가 textarea에만 채워지고 run/queue/history가 바뀌지 않는지 확인한다.

### Tests for User Story 2

- [X] T016 [P] [US2] 각 stage의 command prompt builder와 unavailable pointer 초기 draft prompt의 unit test를 `apps/agentic-workbench/src/features/worktree-workspace/model/sdd-workflow.test.ts`에 추가한다.
- [X] T017 [P] [US2] stage action의 send callback 및 unavailable 상태 draft callback test를 `apps/agentic-workbench/src/features/worktree-workspace/ui/sdd-workflow-controls.test.tsx`에 추가한다.
- [X] T018 [P] [US2] workspace action request가 `send | draft` AgentPromptRequest로 page/agent area에 전달되는 integration-focused source test를 `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.test.tsx`에 추가한다.

### Implementation for User Story 2

- [X] T019 [US2] 단계별 `$speckit-specify`, `$speckit-plan`, `$speckit-tasks`, `$speckit-implement` 요청과 편집 가능한 initial SDD draft를 생성하도록 `apps/agentic-workbench/src/features/worktree-workspace/model/sdd-workflow.ts`를 확장한다.
- [X] T020 [US2] 실행 가능한 stage action과 pointer unavailable 초기 prompt action을 `apps/agentic-workbench/src/features/worktree-workspace/ui/sdd-workflow-controls.tsx`에 연결한다.
- [X] T021 [US2] SDD action request callback을 `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`에서 `send | draft` prompt request로 전달한다.
- [X] T022 [US2] workspace의 typed prompt request를 agent area에 전달하도록 `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`를 확장한다.

**Checkpoint**: valid active feature의 버튼은 현재 agent flow로 요청을 전송하고, unavailable pointer의 시작 안내는 사용자가 명시 전송할 때까지 draft로 남는다.

---

## Phase 5: User Story 3 - 단계 순서와 승인 게이트를 안전하게 지키기 (Priority: P2)

**Goal**: 사용자가 누락된 선행 산출물을 건너뛰거나 검토·재실행 확인 없이 다음 SDD 단계를 실행하지 않는다.

**Independent Test**: spec 없이 Plan을 선택하는 경우, spec/plan review confirmation, completed stage re-run confirmation을 각각 실행해 blocked action은 전송되지 않고 confirm 뒤에만 정확한 request가 전달되는지 확인한다.

### Tests for User Story 3

- [X] T023 [P] [US3] blocked reason, review confirmation, re-run confirmation state transition test를 `apps/agentic-workbench/src/features/worktree-workspace/model/sdd-workflow.test.ts`에 추가한다.
- [X] T024 [P] [US3] disabled button 설명과 review/re-run dialog cancel/confirm 행동 test를 `apps/agentic-workbench/src/features/worktree-workspace/ui/sdd-workflow-controls.test.tsx`에 추가한다.

### Implementation for User Story 3

- [X] T025 [US3] 선행 artifact, tasks progress, review/re-run 확인 필요 여부를 포함한 action availability를 `apps/agentic-workbench/src/features/worktree-workspace/model/sdd-workflow.ts`에 구현한다.
- [X] T026 [US3] disabled 이유, spec/plan review 확인, existing artifact re-run 확인 dialog를 `apps/agentic-workbench/src/features/worktree-workspace/ui/sdd-workflow-controls.tsx`에 구현한다.
- [X] T027 [US3] approval/re-run/disabled 상태 Storybook 예제를 `apps/agentic-workbench/src/stories/organisms.stories.tsx`에 추가한다.

**Checkpoint**: unsafe action은 전송되지 않고, 명시 확인된 action만 올바른 active feature 맥락으로 전송된다.

---

## Phase 6: User Story 4 - tasks 작업을 칸반으로 분리해 보기 (Priority: P3)

**Goal**: 선택한 `tasks.md`의 체크박스 작업을 완료 상태별 칸반과 작업 필요 보기로 읽기 전용 표시한다.

**Independent Test**: 완료·미완료 작업과 여러 heading section이 섞인 fixture에서 Kanban filter가 해당 상태만 표시하고, 작업 필요 보기가 미완료 작업 section의 heading·문맥·미완료 작업만 반환하는지 확인한다.

### Tests for User Story 4

- [X] T032 [P] [US4] checkbox task, heading section, 완료-only section, heading 없는 task parsing unit test를 `apps/agentic-workbench/src/features/worktree-workspace/model/tasks-kanban.test.ts`에 작성한다.
- [ ] T033 [P] [US4] Kanban filter와 작업 필요 보기의 빈 상태·완료 task 숨김 component test를 `apps/agentic-workbench/src/features/worktree-workspace/ui/tasks-kanban-panel.test.tsx`에 작성한다.
- [ ] T034 [P] [US4] `tasks.md` 선택에만 Kanban/작업 필요 전환을 표시하는 integration-focused test를 `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`에 추가한다.

### Implementation for User Story 4

- [X] T035 [US4] checkbox task와 heading-scoped needed section을 읽기 전용으로 추출하는 pure parser를 `apps/agentic-workbench/src/features/worktree-workspace/model/tasks-kanban.ts`에 구현한다.
- [X] T036 [US4] 미완료/완료 열, 상태 filter, 작업 필요 section renderer, empty state를 `apps/agentic-workbench/src/features/worktree-workspace/ui/tasks-kanban-panel.tsx`에 구현한다.
- [X] T037 [US4] 선택 문서가 `tasks.md`일 때 Preview/Kanban/작업 필요 view mode를 연결하고 원본 markdown preview를 유지하도록 `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`를 확장한다.
- [ ] T038 [US4] mixed tasks, completed-only section, needed-tasks state의 Storybook fixture와 organism story를 `apps/agentic-workbench/src/shared/storybook/sample-data.ts` 및 `apps/agentic-workbench/src/stories/organisms.stories.tsx`에 추가한다.

**Checkpoint**: 작업 필요 보기에 완료 task, 완료-only section, 무관한 markdown 영역이 표시되지 않고 원본 파일은 수정되지 않는다.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: 모든 사용자 스토리의 회귀, 갱신, 문서화된 수동 검증을 완료한다.

- [X] T028 [P] `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`에 pointer 변경·watcher refresh와 stale highlight 회귀 test를 추가한다.
- [X] T029 [P] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`와 `apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.test.ts`에서 기존 annotation/send prompt가 계속 즉시 send/queue되는지 회귀 검증한다.
- [X] T030 [P] `pnpm --filter agentic-workbench check-types`와 `pnpm --filter agentic-workbench test`를 실행하고 결과를 `specs/031-sdd-workflow-controls/quickstart.md`의 Validation Log에 기록한다.
- [ ] T031 `specs/031-sdd-workflow-controls/quickstart.md`의 네 가지 수동 시나리오를 수행하고 결과 및 발견 사항을 Validation Log에 기록한다.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 즉시 시작 가능하다.
- **Foundational (Phase 2)**: T001 완료 후 시작하며, 모든 사용자 스토리를 차단한다.
- **US1 (Phase 3)**: T003–T009 완료 후 시작한다.
- **US2 (Phase 4)**: T003–T009 완료 후 시작할 수 있으나, 실제 controls 연결은 T013–T014 이후 수행한다.
- **US3 (Phase 5)**: US2 controls action 이후 시작한다.
- **Polish (Phase 6)**: 필요한 사용자 스토리 완료 후 수행한다.

### User Story Dependencies

- **US1 (P1)**: Foundational 이후 독립 구현 가능하며 MVP의 화면 가시성을 제공한다.
- **US2 (P1)**: Foundational 이후 prompt contract는 독립 구현 가능하지만 controls UI를 위해 US1의 T013–T014에 의존한다.
- **US3 (P2)**: US2의 action controls에 의존한다.
- **US4 (P3)**: 기존 Speckit document selection 이후 독립 구현 가능하며, 새 pure parser(T035)와 UI(T036–T037) 순서를 따른다.

### Parallel Opportunities

- T002는 T001과 병렬 수행할 수 있다.
- T003, T005, T007은 서로 다른 test 파일이므로 병렬로 수행할 수 있다. 해당 구현(T004, T006, T008)은 각 test 다음에 수행한다.
- US1의 T010과 T011, US2의 T016–T018, US3의 T023과 T024는 각 phase의 구현 전 병렬로 수행할 수 있다.
- US4의 T032–T034는 서로 다른 test 파일이므로 병렬로 수행할 수 있다.
- Phase 6의 T028–T030은 서로 다른 대상 파일/검증 명령이므로 병렬 수행할 수 있다.

## Parallel Example: User Story 1

```text
Task: "T010 active feature highlight component test in apps/agentic-workbench/src/features/worktree-workspace/ui/speckit-files-panel.test.tsx"
Task: "T011 stage state component test in apps/agentic-workbench/src/features/worktree-workspace/ui/sdd-workflow-controls.test.tsx"
```

## Implementation Strategy

### MVP First (User Story 1)

1. T001–T009로 pointer/단계 모델과 draft/send contract를 안전하게 만든다.
2. T010–T015로 active feature highlight와 read-only stage 표시를 완성한다.
3. valid·invalid pointer 및 spec-only/plan/tasks 상태를 검증한다.

### Incremental Delivery

1. MVP 후 T016–T022로 stage action과 initial draft injection을 연결한다.
2. T023–T027로 review/re-run confirmation과 blocked action을 추가한다.
3. T028–T031로 watcher refresh, existing prompt flows, typecheck/test, 수동 검증을 마무리한다.
4. T032–T038로 Kanban과 작업 필요 보기의 parser·UI·Storybook을 추가하고, T030–T031 검증을 다시 수행한다.
