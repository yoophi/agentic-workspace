# Feature Specification: ACP Tool List 기반 Prompt Command 자동완성

**Feature Branch**: `[014-acp-tool-autocomplete]`

**Created**: 2026-07-06

**Status**: Draft

**Input**: User description: "#106 ACP tool list 기반 prompt command 자동완성 추가 구현"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 프롬프트 작성 중 tool/command 후보 찾기 (Priority: P1)

Agentic Workbench 사용자는 agent에게 프롬프트를 작성하는 도중 `$` 또는 `/`로 시작하는 토큰을 입력해 현재 세션에서 사용할 수 있는 agent tool 또는 command 후보를 빠르게 찾고 싶다.

**Why this priority**: 이 기능의 핵심 가치는 사용자가 사용 가능한 tool/command 이름을 기억하지 않아도 프롬프트 안에서 바로 발견하고 입력할 수 있게 하는 것이다.

**Independent Test**: 사용 가능한 tool/command 목록이 있는 세션에서 프롬프트 입력란에 `$` 또는 `/`를 입력하고 후보 목록이 표시되는지 확인하면 독립적으로 검증할 수 있다.

**Acceptance Scenarios**:

1. **Given** agent session에 사용할 수 있는 tool/command 후보가 존재하고 prompt 입력란이 비어 있음, **When** 사용자가 `$`를 입력함, **Then** 관련 후보 목록이 prompt 입력란 근처에 표시된다.
2. **Given** agent session에 사용할 수 있는 tool/command 후보가 존재하고 prompt 입력란이 비어 있음, **When** 사용자가 `/`를 입력함, **Then** 관련 후보 목록이 prompt 입력란 근처에 표시된다.
3. **Given** 후보 목록이 표시된 상태, **When** 사용자가 prefix 뒤에 일부 이름을 계속 입력함, **Then** 후보 목록은 입력한 토큰과 관련 있는 항목으로 좁혀진다.

---

### User Story 2 - 후보 내용을 이해하고 선택하기 (Priority: P2)

사용자는 자동완성 목록에서 각 후보가 어떤 tool/command인지 이해하고 키보드로 탐색한 뒤 원하는 후보를 선택해 프롬프트에 삽입하고 싶다.

**Why this priority**: 후보 표시만으로는 실제 작성 속도가 충분히 개선되지 않는다. 설명과 키보드 선택이 있어야 반복 작업에서 마우스 이동 없이 빠르게 사용할 수 있다.

**Independent Test**: 후보 목록이 열린 상태에서 방향키와 선택 키를 사용해 특정 후보가 prompt 입력란에 삽입되는지 확인하면 독립적으로 검증할 수 있다.

**Acceptance Scenarios**:

1. **Given** 자동완성 후보 목록이 표시됨, **When** 사용자가 후보를 살펴봄, **Then** 각 후보에는 이름과 짧은 설명이 함께 표시된다.
2. **Given** 자동완성 후보 목록이 표시됨, **When** 사용자가 키보드로 다음/이전 후보를 이동함, **Then** 현재 선택된 후보가 명확히 구분된다.
3. **Given** 후보 하나가 선택된 상태, **When** 사용자가 선택을 확정함, **Then** 해당 command/token이 현재 작성 중인 prompt의 해당 위치에 삽입된다.
4. **Given** 후보 목록이 표시된 상태, **When** 사용자가 취소 동작을 수행함, **Then** prompt 내용은 변경되지 않고 후보 목록만 닫힌다.

---

### User Story 3 - 후보가 없거나 아직 준비되지 않은 상태에서 계속 작성하기 (Priority: P3)

사용자는 tool/command 목록이 아직 로드되지 않았거나 현재 입력과 일치하는 후보가 없더라도 프롬프트 작성이 방해받지 않기를 원한다.

**Why this priority**: 자동완성은 보조 기능이므로 후보 데이터 상태와 무관하게 기본 prompt 작성과 제출 흐름은 안정적으로 유지되어야 한다.

**Independent Test**: 후보 목록이 비어 있거나 준비되지 않은 세션에서 `$` 또는 `/`를 입력하고 일반 프롬프트 작성과 제출이 계속 가능한지 확인하면 독립적으로 검증할 수 있다.

