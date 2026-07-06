# Tasks: ACP Tool List 기반 Prompt Command 자동완성

**입력**: `specs/014-acp-tool-autocomplete/`의 설계 문서

**전제 조건**: plan.md, spec.md, research.md, data-model.md, contracts/

**테스트**: 사용자 여정 테스트는 spec에서 요구하지 않는 한 선택 사항입니다. Constitution이 요구하는 테스트는 선택 사항이 아닙니다. 순수 로직, 파서, 포매터, 그래프 레이아웃, reducer, 공유 패키지/crate, 안전 경계는 단위 테스트 또는 fixture 테스트가 필요합니다.

**구성**: 각 사용자 스토리를 독립적으로 구현하고 테스트할 수 있도록 작업을 사용자 스토리별로 그룹화합니다.

## 형식: `[ID] [P?] [Story] 설명`

- **[P]**: 병렬 실행 가능 작업입니다. 서로 다른 파일을 다루며 의존성이 없습니다.
- **[Story]**: 해당 작업이 속한 사용자 스토리입니다. 예: US1, US2, US3.
- 설명에는 정확한 파일 경로를 포함합니다.

## 경로 규칙

- **앱 프론트엔드**: `apps/agentic-workbench/src/{app,pages,features,entities,shared,components/ui}`
- **앱 Tauri 백엔드**: `apps/agentic-workbench/src-tauri/src/{domain,application,inbound,infrastructure,ports}`
- **문서**: `specs/014-acp-tool-autocomplete/*.md`
- 경로는 plan.md에서 선택한 구조와 일치해야 합니다.

## Phase 1: 설정 및 공유 인프라 확인

**목적**: 이번 작업에서 변경할 기존 prompt composer, agent-run entity API, Storybook, ACP/MCP 백엔드 경계를 확인합니다.

