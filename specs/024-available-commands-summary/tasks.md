# Tasks: 사용 가능한 명령 요약과 조회

**Input**: Design documents from `/specs/024-available-commands-summary/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/available-commands-ui.md, quickstart.md

**Tests**: FR-011이 command metadata parsing, raw suppression, compact summary, detail 조회, malformed fallback 검증을 요구하므로 focused Vitest/RTL tasks를 포함한다.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **App frontend**: `apps/agentic-workbench/src/{app,pages,features,entities,shared,components/ui}`
- **App Tauri backend**: `apps/agentic-workbench/src-tauri/src/{domain,application,inbound,infrastructure,ports}`
- **Feature docs**: `specs/024-available-commands-summary/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 현재 available command parsing, autocomplete, raw timeline 흐름을 확인해 구현 기준선을 고정한다.

- [X] T001 Review existing `availableCommandCandidatesFromSessionUpdate` and update unwrap behavior in `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.ts`
- [X] T002 [P] Review existing prompt autocomplete command candidate tests in `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts`
- [X] T003 [P] Review existing raw/session update formatting behavior in `apps/agentic-workbench/src/entities/agent-run/model/format.ts`
- [X] T004 [P] Review current non-session raw event reducer behavior for `available_commands_update` in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T005 [P] Review current AgentRunPanel live event listener and header metadata area in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 모든 user story가 공유하는 command metadata parser, summary formatter, state shape를 먼저 만든다.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T006 [P] Add AvailableCommandMetadata and CommandDetailItem types in `apps/agentic-workbench/src/entities/agent-run/model/types.ts`
- [X] T007 Add parser tests for direct, wrapped, empty, and malformed `available_commands_update` payloads in `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts`
- [X] T008 Extend available command parsing to return command detail items with name, description, inputHint, and source in `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.ts`
- [X] T009 Add command summary formatter tests for singular, plural, and empty labels in `apps/agentic-workbench/src/entities/agent-run/model/format.test.ts`
- [X] T010 Add command summary formatter helper for AvailableCommandMetadata in `apps/agentic-workbench/src/entities/agent-run/model/format.ts`
- [X] T011 Extend RunEventState with available command metadata storage in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`

**Checkpoint**: command metadata can be parsed and summarized without rendering UI.

---

## Phase 3: User Story 1 - 명령 목록 갱신을 짧게 파악 (Priority: P1) MVP

**Goal**: `available_commands_update` 전체 JSON을 timeline에 노출하지 않고 command count summary를 표시한다.

**Independent Test**: 3개 command update를 받으면 raw timeline item은 생기지 않고, 최신 command count summary가 보인다.

### Tests for User Story 1

- [X] T012 [P] [US1] Add reducer test that valid `available_commands_update` is suppressed from timeline and stores command metadata in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T013 [P] [US1] Add AgentRunPanel source/UI regression test for command summary rendering and raw suppression in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`
- [X] T014 [P] [US1] Add formatter regression test proving non-command raw events still remain timeline content in `apps/agentic-workbench/src/entities/agent-run/model/format.test.ts`

### Implementation for User Story 1

- [X] T015 [US1] Detect `available_commands_update` as session metadata before timeline item creation in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T016 [US1] Update AgentRunPanel live event listener to store available command metadata before raw timeline append in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T017 [US1] Render compact command count summary in the existing run/session header metadata row in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T018 [US1] Preserve existing autocomplete candidate updates while suppressing raw command payload timeline output in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`

**Checkpoint**: User Story 1 is fully functional and testable independently as the MVP.

---

## Phase 4: User Story 2 - 사용 가능한 명령 상세 조회 (Priority: P2)

**Goal**: 세션 정보 영역에서 command 이름, 설명, 입력 힌트를 compact하게 조회할 수 있게 한다.

**Independent Test**: `mcp`, `review`, `$speckit-implement`가 포함된 update 후 detail view에서 세 command와 `review`의 input hint가 확인된다.

### Tests for User Story 2

- [X] T019 [P] [US2] Add parser test preserving `$skill`, slash command, plain command names, descriptions, and input hints in `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts`
- [X] T020 [P] [US2] Add AgentRunPanel UI/source regression test for command detail trigger, list rendering, description, and input hint text in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`

### Implementation for User Story 2

- [X] T021 [US2] Add bounded command detail list rendering for available commands in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T022 [US2] Add a compact trigger or disclosure control for opening command details in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T023 [US2] Render command description and input hint without raw input schema JSON in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T024 [US2] Ensure parsed command detail items remain compatible with prompt autocomplete candidate normalization in `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.ts`

**Checkpoint**: User Stories 1 and 2 both work independently without raw command payload output.

---

## Phase 5: User Story 3 - 비어 있거나 깨진 목록을 안전하게 처리 (Priority: P3)

**Goal**: empty, partial, malformed command metadata에서도 UI가 안정적으로 렌더링되고 prompt 흐름이 유지된다.

**Independent Test**: 빈 목록과 malformed command 목록을 보내도 raw JSON timeline item이 생기지 않고, 유효 command만 표시하거나 empty fallback이 보인다.

