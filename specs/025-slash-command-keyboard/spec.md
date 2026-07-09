# Feature Specification: Slash Command Keyboard Navigation

**Feature Branch**: `[025-slash-command-keyboard]`

**Created**: 2026-07-09

**Status**: Draft

**Input**: User description: "#152 이슈 진행. 키보드로 이동할 때 포커스된 아이템이 화면 바깥으로 벗어나서 항목을 확인할 수 없음."

**Decision**: 기존 prompt 자동 완성 흐름을 유지하면서 키보드 탐색, 현재 후보 가시성, 선택/취소 동작을 개선한다. 선택된 여러 항목을 쌓는 multi-select 형태의 UI 전환은 이번 범위에서 제외한다.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 키보드로 후보를 이동하며 항상 현재 항목을 확인한다 (Priority: P1)

사용자는 slash command prompt 자동 완성 목록이 열린 상태에서 방향키로 후보를 이동할 때 현재 포커스된 후보를 항상 화면 안에서 확인할 수 있다. 목록이 길거나 스크롤이 필요한 경우에도 포커스가 화면 밖으로 사라지지 않아야 한다.

**Why this priority**: 현재 문제는 키보드 탐색 중 선택 대상이 보이지 않아 사용자가 어떤 명령을 선택하는지 알 수 없게 만드는 핵심 사용성 결함이다.

**Independent Test**: 충분히 긴 command 후보 목록을 열고 키보드로 아래쪽과 위쪽 끝까지 이동했을 때, 매 이동 후 강조된 후보가 자동 완성 목록의 보이는 영역 안에 남아 있는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 자동 완성 후보가 스크롤 가능한 개수로 표시되어 있고 첫 후보가 강조된 상태, **When** 사용자가 `ArrowDown`을 반복해서 누르면, **Then** 강조된 후보가 이동하며 목록이 필요한 만큼 스크롤되어 현재 후보가 보이는 영역 밖으로 벗어나지 않는다.
2. **Given** 자동 완성 후보 목록의 아래쪽 후보가 강조된 상태, **When** 사용자가 `ArrowUp`을 반복해서 누르면, **Then** 강조된 후보가 위로 이동하며 목록이 필요한 만큼 스크롤되어 현재 후보가 보이는 영역 안에 유지된다.
3. **Given** 자동 완성 후보 목록이 화면 하단이나 좁은 영역에서 열려 있는 상태, **When** 사용자가 키보드로 후보를 이동하면, **Then** 현재 후보 텍스트와 주요 설명이 가려지거나 컨테이너 밖으로 밀리지 않는다.

---

### User Story 2 - 마우스 없이 후보를 선택하고 닫는다 (Priority: P2)

사용자는 자동 완성 목록이 열린 상태에서 마우스를 사용하지 않고 현재 후보를 적용하거나 자동 완성을 취소할 수 있다.

**Why this priority**: keyboard-only prompt 작성 흐름은 command 입력 속도와 접근성에 직접적인 영향을 준다.

**Independent Test**: 자동 완성 목록을 열고 키보드만 사용해 후보 선택, 적용, 취소를 수행해 prompt 입력값과 목록 표시 상태가 기대대로 바뀌는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 자동 완성 후보가 열려 있고 특정 후보가 강조된 상태, **When** 사용자가 `Enter`를 누르면, **Then** 강조된 후보가 prompt에 적용되고 자동 완성 목록이 닫힌다.
2. **Given** 자동 완성 후보가 열려 있고 특정 후보가 강조된 상태, **When** 사용자가 `Tab`을 누르면, **Then** 강조된 후보가 prompt에 적용되거나 다음 완성 단계로 자연스럽게 진행된다.
3. **Given** 자동 완성 후보가 열려 있는 상태, **When** 사용자가 `Escape`를 누르면, **Then** prompt 입력값을 불필요하게 변경하지 않고 자동 완성 목록이 닫힌다.

---

### User Story 3 - 후보 목록 변경 후에도 안전하게 탐색한다 (Priority: P3)

사용자는 입력값을 수정해 후보 목록이 바뀌더라도 키보드 탐색 상태가 깨지지 않고, 선택 가능한 후보 범위 안에서 자연스럽게 계속 탐색할 수 있다.

