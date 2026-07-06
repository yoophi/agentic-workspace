# Feature Specification: Queue Prompt Order

**Feature Branch**: `012-queue-prompt-order`

**Created**: 2026-07-06

**Status**: Draft

**Input**: User description: "worktreesessionpage 에서 agent 세션이 최초 실행될 때, 프롬프트 입력<RETURN> 실행하면, 프롬프트가 먼저 agent-run 영역에 표시되고 agent 실행 메시지가 표시되고, agent 응답이 표시됩니다. 이 순서를 변경하여 아래와 같이 변경합니다. - 프롬프트 입력<RETURN> - agent 실행, 입력한 프롬프트는 queue 입력된 프롬프트 형식으로 대기중으로 표시 - agent 실행 완료 후 프롬프트 실행 - 프롬프트에 대한 출력 표시"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 최초 입력 프롬프트를 대기 상태로 표시한다 (Priority: P1)

사용자가 Worktree Session Page에서 아직 agent 세션이 준비되지 않은 상태로 프롬프트를 입력하고 Return으로 실행하면, 사용자는 입력한 프롬프트가 즉시 실행된 메시지가 아니라 대기 중인 queued prompt로 표시되는 것을 본다.

**Why this priority**: 최초 실행 시 agent 준비 과정과 실제 프롬프트 실행이 섞여 보이면 사용자는 프롬프트가 언제 처리되었는지 오해할 수 있다. 대기 상태를 먼저 보여주면 agent 실행 준비와 사용자 프롬프트 처리가 명확히 분리된다.

**Independent Test**: 새 Worktree Session에서 첫 프롬프트를 입력하고 Return을 눌렀을 때, agent-run 영역에 입력 프롬프트가 queued prompt 형식과 대기 상태로 표시되고, 실행 완료 전에는 프롬프트 출력이 나타나지 않는지 확인한다.

**Acceptance Scenarios**:

1. **Given** agent 세션이 아직 최초 실행되지 않은 Worktree Session Page가 열려 있다, **When** 사용자가 프롬프트를 입력하고 Return으로 실행한다, **Then** 입력한 프롬프트는 agent-run 영역에 queued prompt 형식으로 대기 중 상태로 표시된다.
2. **Given** 첫 프롬프트가 queued prompt로 대기 중이다, **When** agent 실행 준비 메시지가 표시된다, **Then** 입력 프롬프트는 실행 완료 메시지보다 앞서 실행된 사용자 메시지처럼 표시되지 않는다.
3. **Given** 첫 프롬프트가 queued prompt로 대기 중이다, **When** agent 세션 실행 준비가 끝나지 않았다, **Then** 프롬프트에 대한 출력은 아직 표시되지 않는다.

---

### User Story 2 - agent 준비 완료 후 대기 프롬프트를 실행한다 (Priority: P2)

사용자는 agent가 먼저 실행되고 준비가 끝난 뒤, 대기 중이던 첫 프롬프트가 실제로 실행되는 순서를 agent-run 영역에서 확인할 수 있다.

**Why this priority**: 사용자에게 보이는 타임라인은 실제 처리 순서를 반영해야 한다. agent 준비 완료 이후 프롬프트 실행이 이어져야 실행 메시지와 프롬프트 응답의 인과관계가 명확해진다.

**Independent Test**: 첫 프롬프트 입력 후 agent 실행 준비가 완료될 때까지 기다린 뒤, queued prompt가 실행 상태 또는 실행된 프롬프트 표시로 전환되고 그 다음에 프롬프트 출력이 표시되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 첫 프롬프트가 queued prompt로 대기 중이다, **When** agent 실행 준비가 완료된다, **Then** 시스템은 대기 중인 프롬프트를 실행 대상으로 전환한다.
2. **Given** 대기 중인 첫 프롬프트가 실행 대상으로 전환되었다, **When** agent가 프롬프트를 처리한다, **Then** 프롬프트 출력은 해당 프롬프트 다음에 표시된다.
3. **Given** agent 실행 메시지와 프롬프트 출력이 모두 표시된다, **When** 사용자가 타임라인을 읽는다, **Then** 순서는 agent 실행, queued prompt 실행, 프롬프트 출력으로 이해 가능해야 한다.

---

### User Story 3 - 기존 실행 중 프롬프트 큐 동작과 일관성을 유지한다 (Priority: P3)

사용자는 최초 실행 프롬프트와 이후 실행 중 추가 입력한 프롬프트가 같은 queued prompt 표현 규칙을 따르는 것을 볼 수 있다.

**Why this priority**: 같은 대기 상태가 상황마다 다르게 표현되면 사용자는 어떤 프롬프트가 대기 중이고 어떤 프롬프트가 이미 실행됐는지 혼동한다.

**Independent Test**: agent가 실행 중일 때 추가 프롬프트를 입력하는 기존 큐 동작과 최초 실행 프롬프트 표시를 비교해, 대기 상태의 시각적 표현과 전환 규칙이 일관적인지 확인한다.

**Acceptance Scenarios**:

1. **Given** agent 실행 중 추가 프롬프트 큐 표시가 존재한다, **When** 최초 실행 프롬프트가 대기 중으로 표시된다, **Then** 두 대기 프롬프트는 사용자가 같은 상태로 인식할 수 있는 형식이어야 한다.
2. **Given** 첫 프롬프트가 대기 중이다, **When** 사용자가 다른 프롬프트를 이어서 입력한다, **Then** 각 프롬프트의 대기 및 실행 순서가 혼동 없이 구분된다.

