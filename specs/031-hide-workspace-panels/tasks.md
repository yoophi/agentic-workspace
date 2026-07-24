# Tasks: Hide Workspace Panels

**Input**: Design documents from `/specs/031-hide-workspace-panels/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [workspace-layout-ui.md](./contracts/workspace-layout-ui.md), [quickstart.md](./quickstart.md)

**Tests**: 레이아웃 선택·폭 제한은 순수 로직이므로 Vitest 단위 테스트를 먼저 작성한다. Worktree별 JSON 영속 경계는 Rust 단위 테스트를 먼저 작성한다. 화면 조립은 관련 React 테스트로 검증한다.

**Organization**: 사용자 스토리별로 작업을 나누어 각 스토리를 독립적으로 구현·검증할 수 있게 한다.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 선행 작업 완료 후 다른 파일에서 병렬 실행 가능
- **[Story]**: 해당 사용자 스토리
- 모든 작업은 정확한 파일 경로를 포함한다.

## Phase 1: Setup

**Purpose**: 현재 레이아웃의 책임 경계와 테스트 대상 파일을 준비한다.

- [ ] T001 `specs/031-hide-workspace-panels/contracts/workspace-layout-ui.md`를 기준으로 바깥·내부 분할의 B 폭, 선택 없음, 접근성 계약을 구현 체크 기준으로 확정한다.
- [ ] T002 [P] `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`와 `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`의 현재 분할 ID·최소 폭·탭 책임을 확인해 후속 변경의 기준선을 기록한다.

---

## Phase 2: Foundational — 패널 선택 상태

**Purpose**: 모든 사용자 스토리가 공통으로 사용하는 패널 식별자, 토글 규칙, 안전한 폭 계산을 준비한다.

**⚠️ CRITICAL**: 이 단계가 끝나기 전에는 사용자 스토리 UI를 구현하지 않는다.

- [ ] T003 `apps/agentic-workbench/src/features/worktree-workspace/model/workspace-layout.test.ts`에 선택된 패널 재선택 시 `null`로 전환하고, 다른 패널 선택 시 이전 선택을 교체하는 순수 상태 테스트를 작성한다.
- [ ] T004 `apps/agentic-workbench/src/features/worktree-workspace/model/workspace-layout.test.ts`에 저장된 B 폭을 현재 컨테이너와 최소 A/B 폭으로 표시 전용 제한하는 테스트를 작성한다.
- [ ] T005 `apps/agentic-workbench/src/features/worktree-workspace/model/workspace-layout.ts`에 `WorkspacePanelId`, 선택 토글, B 폭 정규화·표시 제한 헬퍼를 구현한다.
- [ ] T006 `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`에 선택 없음에서는 Workspace 콘텐츠를 렌더링하지 않고, 선택된 종류만 렌더링하는 화면 계약 테스트를 추가한다.

**Checkpoint**: 패널 선택과 B 폭 계산의 순수 규칙이 테스트로 고정되었다.

---

## Phase 3: User Story 1 — 보조 패널을 완전히 숨기기 (Priority: P1) 🎯 MVP

**Goal**: 현재 선택된 보조 패널을 해제해 Workspace B와 바깥 분할 핸들까지 화면에서 제거한다.

**Independent Test**: Git이 열린 상태에서 같은 제어를 다시 선택하면 Git·Files·Markdown·Speckit 콘텐츠와 바깥 핸들이 사라지고 에이전트 작업 영역만 남는다. 이어서 다른 패널을 선택하면 그 패널만 다시 표시된다.

### Tests for User Story 1

- [ ] T007 [P] [US1] `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.test.tsx`에 선택 없음에서 Workspace B와 바깥 리사이즈 핸들이 렌더링되지 않는 테스트를 작성한다.

### Implementation for User Story 1

- [ ] T008 [US1] `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`에 `WorkspacePanelId | null` 선택 상태를 두고, `null`일 때 Workspace B와 바깥 분할 핸들을 조건부로 제거하며 에이전트 A가 전체 남은 폭을 사용하게 한다.
- [ ] T009 [US1] `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`가 외부에서 전달받은 선택 패널만 렌더링하고, 선택 없음 상태에서는 콘텐츠를 만들지 않도록 props와 렌더 분기를 정리한다.

**Checkpoint**: 보조 패널을 숨기고 다시 여는 MVP 흐름을 독립적으로 검증할 수 있다.

---

## Phase 4: User Story 2 — 화면 오른쪽에서 패널 선택하기 (Priority: P2)

**Goal**: Git, Files, Markdown, Speckit을 화면 가장 오른쪽의 세로·90도 회전 식별 제어에서 선택 또는 해제한다.

**Independent Test**: 화면 오른쪽의 네 접근 가능한 제어 버튼에서 Git을 선택하고 재선택해 해제한 뒤, Files를 선택했을 때 Files만 표시되는지 키보드와 마우스로 확인한다.

### Tests for User Story 2

- [ ] T010 [P] [US2] `apps/agentic-workbench/src/features/worktree-workspace/ui/workspace-panel-selector.test.tsx`에 네 버튼의 접근 가능한 이름, 선택 상태, 재선택 해제 동작을 검증하는 테스트를 작성한다.

### Implementation for User Story 2

- [ ] T011 [US2] `apps/agentic-workbench/src/features/worktree-workspace/ui/workspace-panel-selector.tsx`에 Git·Files·Markdown·Speckit용 토글 버튼 그룹을 구현하고, 보이는 텍스트를 90도 회전하되 접근 가능한 이름과 `aria-pressed` 상태를 제공한다.
- [ ] T012 [US2] `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`에 selector를 가장 오른쪽 세로 영역으로 조립하고, 선택 변경을 Phase 3의 `WorkspacePanelId | null` 상태에 연결한다.

**Checkpoint**: 사용자 스토리 1의 숨김 동작을 유지하면서 오른쪽 제어 영역으로 모든 패널을 선택·해제할 수 있다.

---

## Phase 5: User Story 3 — 패널 없는 상태에서도 기존 작업 유지하기 (Priority: P3)

**Goal**: Workspace를 숨겨도 에이전트 실행, 프롬프트 입력 및 세션 작업 흐름이 계속 동작하게 한다.

**Independent Test**: Workspace를 숨긴 상태에서 프롬프트를 입력하고 실행 상태를 확인한 뒤, Markdown 패널을 다시 열어 기존 annotation prompt 전달 흐름을 사용할 수 있다.

### Tests for User Story 3

- [ ] T013 [P] [US3] `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.test.tsx`에 Workspace 숨김 상태에서도 `WorktreeAgentRunArea`가 유지되고, Workspace 재표시 뒤 annotation prompt 연결이 유지되는 테스트를 작성한다.

### Implementation for User Story 3

- [ ] T014 [US3] `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`에서 Workspace 조건부 마운트가 `workspacePromptRequest`와 `WorktreeAgentRunArea`의 수명·prompt 전달을 끊지 않도록 상태와 callback 배치를 정리한다.
- [ ] T015 [US3] `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`에서 재마운트 시 기존 Git·Files·Markdown·Speckit의 로딩·빈 상태·오류 상태와 watcher 정리 흐름을 유지하도록 effect와 초기 선택 처리를 검토·보완한다.

**Checkpoint**: Workspace의 표시 여부와 관계없이 핵심 에이전트 작업 흐름이 독립적으로 동작한다.

---

## Phase 6: User Story 4 — Worktree별 패널 크기 유지하기 (Priority: P4)

**Goal**: 외부 Workspace B 폭과 Git·Files·Markdown·Speckit 내부 B 폭을 Worktree 및 패널 종류별로 저장·복원하고, 모든 A 영역은 남은 공간을 사용한다.

**Independent Test**: 두 Worktree에서 서로 다른 바깥 B 폭과 내부 B 폭을 저장한 뒤 각각의 세션과 패널을 다시 열어 자기 값만 복원되는지 확인한다. 작은 창에서는 표시 폭만 제한되고 원래 선호 폭은 유지되어야 한다.

### Tests for User Story 4

- [ ] T016 [P] [US4] `apps/agentic-workbench/src-tauri/src/application/worktree_workspace_layout_service.rs`에 경로 정규화, 양의 정수 폭만 유지, 같은 Worktree upsert, 다른 Worktree 격리, 잘못된 폭 제거를 검증하는 Rust 단위 테스트를 작성한다.
- [ ] T017 [P] [US4] `apps/agentic-workbench/src/entities/worktree-workspace-layout/model/types.test.ts`에 Tauri 직렬화 타입과 Worktree·패널 종류별 B 폭 병합 규칙을 검증하는 테스트를 작성한다.
- [ ] T018 [P] [US4] `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx`에 저장된 패널 종류별 내부 B 폭을 올바른 Git·Files·Markdown·Speckit 분할에 적용하고 다른 종류에는 적용하지 않는 테스트를 추가한다.

### Implementation for User Story 4

- [ ] T019 [US4] `apps/agentic-workbench/src-tauri/src/domain/worktree_workspace_layout.rs`와 `apps/agentic-workbench/src-tauri/src/domain/worktree_workspace_layout_repository.rs`에 `WorkspaceLayoutSettings`, 패널 종류별 B 폭, repository port를 정의하고 `apps/agentic-workbench/src-tauri/src/domain/mod.rs`에 등록한다.
- [ ] T020 [US4] `apps/agentic-workbench/src-tauri/src/application/worktree_workspace_layout_service.rs`에 Worktree별 조회·upsert·정규화 규칙을 구현하고 `apps/agentic-workbench/src-tauri/src/application/mod.rs`에 등록한다.
- [ ] T021 [US4] `apps/agentic-workbench/src-tauri/src/infrastructure/json_worktree_workspace_layout_repository.rs`에 앱 데이터 디렉터리의 전용 JSON 저장소를 구현하고 `apps/agentic-workbench/src-tauri/src/infrastructure/mod.rs`에 등록한다.
- [ ] T022 [US4] `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`에 `get_worktree_workspace_layout` 및 `save_worktree_workspace_layout`의 얇은 command를 추가하고 `apps/agentic-workbench/src-tauri/src/lib.rs`에 등록한다.
- [ ] T023 [US4] `apps/agentic-workbench/src/entities/worktree-workspace-layout/model/types.ts`, `apps/agentic-workbench/src/entities/worktree-workspace-layout/api/query-keys.ts`, `apps/agentic-workbench/src/entities/worktree-workspace-layout/api/worktree-workspace-layout-repository.ts`에 레이아웃 타입, Worktree query key, Tauri 호출 래퍼를 구현한다.
- [ ] T024 [US4] `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`에 저장 레이아웃을 먼저 hydrate한 뒤 바깥 B의 픽셀 폭만 적용·저장하고, A가 남은 폭을 사용하게 하는 제어 로직을 구현한다.
- [ ] T025 [US4] `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`에 패널 종류별 내부 B 폭 hydrate·적용·안정 시 저장을 연결하고, Git·Files·Markdown·Speckit의 기존 내부 분할을 모두 `A:B = *:1`로 바꾼다.
- [ ] T026 [US4] `apps/agentic-workbench/src/features/worktree-workspace/model/workspace-layout.ts`와 `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`에서 작은 컨테이너의 표시 폭 제한이 저장된 선호 폭을 다시 저장하지 않도록 보호한다.

**Checkpoint**: Worktree별 바깥·내부 B 폭은 독립적으로 복원되고, 모든 A 영역은 현재 화면의 남은 공간을 사용한다.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: 전체 기능의 회귀, 접근성, 경계 및 검증 가이드를 마무리한다.

- [ ] T027 [P] `apps/agentic-workbench/src/features/worktree-workspace/ui/workspace-panel-selector.test.tsx`와 `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.test.tsx`에 빠른 반복 선택, 화면 축소, 선택 없음의 접근성 회귀 사례를 추가한다.
- [ ] T028 [P] `apps/agentic-workbench/src-tauri/src/infrastructure/json_worktree_workspace_layout_repository.rs`와 관련 Rust 테스트에서 손상되었거나 누락된 JSON 레코드가 다른 Worktree 설정을 손상시키지 않는지 검증한다.
- [ ] T029 `specs/031-hide-workspace-panels/quickstart.md`의 수동 시나리오를 실행하고, 검증 결과와 필요한 후속 문서 변경 여부를 해당 파일에 기록한다.
- [ ] T030 `apps/agentic-workbench`에서 `pnpm --dir apps/agentic-workbench check-types`, `pnpm --dir apps/agentic-workbench test`, `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml`, `git diff --check`를 실행하고 실패 사항을 기록·해결한다.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 즉시 시작 가능하다.
- **Foundational (Phase 2)**: Setup 완료 후 실행하며 모든 사용자 스토리를 막는다.
- **US1 (Phase 3)**: Foundation 뒤에 시작한다. MVP다.
- **US2 (Phase 4)**: US1의 선택 상태와 숨김 렌더를 사용한다.
- **US3 (Phase 5)**: US1의 조건부 Workspace 수명을 사용한다.
- **US4 (Phase 6)**: Foundation 뒤에 시작 가능하지만, 완전한 사용자 흐름 검증을 위해 US1·US2 뒤에 통합한다.
- **Polish (Phase 7)**: 필요한 사용자 스토리 완료 후 실행한다.

### User Story Dependencies

```text
Foundation
  └─ US1 (패널 숨김, MVP)
       ├─ US2 (오른쪽 세로 제어)
       ├─ US3 (에이전트 작업 유지)
       └─ US4 (Worktree별 B 폭 저장·복원)
            └─ Polish
