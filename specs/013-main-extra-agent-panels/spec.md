# Feature Specification: Main and Extra Agent Run Panels

**Feature Branch**: `013-main-extra-agent-panels`

**Created**: 2026-07-06

**Status**: Draft

**Input**: User description: "docs/main-extra-agent-run-panels-design.md 설계 내용 구현"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - main agent 패널을 항상 유지한다 (Priority: P1)

사용자는 Worktree Session에서 기본 agent 작업 흐름을 담당하는 main agent 패널을 항상 볼 수 있고, 이 패널이 실수로 닫히거나 제거되지 않는다는 것을 신뢰할 수 있다.

**Why this priority**: main agent 패널은 goal 자동 이어가기와 기본 작업 흐름의 기준점이다. 이 기준점이 안정적으로 유지되어야 extra agent 패널을 추가해도 사용자가 현재 세션의 중심 작업을 잃지 않는다.

**Independent Test**: Worktree Session을 열고 agent 영역을 확인했을 때 main 패널이 첫 번째 탭으로 표시되며, extra 패널이 없거나 여러 개 있어도 main 패널을 닫는 동작이 제공되지 않는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 사용자가 Worktree Session Page를 열었다, **When** agent 영역이 표시된다, **Then** main agent 패널이 항상 첫 번째 패널로 표시된다.
2. **Given** extra agent 패널이 여러 개 존재한다, **When** 사용자가 탭 목록을 확인한다, **Then** main agent 패널에는 닫기 동작이 제공되지 않는다.
3. **Given** 모든 extra agent 패널이 종료되었다, **When** agent 영역이 갱신된다, **Then** main agent 패널은 그대로 유지되고 active 패널이 된다.

---

### User Story 2 - extra agent 패널을 추가하고 전환한다 (Priority: P2)

사용자는 같은 worktree 안에서 구현, 검토, 실험 같은 별도 작업 의도를 분리하기 위해 extra agent 패널을 필요할 때 추가하고 탭으로 전환할 수 있다.

**Why this priority**: 하나의 timeline에 여러 의도가 섞이면 작업 맥락을 추적하기 어렵다. extra 패널은 병렬 작업을 분리하면서도 같은 worktree 문맥을 공유할 수 있게 한다.

**Independent Test**: 사용자가 extra 추가 동작을 실행했을 때 새 extra 패널이 고유한 이름으로 생성되고 active 탭으로 전환되며, main과 다른 extra 패널로 다시 전환할 수 있는지 확인한다.

**Acceptance Scenarios**:

1. **Given** Worktree Session에 main 패널만 존재한다, **When** 사용자가 extra 패널 추가 동작을 실행한다, **Then** `Extra 1` 패널이 생성되고 active 패널로 전환된다.
2. **Given** `Extra 1`이 존재한다, **When** 사용자가 extra 패널 추가 동작을 다시 실행한다, **Then** 기존 패널과 구분되는 새 extra 패널이 생성된다.
3. **Given** main과 extra 패널들이 존재한다, **When** 사용자가 탭을 선택한다, **Then** 선택한 패널의 prompt, timeline, 실행 상태가 표시된다.

---

### User Story 3 - 패널별 실행 상태와 prompt 흐름을 분리한다 (Priority: P3)

사용자는 각 agent 패널에서 prompt, timeline, active run, queue, permission 응답 상태가 서로 섞이지 않고 독립적으로 관리되는 것을 확인할 수 있다.

**Why this priority**: 여러 agent가 동시에 실행될 수 있는 환경에서 상태가 섞이면 사용자는 어떤 agent가 어떤 요청에 응답하는지 판단할 수 없다.

**Independent Test**: main과 extra 패널에서 각각 prompt를 실행하고, 각 패널이 자기 실행 상태와 timeline만 표시하며 다른 패널의 run 출력이나 queue를 표시하지 않는지 확인한다.

**Acceptance Scenarios**:

1. **Given** main 패널과 extra 패널이 있다, **When** 사용자가 extra 패널에서 prompt를 실행한다, **Then** extra 패널의 timeline과 실행 상태만 해당 prompt 처리를 표시한다.
2. **Given** 한 패널이 실행 중이다, **When** 사용자가 같은 패널에 추가 prompt를 보낸다, **Then** 해당 prompt는 그 패널의 queue에 들어간다.
3. **Given** 두 패널이 동시에 실행 중이다, **When** 각 실행에서 메시지나 permission 요청이 발생한다, **Then** 사용자는 어느 패널의 요청인지 구분할 수 있다.

---

### User Story 4 - workspace annotation prompt를 active 패널로 보낸다 (Priority: P4)

사용자는 workspace에서 annotation prompt를 보낼 때 현재 선택한 agent 패널이 대상이 되며, 전송 대상 패널을 짧은 상태 피드백으로 확인할 수 있다.

**Why this priority**: workspace 작업에서 생성한 prompt가 어느 agent 패널로 전달되는지 명확해야 여러 패널 운용 중 의도치 않은 실행을 줄일 수 있다.

