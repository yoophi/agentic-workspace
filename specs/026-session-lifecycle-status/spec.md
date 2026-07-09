# Feature Specification: Session Lifecycle Status

**Feature Branch**: `[026-session-lifecycle-status]`

**Created**: 2026-07-10

**Status**: Draft

**Input**: User description: "#101 이슈 진행"

**Issue Context**: Worktree session / agent run 화면에서 session 시작 시점과 idle 진입 같은 lifecycle 상태가 충분히 명확하게 드러나지 않는다. 사용자는 현재 agent session이 시작되었는지, 실행 중인지, idle 상태로 전환되었는지 빠르게 파악할 수 있어야 한다. 사용 가능한 command/tool 목록의 raw payload 노출 문제는 #150 / PR #153에서 세션 메타데이터로 처리되었으므로, 이 feature는 남은 session lifecycle/status 표시 UX에 집중한다.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 세션 시작 상태를 즉시 파악한다 (Priority: P1)

사용자는 새 agent session을 시작했을 때 session이 생성되고 작업 흐름이 시작되었음을 짧은 상태 메시지로 확인할 수 있다. 이 메시지는 prompt나 agent 응답처럼 강하게 보이지 않고, 현재 timeline 흐름을 방해하지 않아야 한다.

**Why this priority**: session이 실제로 시작되었는지 알 수 없으면 사용자는 prompt가 전송됐는지, agent가 준비 중인지, UI가 멈춘 것인지 구분하기 어렵다.

**Independent Test**: 새 session을 시작하고 timeline 또는 session 상태 영역에서 짧은 시작 상태 메시지가 표시되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 사용자가 agent run을 시작할 수 있는 상태, **When** 사용자가 새 session을 시작하면, **Then** session 시작을 알리는 짧은 상태 메시지가 표시된다.
2. **Given** session 시작 상태 메시지가 표시된 상태, **When** agent 응답 또는 tool event가 이어지면, **Then** 시작 메시지는 기존 prompt/agent 응답보다 낮은 강조도로 유지된다.
3. **Given** command summary가 header metadata 영역에 표시되는 상태, **When** session 시작 메시지가 표시되면, **Then** command summary와 중복되거나 같은 정보를 반복하지 않는다.

---

### User Story 2 - idle 전환을 놓치지 않는다 (Priority: P2)

사용자는 agent가 active 상태에서 idle 상태로 전환되었을 때 짧은 상태 메시지를 보고 작업이 멈췄거나 대기 상태에 들어갔음을 알 수 있다.

**Why this priority**: active/idle 전환은 사용자가 다음 prompt를 보내도 되는지 판단하는 핵심 신호다.

**Independent Test**: active 상태에서 idle 상태로 전환되는 session update를 수신했을 때, 짧은 idle 상태 메시지가 표시되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** agent가 active 상태로 표시되고 있는 run, **When** session update가 idle 상태를 알리면, **Then** idle 진입을 알리는 짧은 상태 메시지가 표시된다.
2. **Given** idle 상태 메시지가 표시된 상태, **When** 사용자가 timeline을 훑어보면, **Then** 이 메시지는 prompt/agent 응답과 구분되는 보조 상태로 읽힌다.
3. **Given** session metadata header에 현재 agent 상태 badge가 표시되는 상태, **When** idle 상태 메시지가 표시되면, **Then** header의 현재 상태와 timeline/status 메시지는 서로 충돌하지 않는다.

---

### User Story 3 - 반복 상태 메시지로 timeline이 오염되지 않는다 (Priority: P3)

사용자는 동일한 lifecycle/status update가 반복 수신되어도 timeline이 동일한 메시지로 과도하게 채워지지 않는 안정적인 상태 표시를 경험한다.

**Why this priority**: session update는 반복 수신될 수 있으므로, 중복 메시지가 누적되면 실제 작업 흐름을 읽기 어렵다.

**Independent Test**: 동일한 active 또는 idle 상태 update를 여러 번 수신해도 상태 메시지가 과도하게 반복되지 않는지 확인한다.

**Acceptance Scenarios**:

1. **Given** idle 상태 메시지가 이미 표시된 run, **When** 동일한 idle 상태 update가 반복 수신되면, **Then** 동일 메시지가 계속 추가되지 않는다.
2. **Given** active와 idle 상태가 실제로 전환되는 run, **When** 상태가 active에서 idle로 바뀌면, **Then** 의미 있는 전환은 표시되고 반복 수신은 dedupe된다.
3. **Given** malformed 또는 부분적인 session status update가 수신된 상태, **When** 상태 표시를 갱신하려고 하면, **Then** UI가 깨지지 않고 raw JSON이 사용자 메시지로 노출되지 않는다.

### Edge Cases

