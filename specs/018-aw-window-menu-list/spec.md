# Feature Specification: AW Window Menu List

**Feature Branch**: `[018-aw-window-menu-list]`

**Created**: 2026-07-08

**Status**: Draft

**Input**: User description: "#137 AW native Windows 메뉴에 열린 창 목록 표시 - github issue 진행"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 열린 AW 창을 메뉴에서 확인한다 (Priority: P1)

여러 AW 창을 동시에 사용하는 사용자는 데스크톱 앱의 native `Windows` 메뉴를 열어 현재 열린 AW 창 목록을 확인할 수 있다.

**Why this priority**: 창 목록이 보이지 않으면 사용자는 메뉴를 통한 창 전환을 시작할 수 없으므로 이 기능의 최소 가치다.

**Independent Test**: AW 창을 2개 이상 연 뒤 native `Windows` 메뉴를 확인해 각 창이 식별 가능한 제목으로 표시되는지 검증한다.

**Acceptance Scenarios**:

1. **Given** AW 창이 2개 열려 있음, **When** 사용자가 native `Windows` 메뉴를 열면, **Then** 메뉴에는 열린 AW 창 2개가 각각 표시된다.
2. **Given** AW 창이 1개만 열려 있음, **When** 사용자가 native `Windows` 메뉴를 열면, **Then** 메뉴에는 현재 창 1개가 표시된다.

---

### User Story 2 - 메뉴에서 원하는 창으로 전환한다 (Priority: P2)

여러 AW 창을 오가는 사용자는 native `Windows` 메뉴의 창 항목을 선택해 원하는 창을 전면으로 가져오거나 포커스할 수 있다.

**Why this priority**: 창 목록 확인만으로는 탐색 문제가 완전히 해결되지 않으며, 실제 작업 전환을 완료하려면 선택 동작이 필요하다.

**Independent Test**: 배경에 있는 AW 창을 native `Windows` 메뉴에서 선택했을 때 해당 창이 전면으로 이동하고 입력 포커스를 받을 수 있는지 검증한다.

**Acceptance Scenarios**:

1. **Given** AW 창 A가 전면에 있고 창 B가 뒤에 있음, **When** 사용자가 native `Windows` 메뉴에서 창 B를 선택하면, **Then** 창 B가 전면으로 이동하고 활성 창이 된다.
2. **Given** 사용자가 이미 전면에 있는 AW 창을 메뉴에서 선택함, **When** 선택 동작이 완료되면, **Then** 현재 창은 활성 상태를 유지하고 오류나 중복 창 생성이 발생하지 않는다.

---

### User Story 3 - 창 목록이 현재 상태와 동기화된다 (Priority: P3)

사용자는 창을 열거나 닫거나 창 제목이 바뀐 뒤에도 native `Windows` 메뉴에서 최신 창 목록과 제목을 볼 수 있다.

**Why this priority**: 목록이 오래된 상태로 남으면 사용자가 닫힌 창을 선택하거나 잘못된 창을 전환할 수 있다.

**Independent Test**: 창 생성, 닫힘, 제목 변경을 각각 수행한 뒤 native `Windows` 메뉴를 열어 목록과 표시 제목이 현재 상태와 일치하는지 검증한다.

**Acceptance Scenarios**:

1. **Given** 새 AW 창이 생성됨, **When** 사용자가 native `Windows` 메뉴를 열면, **Then** 새 창 항목이 목록에 추가되어 있다.
2. **Given** AW 창이 닫힘, **When** 사용자가 native `Windows` 메뉴를 열면, **Then** 닫힌 창 항목은 목록에서 제거되어 있다.
3. **Given** AW 창 제목이 변경됨, **When** 사용자가 native `Windows` 메뉴를 열면, **Then** 해당 창 항목은 변경된 제목으로 표시된다.

### Edge Cases

