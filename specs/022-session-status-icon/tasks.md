# 작업 목록: 세션 상태 아이콘

**Input**: `specs/022-session-status-icon/`의 설계 문서

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/session-status-ui.md](./contracts/session-status-ui.md), [quickstart.md](./quickstart.md)

**Tests**: FR-008과 프로젝트 constitution에 따라 순수 event classification 및 run state logic 테스트가 필수다.

**Organization**: 각 사용자 스토리를 독립적으로 구현하고 검증할 수 있도록 작업을 story별로 묶는다.

## 형식: `[ID] [P?] [Story] 설명`

- **[P]**: 다른 파일을 다루거나 미완료 작업에 의존하지 않아 병렬 실행 가능함을 뜻한다.
- **[Story]**: [spec.md](./spec.md)의 사용자 스토리와 매핑된다.
- 모든 작업 설명에는 정확한 파일 경로를 포함한다.

## Phase 1: Setup (공통 기반 확인)

**목적**: 현재 agent-run surface를 확인하고 구현 대상 파일을 확정한다.

- [X] T001 `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`의 현재 raw event handling을 확인하고 기존 `session_info_update` 분기 지점을 파악한다.
- [X] T002 `apps/agentic-workbench/src/entities/agent-run/model/format.ts`의 현재 timeline formatting 동작을 확인하고 non-session raw event 동작 유지 필요성을 확인한다.
- [X] T003 `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`의 current run state reducer 동작을 확인하고 active-run filtering, usage event, lifecycle update 적용 위치를 파악한다.

---

## Phase 2: Foundational (차단 기반 작업)

**목적**: 모든 사용자 스토리가 공유하는 typed model/helper surface를 추가한다.

**CRITICAL**: 이 phase가 끝나기 전에는 사용자 스토리 구현을 시작하지 않는다.

- [X] T004 `apps/agentic-workbench/src/entities/agent-run/model/types.ts`에 `AgentThreadStatus`와 `SessionInfoUpdateMetadata` type을 추가한다.
- [X] T005 `apps/agentic-workbench/src/entities/agent-run/model/format.ts`에 `isSessionInfoUpdateEvent`와 `readAgentThreadStatus` export helper skeleton을 추가한다.
- [X] T006 `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`의 `RunEventState`에 기본 unknown/null 상태를 가진 `agentThreadStatus`를 추가한다.

**Checkpoint**: Foundation 준비 완료. 사용자 스토리 구현을 시작할 수 있다.

---

## Phase 3: 사용자 스토리 1 - 세션 메타데이터를 타임라인에서 숨김 (Priority: P1) MVP

**목표**: `session_info_update` event가 raw JSON timeline entry를 만들지 않게 하면서 정상 timeline message는 유지한다.

**독립 테스트**: active, idle, metadata-only `session_info_update` event를 active run에 보내 timeline item count가 변하지 않는지 확인한다. 다른 raw event를 보내 기존 raw behavior가 유지되는지도 확인한다.

### 사용자 스토리 1 테스트

- [X] T007 [P] [US1] `apps/agentic-workbench/src/entities/agent-run/model/format.test.ts`에 direct payload 및 nested `update` payload shape를 다루는 `isSessionInfoUpdateEvent` 실패 테스트를 추가한다.
- [X] T008 [P] [US1] `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`에 active, idle, metadata-only `session_info_update` event가 timeline item을 append하지 않음을 증명하는 실패 reducer test를 추가한다.
- [X] T009 [P] [US1] `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`에 non-session-info raw event는 여전히 raw timeline item으로 append됨을 증명하는 실패 reducer test를 추가한다.

### 사용자 스토리 1 구현

- [X] T010 [US1] `apps/agentic-workbench/src/entities/agent-run/model/format.ts`에 direct 및 nested payload shape를 모두 처리하는 robust `session_info_update` detection을 구현한다.
- [X] T011 [US1] `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`의 `applyRunEvent`를 수정해 `session_info_update` event는 timeline append를 건너뛰게 한다.
- [X] T012 [US1] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`의 live event listener 경로를 수정해 `session_info_update` event가 `addRunEventItem`을 호출하지 않게 한다.
- [X] T013 [US1] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`의 component-local `isIdleThreadStatusEvent` helper를 제거하거나 shared model helper로 대체한다.

**Checkpoint**: raw `session_info_update` payload가 timeline에 보이지 않고 다른 raw event 동작이 유지되면 사용자 스토리 1 완료다.

---

## Phase 4: 사용자 스토리 2 - 현재 Agent 활동 상태 표시 (Priority: P2)

