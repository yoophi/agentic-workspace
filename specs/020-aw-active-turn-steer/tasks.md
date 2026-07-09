# Tasks: AW Active-Turn Steer

**Input**: Design documents from `specs/020-aw-active-turn-steer/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Constitution-required tests are included for pure frontend state transitions and Tauri backend safety/use-case boundaries. User-journey validation is covered through model tests, backend tests, and quickstart scenarios.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 기존 AW 입력/steer 구조를 구현 전 기준선으로 고정하고 작업 경계를 준비한다.

- [X] T001 Capture current steer, queue, prompt dispatch, and cancel/restart code references in `docs/aw-active-turn-steer.md`
- [X] T002 [P] Add active-turn steer design overview and Mermaid state diagram in `docs/aw-active-turn-steer.md`
- [X] T003 [P] Add reusable test fixtures for prompt dispatch state in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T004 [P] Add backend fake session capability fixtures in `apps/agentic-workbench/src-tauri/src/application/send_prompt.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 모든 user story가 공유하는 입력 상태 모델, backend port, event type, Tauri invoke 경계를 만든다.

**CRITICAL**: 이 phase가 끝나기 전에는 user story 구현을 시작하지 않는다.

- [X] T005 Add `SteerInput`, `SteerInputStatus`, `PromptDispatchPhase`, and extended `RunEventState` types in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T006 Add prompt dispatch reducer event types for queue, steer, terminal event, and restart transitions in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T007 Add `steerPending`, `steerAccepted`, and `steerRejected` lifecycle statuses to frontend event types in `apps/agentic-workbench/src/entities/agent-run/model/types.ts`
- [X] T008 Add `steerPending`, `steerAccepted`, and `steerRejected` lifecycle statuses to backend run events in `apps/agentic-workbench/src-tauri/src/domain/events.rs`
- [X] T009 Add `SteerPromptError` variants for empty prompt, inactive run, unsupported capability, and dispatch failure in `apps/agentic-workbench/src-tauri/src/application/agent_run_errors.rs`
- [X] T010 Add `steer_prompt` capability method to the `SessionHandle` port in `apps/agentic-workbench/src-tauri/src/ports/session_handle.rs`
- [X] T011 Add `steerPromptToRun` Tauri invoke adapter in `apps/agentic-workbench/src/entities/agent-run/api/agent-run-repository.ts`
- [X] T012 Register `steer_prompt_to_run` command wiring in `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`

**Checkpoint**: Shared frontend/backend contracts exist and user story implementation can begin.

---

## Phase 3: User Story 1 - 실행 중인 작업을 종료하지 않고 방향 조정하기 (Priority: P1) MVP

**Goal**: 사용자가 실행 중인 작업에 steer를 제출해도 명시적 restart 없이는 기존 run이 취소되지 않고, 지원 가능한 provider에서는 같은 run timeline에 pending/accepted steer가 반영된다.

**Independent Test**: 실행 중인 run에 steer를 제출한 뒤 `cancelAgentRun`이 호출되지 않고, 같은 `activeRunId`에서 pending steer가 accepted 상태로 전환되는지 확인한다.

### Tests for User Story 1

- [X] T013 [P] [US1] Add reducer tests for accepted pending steer without clearing `activeRunId` in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T014 [P] [US1] Add reducer tests proving old inactive run events do not mutate active steer state in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T015 [P] [US1] Add `SteerPromptUseCase` tests for empty prompt, inactive run, accepted steer, and dispatch failure in `apps/agentic-workbench/src-tauri/src/application/steer_prompt.rs`
- [X] T016 [P] [US1] Add ACP session `steer_prompt` adapter tests for supported and unsupported capability paths in `apps/agentic-workbench/src-tauri/src/infrastructure/acp/runner.rs`

### Implementation for User Story 1