**Acceptance Scenarios**:

1. **Given** tool/command 후보 목록이 아직 준비되지 않음, **When** 사용자가 `$` 또는 `/`로 토큰을 입력함, **Then** 입력은 그대로 유지되고 prompt 작성은 계속 가능하다.
2. **Given** 후보 목록은 준비되었지만 입력한 토큰과 일치하는 후보가 없음, **When** 사용자가 계속 문장을 작성함, **Then** 자동완성은 prompt 입력, 줄바꿈, 제출 동작을 방해하지 않는다.
3. **Given** 자동완성 후보가 없는 상태, **When** 사용자가 일반 prompt 제출 동작을 수행함, **Then** 기존 prompt 제출 흐름이 정상적으로 동작한다.

### Edge Cases

- `$` 또는 `/`가 문장 중간, 줄 처음, 코드 블록처럼 보이는 긴 텍스트 내부에 입력될 때도 현재 커서 주변 토큰만 기준으로 후보 표시 여부를 판단한다.
- 자동완성 목록이 열린 상태에서 Enter, Tab, Escape, 방향키, 마우스 클릭 같은 입력이 prompt 작성 및 선택 동작과 충돌하지 않아야 한다.
- 후보 이름이나 설명이 길어도 prompt 입력 영역, saved prompt 영역, queued prompt 영역과 시각적으로 겹치지 않아야 한다.
- 후보 수가 많은 세션에서도 사용자는 긴 목록을 끝까지 훑지 않고 관련 후보를 찾을 수 있어야 한다.
- tool/command 후보가 중복 이름을 갖거나 설명이 비어 있을 때도 목록은 이해 가능한 방식으로 표시되어야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 agent session에서 사용 가능한 tool/command 후보를 prompt 자동완성의 후보 원천으로 사용할 수 있어야 한다.
- **FR-002**: 시스템은 prompt 입력 중 현재 커서 주변 토큰이 `$` 또는 `/`로 시작하는 경우 자동완성 후보 표시를 시작해야 한다.
- **FR-003**: 시스템은 사용자가 prefix 뒤에 입력한 문자에 따라 후보 목록을 관련 항목으로 필터링해야 한다.
- **FR-004**: 자동완성 목록은 각 후보의 이름과 짧은 설명을 표시해야 한다.
- **FR-005**: 사용자는 키보드로 후보 목록을 열람하고 현재 선택 후보를 이동할 수 있어야 한다.
- **FR-006**: 사용자는 키보드 또는 포인터 입력으로 후보를 확정할 수 있어야 하며, 확정된 후보는 현재 prompt의 작성 위치에 command/token으로 삽입되어야 한다.
- **FR-007**: 사용자는 자동완성을 취소할 수 있어야 하며, 취소 시 기존 prompt 내용은 변경되지 않아야 한다.
- **FR-008**: tool/command 후보가 아직 준비되지 않았거나 비어 있는 경우에도 prompt 작성, 줄바꿈, 제출, queued prompt 흐름은 기존처럼 동작해야 한다.
- **FR-009**: 자동완성 선택 동작은 prompt 제출 동작과 명확히 구분되어야 하며, 사용자가 의도하지 않은 prompt 제출을 일으키면 안 된다.
- **FR-010**: 후보 목록은 prompt 입력 영역과 관련 보조 UI를 가리지 않도록 표시되어야 하며, 긴 이름과 설명은 읽기 가능한 범위 안에서 처리되어야 한다.
- **FR-011**: 시스템은 자동완성 후보가 많은 경우에도 사용자가 입력한 토큰에 맞는 후보를 우선적으로 찾을 수 있게 해야 한다.
- **FR-012**: 자동완성 기능은 prompt 안에 삽입된 command/token만 변경해야 하며, 선택만으로 agent 실행, tool 호출, permission 승인 같은 실행성 동작을 시작하면 안 된다.

### Key Entities *(include if feature involves data)*

