# 기능 명세: 세션 정보 메타데이터 표시

**Feature Branch**: `main`

**Created**: 2026-07-09

**Status**: Draft

**Input**: 사용자 설명: "#148 Show session_info_update title and updatedAt in AW UI"

## 사용자 시나리오 및 테스트 *(필수)*

### 사용자 스토리 1 - 세션 제목을 안정적인 위치에서 확인 (Priority: P1)

Agentic Workbench에서 agent session을 보는 사용자는 `session_info_update`가 전달하는 session `title`을 raw JSON 없이 window title에서 확인하고 싶다.

**우선순위 이유**: #145에서는 raw payload 노출을 막고 thread status만 표시했다. 이제 title metadata를 사용자에게 유용한 session identity 정보로 보여줘야 한다.

**독립 테스트**: `title`이 포함된 `session_info_update`를 받은 뒤 OS/window title에 title이 반영되고, timeline에는 raw JSON이 추가되지 않는지 확인한다.

**인수 시나리오**:

1. **Given** agent run panel이 열려 있고, **When** `title`이 포함된 `session_info_update`를 받으면, **Then** window title에 최신 session title이 반영된다.
2. **Given** 기존 title이 window title에 반영된 상태에서, **When** 더 새로운 title update를 받으면, **Then** window title은 최신 값으로 갱신된다.
3. **Given** title update가 도착할 때, **When** timeline을 확인하면, **Then** `session_info_update` raw JSON은 메시지로 표시되지 않는다.

---

### 사용자 스토리 2 - 세션 최신성 정보 확인 (Priority: P2)

여러 작업을 오가며 agent session을 확인하는 사용자는 `updatedAt` metadata를 통해 session 정보가 언제 갱신되었는지 알고 싶다.

**우선순위 이유**: window title의 title만으로는 session freshness를 알기 어렵다. `updatedAt`은 session list나 header에서 사용자가 현재 session의 최신성을 판단하는 데 도움이 된다.

**독립 테스트**: `updatedAt`이 포함된 `session_info_update`를 받은 뒤 run/session header의 보조 metadata 영역에서 읽기 쉬운 최신성 표시가 보이고, malformed 값은 UI를 깨뜨리지 않는지 확인한다.

**인수 시나리오**:

1. **Given** agent run panel이 열려 있고, **When** 유효한 `updatedAt`이 포함된 `session_info_update`를 받으면, **Then** 사용자에게 읽기 쉬운 session update time 또는 freshness 표시가 보인다.
2. **Given** 기존 update time이 표시된 상태에서, **When** 더 새로운 `updatedAt`을 받으면, **Then** 표시 값은 최신 update time을 반영한다.
3. **Given** `updatedAt` 값이 누락되거나 malformed이면, **When** update를 처리할 때, **Then** 기존 UI는 깨지지 않고 fallback 또는 기존 표시를 유지한다.

---

### 사용자 스토리 3 - 기존 상태 표시와 timeline suppression 유지 (Priority: P3)

사용자는 title/update time metadata가 표시되더라도 기존 active/idle agent status indicator와 raw timeline suppression이 유지되기를 원한다.

**우선순위 이유**: #145의 핵심 보장인 raw JSON 노출 방지와 active/idle 상태 표시는 회귀하면 안 된다.

**독립 테스트**: title, updatedAt, threadStatus가 다양한 조합으로 포함된 update를 보내고, status indicator와 metadata 표시가 함께 유지되며 timeline raw JSON이 생성되지 않는지 확인한다.

**인수 시나리오**:

1. **Given** active 또는 idle status indicator가 보이는 상태에서, **When** metadata-only update를 받으면, **Then** 기존 status indicator는 사라지거나 unknown으로 덮이지 않는다.
2. **Given** title/update time과 threadStatus가 같은 update에 포함되면, **When** update를 처리할 때, **Then** metadata 표시와 status indicator가 모두 최신 값을 반영한다.
3. **Given** partial 또는 malformed metadata가 포함되면, **When** update를 처리할 때, **Then** timeline raw JSON이 표시되지 않고 기존 status 표시도 회귀하지 않는다.

### Edge Cases

- `title`만 있고 `updatedAt` 또는 `threadStatus`가 없을 수 있다.
- `updatedAt`만 있고 `title` 또는 `threadStatus`가 없을 수 있다.
- `title`이 빈 문자열이거나 공백만 포함할 수 있다.
- `updatedAt`이 유효하지 않은 날짜 문자열일 수 있다.
- 같은 title 또는 같은 updatedAt update가 반복 도착할 수 있다.
- title/update time update가 active/idle status update보다 먼저 도착할 수 있다.
- 기존 provider session list에 이미 title/updatedAt 정보가 있을 수 있으며 live session metadata와 충돌하지 않아야 한다.

## 요구사항 *(필수)*

### 기능 요구사항

