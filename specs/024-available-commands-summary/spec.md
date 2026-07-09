# 기능 명세: 사용 가능한 명령 요약과 조회

**Feature Branch**: `main`

**Created**: 2026-07-09

**Status**: Draft

**Input**: 사용자 설명: "#150 AW: available_commands_update를 간략 표시하고 세션 정보로 조회"

## 사용자 시나리오 및 테스트 *(필수)*

### 사용자 스토리 1 - 명령 목록 갱신을 짧게 파악 (Priority: P1)

Agentic Workbench에서 agent session을 보는 사용자는 `available_commands_update`가 도착했을 때 긴 JSON 대신 사용 가능한 명령 수와 갱신 여부를 짧게 확인하고 싶다.

**우선순위 이유**: 현재 긴 command payload가 timeline noise로 보일 수 있다. 사용자는 세션 진행 중 핵심 대화와 tool activity를 읽는 데 방해받지 않아야 한다.

**독립 테스트**: `available_commands_update`를 받은 뒤 timeline 또는 상태 영역에 전체 JSON이 노출되지 않고, `N commands available` 같은 간략 요약이 보이는지 확인한다.

**인수 시나리오**:

1. **Given** agent run panel이 열려 있고, **When** 3개의 command가 포함된 `available_commands_update`를 받으면, **Then** 화면에는 전체 JSON 대신 3개 명령이 사용 가능하다는 짧은 요약이 표시된다.
2. **Given** 기존 timeline에 사용자 메시지와 agent 응답이 있는 상태에서, **When** command 목록 update가 도착하면, **Then** 긴 raw payload가 새 timeline 항목으로 노출되지 않는다.
3. **Given** command 목록이 다시 갱신되면, **When** 사용자가 상태 영역을 확인할 때, **Then** 최신 command 수가 반영된다.

---

### 사용자 스토리 2 - 사용 가능한 명령 상세 조회 (Priority: P2)

사용자는 현재 세션에서 사용할 수 있는 명령의 이름, 설명, 입력 힌트를 필요할 때 조회하고 싶다.

**우선순위 이유**: 요약만으로는 어떤 명령을 쓸 수 있는지 판단하기 어렵다. 상세 조회는 사용자가 command를 선택하거나 prompt에 입력할 때 필요한 맥락을 제공한다.

**독립 테스트**: command 목록 update를 받은 뒤 세션 정보 영역에서 각 command의 이름, 설명, 입력 힌트를 확인할 수 있는지 확인한다.

**인수 시나리오**:

1. **Given** command 목록에 `mcp`, `review`, `$speckit-implement`가 포함되어 있고, **When** 사용자가 세션 정보의 command 상세를 열면, **Then** 세 command가 모두 누락 없이 표시된다.
2. **Given** command에 설명이 있으면, **When** 상세 목록을 확인할 때, **Then** command 이름 옆 또는 아래에 설명이 표시된다.
3. **Given** command에 입력 힌트가 있으면, **When** 상세 목록을 확인할 때, **Then** 사용자는 해당 command가 인자를 받을 수 있음을 알 수 있다.

---

### 사용자 스토리 3 - 비어 있거나 깨진 목록을 안전하게 처리 (Priority: P3)

사용자는 command 목록이 비어 있거나 일부 필드가 누락되어도 화면이 깨지지 않고 현재 session 작업을 계속할 수 있어야 한다.

**우선순위 이유**: provider마다 command payload 품질이 다를 수 있다. malformed update가 대화 흐름이나 prompt 작업을 방해하면 안 된다.

**독립 테스트**: 빈 목록, 누락된 설명, 잘못된 입력 힌트가 포함된 update를 보내도 UI가 정상 렌더링되고 raw JSON noise가 생기지 않는지 확인한다.

**인수 시나리오**:

1. **Given** `availableCommands`가 빈 배열이면, **When** update를 처리할 때, **Then** command가 없다는 compact 상태가 표시되고 화면은 깨지지 않는다.
2. **Given** command의 설명이나 입력 힌트가 누락되면, **When** 상세 목록을 확인할 때, **Then** 사용 가능한 필드만 표시되고 누락 필드는 안전하게 생략된다.
3. **Given** malformed command 항목이 포함되면, **When** update를 처리할 때, **Then** 유효한 command만 표시하거나 안전한 fallback을 사용하며 전체 raw payload는 노출되지 않는다.

### Edge Cases

- `availableCommands`가 없거나 배열이 아닐 수 있다.
- command 항목의 `name`이 비어 있거나 문자열이 아닐 수 있다.
- `description`이 없거나 너무 길 수 있다.
- `input`이 `null`, 문자열, 객체, 예상하지 못한 구조일 수 있다.
- `$skill` 형태 명령, slash command, 일반 command 이름이 섞여 있을 수 있다.
- command 수가 많아 목록이 길어질 수 있다.
- 같은 command 목록 update가 반복 도착할 수 있다.
- command 목록 update가 session title/status update보다 먼저 또는 나중에 도착할 수 있다.

## 요구사항 *(필수)*

### 기능 요구사항