- 창 제목이 비어 있거나 아직 로딩 중이면 사용자가 구분 가능한 기본 창 이름을 표시한다.
- 같은 제목을 가진 창이 여러 개 있어도 각 메뉴 항목은 별도 창으로 선택 가능해야 한다.
- 창이 닫히는 순간 사용자가 해당 메뉴 항목을 선택하면 앱은 오류 없이 남아 있는 창 상태로 복구해야 한다.
- 최소화되거나 다른 데스크톱 공간에 있는 창을 선택해도 가능한 범위에서 전면 이동 또는 포커스가 수행되어야 한다.
- native OS가 기본 `Windows` 메뉴 항목을 제공하는 경우 AW 창 목록은 표준 동작과 충돌하지 않아야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: AW는 native `Windows` 메뉴에 현재 열린 AW 창 목록을 표시해야 한다.
- **FR-002**: 각 창 메뉴 항목은 사용자가 창을 구분할 수 있는 최신 표시 제목을 가져야 한다.
- **FR-003**: 사용자는 창 메뉴 항목을 선택해 해당 AW 창을 전면으로 이동하거나 포커스할 수 있어야 한다.
- **FR-004**: AW는 새 창이 생성되면 native `Windows` 메뉴 목록에 해당 창을 추가해야 한다.
- **FR-005**: AW는 창이 닫히면 native `Windows` 메뉴 목록에서 해당 창을 제거해야 한다.
- **FR-006**: AW는 창 제목이 변경되면 native `Windows` 메뉴의 해당 항목 제목을 갱신해야 한다.
- **FR-007**: AW는 같은 제목의 창이 여러 개 있어도 선택한 메뉴 항목이 올바른 창으로 연결되도록 해야 한다.
- **FR-008**: AW는 닫힌 창이나 사용할 수 없는 창 항목 선택 시 사용자 작업을 방해하는 오류 없이 메뉴 상태를 최신화해야 한다.
- **FR-009**: AW는 운영체제가 제공하는 표준 창 메뉴 기대 동작을 훼손하지 않아야 한다.

### Key Entities

- **AW Window**: 사용자가 열어 둔 Agentic Workbench 창이다. 표시 제목, 활성/비활성 상태, 열림/닫힘 상태를 가진다.
- **Window Menu Item**: native `Windows` 메뉴에서 하나의 AW 창을 나타내는 항목이다. 표시 제목과 대상 창 연결을 가진다.
- **Window Menu State**: native `Windows` 메뉴에 반영되어야 하는 현재 AW 창 목록과 각 항목의 최신 제목 집합이다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 범위는 `apps/agentic-workbench`에 한정한다. 다른 앱과 공유할 요구가 없으므로 cross-app 패키지는 만들지 않는다.
- **Frontend layering**: 사용자에게 보이는 화면 컴포넌트 변경은 필수가 아니다. 창 제목 표시 상태가 frontend에서 관리될 경우 기존 AW FSD 경계에 따라 app 또는 entities 계층의 창/세션 상태만 사용한다.
- **Backend boundary**: native menu와 창 제어는 AW Tauri backend 경계 안에서 다룬다. 입력 어댑터는 메뉴 이벤트를 받고, application 계층은 창 목록 동기화 규칙을 조율하며, infrastructure 계층은 OS/native window 기능과 연결한다.
- **Shared core vs UI**: 현재 요구는 AW 전용 native 메뉴 동작이므로 shared UI나 shared core 추출은 범위 밖이다.
- **Persistence and safety**: 영구 저장은 필요하지 않다. 창 선택 동작은 현재 앱 인스턴스의 열린 창만 대상으로 하며 닫힌 창 또는 유효하지 않은 창 참조를 안전하게 무시하고 최신 상태로 복구해야 한다.
- **Documentation and Storybook**: 화면 UI 컴포넌트가 추가되지 않으면 Storybook은 필요하지 않다. native menu 동작과 검증 절차는 필요 시 `docs/*.md`의 한국어 문서로 보강한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 사용자는 AW 창을 3개 연 상태에서 native `Windows` 메뉴를 열어 2초 이내에 열린 창 3개를 모두 확인할 수 있다.
- **SC-002**: 사용자는 native `Windows` 메뉴에서 배경 창을 선택한 뒤 1초 이내에 해당 창이 활성화된 것을 확인할 수 있다.
- **SC-003**: 창 생성, 닫힘, 제목 변경 후 native `Windows` 메뉴를 다시 열면 100%의 수동 검증 시나리오에서 목록이 현재 창 상태와 일치한다.
- **SC-004**: 같은 제목의 창 2개가 있는 상태에서도 메뉴 항목 선택 테스트 10회 중 10회가 선택한 대상 창으로 전환된다.
- **SC-005**: 닫힘 직전 또는 닫힌 창 항목과 관련된 비정상 선택 상황에서도 앱이 중단되지 않고 사용자는 남아 있는 창을 계속 사용할 수 있다.

## Assumptions

- 대상 사용자는 AW를 데스크톱 앱으로 실행하며 여러 창을 동시에 사용할 수 있다.
- `Windows` 메뉴는 macOS 등 native menu를 제공하는 데스크톱 환경에서 검증하는 것을 기본으로 한다.
- 창 제목은 기존 AW 창 제목 정책을 따른다.
- 이 기능은 열린 AW 창 목록과 전환 동작만 다루며, 새 창 생성 명령이나 창 정렬 명령 추가는 포함하지 않는다.
- 운영체제별 native menu 제약으로 완전히 동일한 표현이 어려운 경우에도 사용자는 열린 창을 식별하고 전환할 수 있어야 한다.