- **FR-001**: 시스템은 `session_info_update`의 `title`이 존재하고 의미 있는 값일 때 현재 AW window title에 반영해야 한다.
- **FR-002**: 시스템은 `session_info_update`의 `updatedAt`이 유효한 값일 때 run/session header의 user-facing metadata 영역에 읽기 쉬운 최신성 정보로 표시해야 한다.
- **FR-003**: 시스템은 `title` 또는 `updatedAt`만 포함된 metadata-only update가 현재 active/idle agent status indicator를 지우거나 unknown으로 덮어쓰지 않도록 해야 한다.
- **FR-004**: 시스템은 `title`, `updatedAt`, `threadStatus`가 함께 포함된 update에서 title은 window title에, updatedAt과 threadStatus는 각자의 user-facing 위치에 독립적으로 반영해야 한다.
- **FR-005**: 시스템은 missing, partial, malformed metadata를 받아도 run panel 또는 session list가 깨지지 않도록 fallback해야 한다.
- **FR-006**: 시스템은 모든 `session_info_update` raw payload가 agent timeline에 표시되지 않는 기존 동작을 유지해야 한다.
- **FR-007**: 시스템은 일반 user message, agent message, tool activity, lifecycle event, non-session raw event의 기존 timeline 동작을 회귀시키지 않아야 한다.
- **FR-008**: 기능은 title handling, updatedAt handling, metadata-only status preservation, timeline suppression에 대한 검증을 포함해야 한다.

### 핵심 엔티티

- **Session Info Metadata**: `session_info_update`에서 전달되는 title, updatedAt, threadStatus 같은 session-level metadata다. timeline message가 아니다.
- **Session Display Title**: 사용자가 현재 agent session을 식별할 수 있도록 AW window title에 반영되는 최신 의미 있는 title이다.
- **Session Freshness**: updatedAt metadata를 기반으로 session 정보가 언제 갱신되었는지 보여주는 user-facing 표시다.
- **Agent Thread Status**: #145에서 도입된 active/idle/unknown 상태 표시다. title/update time update와 독립적으로 유지되어야 한다.
- **Timeline Content**: 사용자에게 메시지로 보여야 하는 user prompt, agent response, tool activity, lifecycle/error/raw diagnostic content다. `session_info_update`는 여기에 포함되지 않는다.

## Constitution Alignment *(필수)*

- **Monorepo boundary**: 범위는 `apps/agentic-workbench`와 `specs/023-session-info-metadata` 산출물로 제한한다. cross-app 공유는 필요하지 않다.
- **Frontend layering**: Session metadata model과 formatting은 `entities/agent-run/model`, run panel state는 `features/agent-run/model`, visible UI는 `features/agent-run/ui`에 둔다.
- **Backend boundary**: #145에서 추가된 typed session info event 흐름을 유지한다. backend 변경이 필요하면 ACP session update mapper 안에서 event payload 의미 보존에 한정하고, Tauri command나 persistence 경계를 확장하지 않는다.
- **Shared core vs UI**: AW 단일 화면 기능이므로 shared UI package를 만들지 않는다. reusable UI를 새로 만들 필요가 생기면 먼저 app-local component로 검증한다.
- **Persistence and safety**: 영구 저장소 변경은 범위 밖이다. live session metadata는 현재 visible run/session 상태에만 반영한다.
- **Documentation and Storybook**: 새 reusable status/metadata component를 도입할 때만 Storybook coverage를 추가한다. 그렇지 않으면 focused test와 quickstart 수동 검증으로 충분하다.

## 성공 기준 *(필수)*

### 측정 가능한 결과

- **SC-001**: title이 포함된 대표 update 100%에서 raw JSON timeline item 없이 AW window title이 갱신된다.
- **SC-002**: 유효한 updatedAt이 포함된 대표 update 100%에서 raw JSON timeline item 없이 user-facing freshness 표시가 갱신된다.
- **SC-003**: metadata-only update 이후 기존 active/idle status indicator가 100% 유지된다.
- **SC-004**: missing, partial, malformed metadata 대표 케이스에서 visible runtime error 없이 run panel이 계속 렌더링된다.
- **SC-005**: focused test 또는 문서화된 수동 검증에서 `session_info_update` raw payload가 timeline에 0건 표시됨을 확인할 수 있다.

## 가정

- title의 primary target은 AW window title이다.
- updatedAt의 기본 표시 위치는 현재 run/session header 또는 session summary의 보조 metadata 영역이다.
- #148에서는 window title에 session title을 반영하되, active/idle 같은 작업 진행 상태 prefix/suffix 정책은 #113 범위로 남긴다.
- 기존 provider session list에 이미 표시되는 저장된 session title/freshness는 유지하되, 이 feature의 live update 목표는 현재 visible run/session에 한정한다.
- updatedAt은 가능한 경우 사용자에게 읽기 쉬운 절대 또는 상대 시간으로 표시한다. 유효하지 않은 값은 표시하지 않거나 기존 표시를 유지한다.
- 빈 title 또는 공백 title은 의미 있는 title로 간주하지 않는다.
- 이 feature는 historical raw timeline item migration을 포함하지 않는다.
