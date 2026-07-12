# Feature Specification: Markdown 작업 보드

**Feature Branch**: `042-markdown-work-board`  
**Status**: Draft  
**Input**: 여러 Markdown 문서의 작업 진행률을 한 화면에서 확인한다.

관련 문서: [[speckit-plan | 구현 계획]] · [[speckit-tasks | 작업 목록]] · [[speckit-checklist | 요구사항 체크리스트]]

## User Scenarios & Testing

### User Story 1 - 문서별 진행률 확인 (Priority: P1)

사용자는 프로젝트 문서마다 완료된 task와 남은 task의 수를 즉시 확인한다.

**Independent Test**: task가 있는 Markdown 파일 세 개를 열고 각 문서의 집계가 원문과 일치하는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 완료 2개와 미완료 1개가 있는 문서가 있을 때, **When** 작업 보드를 열면, **Then** `2 완료 / 1 미완료`가 표시된다.
2. **Given** task가 없는 문서가 있을 때, **When** 작업 보드를 열면, **Then** 해당 문서는 일반 문서로 표시된다.

### User Story 2 - 원본 task로 이동 (Priority: P2)

사용자는 미완료 task를 선택하여 해당 문서와 chapter로 이동한다.

## Edge Cases

- 대문자 `X`로 완료 표시한 task도 완료로 집계한다.
- 코드 블록 안의 `- [ ]` 문자열은 task로 집계하지 않는다.
- H1 이전에 있는 task는 문서 서문 범위로 처리한다.

## Requirements

### Functional Requirements

- **FR-001**: 시스템은 `- [ ]`, `- [x]`, `- [X]` task를 구분해야 한다.
- **FR-002**: 시스템은 문서와 H1 chapter 단위 진행률을 제공해야 한다.
- **FR-003**: 사용자는 작업 보드에서 원본 문서로 이동할 수 있어야 한다.
- **FR-004**: 일반 목록과 코드 블록은 task 집계에서 제외해야 한다.

## Success Criteria

- **SC-001**: 기준 문서 100개에서 task 집계 정확도가 100%이다.
- **SC-002**: 사용자는 3초 이내에 가장 많은 미완료 task를 가진 문서를 찾을 수 있다.