- [X] T001 `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에서 prompt composer 상태, 제출 처리, 히스토리 이동, 큐 단축키, 저장된 프롬프트 통합을 검토한다
- [X] T002 [P] `apps/agentic-workbench/src/components/ui/prompt-input.tsx`의 기존 prompt input primitive 키보드 동작을 검토한다
- [X] T003 [P] `apps/agentic-workbench/src/entities/agent-run/model/types.ts`와 `apps/agentic-workbench/src/entities/agent-run/api/agent-run-repository.ts`의 기존 agent-run 타입 및 repository 함수를 검토한다
- [X] T004 [P] `apps/agentic-workbench/src/stories/organisms.stories.tsx`의 기존 Agentic Workbench Storybook 구성을 검토한다
- [X] T005 [P] `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/title_tool.rs`와 `apps/agentic-workbench/src-tauri/src/infrastructure/acp/runner.rs`에서 현재 MCP tool catalog와 ACP session 시작 흐름을 검토한다

---

## Phase 2: 기반 작업

**목적**: 모든 사용자 스토리에 필요한 candidate 데이터 계약, session-scoped 조회 경계, 순수 autocomplete helper를 마련합니다.

**중요**: 이 phase가 완료되기 전에는 사용자 스토리 구현을 시작할 수 없습니다.

- [X] T006 `apps/agentic-workbench/src/entities/agent-run/model/types.ts`에 `AgentToolCommandCandidate`, `AgentToolCommandCandidateScope`, `AgentToolCommandCandidateStatus` 프론트엔드 타입을 추가한다
- [X] T007 `apps/agentic-workbench/src/entities/agent-run/api/agent-run-repository.ts`에 `listAgentToolCommandCandidates` repository adapter signature를 추가한다
- [X] T008 `apps/agentic-workbench/src/entities/agent-run/api/query-keys.ts`와 `apps/agentic-workbench/src/entities/agent-run/api/query-options.ts`에 session tool command candidate용 React Query key/options를 추가한다
- [X] T009 [P] `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts`에 trigger parsing, filtering, insertion 실패 우선 단위 테스트를 추가한다
- [X] T010 `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.ts`에 trigger parsing, candidate filtering, highlighted index clamping, token replacement helper를 구현한다
- [X] T011 [P] `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts`에 프론트엔드 candidate normalization 실패 우선 테스트를 추가한다
- [X] T012 `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.ts`에 빈 name/insertText를 거부하고 `$` 또는 `/` 입력 prefix를 보존하는 candidate normalization helper를 구현한다
- [X] T013 [P] `apps/agentic-workbench/src-tauri/src/domain/agent_tool_candidate.rs`에 candidate normalization과 empty candidate fallback용 Rust domain model 테스트를 추가한다
- [X] T014 `apps/agentic-workbench/src-tauri/src/domain/agent_tool_candidate.rs`에 Rust `AgentToolCandidate`, `AgentToolCandidateScope`, response status domain type을 추가한다
- [X] T015 `apps/agentic-workbench/src-tauri/src/domain/mod.rs`에 새 backend domain module을 등록한다
- [X] T016 [P] `apps/agentic-workbench/src-tauri/src/application/agent_tool_candidate_service.rs`에 run owner mismatch, missing active session, empty source, successful candidate normalization 실패 우선 application service 테스트를 추가한다
- [X] T017 `apps/agentic-workbench/src-tauri/src/application/agent_tool_candidate_service.rs`에 session/run owner scope check와 no persistence 보장을 포함한 `AgentToolCandidateService`를 구현한다
- [X] T018 `apps/agentic-workbench/src-tauri/src/application/mod.rs`에 새 backend application module을 등록한다
- [X] T019 `apps/agentic-workbench/src-tauri/src/ports/session_registry.rs`와 `apps/agentic-workbench/src-tauri/src/infrastructure/agent_session_registry.rs`에서 현재 run/session candidate metadata를 읽는 데 필요한 범위로만 session registry 또는 ACP session accessor를 확장한다
- [X] T020 `apps/agentic-workbench/src-tauri/src/infrastructure/mcp/title_tool.rs`에서 raw protocol을 노출하지 않고 normalized app MCP tool metadata를 candidate source로 제공한다
- [X] T021 `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`에 `AgentToolCandidateService`로 위임하는 `list_agent_tool_command_candidates` Tauri command를 추가한다
- [X] T022 `apps/agentic-workbench/src-tauri/src/lib.rs`의 Tauri command handler 목록에 `list_agent_tool_command_candidates`를 등록한다

**체크포인트**: foundation 준비 완료. 프론트엔드/백엔드 candidate 계약이 존재하고, 순수 helper 및 safety 테스트가 공유 동작을 검증합니다.

---

## Phase 3: User Story 1 - 프롬프트 작성 중 tool/command 후보 찾기 (Priority: P1) MVP

**목표**: prompt composer에서 `$` 또는 `/`를 입력하면 현재 session과 관련된 tool/command candidate 목록이 열리고, token query 변경에 따라 목록이 필터링됩니다.

**독립 테스트**: candidate data가 있는 상태에서 prompt textarea에 focus하고 `$` 또는 `/`를 입력한 뒤 candidate name 일부를 계속 입력합니다. 프롬프트가 제출되지 않고 목록이 표시되며 좁혀지는지 확인합니다.

### User Story 1 테스트

- [X] T023 [P] [US1] `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx`에 prompt autocomplete trigger와 filtering UI 테스트를 추가한다
- [X] T024 [P] [US1] `apps/agentic-workbench/src/entities/agent-run/api/query-options.test.ts`에 candidate status mapping용 repository/query 테스트를 추가한다

### User Story 1 구현

- [X] T025 [US1] `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.tsx`에 loading, ready, filtered candidate 상태를 표시하는 `PromptCommandAutocomplete` display component를 생성한다
- [X] T026 [US1] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에 prompt mode에서만 동작하는 autocomplete open/close 및 query derivation 상태를 추가한다
- [X] T027 [US1] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에서 selected agent, working directory, session mode, active run id를 사용해 `listAgentToolCommandCandidates` query를 prompt composer에 연결한다
- [X] T028 [US1] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에서 saved prompt toolbar, queued prompt timeline, prompt actions와 겹치지 않도록 prompt textarea 근처에 autocomplete surface를 렌더링한다
- [X] T029 [US1] `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.ts`에서 `$`와 `/`가 동일한 candidate set을 열되 사용자가 입력한 prefix를 보존하도록 보장한다

**체크포인트**: User Story 1이 MVP로 독립적으로 동작하고 테스트 가능합니다.

---

## Phase 4: User Story 2 - 후보 내용을 이해하고 선택하기 (Priority: P2)

**목표**: 사용자는 candidate name/description을 이해하고, 키보드 입력으로 highlight를 이동하고, candidate를 선택해 tool을 실행하거나 prompt를 제출하지 않은 채 active prompt token에 삽입할 수 있습니다.

**독립 테스트**: autocomplete를 열고 name/description이 있는 row를 확인합니다. ArrowUp/ArrowDown으로 highlight를 이동하고 Enter 또는 Tab을 눌러 active trigger token만 교체되는지 확인합니다.

### User Story 2 테스트

- [X] T030 [P] [US2] `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx`에 keyboard navigation, Enter selection, Tab selection, Escape cancellation UI 테스트를 추가한다
- [X] T031 [P] [US2] `apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts`에 token replacement cursor restoration 테스트를 추가한다
- [X] T032 [P] [US2] `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx`에 autocomplete 선택 중 prompt submit 없음, queue 없음, permission 변경 없음에 대한 non-execution regression 테스트를 추가한다

### User Story 2 구현

- [X] T033 [US2] `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.tsx`에 name, description, duplicate source hint, highlight state, accessible label을 갖춘 listbox style candidate row를 구현한다
- [X] T034 [US2] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에서 prompt submit/history/queue shortcut보다 먼저 ArrowUp, ArrowDown, Enter, Tab, Escape, pointer selection 처리를 수행한다
- [X] T035 [US2] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에서 candidate token replacement를 적용하고 선택 후 textarea focus/cursor를 복원한다
- [X] T036 [US2] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에서 autocomplete가 닫혀 있을 때 기존 prompt submit, history navigation, queue shortcut 동작을 보존한다
- [X] T037 [US2] `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.tsx`에서 긴 candidate name/description이 안정적인 row dimension 안에 표시되도록 처리한다

**체크포인트**: User Story 1과 2가 accidental prompt execution 없이 독립적으로 동작합니다.

---

## Phase 5: User Story 3 - 후보가 없거나 아직 준비되지 않은 상태에서 계속 작성하기 (Priority: P3)

**목표**: loading, empty, no-match, error 상태가 일반 prompt typing, line break, queueing, submission을 막지 않습니다.

**독립 테스트**: ready candidate가 없는 session을 사용하거나 일치하지 않는 token을 입력합니다. prompt를 계속 편집하고 정상 flow로 제출되는지 확인합니다.

### User Story 3 테스트

- [X] T038 [P] [US3] `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx`에 loading, empty, no-match, error state UI 테스트를 추가한다
- [X] T039 [P] [US3] `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx`에 autocomplete가 닫혔거나 사용 불가한 상태에서 기존 prompt submit, Shift+Enter, Tab queue, history navigation regression 테스트를 추가한다
- [X] T040 [P] [US3] `apps/agentic-workbench/src-tauri/src/application/agent_tool_candidate_service.rs`에 candidate lookup failure가 run을 시작하지 않고 empty/error response로 degrade되는 Rust service boundary 테스트를 추가한다

### User Story 3 구현

- [X] T041 [US3] `apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.tsx`에 textarea를 계속 enabled 상태로 유지하는 empty, no-match, loading, error UI state를 구현한다
- [X] T042 [US3] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에서 candidate query failure가 non-blocking으로 동작하고 prompt draft update가 local에 머물도록 한다
- [X] T043 [US3] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에서 Ralph loop input mode가 prompt command autocomplete를 열지 않도록 보장한다
- [X] T044 [US3] `apps/agentic-workbench/src-tauri/src/application/agent_tool_candidate_service.rs`에서 candidate lookup이 run 생성, prompt 전송, tool 호출, data persistence, permission mode 변경을 수행하지 않도록 보장한다
- [X] T045 [US3] `apps/agentic-workbench/src/stories/organisms.stories.tsx`에 loading, ready, many candidates, no-match, empty source, long content, keyboard-selected autocomplete용 Storybook state를 추가한다

**체크포인트**: 모든 사용자 스토리가 독립적으로 동작하고 fallback-safe합니다.

---

## Phase 6: 마무리 및 교차 관심사

**목적**: 기능을 end-to-end로 검증하고, 문서 산출물을 업데이트하며, story 간 동작을 정리합니다.

- [X] T046 [P] `pnpm --filter @yoophi/agentic-workbench check-types`를 실행하고 결과를 `specs/014-acp-tool-autocomplete/quickstart.md`에 기록한다
- [X] T047 [P] `pnpm --filter @yoophi/agentic-workbench test`를 실행하고 결과를 `specs/014-acp-tool-autocomplete/quickstart.md`에 기록한다
- [X] T048 [P] backend candidate lookup을 구현한 경우 `cargo test -p agentic-workbench`를 실행하고 결과를 `specs/014-acp-tool-autocomplete/quickstart.md`에 기록한다
- [X] T049 `specs/014-acp-tool-autocomplete/contracts/prompt-autocomplete-ui.md`의 Storybook autocomplete state를 `apps/agentic-workbench/src/stories/organisms.stories.tsx`와 대조해 검증한다
- [ ] T050 `specs/014-acp-tool-autocomplete/quickstart.md`의 `$`, `/`, filtering, keyboard selection, running-state Tab conflict, empty/fallback scenario를 수동으로 실행한다
- [X] T051 `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`에서 stale active run id, accidental submit path, prompt history regression, queue regression, Ralph loop leakage 여부를 검토한다
- [X] T052 `apps/agentic-workbench/src`와 `apps/agentic-workbench/src-tauri/src`에 app-to-app import, 새 cross-app package dependency, candidate persistence, raw ACP protocol object가 도입되지 않았는지 확인한다

---

## 의존성 및 실행 순서

### Phase 의존성

- **설정(Phase 1)**: 의존성이 없으므로 즉시 시작할 수 있습니다.
- **기반 작업(Phase 2)**: 설정 완료에 의존하며 모든 사용자 스토리를 blocking합니다.
- **User Story 1(Phase 3)**: 기반 작업 완료에 의존하며 MVP입니다.
- **User Story 2(Phase 4)**: 기반 작업 완료에 의존하며, US1이 autocomplete surface를 노출한 뒤 시작할 수 있습니다.
- **User Story 3(Phase 5)**: 기반 작업 완료에 의존하며, US1이 candidate status rendering을 정의한 뒤 시작할 수 있습니다.
- **마무리(Phase 6)**: 구현 대상 사용자 스토리가 모두 완료된 뒤 진행합니다.

### User Story 의존성

- **User Story 1(P1)**: 기반 작업 이후 시작할 수 있으며 다른 story에 의존하지 않습니다.
- **User Story 2(P2)**: 기반 작업 이후 시작할 수 있지만, 통합을 위해 US1 component/query shape에 의존합니다.
- **User Story 3(P3)**: 기반 작업 이후 시작할 수 있지만, US1 status model과 surface placement에 의존합니다.

### 각 User Story 내부 순서

- Constitution-required test는 구현 전에 작성하고 실패를 확인해야 합니다.
- 순수 helper를 UI 통합보다 먼저 구현합니다.
- domain/application safety를 inbound Tauri command wiring보다 먼저 구현합니다.
- candidate query contract를 prompt panel 소비보다 먼저 정의합니다.
- UI behavior를 Storybook/manual validation보다 먼저 구현합니다.

### 병렬화 가능 작업

- T002-T005는 T001과 병렬 실행할 수 있습니다.
- T009, T011, T013, T016은 type name 합의 후 병렬 작성할 수 있습니다.
- T023과 T024는 US1에서 병렬 실행할 수 있습니다.
- T030, T031, T032는 US2에서 병렬 실행할 수 있습니다.
- T038, T039, T040은 US3에서 병렬 실행할 수 있습니다.
- T046, T047, T048은 구현 완료 후 병렬 실행할 수 있습니다.

---

## 병렬 작업 예시: User Story 1

```bash
Task: "apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx에 prompt autocomplete trigger와 filtering UI 테스트 추가"
Task: "apps/agentic-workbench/src/entities/agent-run/api/query-options.test.ts에 candidate status mapping용 repository/query 테스트 추가"
```

## 병렬 작업 예시: User Story 2

```bash
Task: "apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx에 keyboard navigation, Enter selection, Tab selection, Escape cancellation UI 테스트 추가"
Task: "apps/agentic-workbench/src/entities/agent-run/model/prompt-autocomplete.test.ts에 token replacement cursor restoration 테스트 추가"
Task: "apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx에 autocomplete 선택 중 prompt submit 없음, queue 없음, permission 변경 없음 regression 테스트 추가"
```

## 병렬 작업 예시: User Story 3

```bash
Task: "apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx에 loading, empty, no-match, error state UI 테스트 추가"
Task: "apps/agentic-workbench/src/features/agent-run/ui/prompt-command-autocomplete.test.tsx에 autocomplete closed/unavailable 상태의 기존 prompt submit, Shift+Enter, Tab queue, history navigation regression 테스트 추가"
Task: "apps/agentic-workbench/src-tauri/src/application/agent_tool_candidate_service.rs에 candidate lookup failure가 run을 시작하지 않고 empty/error response로 degrade되는 Rust service boundary 테스트 추가"
```

---

## 구현 전략

### MVP 우선: User Story 1만 구현

1. Phase 1 설정을 완료합니다.
2. Phase 2 기반 작업을 완료합니다.
3. Phase 3 User Story 1을 완료합니다.
4. `$`와 `/`가 prompt를 제출하지 않고 session candidate row를 열고 필터링하는지 검증한 뒤 멈춥니다.

### 점진적 전달

1. 설정과 기반 작업을 완료합니다.
2. User Story 1을 추가하고 candidate discovery/filtering을 검증합니다.
3. User Story 2를 추가하고 keyboard/pointer selection 및 accidental execution 방지를 검증합니다.
4. User Story 3을 추가하고 loading/empty/no-match/error fallback 동작을 검증합니다.
5. quickstart의 static, Storybook, manual check를 실행합니다.

### 병렬 팀 전략

1. 설정과 기반 작업을 함께 완료합니다.
2. US1을 먼저 구현해 component/query surface를 확정합니다.
3. US1 shape가 안정되면 한 명은 US2 keyboard/selection을 구현하고, 다른 한 명은 US3 fallback state와 Storybook coverage를 병렬로 구현할 수 있습니다.

---

## 참고

- [P] 작업은 서로 다른 파일 또는 독립 테스트 케이스를 사용하며, 미완료 구현 작업에 의존하지 않습니다.
- [US1], [US2], [US3] label은 `specs/014-acp-tool-autocomplete/spec.md`의 사용자 스토리에 직접 대응합니다.
- 이 기능은 ACP/session tool list 기반으로 정의되었기 때문에 backend 작업을 포함합니다. 구현 중 기존 frontend candidate source가 이미 `contracts/session-tool-candidates.md`를 만족한다고 확인되면 동일한 service/query contract를 유지하고, 중복 backend adapter 작업은 근거를 남긴 뒤 구현 중 불필요 항목으로 표시합니다.
