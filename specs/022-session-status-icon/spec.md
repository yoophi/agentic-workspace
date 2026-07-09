# 기능 명세: 세션 상태 아이콘

**Feature Branch**: `022`

**Created**: 2026-07-09

**Status**: Draft

**Input**: 사용자 설명: "#145 session_info_update를 raw JSON 대신 agent 상태 아이콘으로 표시"

## 사용자 시나리오 및 테스트 *(필수)*

### 사용자 스토리 1 - 세션 메타데이터를 타임라인에서 숨김 (Priority: P1)

Agentic Workbench 세션을 보는 사용자는 내부 세션 메타데이터가 메시지 타임라인에 섞이지 않기를 원한다. 그래야 raw diagnostic payload 없이 agent 대화를 읽을 수 있다.

**우선순위 이유**: 타임라인에 raw 세션 메타데이터가 보이는 것이 핵심 문제다. 이를 제거하면 대화 가독성이 회복되고, 사용자가 내부 상태 업데이트를 agent 메시지로 오해하지 않는다.

**독립 테스트**: 세션 정보 업데이트를 받는 세션을 열고, 타임라인에 사용자에게 보여야 하는 메시지, tool 활동, 의도된 agent 내용만 남는지 확인한다.

**인수 시나리오**:

1. **Given** agent 세션이 열려 있고, **When** idle 상태를 포함한 세션 정보 업데이트를 받으면, **Then** raw metadata payload가 메시지로 표시되지 않는다.
2. **Given** agent 세션이 열려 있고, **When** title 또는 updated time metadata를 포함한 세션 정보 업데이트를 받으면, **Then** raw metadata payload가 메시지로 표시되지 않는다.
3. **Given** metadata update 앞뒤에 정상 agent 메시지가 있는 세션에서, **When** 타임라인을 표시하면, **Then** 정상 메시지는 예상 순서대로 계속 보인다.

---

### 사용자 스토리 2 - 현재 Agent 활동 상태 확인 (Priority: P2)

Agent run을 모니터링하는 사용자는 agent가 현재 작업 중인지 대기 중인지 빠르게 알고 싶다.

**우선순위 이유**: raw metadata를 숨긴 뒤에도 그 안의 유용한 정보는 사용자에게 보이는 형태로 제공되어야 한다.

**독립 테스트**: active와 idle 세션 상태 업데이트를 보내고, 세션 상태 indicator가 그에 맞게 바뀌는지 확인한다.

**인수 시나리오**:

1. **Given** agent 세션이 보이고, **When** 세션 상태가 active가 되면, **Then** 작업 중임을 나타내는 명확한 시각 상태가 보인다.
2. **Given** agent 세션이 보이고, **When** 세션 상태가 idle이 되면, **Then** 대기/준비 상태를 나타내는 명확한 시각 상태가 보인다.
3. **Given** 상태 업데이트가 발생한 뒤 사용자가 세션으로 돌아오면, **When** 세션 UI가 렌더링될 때, **Then** 마지막으로 알려진 상태가 일관되게 표시된다.

---

### 사용자 스토리 3 - 세션 메타데이터의 유용한 효과 보존 (Priority: P3)

여러 agent 세션을 탐색하는 사용자는 title, update time 같은 유용한 세션 메타데이터가 raw content로 노출되지 않으면서 세션 표시 품질을 높이기를 원한다.

**우선순위 이유**: 메타데이터는 세션 식별과 최신성 표현에 유용할 수 있지만, 대화 타임라인을 오염시키면 안 된다.

**독립 테스트**: metadata-only 세션 업데이트를 보내고, 지원되는 세션 요약 필드는 필요한 경우 갱신되며 메시지 타임라인은 깨끗하게 유지되는지 확인한다.

**인수 시나리오**:

1. **Given** 세션 정보 업데이트에 title이 포함되고, **When** 세션 목록 또는 헤더가 title 표시를 지원하면, **Then** 보이는 세션 title은 최신 의미 있는 title을 반영한다.
2. **Given** 세션 정보 업데이트에 updated time이 포함되고, **When** 세션 최신성 정보가 표시되면, **Then** 최신성 표시는 최신 의미 있는 update를 반영할 수 있다.
3. **Given** 아직 UI에 표현되지 않는 metadata가 세션 정보 업데이트에 포함되면, **When** 업데이트를 수신할 때, **Then** raw timeline 메시지를 만들지 않고 무시하거나 저장한다.

### Edge Cases

- 세션 정보 업데이트가 사용자에게 보여야 하는 agent 메시지보다 먼저 도착할 수 있다.
- 같은 상태 값의 status update가 반복 도착해도 중복 timeline entry를 만들면 안 된다.
- status update가 agent status 없이 다른 metadata만 포함할 수 있다.
- 알 수 없는 status 값은 세션 view를 깨뜨리지 않고 눈에 거슬리지 않는 unknown 또는 neutral 상태로 fallback해야 한다.
- 과거 세션에는 이미 raw metadata 메시지가 저장되어 있을 수 있다. 이 기능은 새로 처리되는 세션 정보 업데이트에 대한 동작만 보장한다.

