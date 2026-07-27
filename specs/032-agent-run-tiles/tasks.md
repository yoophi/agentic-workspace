# Tasks: Agent Run 탭·타일 워크스페이스

**Input**: Design documents from `/specs/032-agent-run-tiles/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Constitution-required pure reducer, exchange state, owner-scope and adapter tests are included and must be written before implementation.

**Organization**: Tasks are grouped by user story so each increment can be verified independently.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 현재 구조를 보존하며 feature-local 모델과 backend hexagonal module 경계를 준비한다.

- [x] T001 Verify repository ignore/configuration coverage in `.gitignore` and `apps/agentic-workbench/eslint.config.js`
- [x] T002 Create frontend workspace and exchange model/API file skeletons under `apps/agentic-workbench/src/entities/agent-run/`
- [x] T003 Create backend agent exchange domain/application/port/infrastructure module skeletons under `apps/agentic-workbench/src-tauri/src/`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 모든 사용자 스토리가 공유하는 panel identity, layout tree, exchange contract를 확립한다.

**⚠️ CRITICAL**: 이 단계가 완료되기 전에는 UI 및 transport 작업을 시작하지 않는다.

- [x] T004 [P] Write failing tile layout invariant/split/remove/resize tests in `apps/agentic-workbench/src/entities/agent-run/model/tile-layout.test.ts`
- [x] T005 Implement pure leaf/split tree operations and validation in `apps/agentic-workbench/src/entities/agent-run/model/tile-layout.ts`
- [x] T006 [P] Write failing workspace mode/focus/slot-state tests in `apps/agentic-workbench/src/entities/agent-run/model/agent-run-workspace.test.ts`
- [x] T007 Implement workspace reducer and slot/layout coordination in `apps/agentic-workbench/src/entities/agent-run/model/agent-run-workspace.ts`
- [x] T008 [P] Define frontend exchange DTO/state types in `apps/agentic-workbench/src/entities/agent-run/model/agent-exchange.ts`
- [x] T009 [P] Write failing Rust validation/state-transition tests in `apps/agentic-workbench/src-tauri/src/domain/agent_exchange.rs`
- [x] T010 Implement Rust workspace endpoint and exchange domain model in `apps/agentic-workbench/src-tauri/src/domain/agent_exchange.rs`

**Checkpoint**: Pure workspace and exchange foundations pass focused TypeScript/Rust tests.

---

## Phase 3: User Story 1 - 탭과 타일 보기 전환 (Priority: P1) 🎯 MVP

**Goal**: 기존 탭 동작을 유지하며 같은 panel instances를 타일 projection으로 전환한다.

**Independent Test**: 세 panel의 run/draft 상태를 준비하고 탭·타일을 반복 전환하여 panel identity, focused panel, 실행 상태가 유지되는지 검증한다.

### Tests for User Story 1

- [x] T011 [P] [US1] Write view toggle and mounted panel contract tests in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.test.tsx`
- [x] T012 [P] [US1] Write toolbar accessibility contract tests in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-workspace-toolbar.test.tsx`

### Implementation for User Story 1

- [x] T013 [US1] Refactor panel slots to consume workspace reducer compatibility helpers in `apps/agentic-workbench/src/features/agent-run/model/agent-run-panel-slots.ts`
- [x] T014 [P] [US1] Implement tab/tile view toolbar in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-workspace-toolbar.tsx`
- [x] T015 [US1] Implement recursive tile projection with stable panel children in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-tile-layout.tsx`
- [x] T016 [US1] Integrate view mode and focused panel routing without remounting runs in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`

**Checkpoint**: 탭/타일 전환 MVP가 기존 agent run을 중단하지 않고 동작한다.

---

## Phase 4: User Story 2 - 현재 타일 옆에 새 에이전트 런 열기 (Priority: P1)

**Goal**: 현재 panel 기준 오른쪽/아래 split 명령으로 새 extra panel을 생성한다.

**Independent Test**: Main 오른쪽에 extra를 열고 그 extra 아래에 또 열어 tree와 화면 위치 및 새 focus를 검증한다.

