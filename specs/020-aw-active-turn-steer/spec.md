# Feature Specification: AW Active-Turn Steer

**Feature Branch**: `[020-aw-active-turn-steer]`

**Created**: 2026-07-09

**Status**: Draft

**Input**: User description: "github issue #143 진행"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 실행 중인 작업을 종료하지 않고 방향 조정하기 (Priority: P1)

사용자는 Agentic Workbench에서 에이전트가 긴 작업을 수행하는 도중 추가 지시를 입력해 현재 진행 중인 작업에 반영되게 할 수 있다. 이때 기존 에이전트 세션, 진행 중인 도구 작업, 타임라인 맥락이 사용자의 명시적 취소 없이 종료되면 안 된다.

**Why this priority**: 이 기능의 핵심 가치는 steer가 "취소 후 재시작"이 아니라 "현재 작업에 추가 입력"으로 동작하게 만드는 것이다. 세션이 끊기면 사용자는 작업 맥락과 신뢰를 잃는다.

**Independent Test**: 실행 중인 AW 에이전트 작업에 steer 입력을 보내고, 기존 작업 타임라인이 유지된 상태에서 추가 지시가 같은 작업 흐름에 반영되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 에이전트가 응답 또는 도구 작업을 진행 중인 상태, **When** 사용자가 steer 입력을 제출하면, **Then** 현재 에이전트 세션은 종료되지 않고 steer 입력이 현재 작업의 pending 입력으로 표시된다.
2. **Given** steer 입력이 현재 작업에 수락된 상태, **When** 에이전트가 다음 입력 처리 지점에 도달하면, **Then** 사용자는 같은 타임라인에서 steer 내용이 반영된 후속 진행을 볼 수 있다.
3. **Given** 사용자가 steer를 보낸 뒤 작업을 명시적으로 취소하지 않은 상태, **When** 작업 상태가 갱신되면, **Then** UI는 기존 run을 cancelled 또는 restarted로 오해하게 만드는 상태 전환을 보여주지 않는다.

---

### User Story 2 - queue와 steer를 구분해 입력 흐름 파악하기 (Priority: P2)

사용자는 실행 중인 작업에 즉시 반영하려는 steer 입력과, 현재 작업 완료 후 다음 작업으로 실행될 queued prompt를 구분해서 볼 수 있다.

**Why this priority**: 현재 입력이 언제 실행되는지 명확하지 않으면 사용자는 중요한 지시를 잘못된 시점에 보낼 수 있다. queue와 steer의 구분은 실행 예측 가능성을 높인다.

**Independent Test**: 작업 실행 중 steer 입력과 queued prompt를 각각 추가하고, UI가 두 입력의 목적과 처리 순서를 서로 다른 상태로 표시하는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 에이전트 작업이 진행 중인 상태, **When** 사용자가 steer 입력을 추가하면, **Then** 입력은 현재 작업에 반영될 pending steer로 표시된다.
2. **Given** 에이전트 작업이 진행 중인 상태, **When** 사용자가 후속 작업용 prompt를 queue에 추가하면, **Then** 입력은 현재 작업 이후 실행될 queued prompt로 표시된다.
3. **Given** pending steer와 queued prompt가 모두 존재하는 상태, **When** 사용자가 입력 목록을 확인하면, **Then** 각 입력의 실행 대상과 순서를 혼동 없이 구분할 수 있다.

---

### User Story 3 - steer 불가 상태에서 안전하게 대체 흐름 제공하기 (Priority: P3)

사용자는 현재 에이전트 또는 작업 상태가 active-turn steer를 지원하지 않는 경우, 입력이 사라지거나 기존 작업이 예기치 않게 종료되지 않도록 명확한 대체 선택지를 받는다.

**Why this priority**: 모든 에이전트나 작업 상태가 steer를 지원한다는 보장은 없다. 미지원 상태에서도 사용자의 입력과 진행 중인 작업을 보호해야 한다.