**Independent Test**: active 탭을 main 또는 extra로 바꾼 뒤 annotation prompt를 전송하면 해당 active 패널에만 prompt가 전달되고 대상 패널 이름이 사용자에게 표시되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** extra 패널이 active 상태다, **When** 사용자가 workspace annotation prompt를 agent로 전송한다, **Then** prompt는 active extra 패널로 전달된다.
2. **Given** active 패널이 실행 중이다, **When** annotation prompt가 전달된다, **Then** prompt는 active 패널의 queue에 추가된다.
3. **Given** annotation prompt가 전달되었다, **When** 사용자가 화면 상태를 확인한다, **Then** 대상 패널 이름을 확인할 수 있다.

---

### User Story 5 - 실행 중인 extra 패널 종료를 안전하게 처리한다 (Priority: P5)

사용자는 실행 중인 extra 패널을 닫으려 할 때 해당 실행을 취소하고 닫을지, 닫기를 취소할지 명시적으로 선택할 수 있다. 패널이 닫힌 뒤에는 해당 extra agent session에 연결된 실행 프로세스와 대기 상태가 남지 않아야 한다.

**Why this priority**: extra 패널을 닫는 행동이 진행 중인 agent 작업을 중단할 수 있으므로, 사용자가 작업 손실 가능성을 인지하고 결정해야 한다.

**Independent Test**: 실행 중인 extra 패널에서 닫기 동작을 실행했을 때 확인 절차가 표시되고, 취소 선택 시 패널이 유지되며 종료 선택 시 실행 취소, agent session 정리, 패널 제거가 모두 완료되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** extra 패널이 대기 중이다, **When** 사용자가 닫기 동작을 실행한다, **Then** 해당 extra 패널이 제거된다.
2. **Given** extra 패널이 실행 중이다, **When** 사용자가 닫기 동작을 실행한다, **Then** 실행 취소 여부를 묻는 확인 절차가 표시된다.
3. **Given** 실행 중인 extra 패널 닫기 확인 절차가 표시된다, **When** 사용자가 닫기를 취소한다, **Then** 실행과 패널은 유지된다.
4. **Given** 실행 중인 extra 패널 닫기 확인 절차가 표시된다, **When** 사용자가 실행 취소 후 닫기를 확정한다, **Then** 실행은 취소되고 패널은 제거된다.
5. **Given** extra 패널이 닫혔다, **When** 사용자가 같은 worktree에서 새 실행을 시작하거나 세션을 종료한다, **Then** 닫힌 extra 패널의 agent session 프로세스, 대기 prompt, permission 요청은 더 이상 남아 있지 않다.

### Edge Cases

- extra 패널이 여러 개 생성되고 삭제되어도 패널 이름과 내부 식별은 서로 충돌하지 않아야 한다.
- active extra 패널이 삭제되면 남은 extra 패널 또는 main 패널이 예측 가능한 active 패널이 되어야 한다.
- 실행 중인 패널이 여러 개일 때 running 상태와 경고가 사용자에게 혼동 없이 표시되어야 한다.
- 동시에 edit 가능한 agent 실행이 여러 개 있으면 사용자는 같은 worktree 변경 충돌 가능성을 인지할 수 있어야 한다.
- 동시 실행 제한 때문에 새 run을 시작할 수 없으면 사용자는 제한 상황과 다음 행동을 이해할 수 있어야 한다.
- extra 패널은 goal 자동 이어가기나 settings 저장을 중복 수행하지 않아야 한다.
- extra 패널 탭이 닫힌 뒤에도 해당 agent session 프로세스가 백그라운드에서 계속 실행되거나 새 이벤트를 보내면 안 된다.
- extra 패널 닫기와 실행 종료 이벤트가 거의 동시에 발생해도 취소 요청, 완료 이벤트, 패널 제거가 중복 처리되어 오류가 발생하면 안 된다.
- 창 또는 세션 종료 시 같은 세션에 속한 실행들이 기존 소유 범위 규칙을 벗어나면 안 된다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST show exactly one non-removable main agent panel for each Worktree Session.
- **FR-002**: System MUST allow users to create extra agent panels in the same Worktree Session.
- **FR-003**: System MUST make a newly created extra agent panel the active panel immediately after creation.
- **FR-004**: System MUST allow users to switch between main and extra agent panels through an agent panel tab interface.
- **FR-005**: System MUST provide close controls only for extra agent panels, not for the main agent panel.
- **FR-006**: System MUST keep each panel's prompt input, timeline, active run identity, prompt queue, running state, and permission response state independent from other panels.
- **FR-007**: System MUST route workspace annotation prompts to the currently active agent panel by default.
- **FR-008**: System MUST show a short confirmation or status indication identifying the target panel when an annotation prompt is sent.
- **FR-009**: System MUST queue prompts on the target panel when that panel is already running.
- **FR-010**: System MUST start prompt processing on the target panel when that panel is ready and required session conditions are available.
- **FR-011**: System MUST keep goal display, goal editing, and goal automatic continuation limited to the main panel for the MVP.
- **FR-012**: System MUST prevent extra panels from automatically persisting shared worktree agent settings during the MVP.
- **FR-013**: System MUST allow extra panels to maintain panel-local agent/profile/permission choices for their lifetime.
- **FR-014**: System MUST show running state on tabs or equivalent panel selectors when a panel has an active run.
- **FR-015**: System MUST require explicit confirmation before closing an extra panel with an active run.
- **FR-016**: System MUST let users cancel closing an active extra panel without interrupting the run.
- **FR-017**: System MUST cancel the active run before removing an active extra panel when the user confirms close.
- **FR-018**: System MUST release or unregister the closed extra panel's agent session process, pending prompt queue, permission request state, and event subscriptions after the panel is removed.
- **FR-019**: System MUST ignore or safely discard late events from an extra panel run after that panel has been closed and its run has been canceled or settled.
- **FR-020**: System MUST make extra panel close cleanup idempotent so simultaneous user close, run completion, cancellation, or session shutdown does not leave duplicate cleanup errors or orphaned runs.
- **FR-021**: System MUST warn users when multiple panels are running in the same worktree and file-change conflicts may occur.
- **FR-022**: System MUST preserve existing behavior for closing the session window and canceling runs owned by that session.
- **FR-023**: System MUST communicate concurrent run limit failures in a user-understandable way when an extra panel cannot start a run.

