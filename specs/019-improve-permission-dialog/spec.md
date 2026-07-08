# Feature Specification: 긴 Permission 다이얼로그 레이아웃 개선

**Feature Branch**: `[019-improve-permission-dialog]`

**Created**: 2026-07-09

**Status**: Draft

**Input**: User description: "GitHub issue #138 - 긴 permission 다이얼로그 레이아웃 개선"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 긴 권한 요청을 안정적으로 검토한다 (Priority: P1)

AW 사용자는 긴 명령어, 긴 본문, JSON 형태의 payload, 여러 줄 markdown이 포함된 permission 요청을 받을 때 다이얼로그가 깨지지 않은 상태로 승인 대상과 상세 내용을 검토할 수 있다.

**Why this priority**: permission 요청은 사용자가 실행 권한을 승인하기 전 마지막 확인 지점이므로, 레이아웃이 깨지면 승인 범위를 오해하거나 필요한 작업을 진행하지 못할 수 있다.

**Independent Test**: 매우 긴 명령어와 여러 줄 payload가 포함된 permission 요청을 표시했을 때 메시지, 상세 command, 버튼 영역이 겹치지 않고 모두 확인 가능한지 검증한다.

**Acceptance Scenarios**:

1. **Given** 긴 command와 여러 줄 payload가 포함된 permission 요청이 도착했을 때, **When** 사용자가 permission 다이얼로그를 열면, **Then** 다이얼로그는 화면 안에 유지되고 command 상세는 읽거나 스크롤할 수 있어야 한다.
2. **Given** command 텍스트가 화면 너비보다 길 때, **When** 다이얼로그가 표시되면, **Then** 긴 텍스트가 버튼 영역을 밀어내거나 화면 밖으로 넘치지 않아야 한다.
3. **Given** permission 요청에 command, cwd, 승인 범위가 포함되어 있을 때, **When** 사용자가 승인 여부를 판단하면, **Then** 세 정보가 잘리지 않고 확인 가능해야 한다.

---

### User Story 2 - 승인 옵션을 빠르게 이해하고 선택한다 (Priority: P2)

AW 사용자는 승인 버튼에서 전체 command 대신 짧고 명확한 승인 범위 요약을 보고, 필요한 경우 상세 command 영역을 통해 원문을 확인할 수 있다.

**Why this priority**: 버튼 텍스트가 길면 버튼 행이 읽기 어렵고 조작하기 어려워져 승인 결정의 정확성과 속도가 떨어진다.

**Independent Test**: 긴 승인 prefix가 포함된 permission 요청을 표시했을 때 버튼 label이 짧고 조작 가능한 크기로 유지되며, 상세 정보가 별도로 확인되는지 검증한다.

**Acceptance Scenarios**:

1. **Given** 긴 승인 prefix 또는 긴 승인 옵션 텍스트가 있는 permission 요청이 도착했을 때, **When** 다이얼로그가 표시되면, **Then** 버튼은 짧은 요약 label을 표시하고 클릭 가능한 크기를 유지해야 한다.
2. **Given** 사용자가 요약 label만으로 승인 범위를 확신하기 어려울 때, **When** 상세 영역을 확인하면, **Then** 원문 command와 승인 범위를 비교할 수 있어야 한다.

---

### User Story 3 - 좁은 화면에서도 승인 흐름을 완료한다 (Priority: P3)

AW 사용자는 모바일에 준하는 좁은 창 또는 작은 데스크톱 창에서도 permission 요청을 읽고 승인 또는 거절을 완료할 수 있다.

**Why this priority**: Tauri 앱 창은 사용자가 자유롭게 크기를 조절할 수 있으므로, 좁은 창에서도 permission 다이얼로그가 업무 흐름을 막지 않아야 한다.

**Independent Test**: 좁은 창 크기에서 긴 permission 요청을 표시하고, 메시지와 버튼이 겹치지 않으며 승인 또는 거절을 완료할 수 있는지 검증한다.

**Acceptance Scenarios**:

1. **Given** 좁은 창에서 긴 permission 요청이 도착했을 때, **When** 다이얼로그가 표시되면, **Then** 버튼 영역은 하단에 안정적으로 배치되고 메시지 영역과 겹치지 않아야 한다.
2. **Given** 작은 화면에서 상세 command가 길 때, **When** 사용자가 내용을 확인하면, **Then** 다이얼로그 전체가 화면 밖으로 밀려나지 않고 상세 내용은 제한된 영역 안에서 확인 가능해야 한다.

### Edge Cases

