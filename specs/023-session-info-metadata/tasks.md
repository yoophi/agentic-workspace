# Tasks: 세션 정보 메타데이터 표시

**Input**: Design documents from `/specs/023-session-info-metadata/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/session-info-metadata-ui.md, quickstart.md

**Tests**: FR-008이 title handling, updatedAt handling, metadata-only status preservation, timeline suppression 검증을 요구하므로 focused Vitest/RTL tasks를 포함한다.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **App frontend**: `apps/agentic-workbench/src/{app,pages,features,entities,shared,components/ui}`
- **App Tauri backend**: `apps/agentic-workbench/src-tauri/src/{domain,application,inbound,infrastructure,ports}`
- **Feature docs**: `specs/023-session-info-metadata/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 현재 구현 경로와 테스트 fixture를 확인해 story 작업 전 기준선을 고정한다.

- [X] T001 Review existing window title event constants and standalone title application in `apps/agentic-workbench/src/app/App.tsx`
- [X] T002 [P] Review existing session metadata parsing and typed `sessionInfo` handling in `apps/agentic-workbench/src/entities/agent-run/model/format.ts`
- [X] T003 [P] Review existing active run event handling and suppression path in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T004 [P] Review existing pure state reducer behavior for `sessionInfo` events in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 모든 user story가 공유하는 session metadata read/update helper와 test fixture 기반을 먼저 만든다.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T005 [P] Add typed and raw `session_info_update` fixture cases for title, updatedAt, and threadStatus combinations in `apps/agentic-workbench/src/entities/agent-run/model/format.test.ts`
- [X] T006 Add pure metadata extraction coverage for typed `sessionInfo` title and updatedAt in `apps/agentic-workbench/src/entities/agent-run/model/format.test.ts`
- [X] T007 Extend `readSessionInfoUpdateMetadata` or add a typed metadata reader so both typed `sessionInfo` and raw fallback events expose title, updatedAt, and threadStatus in `apps/agentic-workbench/src/entities/agent-run/model/format.ts`
- [X] T008 Update exported metadata types if needed for title and updatedAt consumption in `apps/agentic-workbench/src/entities/agent-run/model/types.ts`

**Checkpoint**: Session metadata can be read consistently before UI behavior is implemented.

---

## Phase 3: User Story 1 - 세션 제목을 안정적인 위치에서 확인 (Priority: P1) MVP

**Goal**: `session_info_update.title`을 raw timeline 없이 AW window title에 반영한다.

**Independent Test**: title이 포함된 update를 수신하면 standalone AW window title이 최신 title로 갱신되고 timeline에는 raw JSON row가 추가되지 않는다.

### Tests for User Story 1

- [X] T009 [P] [US1] Add window-title normalization edge cases for session title reuse in `apps/agentic-workbench/src/entities/project/lib/worktree-window-title.test.ts`
- [X] T010 [P] [US1] Add App-level test coverage for MCP window title event/fallback applying session title in `apps/agentic-workbench/src/app/App.test.tsx`
- [X] T011 [P] [US1] Add AgentRunPanel UI test that a title-only `sessionInfo` event dispatches the window title update and does not render a raw timeline item in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`
- [X] T012 [P] [US1] Add reducer test that title-only session info is suppressed from timeline in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`

### Implementation for User Story 1

- [X] T013 [US1] Export or centralize the existing MCP window title event dispatch/fallback helper for reuse by AgentRunPanel in `apps/agentic-workbench/src/app/App.tsx`
- [X] T014 [US1] Dispatch the normalized session title from the active-run `sessionInfo` handling branch in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T015 [US1] Ensure invalid title values do not overwrite the current title by using existing title normalization behavior in `apps/agentic-workbench/src/entities/project/lib/worktree-window-title.ts`
- [X] T016 [US1] Keep typed and raw `session_info_update` events out of rendered timeline items after title dispatch in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`

**Checkpoint**: User Story 1 is fully functional and testable independently as the MVP.

---

## Phase 4: User Story 2 - 세션 최신성 정보 확인 (Priority: P2)

**Goal**: 유효한 `updatedAt`을 run/session header의 보조 metadata로 표시하고 malformed 값은 안전하게 무시한다.

**Independent Test**: `updatedAt`이 포함된 update를 받으면 header에 읽기 쉬운 최신성 표시가 보이고 invalid 값은 UI를 깨뜨리지 않는다.

### Tests for User Story 2

- [X] T017 [P] [US2] Add pure formatting tests for valid, missing, and invalid session updatedAt values in `apps/agentic-workbench/src/entities/agent-run/model/format.test.ts`
- [X] T018 [P] [US2] Add reducer tests for preserving latest valid session updatedAt and ignoring malformed values in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T019 [P] [US2] Add AgentRunPanel UI test for rendering the session freshness label in the run/session header in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`

### Implementation for User Story 2

- [X] T020 [US2] Add a `formatSessionFreshnessLabel` or equivalent helper for updatedAt display in `apps/agentic-workbench/src/entities/agent-run/model/format.ts`
- [X] T021 [US2] Extend run panel state to track the latest valid session updatedAt metadata in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T022 [US2] Add AgentRunPanel component state for latest valid session updatedAt and reset it when the active run changes in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T023 [US2] Render the formatted session freshness label in the existing run/session header metadata row in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`

**Checkpoint**: User Stories 1 and 2 both work independently without raw timeline output.

---

## Phase 5: User Story 3 - 기존 상태 표시와 timeline suppression 유지 (Priority: P3)

**Goal**: title/update time 표시를 추가해도 active/idle status indicator와 timeline suppression이 회귀하지 않도록 한다.

**Independent Test**: title, updatedAt, threadStatus 조합 update를 보내면 status, title, freshness가 독립적으로 갱신되고 raw JSON timeline item은 생기지 않는다.

### Tests for User Story 3

- [X] T024 [P] [US3] Add reducer tests that metadata-only updates do not clear existing active or idle status in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T025 [P] [US3] Add AgentRunPanel UI test for combined title, updatedAt, and threadStatus updates preserving the status badge and suppressing raw output in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`
- [X] T026 [P] [US3] Confirm backend mapper regression coverage for title and updatedAt contract in `apps/agentic-workbench/src-tauri/src/infrastructure/acp/session_update_mapper.rs`