**Independent Test**: steer가 지원되지 않는 에이전트 또는 상태에서 steer 입력을 제출하고, 시스템이 입력을 보존하며 사용자가 queue, 재시작, 취소 중 적절한 대안을 선택할 수 있는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 현재 작업이 steer를 받을 수 없는 상태, **When** 사용자가 steer 입력을 제출하면, **Then** 시스템은 입력을 잃지 않고 현재 작업을 자동 종료하지 않는다.
2. **Given** steer 입력이 거절된 상태, **When** 사용자가 대체 흐름을 선택하면, **Then** 시스템은 선택에 따라 입력을 다음 작업 queue로 이동하거나 명시적 재시작 흐름으로 전환한다.
3. **Given** 사용자가 명시적으로 재시작을 선택한 상태, **When** 시스템이 기존 작업을 종료하고 새 작업을 시작하면, **Then** UI는 이 동작이 active-turn steer가 아니라 restart-with-steering임을 명확히 보여준다.

### Edge Cases

- steer 제출 직후 기존 작업의 완료 또는 취소 이벤트가 늦게 도착해도 새 입력 상태나 현재 작업 상태가 잘못 초기화되지 않아야 한다.
- 동일한 작업에 여러 steer 입력이 빠르게 제출되어도 입력 순서가 보존되어야 한다.
- 작업이 완료되는 순간 steer 입력을 제출한 경우, 입력은 사용자가 이해할 수 있는 방식으로 다음 작업 queue 또는 재시작 후보로 보존되어야 한다.
- 에이전트 세션 소유자 또는 현재 panel이 바뀐 상태에서는 이전 panel의 steer 또는 queue가 다른 작업에 잘못 적용되지 않아야 한다.
- steer 처리 중 오류가 발생해도 원본 입력은 복구 가능해야 하며 사용자에게 재시도 또는 queue 이동 선택지가 제공되어야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 실행 중인 AW 에이전트 작업에 대해 사용자가 현재 작업을 종료하지 않는 steer 입력을 제출할 수 있게 해야 한다.
- **FR-002**: 시스템은 steer 입력과 queued prompt를 별도 상태로 구분해 저장하고 표시해야 한다.
- **FR-003**: 시스템은 현재 작업이 steer를 수락할 수 있는지 판단하고, 수락할 수 없는 경우 입력을 삭제하지 않고 사용자에게 안전한 대체 흐름을 제공해야 한다.
- **FR-004**: 시스템은 steer 입력이 수락된 후 해당 입력을 같은 작업 타임라인에서 pending 상태로 표시해야 한다.
- **FR-005**: 시스템은 사용자가 명시적으로 재시작을 선택한 경우에만 기존 작업을 종료하고 새 작업을 시작해야 한다.
- **FR-006**: 시스템은 기존 작업에서 늦게 도착한 상태 이벤트가 현재 작업의 queue, pending steer, active run 상태를 잘못 지우지 않도록 해야 한다.
- **FR-007**: 시스템은 같은 작업에 여러 steer 입력이 제출될 때 제출 순서를 보존해야 한다.
- **FR-008**: 시스템은 queue 자동 실행이 steer 처리와 충돌하지 않도록, 현재 작업 대상 입력과 다음 작업 대상 입력의 실행 조건을 분리해야 한다.
- **FR-009**: 시스템은 steer 지원 여부와 실제 동작(현재 작업 주입, 다음 작업 queue, 명시적 재시작)을 사용자가 구분할 수 있는 상태 표현을 제공해야 한다.
- **FR-010**: 시스템은 steer, queue, 명시적 취소, 명시적 재시작의 주요 상태 전환을 검증 가능한 사용자 시나리오로 테스트할 수 있어야 한다.

### Key Entities *(include if feature involves data)*