**Why this priority**: command 입력 중 후보 목록은 자주 필터링되므로, 목록 변경 시 포커스가 사라지거나 잘못된 후보를 선택하는 문제를 막아야 한다.

**Independent Test**: 자동 완성 목록이 열린 상태에서 입력값을 바꿔 후보 수를 줄이거나 늘린 뒤 방향키와 선택 키가 정상 동작하는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 여러 후보 중 뒤쪽 후보가 강조된 상태, **When** 사용자가 입력값을 수정해 후보 수가 줄어들면, **Then** 강조 상태는 남아 있는 후보 중 유효한 항목으로 조정된다.
2. **Given** 자동 완성 후보가 없는 상태, **When** 사용자가 방향키, `Enter`, `Tab`, `Escape`를 누르면, **Then** 오류 없이 prompt 입력 흐름이 유지된다.

### Edge Cases

- 후보 목록이 한 화면에 모두 들어가는 경우에는 불필요한 스크롤 이동이 발생하지 않는다.
- 후보 목록이 매우 긴 경우에도 키보드 이동 시 현재 후보가 보이는 영역 안에 유지된다.
- 긴 command 이름, 긴 설명, 입력 힌트가 있는 후보도 목록 컨테이너의 가로 너비를 넘지 않는다.
- 후보를 선택해도 여러 후보가 chip 또는 label처럼 누적 선택되지 않고, 현재 입력 중인 command 후보 하나만 prompt에 적용된다.
- 자동 완성 목록이 열려 있지 않을 때 방향키와 `Enter`는 기존 prompt 입력 동작을 해치지 않는다.
- 후보 목록이 필터링되어 현재 강조 인덱스가 더 이상 유효하지 않을 때 안전한 후보로 조정된다.
- 마우스로 후보를 hover한 뒤 키보드 이동을 시작해도 강조 상태가 혼란스럽게 분리되지 않는다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 사용자는 slash command 자동 완성 후보가 열린 상태에서 `ArrowDown`으로 다음 후보로 이동할 수 있어야 한다.
- **FR-002**: 사용자는 slash command 자동 완성 후보가 열린 상태에서 `ArrowUp`으로 이전 후보로 이동할 수 있어야 한다.
- **FR-003**: 현재 강조된 후보는 키보드 이동 후 항상 자동 완성 목록의 보이는 영역 안에 유지되어야 한다.
- **FR-004**: 후보 목록이 스크롤 가능한 경우, 현재 강조된 후보가 위 또는 아래 경계 밖으로 이동하려 할 때 목록은 자동으로 스크롤되어야 한다.
- **FR-005**: 사용자는 `Enter`로 현재 강조된 후보를 적용할 수 있어야 한다.
- **FR-006**: 사용자는 `Tab`으로 현재 강조된 후보를 적용하거나 다음 완성 단계로 진행할 수 있어야 한다.
- **FR-007**: 사용자는 `Escape`로 자동 완성 목록을 닫을 수 있어야 하며, 이 동작은 prompt 내용을 예기치 않게 변경하지 않아야 한다.
- **FR-008**: 후보 목록이 변경될 때 강조 상태는 존재하는 후보 범위 안으로 조정되어야 한다.
- **FR-009**: 후보가 없거나 자동 완성 목록이 닫힌 상태에서 키보드 입력은 오류를 발생시키지 않아야 하며 기존 prompt 입력 흐름을 방해하지 않아야 한다.
- **FR-010**: 긴 command 이름, 설명, 입력 힌트는 후보 목록의 가로 너비를 넘지 않고 사용자가 읽을 수 있는 형태로 표시되어야 한다.
- **FR-011**: 키보드로 강조된 후보와 마우스 hover로 강조된 후보가 동시에 서로 다른 선택처럼 보이지 않아야 한다.
- **FR-012**: 자동 완성 후보 적용 후 prompt에는 사용자가 선택한 command가 명확히 반영되어야 한다.
- **FR-013**: 자동 완성은 한 번에 하나의 command 후보를 현재 prompt 입력 위치에 적용해야 하며, 다중 선택 목록이나 선택 chip을 만들지 않아야 한다.
- **FR-014**: 기존 prompt 작성 흐름, trigger 기반 후보 표시, 후보 필터링, command 적용 의미는 사용자가 체감하는 방식으로 유지되어야 한다.

### Key Entities *(include if feature involves data)*