- command 또는 payload가 공백 없이 매우 긴 단일 문자열인 경우에도 다이얼로그 너비를 깨뜨리지 않아야 한다.
- 여러 줄 markdown, escape 문자, JSON 문자열이 섞인 요청도 가독성을 잃지 않아야 한다.
- cwd가 길거나 중첩 경로인 경우에도 승인 판단에 필요한 수준으로 확인 가능해야 한다.
- 승인 옵션이 여러 개이거나 label이 모두 긴 경우에도 버튼 영역이 겹치거나 화면 밖으로 밀려나지 않아야 한다.
- 사용자가 키보드만 사용하는 경우에도 상세 확인, 승인, 거절 흐름을 완료할 수 있어야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST keep the permission dialog within the visible app window when permission request content is long.
- **FR-002**: System MUST present long command and payload content in a bounded detail area that supports full review without displacing the action buttons.
- **FR-003**: System MUST keep the action button area visually separated from the message and detail content.
- **FR-004**: System MUST preserve access to the full command text, cwd, and approval scope needed for an informed approval decision.
- **FR-005**: System MUST use concise approval option labels when the original approval text is too long for reliable button display.
- **FR-006**: Users MUST be able to distinguish the short approval summary from the full command detail.
- **FR-007**: System MUST handle long single-line text, multi-line markdown, escaped characters, and JSON-like payloads without overlapping UI elements.
- **FR-008**: System MUST support permission approval and rejection in narrow app windows without requiring users to resize the window.
- **FR-009**: System MUST ensure all actionable controls remain large enough to identify and activate in long-content permission scenarios.
- **FR-010**: System MUST maintain keyboard-accessible approval and rejection flow while long details are present.

### Key Entities

- **Permission Request**: 사용자가 승인하거나 거절해야 하는 실행 요청. command, cwd, payload 또는 메시지, 승인 옵션, 승인 범위를 포함한다.
- **Command Detail**: 사용자가 승인 전 검토해야 하는 원문 command 및 관련 상세 내용. 길거나 여러 줄일 수 있다.
- **Approval Option**: 사용자가 선택할 수 있는 승인 또는 거절 동작. 표시용 요약과 실제 승인 범위를 구분해 다룬다.
- **Dialog Layout Region**: permission 다이얼로그 안에서 메시지, 상세 정보, action controls가 차지하는 논리적 영역.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 범위는 AW(`apps/agentic-workbench`) permission 요청 UI에 한정한다. 다른 앱과 공유할 필요가 입증되지 않는 한 cross-app 공유 모듈은 만들지 않는다.
- **Frontend layering**: permission 승인 흐름과 사용자 action은 `features`, permission 요청의 표시 모델과 요약 규칙은 필요 시 `entities`, 재사용 가능한 bounded text/detail 표시 요소는 `shared` 또는 기존 `components/ui` 규칙에 맞춘다. 화면 조립은 기존 AW app/page 경계를 따른다.
- **Backend boundary**: 이번 기능은 permission 요청 표시와 승인 UX 개선이 핵심이며, Tauri backend 동작 변경은 범위 밖으로 둔다. 승인 의미나 실행 권한 자체를 바꾸는 변경이 필요해지면 별도 계획에서 inbound/application/domain 책임을 분리한다.
- **Shared core vs UI**: command 요약 규칙이 여러 permission 표시 지점에서 재사용될 때만 headless helper로 분리한다. UI 공유는 AW 안에서 실제 중복이 확인된 경우에만 도입한다.
- **Persistence and safety**: permission 요청의 command, cwd, 승인 범위는 사용자가 승인 판단에 필요한 정보이므로 숨기거나 임의로 생략하지 않는다. 이 기능은 기존 permission 의미를 축소하거나 확대하지 않는다.
- **Documentation and Storybook**: 긴 command, 긴 JSON payload, 좁은 창, 긴 승인 label 상태를 포함한 Storybook 사례를 추가한다. 별도 프로젝트 문서가 필요하면 `docs/*.md`에 영어 파일명과 한국어 내용으로 작성한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 5,000자 이상의 command 또는 payload가 포함된 permission 요청에서도 다이얼로그의 메시지와 버튼 영역이 겹치지 않는다.
- **SC-002**: 360px 너비에 준하는 좁은 창에서도 사용자가 permission 요청을 읽고 승인 또는 거절을 완료할 수 있다.
- **SC-003**: 긴 승인 옵션이 포함된 요청에서 모든 action control의 label은 한눈에 식별 가능한 요약으로 표시되고, 원문 상세는 별도 영역에서 확인 가능하다.
- **SC-004**: 사용자는 command, cwd, 승인 범위 세 가지 핵심 정보를 모두 확인한 뒤 승인 여부를 결정할 수 있다.
- **SC-005**: 긴 permission 요청 검증 사례에서 레이아웃 겹침, 화면 밖 버튼 이탈, 읽을 수 없는 버튼 label 문제가 0건이어야 한다.

## Assumptions

- permission 다이얼로그는 AW 사용자가 agent command 실행 전 권한을 승인하거나 거절하는 주요 UI이다.
- 이번 변경은 permission 승인 의미, 권한 정책, 실행 가능 command의 범위를 바꾸지 않는다.
- 긴 command 원문은 보안과 신뢰성 측면에서 사용자가 접근 가능해야 하므로 요약만 표시하는 방식은 허용하지 않는다.
- 좁은 창 대응은 일반적인 데스크톱 앱 resize 상황을 포함하며, 별도 모바일 앱 UX까지 확장하지 않는다.
- 기존 디자인 시스템과 접근성 기준을 유지하면서 긴 콘텐츠 상태를 추가한다.