### Tests for User Story 2

- [x] T017 [P] [US2] Add adjacent-open and limit rejection reducer tests in `apps/agentic-workbench/src/entities/agent-run/model/agent-run-workspace.test.ts`
- [x] T018 [P] [US2] Write tile header command/accessibility tests in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-tile.test.tsx`

### Implementation for User Story 2

- [x] T019 [US2] Add right/below open commands and capacity/depth guards in `apps/agentic-workbench/src/entities/agent-run/model/agent-run-workspace.ts`
- [x] T020 [P] [US2] Implement tile header and adjacent-open menu in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-tile.tsx`
- [x] T021 [US2] Wire tab plus and tile commands to deterministic splits in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`

**Checkpoint**: 오른쪽/아래 새 타일 명령이 독립적으로 검증된다.

---

## Phase 5: User Story 3 - 타일 배치 조작 및 닫기 (Priority: P2)

**Goal**: split resize, focus 이동, 닫기 후 tree 축약과 기존 running-close 보호를 제공한다.

**Independent Test**: 중첩 타일 resize와 keyboard focus 이동 후 idle/running extra를 닫고 sibling 공간 회수 및 focus fallback을 검증한다.

### Tests for User Story 3

- [x] T022 [P] [US3] Add spatial focus and close-collapse tests in `apps/agentic-workbench/src/entities/agent-run/model/agent-run-workspace.test.ts`
- [x] T023 [P] [US3] Add nested resize/render contract tests in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-tile-layout.test.tsx`

### Implementation for User Story 3

- [x] T024 [US3] Implement split resize callbacks and keyboard separators in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-tile-layout.tsx`
- [x] T025 [US3] Integrate tile focus, close confirmation, sibling fallback and status messaging in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`

**Checkpoint**: nested tile layout remains valid through resize/focus/close operations.

---

## Phase 6: User Story 4 - 에이전트 런 간 메시지 전달 (Priority: P2)

**Goal**: 동일 session window/worktree에서 사용자와 MCP agent가 peer를 조회하고 send/queue/draft exchange를 전달·추적한다.

**Independent Test**: 동일 창 두 panel 사이 세 delivery mode와 duplicate retry를 검증하고 다른 창/worktree/stale run 대상이 차단되는지 확인한다.

### Tests for User Story 4

- [x] T026 [P] [US4] Write frontend repository invoke/listen contract tests in `apps/agentic-workbench/src/entities/agent-run/api/agent-exchange-repository.test.ts`
- [x] T027 [P] [US4] Write backend registry revision/idempotency tests in `apps/agentic-workbench/src-tauri/src/infrastructure/in_memory_agent_workspace_registry.rs`
- [x] T028 [P] [US4] Write application owner/scope/stale-run tests in `apps/agentic-workbench/src-tauri/src/application/agent_exchange_service.rs`
- [x] T029 [P] [US4] Write MCP schema/tool dispatch tests in `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/agent_exchange_tool.rs`
- [x] T030 [P] [US4] Write peer dialog and targeted delivery tests in `apps/agentic-workbench/src/features/agent-run/ui/agent-peer-message-dialog.test.tsx`

### Implementation for User Story 4

- [x] T031 [US4] Define workspace registry and exchange event ports in `apps/agentic-workbench/src-tauri/src/ports/agent_workspace_registry.rs`
- [x] T032 [US4] Implement in-memory snapshots, exchange retention and dedupe in `apps/agentic-workbench/src-tauri/src/infrastructure/in_memory_agent_workspace_registry.rs`
- [x] T033 [US4] Implement sync/list/send/ack application use cases and owner validation in `apps/agentic-workbench/src-tauri/src/application/agent_exchange_service.rs`
- [x] T034 [P] [US4] Implement Tauri invoke/listen wrappers in `apps/agentic-workbench/src/entities/agent-run/api/agent-exchange-repository.ts`
- [x] T035 [US4] Add sync/send/ack/list commands and state management in `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs` and `apps/agentic-workbench/src-tauri/src/lib.rs`
- [x] T036 [US4] Implement peer list/send/status MCP schemas in `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/agent_exchange_tool.rs`
- [x] T037 [US4] Dispatch exchange MCP tools and agent instructions in `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/mod.rs`
- [x] T038 [P] [US4] Implement peer target/delivery dialog in `apps/agentic-workbench/src/features/agent-run/ui/agent-peer-message-dialog.tsx`
- [x] T039 [US4] Synchronize workspace snapshots and route/ack exchange events in `apps/agentic-workbench/src/features/agent-run/ui/worktree-agent-run-area.tsx`
- [x] T040 [US4] Surface exchange entry points/status on tile headers in `apps/agentic-workbench/src/features/agent-run/ui/agent-run-tile.tsx`