- **FR-001**: 시스템은 `available_commands_update`를 session-level metadata로 인식해야 한다.
- **FR-002**: 시스템은 `available_commands_update` 전체 raw payload를 사용자-facing timeline content로 노출하지 않아야 한다.
- **FR-003**: 시스템은 command 목록이 유효할 때 사용 가능한 command 수를 짧은 상태 또는 요약으로 표시해야 한다.
- **FR-004**: 시스템은 세션 정보 영역 또는 동등한 조회 위치에서 command 이름을 확인할 수 있게 해야 한다.
- **FR-005**: 시스템은 command 설명이 제공되면 상세 조회에서 표시해야 한다.
- **FR-006**: 시스템은 command 입력 힌트가 제공되면 사용자가 인자 필요 여부를 알 수 있게 표시해야 한다.
- **FR-007**: 시스템은 `$skill` 형태 명령, slash command, 일반 command 이름을 동일한 command 목록에서 누락 없이 다뤄야 한다.
- **FR-008**: 시스템은 command 목록이 많아도 주요 작업 화면의 레이아웃을 깨뜨리지 않는 compact 표시 또는 상세 조회 방식을 제공해야 한다.
- **FR-009**: 시스템은 empty, partial, malformed command metadata를 받아도 run panel과 prompt 입력 흐름이 계속 동작하도록 fallback해야 한다.
- **FR-010**: 시스템은 일반 user message, agent message, tool activity, lifecycle event, non-command raw event의 기존 timeline 동작을 회귀시키지 않아야 한다.
- **FR-011**: 기능은 command metadata parsing, raw suppression, compact summary, detail 조회, malformed fallback에 대한 검증을 포함해야 한다.

### 핵심 엔티티

- **Available Command Metadata**: 세션에서 사용할 수 있는 command 목록과 갱신 상태를 나타내는 session-level metadata다. timeline 대화 메시지가 아니다.
- **Command Summary**: 사용 가능한 command 수와 갱신 여부를 짧게 보여주는 user-facing 표시다.
- **Command Detail Item**: command 이름, 설명, 입력 힌트를 포함하는 상세 조회 항목이다.
- **Command Input Hint**: command가 추가 입력을 받을 수 있음을 사용자가 알 수 있게 하는 설명 또는 힌트다.
- **Timeline Content**: 사용자 메시지, agent 응답, tool activity, lifecycle/error/diagnostic content다. `available_commands_update` 전체 raw payload는 여기에 포함되지 않는다.

## Constitution Alignment *(필수)*

- **Monorepo boundary**: 범위는 `apps/agentic-workbench`와 `specs/024-available-commands-summary` 산출물로 제한한다. cross-app 공유는 필요하지 않다.
- **Frontend layering**: command metadata model과 formatting은 `entities/agent-run/model`, run panel state는 `features/agent-run/model`, visible summary/detail UI는 `features/agent-run/ui`에 둔다.
- **Backend boundary**: backend 변경이 필요하면 ACP session update mapper에서 command metadata 의미 보존에 한정한다. Tauri command나 persistence 경계는 확장하지 않는다.
- **Shared core vs UI**: AW 단일 화면 기능이므로 shared UI package를 만들지 않는다. reusable UI가 필요해지면 app-local component로 먼저 검증한다.
- **Persistence and safety**: 영구 저장소 변경은 범위 밖이다. command metadata는 현재 visible run/session 상태에만 반영한다.
- **Documentation and Storybook**: 새 reusable command detail component를 도입할 때만 Storybook coverage를 추가한다. 그렇지 않으면 focused test와 quickstart 수동 검증으로 충분하다.

## 성공 기준 *(필수)*

### 측정 가능한 결과

- **SC-001**: 대표 `available_commands_update` 100%에서 전체 raw JSON timeline item 없이 command count 요약이 갱신된다.
- **SC-002**: 이름, 설명, 입력 힌트가 포함된 command update 100%에서 상세 조회에 해당 정보가 표시된다.
- **SC-003**: `$skill`, slash command, 일반 command가 섞인 대표 목록에서 유효한 command 이름이 100% 누락 없이 표시된다.
- **SC-004**: 50개 이상의 command가 포함된 목록에서도 주요 run panel 레이아웃이 깨지지 않고 사용자가 상세 조회를 닫거나 스크롤할 수 있다.
- **SC-005**: empty, partial, malformed metadata 대표 케이스에서 visible runtime error 없이 run panel이 계속 렌더링된다.
- **SC-006**: focused test 또는 문서화된 수동 검증에서 `available_commands_update` 전체 raw payload가 timeline에 0건 표시됨을 확인할 수 있다.

## 가정

- command 목록은 현재 visible run/session의 live metadata로만 표시한다.
- command 상세 조회의 기본 위치는 run/session header 또는 세션 정보 영역이다.
- command 목록은 prompt autocomplete source와 공유될 수 있지만, 이 feature의 primary goal은 raw noise 제거와 조회 가능성이다.
- command input hint는 사람이 읽을 수 있는 간단한 텍스트로 표시한다. 구조화된 schema 전체를 펼쳐 보여주는 것은 범위 밖이다.
- command 설명이 너무 긴 경우 주요 목록에서는 줄이거나 접고, 상세 조회에서 읽을 수 있게 한다.
- historical raw timeline item migration은 포함하지 않는다.