- session 시작 직후 active 상태 update가 여러 번 수신되어도 중복 시작 메시지가 과도하게 쌓이지 않는다.
- idle update가 run 완료 이벤트 직후 또는 거의 동시에 도착해도 사용자에게 모순된 상태를 보여주지 않는다.
- threadStatus가 없거나 알 수 없는 값인 session update는 raw JSON timeline noise로 표시하지 않는다.
- command summary 또는 available commands detail UI는 이 feature에서 다시 구현하거나 중복 표시하지 않는다.
- status 메시지가 긴 agent 응답, queued prompt, permission request, tool event와 함께 표시되어도 레이아웃이 깨지지 않는다.
- 새 run을 시작하면 이전 run의 lifecycle/status 메시지가 새 run 상태와 섞이지 않는다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 새 agent session 시작 시 사용자가 인지할 수 있는 짧은 시작 상태 메시지를 표시해야 한다.
- **FR-002**: 시스템은 agent가 idle 상태로 진입할 때 사용자가 인지할 수 있는 짧은 idle 상태 메시지를 표시해야 한다.
- **FR-003**: lifecycle/status 메시지는 기존 prompt와 agent 응답보다 낮은 시각적 강조도로 표시되어야 한다.
- **FR-004**: 시스템은 동일한 lifecycle/status update가 반복 수신될 때 동일 메시지를 과도하게 누적하지 않아야 한다.
- **FR-005**: 시스템은 실제 상태 전환이 발생했을 때 반복 update와 구분해 의미 있는 전환 상태를 표시해야 한다.
- **FR-006**: 시스템은 malformed, missing, unknown status update를 raw JSON 메시지로 노출하지 않아야 한다.
- **FR-007**: lifecycle/status 메시지는 기존 session header 상태 badge 및 command summary와 충돌하거나 중복되지 않아야 한다.
- **FR-008**: lifecycle/status 메시지는 run 단위로 분리되어야 하며, 이전 run의 상태가 새 run timeline에 섞이지 않아야 한다.
- **FR-009**: 상태 메시지 표시는 queued prompt, rejected steer, permission request, tool event 등 기존 timeline 요소의 읽기 흐름을 방해하지 않아야 한다.
- **FR-010**: 관련 상태 표시에는 rendering test 또는 Storybook 상태가 포함되어야 한다.

### Key Entities *(include if feature involves data)*

- **Session Lifecycle Status**: session이 시작됨, active 상태, idle 상태, 알 수 없음 같은 사용자에게 의미 있는 상태를 나타낸다.
- **Lifecycle Status Message**: timeline 또는 status 영역에 표시되는 짧은 보조 메시지다. run id, status type, message text, 발생 시점, dedupe key를 가진다.
- **Status Transition**: 이전 상태와 다음 상태의 변화다. 반복 update와 실제 상태 전환을 구분하는 기준으로 사용된다.
- **Run Status Scope**: 상태 메시지가 어느 agent run에 속하는지 나타내는 범위다. 이전 run의 상태 메시지가 새 run과 섞이지 않도록 한다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 범위는 `apps/agentic-workbench`의 agent run UI와 관련 모델에 한정한다. 다른 앱과의 공유 모듈 추가는 필요하지 않으며, cross-app import는 만들지 않는다.
- **Frontend layering**: session status 표시와 timeline 상호작용은 `features/agent-run`에 둔다. status parsing, formatting, transition/dedupe 같은 순수 로직은 기존 `entities/agent-run/model` 경계를 우선 사용한다.
- **Backend boundary**: 이 feature는 기존 session update 이벤트를 사용자에게 더 명확히 표시하는 UI/model 작업이다. 새로운 Tauri command나 persistence는 기본 범위가 아니다.
- **Shared core vs UI**: 재사용 가능한 로직은 상태 전환/dedupe helper처럼 headless model로 먼저 둔다. 공유 UI 패키지 추가는 하지 않는다.
- **Persistence and safety**: 새 파일 접근, persistence, agent permission 변경은 범위가 아니다. run/session status는 현재 run scope와 섞이지 않도록 해야 한다.
- **Documentation and Storybook**: feature 문서는 `specs/026-session-lifecycle-status`에 둔다. 상태 메시지 UI가 시각 상태를 갖는 경우 Storybook 또는 rendering test를 추가한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 새 session 시작 후 사용자는 1초 이내에 시작 상태를 나타내는 짧은 메시지 또는 상태 표시를 확인할 수 있다.
- **SC-002**: active에서 idle로 전환되는 update를 수신하면 사용자는 1초 이내에 idle 진입 상태를 확인할 수 있다.
- **SC-003**: 동일한 idle update가 5회 연속 수신되어도 동일한 사용자 표시 메시지는 1개를 초과해 추가되지 않는다.
- **SC-004**: malformed 또는 unknown status update를 수신해도 raw JSON이 timeline message로 표시되지 않는다.
- **SC-005**: command summary UI와 lifecycle/status 메시지가 함께 표시되어도 사용자는 command 목록과 session 상태를 서로 다른 정보로 구분할 수 있다.

## Assumptions

- #150 / PR #153에서 command summary와 available commands detail 조회는 이미 처리되었으므로 이 feature에서 다시 구현하지 않는다.
- #145 / #148 흐름에서 session_info_update raw JSON suppression과 header 상태 badge는 이미 존재한다고 가정하고, 이 feature는 timeline/status message UX를 보강한다.
- 상태 메시지는 run-local 정보이며 앱 재시작 후 복원할 필요가 없다.
- backend event contract가 이미 필요한 status 정보를 제공한다고 가정한다. 부족한 경우에도 새 persistence나 settings 저장은 만들지 않는다.
- 모바일 전용 UI는 별도 범위로 두고, 현재 desktop work surface에서 읽기 쉬운 상태 표시를 우선한다.