## 요구사항 *(필수)*

### 기능 요구사항

- **FR-001**: 시스템은 세션 정보 업데이트가 agent timeline에 raw 메시지로 표시되지 않도록 해야 한다.
- **FR-002**: 시스템은 agent의 현재 활동 상태를 전달하는 세션 정보 업데이트를 인식해야 한다.
- **FR-003**: 사용자는 compact한 시각 status indicator를 통해 최소 active와 idle 상태를 구분할 수 있어야 한다.
- **FR-004**: 세션 정보 업데이트 처리 후에도 정상 사용자 메시지, agent 메시지, tool 활동, 기타 의도된 timeline content는 계속 표시되어야 한다.
- **FR-005**: 시스템은 title 또는 updated time metadata를 포함한 세션 정보 업데이트를 raw payload 메시지로 표시하지 않고 처리해야 한다.
- **FR-006**: 시스템은 더 새로운 status update가 오거나 세션 view가 reset되기 전까지 visible session의 마지막 agent status를 유지해야 한다.
- **FR-007**: 시스템은 반복, 부분, 알 수 없는 세션 정보 업데이트를 timeline 중단이나 유효한 user-facing 메시지 숨김 없이 허용해야 한다.
- **FR-008**: 기능은 metadata filtering, active/idle 상태 표시, 일반 timeline 메시지 보존을 검증하는 테스트를 포함해야 한다.

### 핵심 엔티티

- **Session Information Update**: 현재 agent 상태, title, updated time 같은 세션 metadata에 대한 시스템-originated update다. 사용자에게 보이는 메시지 content가 아니다.
- **Agent Status**: agent 세션의 마지막으로 알려진 활동 상태다. 초기 지원 상태는 active와 idle이며, 알 수 없거나 누락된 상태를 안전하게 처리한다.
- **Timeline Message**: 사용자 prompt, agent response, 의도된 작업 활동을 시간순으로 보여주는 user-facing 세션 content다.
- **Session Summary**: title, freshness, current status처럼 timeline 밖에서 표시될 수 있는 user-facing 세션 metadata다.

## Constitution Alignment *(필수)*

- **Monorepo boundary**: 범위는 `apps/agentic-workbench`의 Agentic Workbench로 제한한다. 이 기능에는 cross-app 공유가 필요하지 않다.
- **Frontend layering**: UI와 상태 변경은 agent session 및 timeline rendering과 관련된 Agentic Workbench feature/entity layer에 둔다. shared UI primitive는 사용할 수 있지만 신규 app-specific 동작은 재사용 근거가 생기기 전까지 local에 둔다.
- **Backend boundary**: 새 Tauri backend 동작은 필요하지 않다. 기존 session event handling이 backend boundary를 이미 지나가는 경우에도 domain, application, inbound, infrastructure 책임 분리를 유지한다.
- **Shared core vs UI**: 신규 shared UI package는 필요하지 않다. event classification logic을 재사용해야 한다면 UI보다 먼저 headless/testable 형태로 유지한다.
- **Persistence and safety**: 기능은 agent session metadata를 다루며 기존 session ownership boundary를 보존해야 한다. raw session payload를 user-visible content로 노출하지 않는다.
- **Documentation and Storybook**: 재사용 가능한 status indicator component를 새로 도입할 때만 Storybook coverage가 필요하다. 그렇지 않으면 targeted test와 수동 session UI 확인으로 검증한다.

## 성공 기준 *(필수)*

### 측정 가능한 결과

- **SC-001**: 세션 정보 업데이트가 포함된 테스트 세션에서 visible agent timeline에 raw session information payload가 0건 표시된다.
- **SC-002**: session view가 update를 받은 뒤 1초 이내에 active/idle 상태 변경이 사용자에게 보인다.
- **SC-003**: active, idle, metadata-only update 대표 시나리오에서 정상 user-facing timeline 메시지의 100%가 보존되고 올바른 순서로 표시된다.
- **SC-004**: metadata update가 timeline 메시지를 만들지 않고 active/idle 상태가 서로 다르게 렌더링됨을 증명하는 자동화 또는 문서화된 검증 경로가 최소 1개 존재한다.
- **SC-005**: unknown 또는 partial session information update가 대표 검증 시나리오에서 visible error를 만들거나 session timeline을 비우지 않는다.

## 가정

- 첫 visible 상태는 issue #145에 기록된 active와 idle을 지원한다.
- session header, run header, agent identity 주변의 compact status indicator면 raw diagnostics를 열지 않아도 상태를 볼 수 있으므로 충분하다.
- 사용자 prompt, agent response, tool activity에 대한 기존 timeline 동작은 범위 안에 있으며 회귀하면 안 된다.
- 이 변경 이전에 이미 저장된 historical raw metadata message는 현재 event processing에서 다시 생성되는 경우가 아니라면 migration하지 않는다.
- title과 updated time metadata는 현재 product에 적절한 user-facing 위치가 있을 때만 적용하고, 없으면 이 기능에서는 안전하게 무시한다.