### Implementation for User Story 3

- [X] T027 [US3] Preserve current `agentThreadStatus` when metadata-only session updates omit threadStatus in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T028 [US3] Preserve current `agentThreadStatus` in the AgentRunPanel live event listener when title or updatedAt updates omit threadStatus in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T029 [US3] Verify typed backend `SessionInfo` still includes title and updatedAt without adding persistence or Tauri command behavior in `apps/agentic-workbench/src-tauri/src/domain/events.rs`
- [X] T030 [US3] Verify ACP mapper still maps `session_info_update` title and updatedAt into typed `SessionInfo` without raw timeline dependence in `apps/agentic-workbench/src-tauri/src/infrastructure/acp/session_update_mapper.rs`

**Checkpoint**: All user stories are independently functional and #145 behavior remains intact.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: 최종 검증, cleanup, manual acceptance를 수행한다.

- [X] T031 [P] Run focused Vitest commands from `specs/023-session-info-metadata/quickstart.md`
- [X] T032 [P] Run `pnpm --filter agentic-workbench check-types` for `apps/agentic-workbench`
- [X] T033 [P] Run `pnpm --filter agentic-workbench test` for `apps/agentic-workbench`
- [X] T034 Run `cargo test -p agentic-workbench session_update_mapper` only if Tauri mapper/domain files changed in `apps/agentic-workbench/src-tauri/src`
- [X] T035 Verify no app-to-app imports, shared package additions, or persistence changes were introduced in `apps/agentic-workbench/src`
- [X] T036 Run manual title, updatedAt, malformed metadata, and route-reset checks from `specs/023-session-info-metadata/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup completion and blocks all user stories.
- **User Story 1 (Phase 3)**: Depends on Foundational. This is the MVP.
- **User Story 2 (Phase 4)**: Depends on Foundational. Can be implemented after or alongside US1, but header rendering should not break US1 title behavior.
- **User Story 3 (Phase 5)**: Depends on Foundational and validates regressions across US1/US2.
- **Polish (Phase 6)**: Depends on desired user stories being complete.

### User Story Dependencies

- **US1 (P1)**: No dependency on other stories after Foundational.
- **US2 (P2)**: No hard dependency on US1 after Foundational, but can reuse the same metadata reader.
- **US3 (P3)**: Uses the combined behavior from US1/US2 plus #145 status handling.

### Within Each User Story

- Write story tests first and confirm they fail before implementation.
- Implement pure helpers before state reducers.
- Implement state reducers before UI rendering/listener integration.
- Keep backend changes limited to mapper/domain contract verification if needed.

---

## Parallel Opportunities

- T002, T003, and T004 can run in parallel after T001 starts.
- T005 and T006 can run in parallel because they are test fixtures for the same formatter file and should be merged carefully.
- T009, T010, T011, and T012 can run in parallel for US1 test coverage.
- T017, T018, and T019 can run in parallel for US2 test coverage.
- T024, T025, and T026 can run in parallel for US3 regression coverage.
- T031, T032, and T033 can run in parallel once implementation is complete.

---

## Parallel Example: User Story 1

```bash
Task: "T009 [P] [US1] Add window-title normalization edge cases in apps/agentic-workbench/src/entities/project/lib/worktree-window-title.test.ts"
Task: "T010 [P] [US1] Add App-level title event coverage in apps/agentic-workbench/src/app/App.test.tsx"
Task: "T011 [P] [US1] Add AgentRunPanel title dispatch and timeline suppression UI test in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
Task: "T012 [P] [US1] Add reducer suppression test in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
```

## Parallel Example: User Story 2

```bash
Task: "T017 [P] [US2] Add updatedAt formatting tests in apps/agentic-workbench/src/entities/agent-run/model/format.test.ts"
Task: "T018 [P] [US2] Add reducer updatedAt tests in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "T019 [P] [US2] Add header freshness UI test in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
```

## Parallel Example: User Story 3

```bash
Task: "T024 [P] [US3] Add metadata-only status preservation reducer tests in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "T025 [P] [US3] Add combined metadata/status UI regression test in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
Task: "T026 [P] [US3] Add backend mapper regression test if backend contract changes in apps/agentic-workbench/src-tauri/src/infrastructure/acp/session_update_mapper.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational metadata reader/test fixture work.
3. Complete Phase 3: User Story 1.
4. Stop and validate that title updates the AW window title and timeline raw output remains suppressed.

### Incremental Delivery

1. Setup + Foundational -> metadata reader ready.
2. Add US1 -> window title support.
3. Add US2 -> header freshness metadata.
4. Add US3 -> status preservation and raw suppression regression hardening.
5. Run focused tests, app typecheck/test, and quickstart manual checks.

### Parallel Team Strategy

1. One developer prepares shared metadata reader and fixtures.
2. After Foundational, split by story:
   - Developer A: US1 window title bridge.
   - Developer B: US2 freshness header.
   - Developer C: US3 status/suppression regression coverage.
3. Merge after each story's independent tests pass.

---

## Notes

- [P] tasks use different files or can be handled independently with careful merge.
- Story labels map directly to spec user stories.
- `updatedAt` does not belong in the window title.
- #113 active/idle window-title prefix/suffix policy remains out of scope.