### Tests for User Story 3

- [X] T025 [P] [US3] Add parser tests ignoring missing-name command entries and preserving valid entries in `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts`
- [X] T026 [P] [US3] Add reducer tests for empty and malformed command updates suppressing timeline raw output in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T027 [P] [US3] Add AgentRunPanel UI/source regression test for empty command summary and bounded long-list detail rendering in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`

### Implementation for User Story 3

- [X] T028 [US3] Add fallback handling for empty or invalid `availableCommands` in `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.ts`
- [X] T029 [US3] Render empty command summary or empty detail state without throwing in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T030 [US3] Bound long command detail lists with scrolling or max height in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T031 [US3] Confirm backend mapper changes are unnecessary or keep any optional event contract change limited to `apps/agentic-workbench/src-tauri/src/infrastructure/acp/session_update_mapper.rs`

**Checkpoint**: All user stories are independently functional and non-command raw event behavior remains intact.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: 최종 검증, cleanup, manual acceptance를 수행한다.

- [X] T032 [P] Run focused Vitest commands from `specs/024-available-commands-summary/quickstart.md`
- [X] T033 [P] Run `pnpm --filter agentic-workbench check-types` for `apps/agentic-workbench`
- [X] T034 [P] Run `pnpm --filter agentic-workbench test` for `apps/agentic-workbench`
- [X] T035 Run `cargo test -p agentic-workbench session_update_mapper` only if Tauri mapper/domain files changed in `apps/agentic-workbench/src-tauri/src`
- [X] T036 Verify no app-to-app imports, shared package additions, or persistence changes were introduced in `apps/agentic-workbench/src`
- [X] T037 Run manual command summary, detail, empty, malformed, long-list, and non-command raw checks from `specs/024-available-commands-summary/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup completion and blocks all user stories.
- **User Story 1 (Phase 3)**: Depends on Foundational. This is the MVP.
- **User Story 2 (Phase 4)**: Depends on Foundational and can build on the same metadata list as US1.
- **User Story 3 (Phase 5)**: Depends on Foundational and hardens US1/US2 behavior.
- **Polish (Phase 6)**: Depends on desired user stories being complete.

### User Story Dependencies

- **US1 (P1)**: No dependency on other stories after Foundational.
- **US2 (P2)**: Can start after Foundational, but uses the metadata created for US1 summary.
- **US3 (P3)**: Validates fallback and long-list behavior across US1/US2.

### Within Each User Story

- Write story tests first and confirm they fail before implementation.
- Implement pure parser/formatter changes before reducer changes.
- Implement reducer state before AgentRunPanel rendering.
- Keep backend changes optional and limited to ACP session update mapping if actually needed.

---

## Parallel Opportunities

- T002, T003, T004, and T005 can run in parallel after T001 starts.
- T006 and T009 can run in parallel because they touch separate model files.
- T012, T013, and T014 can run in parallel for US1 test coverage.
- T019 and T020 can run in parallel for US2 test coverage.
- T025, T026, and T027 can run in parallel for US3 fallback coverage.
- T032, T033, and T034 can run in parallel once implementation is complete.

---

## Parallel Example: User Story 1

```bash
Task: "T012 [P] [US1] Add reducer suppression/storage test in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "T013 [P] [US1] Add AgentRunPanel summary/suppression regression test in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
Task: "T014 [P] [US1] Add non-command raw event regression test in apps/agentic-workbench/src/entities/agent-run/model/format.test.ts"
```

## Parallel Example: User Story 2

```bash
Task: "T019 [P] [US2] Add command detail parser test in apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts"
Task: "T020 [P] [US2] Add command detail UI/source regression test in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
```

## Parallel Example: User Story 3

```bash
Task: "T025 [P] [US3] Add malformed command parser tests in apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts"
Task: "T026 [P] [US3] Add empty/malformed reducer suppression tests in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "T027 [P] [US3] Add empty and long-list UI/source regression test in apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational parser/summary/state shape work.
3. Complete Phase 3: User Story 1.
4. Stop and validate that command updates show a compact count and full raw JSON is suppressed from timeline.

### Incremental Delivery

1. Setup + Foundational -> command metadata ready.
2. Add US1 -> compact summary and raw suppression.
3. Add US2 -> detail 조회.
4. Add US3 -> malformed/empty/long-list fallback.
5. Run focused tests, app typecheck/test, and quickstart manual checks.

### Parallel Team Strategy

1. One developer prepares parser, summary formatter, and state type foundation.
2. After Foundational, split by story:
   - Developer A: US1 summary/suppression.
   - Developer B: US2 detail 조회.
   - Developer C: US3 fallback and long-list hardening.
3. Merge after each story's independent tests pass.

---

## Notes

- [P] tasks use different files or can be handled independently with careful merge.
- Story labels map directly to spec user stories.
- Full `input` schema rendering and historical raw timeline migration are out of scope.
- `available_commands_update` should not disable existing prompt autocomplete behavior.