- **Autocomplete Candidate**: 사용자가 선택할 수 있는 command 후보를 나타낸다. 이름, 설명, 입력 힌트, 표시 순서, 선택 가능 여부를 포함한다.
- **Highlighted Candidate**: 현재 키보드 또는 포인터 상호작용으로 강조된 후보를 나타낸다. 후보 목록 안에서 유효한 하나의 항목이거나, 후보가 없을 때는 없음 상태다.
- **Autocomplete List View**: 후보를 표시하는 보이는 영역이다. 후보 수가 많을 때 현재 강조 항목을 사용자가 확인할 수 있도록 스크롤 상태를 가진다.
- **Prompt Command Insertion**: 사용자가 선택한 단일 후보를 현재 prompt의 command 입력 부분에 반영하는 결과를 나타낸다. 여러 후보를 동시에 선택하거나 저장하는 데이터는 만들지 않는다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 범위는 `apps/agentic-workbench`의 prompt 자동 완성 UI와 관련 모델에 한정한다. 다른 앱과의 공유 모듈 추가는 필요하지 않으며, cross-app import는 만들지 않는다.
- **Frontend layering**: 사용자 상호작용은 `features/agent-run`의 prompt 입력 및 자동 완성 UI가 담당한다. command 후보 모델과 필터링 같은 도메인 성격의 로직은 기존 `entities/agent-run` 경계를 따른다. 공용 UI primitive 변경이 필요한 경우에만 `components/ui`를 검토한다.
- **Backend boundary**: 이 기능은 사용자가 이미 받은 command 후보를 탐색하고 선택하는 UI 동작이므로 Tauri backend 변경은 기본 범위가 아니다.
- **Shared core vs UI**: 재사용 가능한 순수 로직이 필요하면 후보 인덱스 보정, 필터링, 선택 적용 같은 headless helper를 우선한다. 공유 UI 패키지 추가는 하지 않는다.
- **Persistence and safety**: 새 persistence, 파일 접근, agent session owner 변경은 범위가 아니다. 기존 run/session prompt 입력 흐름을 유지한다.
- **Documentation and Storybook**: 관련 UI 상태 검증 또는 source/rendering 테스트를 추가한다. Storybook 갱신은 기존 prompt 자동 완성 story가 있거나 컴포넌트가 reusable 상태로 분리되어 있을 때 수행한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 후보가 20개 이상인 자동 완성 목록에서 사용자가 `ArrowDown`으로 처음부터 끝까지 이동해도 강조된 후보가 100%의 이동 단계에서 보이는 영역 안에 유지된다.
- **SC-002**: 후보가 20개 이상인 자동 완성 목록에서 사용자가 `ArrowUp`으로 끝에서 처음까지 이동해도 강조된 후보가 100%의 이동 단계에서 보이는 영역 안에 유지된다.
- **SC-003**: 사용자는 마우스를 사용하지 않고 후보 열기, 후보 이동, 후보 적용, 자동 완성 닫기를 완료할 수 있다.
- **SC-004**: 후보 목록이 비어 있거나 입력 중 후보 수가 줄어드는 경우에도 키보드 조작으로 오류 상태나 깨진 강조 상태가 발생하지 않는다.
- **SC-005**: 긴 command 이름이나 설명을 가진 후보를 포함해도 자동 완성 목록의 텍스트가 컨테이너 가로 너비를 넘지 않는다.

## Assumptions

- 대상 사용자는 Agentic Workbench에서 slash command 또는 skill command를 prompt에 입력하는 사용자다.
- 자동 완성 후보는 기존 command 후보 제공 흐름을 통해 이미 수집된 목록을 사용한다.
- 이 기능은 prompt 입력 중 표시되는 자동 완성 목록의 키보드 탐색과 가시성 개선에 집중한다.
- 이번 범위는 기존 prompt 자동 완성 UI를 안정화하는 것이며, multi-select 또는 command palette 형태의 전면 전환은 포함하지 않는다.
- 접근성 패턴은 참고할 수 있지만 사용자가 보는 동작은 하나의 command 후보를 탐색하고 적용하는 autocomplete 흐름으로 유지한다.
- 모바일 전용 입력 방식이나 별도 touch gesture는 이번 범위에 포함하지 않는다.
- 기존 command 후보 정렬, 후보 생성 정책, command 실행 의미는 변경하지 않는다.