**Checkpoint**: UI and MCP exchange flows enforce owner scope and apply each request once.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: 문서, Storybook, 회귀 및 전체 검증을 완료한다.

- [x] T041 [P] Add Korean Mermaid architecture and exchange flow documentation in `docs/agent-run-tile-workspace.md`
- [x] T042 [P] Register toolbar/tile/layout/dialog states in `apps/agentic-workbench/src/stories/molecules.stories.tsx`, `apps/agentic-workbench/src/stories/organisms.stories.tsx`, and `apps/agentic-workbench/src/stories/pages.stories.tsx`
- [x] T043 Run focused frontend reducer/UI tests and fix regressions in `apps/agentic-workbench/src/entities/agent-run/` and `apps/agentic-workbench/src/features/agent-run/`
- [x] T044 Run `pnpm --filter @yoophi/agentic-workbench run check-types` and full AW tests
- [x] T045 Run `cargo test -p agentic-workbench` and `cargo check -p agentic-workbench`
- [x] T046 Validate `specs/032-agent-run-tiles/quickstart.md`, task completion, diff hygiene and existing untracked-file preservation

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup and blocks all stories.
- **US1 (Phase 3)**: Depends on Foundational and delivers the tab/tile MVP.
- **US2 (Phase 4)**: Depends on workspace projection from US1.
- **US3 (Phase 5)**: Depends on layout/tree UI from US1 and US2.
- **US4 (Phase 6)**: Depends on stable panel identity from US1; backend tasks can proceed independently after Foundational.
- **Polish (Phase 7)**: Depends on all selected stories.

### User Story Completion Order

```text
Foundation
  └─ US1 탭/타일 전환
       ├─ US2 인접 타일 생성
       │    └─ US3 resize/focus/close
       └─ US4 런 간 exchange
             └─ Polish + full validation
```

### Parallel Opportunities

- T004/T006/T008/T009는 서로 다른 foundational files에서 병렬 가능하다.
- US1의 toolbar tests/UI는 workspace integration과 분리 가능하다.
- US4의 frontend repository, Rust registry/service/MCP tests, peer dialog tests는 각각 다른 파일에서 병렬 가능하다.
- 문서와 Storybook은 runtime 구현 완료 후 서로 병렬 가능하다.

## Parallel Example: User Story 4

```text
Task T026: frontend invoke/listen contract tests
Task T027: Rust in-memory registry tests
Task T028: Rust application ownership tests
Task T029: MCP schema tests
Task T030: peer dialog tests
```

## Implementation Strategy

### MVP First

1. Phase 1 Setup
2. Phase 2 pure workspace foundation
3. Phase 3 US1 tab/tile view toggle
4. Focused tests and manual state-preservation validation

### Incremental Delivery

1. US1: view projection without behavior loss
2. US2: deterministic adjacent open
3. US3: operational tile management
4. US4: scoped communication interface
5. Documentation, Storybook and full verification

## Format Validation

- 모든 task는 `- [ ] T### ...` checklist 형식을 따른다.
- User Story phase task는 `[US#]` label을 가진다.
- `[P]`는 다른 incomplete task와 파일 충돌 없이 실행 가능한 task에만 사용한다.
- 모든 task description은 대상 파일 경로 또는 실행할 검증 범위를 명시한다.