**목표**: 세션 metadata에서 active/idle agent thread state를 추출하고 compact한 visible status로 렌더링한다.

**독립 테스트**: active 후 idle status update를 active run에 보내 reducer test에서 latest state 변경을 확인하고, UI가 접근 가능한 서로 다른 status text를 렌더링하는지 확인한다.

### 사용자 스토리 2 테스트

- [X] T014 [P] [US2] `apps/agentic-workbench/src/entities/agent-run/model/format.test.ts`에 active, idle, unknown, missing metadata, repeated status payload를 다루는 `readAgentThreadStatus` 실패 unit test를 추가한다.
- [X] T015 [P] [US2] `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`에 active/idle update가 `agentThreadStatus`를 변경하고 idle이 `isAwaitingPromptResponse`를 clear함을 증명하는 실패 reducer test를 추가한다.
- [X] T016 [P] [US2] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`에 active, idle, unknown status indicator rendering 실패 UI test를 추가한다.

### 사용자 스토리 2 구현

- [X] T017 [US2] `apps/agentic-workbench/src/entities/agent-run/model/format.ts`에 active, idle, unknown/unchanged 결과를 반환하는 `readAgentThreadStatus`를 구현한다.
- [X] T018 [US2] `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`의 `applyRunEvent`를 수정해 active/idle session update의 latest `agentThreadStatus`를 보존한다.
- [X] T019 [US2] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에 live `agentThreadStatus` React state를 추가하고 raw session info update에서 갱신한다.
- [X] T020 [US2] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`의 run/session header 또는 agent identity 주변에 compact active/idle/unknown status indicator UI를 추가한다.
- [X] T021 [US2] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에 active, idle, unknown status state의 accessible label을 추가한다.

**Checkpoint**: active/idle update가 timeline 증가 없이 run panel 안에서 보이면 사용자 스토리 2 완료다.

---

## Phase 5: 사용자 스토리 3 - 세션 메타데이터 효과 보존 (Priority: P3)

**목표**: metadata-only session update를 안전하게 처리한다. raw JSON을 보이지 않게 하고, 유용한 status를 clear하지 않으며, 적절한 기존 UI가 있을 때만 session summary field를 갱신한다.

**독립 테스트**: active status 이후 metadata-only update를 보내 status가 active로 유지되고, timeline이 변하지 않으며, visible error가 없는지 확인한다.

### 사용자 스토리 3 테스트