- [X] T017 [US1] Implement pure steer submit, accept, reject, and target-run validation helpers in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T018 [US1] Implement `SteerPromptUseCase` without calling `cancel_run` in `apps/agentic-workbench/src-tauri/src/application/steer_prompt.rs`
- [X] T019 [US1] Implement default unsupported `steer_prompt` behavior for existing session handles in `apps/agentic-workbench/src-tauri/src/ports/session_handle.rs`
- [X] T020 [US1] Implement ACP runner `steer_prompt` capability dispatch or structured unsupported response in `apps/agentic-workbench/src-tauri/src/infrastructure/acp/runner.rs`
- [X] T021 [US1] Expose `steer_prompt_to_run` inbound command through application use case delegation in `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`
- [X] T022 [US1] Replace direct steer cancel-and-restart submission with `steerPromptToRun` in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T023 [US1] Update run event handling to render pending and accepted steer timeline state for the same run in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`

**Checkpoint**: US1 MVP is independently functional: supported steer does not terminate the current run.

---

## Phase 4: User Story 2 - queue와 steer를 구분해 입력 흐름 파악하기 (Priority: P2)

**Goal**: 사용자가 pending steer와 queued prompt를 서로 다른 실행 대상과 순서로 구분해서 볼 수 있다.

**Independent Test**: 실행 중인 작업에 steer와 queued prompt를 모두 추가한 뒤 UI와 state가 `pendingSteers`와 `queuedPrompts`를 별도로 유지하고 표시하는지 확인한다.

### Tests for User Story 2

- [X] T024 [P] [US2] Add reducer tests for independent `pendingSteers`, `rejectedSteers`, and `queuedPrompts` ownership in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T025 [P] [US2] Add reducer tests for preserving queued prompt order when a queued prompt is converted to steer in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T026 [P] [US2] Add Storybook states for queued, pending steer, rejected steer, and long-content input lists in `apps/agentic-workbench/src/stories/organisms.stories.tsx`

### Implementation for User Story 2

- [X] T027 [US2] Implement queue-to-steer transition helpers that move exactly one prompt out of `queuedPrompts` in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T028 [US2] Update queued prompt auto-dispatch logic to suppress dispatch while `pendingSteers` is non-empty in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T029 [US2] Split queued prompt and pending steer rendering into distinguishable sections in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T030 [US2] Update queued prompt action buttons to call the queue-to-steer flow instead of restart flow in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T031 [US2] Update accessibility labels and visible status copy for queued prompt versus pending steer controls in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`

**Checkpoint**: US2 is independently functional: users can distinguish current-work steer inputs from next-work queued prompts.

---

## Phase 5: User Story 3 - steer 불가 상태에서 안전하게 대체 흐름 제공하기 (Priority: P3)

**Goal**: provider 또는 현재 상태가 steer를 지원하지 않을 때 입력을 잃지 않고 queue fallback 또는 명시적 restart-with-steering을 선택할 수 있다.

**Independent Test**: unsupported steer 응답을 받은 뒤 현재 run이 자동 취소되지 않고, 원본 입력이 rejected steer로 보존되며 사용자가 queue 이동 또는 restart를 명시적으로 선택할 수 있는지 확인한다.

### Tests for User Story 3

- [X] T032 [P] [US3] Add reducer tests for unsupported steer preserving rejected input and active run state in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T033 [P] [US3] Add reducer tests for explicit restart-with-steering preserving remaining queue and ignoring late old-run terminal events in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`
- [X] T034 [P] [US3] Add backend unsupported capability tests for `steer_prompt_to_run` command behavior in `apps/agentic-workbench/src-tauri/src/application/steer_prompt.rs`

### Implementation for User Story 3

- [X] T035 [US3] Implement rejected steer fallback helpers for queue move, delete, retry, and restart request in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T036 [US3] Implement explicit restart-with-steering transition that is the only path allowed to call `cancelAgentRun` for steer fallback in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T037 [US3] Add rejected steer fallback UI actions for queue move, retry, delete, and restart in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T038 [US3] Map `steer_prompt_to_run` unsupported and dispatch errors to preserved rejected steer state in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`
- [X] T039 [US3] Ensure restart fallback restores prompt and queue state on start failure in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`

**Checkpoint**: US3 is independently functional: unsupported steer never loses user input or automatically terminates the current run.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: 문서, 검증, 회귀 방지, UX polish를 마무리한다.

- [X] T040 [P] Update Korean architecture and user-flow documentation in `docs/aw-active-turn-steer.md`
- [X] T041 [P] Update lifecycle and prompt dispatch examples in `specs/020-aw-active-turn-steer/quickstart.md`
- [X] T042 [P] Run frontend typecheck and test commands from quickstart in `apps/agentic-workbench/package.json`
- [X] T043 [P] Run Tauri Rust tests for the affected app in `apps/agentic-workbench/src-tauri/Cargo.toml`
- [X] T044 Verify no app-to-app imports or new shared package/crate dependencies were introduced in `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`
- [X] T045 Verify manual quickstart scenarios for supported steer, unsupported fallback, queued prompt conversion, and late event isolation in `specs/020-aw-active-turn-steer/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies
- **Foundational (Phase 2)**: Depends on Setup completion and blocks all user stories
- **US1 (Phase 3)**: Depends on Foundational and is the MVP
- **US2 (Phase 4)**: Depends on Foundational; can run after or alongside US1 once shared state contracts exist, but final UI integration should account for US1 steer events
- **US3 (Phase 5)**: Depends on Foundational; restart fallback depends on US1/US2 state helpers for best integration
- **Polish (Phase 6)**: Depends on selected user stories being complete