- **Agent Work Item**: AW 화면에서 사용자가 관찰하는 하나의 에이전트 실행 흐름이다. 현재 상태, 소유 panel, 타임라인, 입력 처리 상태를 가진다.
- **Steer Input**: 현재 실행 중인 작업에 추가로 반영하려는 사용자 입력이다. 제출 순서, 표시 상태, 수락/거절 결과, 대상 작업을 가진다.
- **Queued Prompt**: 현재 작업 완료 후 다음 작업으로 실행될 사용자 입력이다. 제출 순서, 자동 실행 가능 여부, 표시 상태를 가진다.
- **Restart-with-Steering Request**: 사용자가 명시적으로 기존 작업을 종료하고 steer 내용을 포함한 새 작업을 시작하기로 선택한 요청이다.
- **Provider Capability**: 현재 에이전트 또는 세션이 active-turn steer를 지원하는지에 대한 사용자-visible 판단 결과이다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 범위는 `apps/agentic-workbench`에 한정한다. cross-app 공유는 이번 기능의 기본 범위가 아니며, 순수 입력 상태 모델이 다른 앱에서도 필요하다는 근거가 생길 때만 `packages/*`로 분리한다.
- **Frontend layering**: 화면 수준 조합은 `pages`, 입력 전송/queue/steer 상호작용은 `features`, run 및 prompt 상태 모델은 `entities`, 재사용 가능한 작은 UI 요소는 `shared` 또는 기존 `components/ui` 경계를 따른다.
- **Backend boundary**: Tauri 명령은 inbound adapter로 남기고, steer/queue/cancel/restart 의미 단위의 규칙은 application service와 domain port로 분리한다. agent provider와 세션 전송 세부사항은 infrastructure adapter가 책임진다.
- **Shared core vs UI**: 먼저 prompt dispatch 상태 전이와 session/run 식별 규칙을 순수 모델로 검증한다. 공유 UI는 여러 화면에서 동일한 pending/queued 입력 표현이 필요할 때만 도입한다.
- **Persistence and safety**: steer와 queue는 run/session owner 범위를 검증해야 하며, 이전 작업의 늦은 이벤트가 현재 작업 상태를 오염시키지 않도록 대상 run/session 식별자를 확인해야 한다.
- **Documentation and Storybook**: AW steer/queue 사용자 흐름과 상태 전이는 `docs/*.md`에 Korean documentation으로 기록한다. 재사용 UI를 만들 경우 Storybook에 atomic category에 맞는 기본, pending, rejected, queued, long-content 상태를 등록한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 지원되는 에이전트에서 실행 중 steer를 20회 연속 제출해도 사용자 명시 취소 없이 기존 작업이 종료되는 사례가 0건이어야 한다.
- **SC-002**: 사용자는 실행 중 입력 목록에서 steer 입력과 queued prompt의 차이를 5초 이내에 식별할 수 있어야 한다.
- **SC-003**: steer가 지원되지 않는 상태에서 제출된 입력은 100% 보존되어 queue 이동, 재시작, 취소 중 하나의 사용자 선택으로 처리 가능해야 한다.
- **SC-004**: steer와 queue가 혼재된 대표 시나리오에서 입력 처리 순서가 제출 순서 및 사용자 선택과 일치하는 비율이 100%여야 한다.
- **SC-005**: 늦게 도착한 이전 작업 상태 이벤트로 인해 현재 작업의 queue 또는 pending steer가 사라지는 회귀 사례가 자동 검증에서 0건이어야 한다.

## Assumptions

- GitHub issue #143의 조사 결론을 근거로, 현재 AW의 steer는 cancel-and-restart 성격이며 이번 기능은 그 동작을 active-turn steer 중심으로 개선하는 것을 목표로 한다.
- active-turn steer를 지원하지 않는 agent/provider가 존재할 수 있으므로, 명시적 restart-with-steering은 fallback 동작으로 유지한다.
- v1 범위는 Agentic Workbench의 단일 worktree session panel에서 사용자가 직접 입력하는 prompt/steer/queue 흐름이다.
- queue와 steer 상태는 적어도 현재 작업 UI가 유지되는 동안 일관되게 보존되어야 하며, 영구 저장은 별도 요구가 확인될 때 다룬다.
- 권한, 파일 접근, 외부 도구 실행 정책은 기존 AW run/session owner 검증과 permission 흐름을 따른다.