- [X] T022 [P] [US3] `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts`에 metadata-only `session_info_update`가 이전 `agentThreadStatus`를 보존함을 증명하는 실패 reducer test를 추가한다.
- [X] T023 [P] [US3] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx`에 metadata-only `session_info_update`가 raw JSON을 렌더링하지 않고 current status indicator를 제거하지 않음을 증명하는 실패 UI test를 추가한다.

### 사용자 스토리 3 구현

- [X] T024 [US3] `apps/agentic-workbench/src/entities/agent-run/model/format.ts`에서 metadata-only `session_info_update`가 idle/unknown overwrite 대신 unchanged status result를 반환하도록 보장한다.
- [X] T025 [US3] `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`에서 metadata-only update 처리 시 기존 `agentThreadStatus`를 보존한다.
- [X] T026 [US3] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에서 metadata-only update 처리 시 기존 status indicator state를 보존한다.
- [X] T027 [US3] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에 title 또는 updatedAt metadata를 적용할 기존 supported session header/list field가 있는지 확인하고, 없으면 의도적 no-op임을 설명하는 짧은 inline code comment를 남긴다.

**Checkpoint**: metadata-only update가 timeline에서 숨겨지고 visible status를 회귀시키지 않으면 사용자 스토리 3 완료다.

---

## Phase 6: Polish & Cross-Cutting Concerns

**목적**: 모든 story에 대한 최종 검증, 정리, regression check를 수행한다.

- [X] T028 [P] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에서 obsolete duplicate session-info parsing code를 제거한다.
- [X] T029 [P] `apps/agentic-workbench/src/entities/agent-run/model/index.ts`의 export name을 검토해 새 helper/type이 circular import 없이 접근 가능한지 확인한다.
- [X] T030 `specs/022-session-status-icon/quickstart.md`의 focused test 명령 `pnpm --filter agentic-workbench exec vitest run src/entities/agent-run/model/format.test.ts src/features/agent-run/model/run-panel-state.test.ts src/features/agent-run/ui/agent-run-panel.test.tsx`를 실행한다.
- [X] T031 `pnpm --filter agentic-workbench check-types`를 실행한다.
- [X] T032 `pnpm --filter agentic-workbench test`를 실행한다.
- [X] T033 `specs/022-session-status-icon/quickstart.md`의 시나리오에 따라 active, idle, metadata-only, other raw event 동작을 수동 검증한다.
- [X] T034 `git diff --stat` 및 변경 import path를 확인해 app-to-app import, backend change, shared package change, persistence change가 추가되지 않았는지 검증한다.

---

## 의존성 및 실행 순서

### Phase 의존성

- **Setup (Phase 1)**: 의존성 없음. 즉시 시작 가능하다.
- **Foundational (Phase 2)**: Setup 완료 후 진행한다. 모든 사용자 스토리를 차단한다.
- **User Story 1 (Phase 3)**: Foundational에 의존하며 MVP다.
- **User Story 2 (Phase 4)**: Foundational에 의존하고 US1 filtering의 이점을 받는다. helper return shape가 정해진 뒤 reducer/UI status test는 병렬로 작성할 수 있다.
- **User Story 3 (Phase 5)**: 기존 visible status 보존을 검증하므로 US2 status state에 의존한다.
- **Polish (Phase 6)**: 선택한 사용자 스토리 완료 후 진행한다.

### 사용자 스토리 의존성

- **US1 (P1)**: 필수 MVP다. US2 또는 US3에 의존하지 않는다.
- **US2 (P2)**: Foundation의 shared session-info helper surface가 필요하다. US1 이후에 진행하거나 helper return shape 합의 후 병렬 개발할 수 있다.
- **US3 (P3)**: metadata-only 보존을 검증하려면 US2의 latest-status state가 필요하다.

### 각 사용자 스토리 내부 순서

- 순수 helper/reducer 동작에 대한 실패 테스트를 먼저 작성한다.
- entity model helper를 run-panel state update보다 먼저 구현한다.
- state update를 UI rendering보다 먼저 구현한다.
- 각 story를 독립 검증한 뒤 다음 priority로 이동한다.

### 병렬화 기회

- T001, T002, T003은 서로 다른 파일을 읽으므로 병렬 가능하다.
- T007, T008, T009는 T004-T006 이후 병렬 작성 가능하다.
- T014, T015, T016은 helper/state shape가 정해진 뒤 병렬 작성 가능하다.
- T022, T023은 US2 status state가 준비된 뒤 병렬 작성 가능하다.
- T028, T029는 최종 validation command 전 병렬 검토 가능하다.

---

## 병렬 예시: 사용자 스토리 1

```text
Task: "T007 [P] [US1] apps/agentic-workbench/src/entities/agent-run/model/format.test.ts에 isSessionInfoUpdateEvent 실패 unit test 추가"
Task: "T008 [P] [US1] apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts에 timeline suppression 실패 reducer test 추가"
Task: "T009 [P] [US1] apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts에 other raw event 보존 실패 reducer test 추가"
```

## 병렬 예시: 사용자 스토리 2

```text
Task: "T014 [P] [US2] apps/agentic-workbench/src/entities/agent-run/model/format.test.ts에 readAgentThreadStatus 실패 unit test 추가"
Task: "T015 [P] [US2] apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts에 active/idle state update 실패 reducer test 추가"
Task: "T016 [P] [US2] apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx에 status indicator rendering 실패 UI test 추가"
```

## 병렬 예시: 사용자 스토리 3

```text
Task: "T022 [P] [US3] apps/agentic-workbench/src/features/agent-run/model/run-panel-state.test.ts에 metadata-only status preservation 실패 reducer test 추가"
Task: "T023 [P] [US3] apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.test.tsx에 metadata-only UI preservation 실패 UI test 추가"
```

---

## 구현 전략

### MVP First (사용자 스토리 1만)

1. Phase 1과 Phase 2를 완료한다.
2. US1의 T007-T013을 완료한다.
3. `format.test.ts`와 `run-panel-state.test.ts` focused test를 실행한다.
4. `session_info_update` raw JSON이 더 이상 보이지 않고 다른 raw event가 기존대로 동작하는지 검증한다.

### 점진적 전달

1. US1을 전달해 raw timeline noise를 제거한다.
2. US2를 전달해 active/idle status를 시각적으로 노출한다.
3. US3를 전달해 metadata-only behavior를 강화한다.
4. focused test, typecheck, app test, quickstart 수동 검증을 실행한다.

### 참고

- `[P]` 작업은 별도 파일 또는 독립 test case를 사용하므로 전제 조건 충족 후 병렬화할 수 있다.
- 모든 task path는 [plan.md](./plan.md)의 선택 구조와 일치한다.
- backend, persistence, `packages/*`, `crates/*` 작업은 이 feature 범위 밖이므로 포함하지 않는다.