### User Story Dependencies

- **US1 (P1)**: No dependency on other stories after Foundation
- **US2 (P2)**: Can start after Foundation; integrates cleanly after US1 event names are stable
- **US3 (P3)**: Can start after Foundation; explicit restart fallback is safest after US1 command and US2 state ownership are available

### Within Each User Story

- Write constitution-required reducer/use-case tests first and confirm they fail before implementation
- Pure frontend state helpers before UI integration
- Backend port/use case before inbound command wiring
- Tauri command adapter before frontend invoke usage
- Story checkpoint validation before moving to the next priority

## Parallel Opportunities

- T002, T003, and T004 can run in parallel after T001
- T007, T008, T009, T010, and T011 can run in parallel after T005 and T006 are understood
- US1 tests T013, T014, T015, and T016 can run in parallel
- US2 tests T024, T025, and T026 can run in parallel
- US3 tests T032, T033, and T034 can run in parallel
- Polish verification tasks T040, T041, T042, and T043 can run in parallel after implementation stabilizes

## Parallel Example: User Story 1

```bash
Task: "T013 [P] [US1] Add reducer tests for accepted pending steer without clearing activeRunId in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "T015 [P] [US1] Add SteerPromptUseCase tests for empty prompt, inactive run, accepted steer, and dispatch failure in apps/agentic-workbench/src-tauri/src/application/steer_prompt.rs"
Task: "T016 [P] [US1] Add ACP session steer_prompt adapter tests for supported and unsupported capability paths in apps/agentic-workbench/src-tauri/src/infrastructure/acp/runner.rs"
```

## Parallel Example: User Story 2

```bash
Task: "T024 [P] [US2] Add reducer tests for independent pendingSteers, rejectedSteers, and queuedPrompts ownership in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "T026 [P] [US2] Add Storybook states for queued, pending steer, rejected steer, and long-content input lists in apps/agentic-workbench/src/stories/organisms.stories.tsx"
```

## Parallel Example: User Story 3

```bash
Task: "T032 [P] [US3] Add reducer tests for unsupported steer preserving rejected input and active run state in apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts"
Task: "T034 [P] [US3] Add backend unsupported capability tests for steer_prompt_to_run command behavior in apps/agentic-workbench/src-tauri/src/application/steer_prompt.rs"
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 setup.
2. Complete Phase 2 foundation.
3. Complete Phase 3 US1.
4. Validate that supported steer does not call cancel/restart and stays on the same run.
5. Stop and demo MVP behavior before adding queue display and fallback UX.

### Incremental Delivery

1. Foundation creates shared prompt dispatch and backend steer contracts.
2. US1 delivers cancel-free steer for supported providers.
3. US2 makes queue and steer status clear to users.
4. US3 handles unsupported providers safely with preserved input and explicit restart.
5. Polish closes documentation, Storybook, typecheck, frontend tests, Rust tests, and quickstart validation.

### Parallel Team Strategy

After Phase 2, one engineer can work on backend US1 tasks while another works on frontend reducer/UI tests. US2 UI story work can begin once state ownership is stable. US3 fallback work should coordinate with US1 command errors and US2 rejected steer display.

## Notes

- `[P]` tasks touch different files or independent test sections and can run in parallel.
- `[US1]`, `[US2]`, and `[US3]` map directly to the prioritized user stories in `spec.md`.
- Every implementation task includes an exact repository path and should be executable without additional planning context.