### Edge Cases

- agent 세션 최초 실행이 실패하면 queued prompt는 실행된 것처럼 표시되지 않고 실패 또는 대기 해제 상태를 사용자가 이해할 수 있어야 한다.
- 사용자가 빈 프롬프트나 공백뿐인 프롬프트를 Return으로 제출하면 agent 실행이나 queue 표시가 시작되지 않아야 한다.
- 최초 프롬프트가 대기 중인 동안 사용자가 추가 프롬프트를 입력하면 입력 순서가 보존되어야 한다.
- 사용자가 실행 직후 페이지를 새로고침하거나 다른 화면으로 이동했다가 돌아와도 이미 표시된 실행 순서가 뒤바뀌어 보이면 안 된다.
- agent 준비 메시지가 빠르게 끝나는 경우에도 사용자는 프롬프트가 agent 준비보다 먼저 실행된 것처럼 보지 않아야 한다.
- agent가 응답 없이 종료되거나 취소되면 queued prompt와 출력 영역은 완료되지 않은 상태를 명확히 드러내야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST treat the first prompt submitted before an agent session is ready as a queued prompt rather than as an already executed prompt.
- **FR-002**: System MUST display the submitted first prompt in the agent-run area using the same user-recognizable queued prompt format used for pending prompts.
- **FR-003**: System MUST start or prepare the agent session after the first prompt is submitted without displaying that prompt as completed execution first.
- **FR-004**: System MUST execute the queued first prompt only after the agent session is ready to accept prompt work.
- **FR-005**: System MUST display the prompt output after the queued first prompt is executed.
- **FR-006**: System MUST preserve the visible order: prompt submission, queued prompt waiting state, agent execution/preparation, queued prompt execution, prompt output.
- **FR-007**: System MUST keep queued prompt ordering stable when multiple prompts are submitted before the first prompt finishes.
- **FR-008**: System MUST avoid duplicating the first prompt as both a queued prompt and an already executed prompt in the same run timeline.
- **FR-009**: System MUST expose failure, cancellation, or unavailable-agent outcomes without making the queued first prompt appear successfully executed.
- **FR-010**: System MUST keep existing non-initial prompt queue behavior consistent unless a difference is required to represent first-run preparation.
- **FR-011**: System MUST not change the text content submitted by the user while moving it between queued, executing, and output states.

### Key Entities

- **First Submitted Prompt**: The prompt entered with Return before the agent session has completed its initial execution or preparation.
- **Queued Prompt**: A prompt that has been accepted by the interface but is waiting for the agent to become ready or finish earlier work.
- **Agent Run Timeline**: The ordered visible sequence of agent preparation messages, queued prompts, prompt execution state, and prompt output.
- **Prompt Output**: The content produced in response to a submitted prompt after that prompt has actually run.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: Scope is limited to `apps/agentic-workbench` because Worktree Session Page, agent session startup, and agent-run timeline display are app-specific. Cross-app sharing is not required.
- **Frontend layering**: Worktree Session Page screen behavior belongs in `pages`; prompt submission and queue interactions belong in `features`; agent run and prompt timeline state should use `entities`; reusable display primitives may belong in `shared` or existing UI component locations when already established.
- **Backend boundary**: If agent startup or prompt dispatch sequencing requires backend changes, session readiness and prompt dispatch rules belong in application services, while Tauri commands remain inbound adapters and provider/session persistence remains infrastructure. Domain models must not depend on Tauri or storage details.
- **Shared core vs UI**: Reuse should focus on headless prompt queue state and timeline status modeling if multiple UI surfaces consume it. Shared UI is not required unless an existing reusable queued prompt component is being formalized.
- **Persistence and safety**: The feature must preserve run/session owner scope. It must not dispatch a queued prompt to the wrong agent run, duplicate prompt execution, or persist inconsistent timeline state.
- **Documentation and Storybook**: Add or update Korean project documentation under `docs/*.md` only if the agent startup/prompt queue flow is documented. Storybook updates are required if a reusable queued prompt component or new visual state is introduced.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of tested first-run submissions, the entered prompt is first shown as a queued prompt before any prompt-specific output appears.
- **SC-002**: In 100% of tested successful first-run submissions, prompt output appears only after the agent has reached a ready/executing state for that prompt.
- **SC-003**: In 100% of tested first-run submissions, the entered prompt appears no more than once as the user-submitted prompt in the agent-run timeline.
- **SC-004**: At least 95% of users reviewing the first-run timeline can correctly identify that the agent starts first and the queued prompt runs afterward.
- **SC-005**: In multi-prompt first-run tests, prompts are executed and displayed in submission order with zero observed reordering.

## Assumptions

- "최초 실행" means the first prompt submitted in a Worktree Session before the agent session is ready to process prompts.
- The queued prompt visual format already exists or is expected from current behavior for prompts submitted while an agent is busy.
- The user wants the visible timeline order changed without changing the actual prompt text or the user's Return-to-submit interaction.
- The feature is scoped to Worktree Session Page agent-run display and dispatch ordering; broader prompt history, model selection, and permission approval changes are out of scope.
- Existing agent execution messages remain visible, but they should no longer make the first prompt look like it ran before agent startup completed.