### Key Entities

- **Agent Panel**: A user-visible agent workspace inside a Worktree Session. It has a title, active/inactive state, prompt input, timeline, queue, running state, and optional active run.
- **Main Agent Panel**: The required primary agent panel for a Worktree Session. It owns the default work flow, goal continuation, and persisted settings behavior.
- **Extra Agent Panel**: A user-created additional agent panel used for manual prompts, review, experiments, or read-only exploration in the same worktree.
- **Panel Prompt Request**: A prompt sent to a specific panel from direct input or workspace annotation actions.
- **Panel Run State**: The current execution state for a panel, including whether it is running and which active run it owns.
- **Workspace Annotation Prompt**: A prompt generated from the workspace area and routed to the active agent panel.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: Scope is limited to `apps/agentic-workbench`; cross-app sharing is intentionally avoided because the behavior is specific to Worktree Session agent operation.
- **Frontend layering**: Screen composition remains in `pages`; tabbed agent panel orchestration and prompt routing belong in `features`; agent run models and API adapters remain in `entities`; reusable low-level controls may use `shared` or existing generated UI primitives.
- **Backend boundary**: Backend changes are expected to be minimal. If run cancellation, concurrent limit messaging, or owner-scope validation needs adjustment, command boundaries must delegate to application services and keep registry/session details in infrastructure.
- **Shared core vs UI**: Shared packages are not required for the MVP. Any reusable logic should start as panel state modeling or prompt routing helpers inside the app before being promoted.
- **Persistence and safety**: The feature must preserve worktree/session owner checks, avoid dispatching events or permission responses to the wrong panel, and prevent extra panels from overwriting main panel settings.
- **Documentation and Storybook**: Reusable panel tabs or agent area components require Storybook coverage under the appropriate atomic category. Korean project documentation under `docs/*.md` should be updated if the delivered behavior differs from `docs/main-extra-agent-run-panels-design.md`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of Worktree Session loads, exactly one main agent panel is visible and cannot be closed.
- **SC-002**: Users can create a new extra agent panel and begin typing in it within 2 seconds under normal local app conditions.
- **SC-003**: In 100% of tested main/extra concurrent runs, each panel displays only the timeline entries and queued prompts for its own active run.
- **SC-004**: In 100% of tested annotation prompt sends, the prompt is delivered to the active panel and the user can identify the target panel from visible feedback.
- **SC-005**: In 100% of tested active extra panel close attempts, the user is offered a choice to cancel closing or cancel the run and close the panel.
- **SC-006**: In 100% of tested extra panel runs, goal automatic continuation is not started by an extra panel.
- **SC-007**: In 100% of tested extra panel setting changes, main panel persisted settings are not overwritten by extra panel lifecycle changes.
- **SC-008**: In 100% of tested extra panel close flows, no closed extra panel run remains active, no closed panel permission request remains actionable, and no closed panel queue item continues processing.
- **SC-009**: At least 90% of users in a review task can correctly identify which panel is running and which panel will receive the next annotation prompt.

## Assumptions

- The MVP keeps extra panel state local to the current Worktree Session and does not restore extra panel layouts after reload.
- The default annotation prompt target is the currently active agent panel.
- The main panel remains the only panel that displays and mutates goal workflow state for the MVP.
- Extra panels are manual-prompt surfaces and do not automatically continue goals or merge conversation histories.
- Extra panels may copy initial defaults from main panel settings, but their changes are panel-local for the current lifetime.
- Existing backend support for multiple distinct run identities is sufficient for the MVP, with only user-facing error handling or cancellation wiring adjusted as needed.
- The worktree changes view remains worktree-wide rather than panel-specific.