```

### Parallel Opportunities

- Foundation의 T003과 T004는 같은 테스트 파일을 변경하므로 병렬 실행하지 않는다. 다른 파일의 T006은 T005 후에 실행한다.
- US4에서 T016, T017, T018은 서로 다른 파일의 테스트이므로 병렬 실행할 수 있다.
- US4의 도메인·프런트엔드 entity 작업은 계약을 공유하므로 T019의 모델 정의 후 T020과 T023을 병렬 실행할 수 있다.
- Polish의 T027과 T028은 서로 다른 프런트엔드·백엔드 파일에서 병렬 실행할 수 있다.

## Parallel Example: User Story 4

```text
Task: "T016 Rust persistence service tests in apps/agentic-workbench/src-tauri/src/application/worktree_workspace_layout_service.rs"
Task: "T017 Type serialization tests in apps/agentic-workbench/src/entities/worktree-workspace-layout/model/types.test.ts"
Task: "T018 Internal panel layout tests in apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.test.tsx"
```

## Implementation Strategy

### MVP First

1. Phase 1과 Phase 2를 완료한다.
2. Phase 3에서 선택된 패널을 다시 눌러 완전히 숨기는 흐름을 구현한다.
3. US1 독립 테스트를 통과시킨다.

### Incremental Delivery

1. US1로 Workspace를 숨기고 다시 표시한다.
2. US2로 오른쪽 세로 제어와 접근성을 제공한다.
3. US3로 Workspace 재마운트가 에이전트 작업을 방해하지 않음을 보장한다.
4. US4로 Worktree별 바깥·내부 B 폭 영속화를 추가한다.
5. Phase 7의 전체 검증을 통과시킨다.