- **Prompt Draft**: 사용자가 현재 작성 중인 프롬프트 텍스트와 커서 위치를 포함하는 작성 상태.
- **Autocomplete Trigger Token**: 현재 커서 주변에서 `$` 또는 `/`로 시작해 자동완성 후보 표시를 유발하는 부분 문자열.
- **Tool/Command Candidate**: prompt에 삽입할 수 있는 agent tool 또는 command 항목. 이름, 설명, 표시 가능한 식별 정보, 삽입될 token 값을 가진다.
- **Autocomplete Selection State**: 표시 중인 후보 목록, 현재 강조된 후보, 열림/닫힘 상태, 취소 또는 확정 상태.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 범위는 `apps/agentic-workbench`의 agent run prompt 작성 경험이다. 다른 앱과 공유할 순수 후보 필터링 규칙이 필요해지는 경우에만 `packages/*`로 분리하고, 이번 명세의 기본 범위는 AW 내부 기능으로 둔다.
- **Frontend layering**: prompt 입력 및 자동완성 상호작용은 `features` 범위의 사용자 행동으로 다루며, agent session의 tool/command 후보 모델과 조회 상태는 `entities` 경계에서 제공한다. 재사용 가능한 작은 UI 원자는 필요 시 `shared` 또는 기존 UI primitive를 사용한다.
- **Backend boundary**: 이 기능은 사용자 입력 경험이 중심이며, backend 변경이 필요한 경우에도 agent session 후보 제공은 application service와 adapter 경계를 통해 노출되어야 한다. persistence나 business logic이 command boundary에 직접 들어가면 안 된다.
- **Shared core vs UI**: 후보 매칭, 정렬, 삽입 범위 계산이 여러 UI에서 재사용될 정도로 커지면 headless helper로 먼저 분리한다. 화면 배치와 prompt composer UI는 AW 경험에 맞게 local UI로 유지한다.
- **Persistence and safety**: 자동완성 후보 선택은 prompt draft만 수정하며 agent 실행, tool 호출, permission 응답, 파일 변경을 자동 수행하지 않는다. session/run scope가 있는 후보는 현재 prompt가 속한 session 범위 안에서만 표시되어야 한다.
- **Documentation and Storybook**: prompt composer의 자동완성 상태는 Storybook interaction 또는 동등한 UI 검증 상태로 등록한다. 별도 프로젝트 문서가 필요하면 `docs/*.md`에 영어 파일명과 한국어 내용으로 작성한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 사용자는 tool/command 이름의 일부만 알고 있어도 `$` 또는 `/` 입력 후 3초 이내에 관련 후보를 찾아 prompt에 삽입할 수 있다.
- **SC-002**: 대표 세션에서 자동완성 후보가 50개 이상이어도 사용자는 5회 이하의 키 입력으로 원하는 후보를 선택할 수 있다.
- **SC-003**: 자동완성 후보가 준비되지 않았거나 일치 항목이 없는 상태에서도 기존 prompt 작성 및 제출 성공률은 100% 유지된다.
- **SC-004**: 자동완성 목록이 열린 상태에서 키보드 탐색, 선택, 취소 동작은 테스트된 주요 시나리오에서 의도하지 않은 prompt 제출을 발생시키지 않는다.
- **SC-005**: 사용자 검증에서 신규 또는 가끔 사용하는 tool/command를 찾는 데 필요한 기억 의존도가 감소했다고 응답한 비율이 80% 이상이다.

## Assumptions

- `$`와 `/`는 모두 command/tool 자동완성 진입 prefix로 지원하되, prefix별 의미 구분은 사용자가 선택한 token 표시와 후속 설계에서 확장 가능하게 둔다.
- 자동완성 후보는 현재 agent session 또는 prompt composer가 접근 가능한 세션 범위의 tool/command 목록을 기준으로 한다.
- 후보 선택은 command/token 문자열 삽입까지만 수행하며, 실제 agent 실행이나 tool 호출은 사용자의 별도 prompt 제출 이후에만 일어난다.
- 후보 설명이 없는 항목은 이름 중심으로 표시하고, 설명 영역은 비어 있거나 안전한 기본 문구로 처리한다.
- 모바일 전용 입력 최적화는 이번 범위의 핵심이 아니며, 데스크톱 키보드 중심 작업 흐름을 우선한다.
