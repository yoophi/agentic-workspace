# Feature Specification: MA markdown viewer heading 기반 Table of Contents

**Feature Branch**: `009-ma-heading-toc`

**Created**: 2026-07-02

**Status**: Draft

**Input**: User description: "https://github.com/yoophi/agentic-workspace/issues/124 이슈 진행 — MA: markdown render 시 h1~h3 heading 기반 table of contents 제공"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - TOC로 문서 내 빠른 이동 (Priority: P1)

사용자가 markdown-annotator에서 긴 markdown 문서를 열었을 때, 문서의 h1~h3 heading이 목차(TOC)로 표시되고, 목차 항목을 클릭하면 viewer가 해당 heading 위치로 스크롤 이동한다. 이를 통해 스크롤을 길게 내리지 않고도 원하는 섹션으로 즉시 이동할 수 있다.

**Why this priority**: 이 기능의 존재 이유 자체다. 문서가 길어질수록 특정 섹션 탐색 비용이 커지는 문제를 직접 해결하며, 이 스토리만 구현해도 사용자 가치가 완결된다.

**Independent Test**: h1~h3 heading이 여러 개 포함된 문서를 열어 TOC가 표시되는지 확인하고, 임의의 TOC 항목을 클릭해 viewer가 해당 heading으로 스크롤되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** h1~h3 heading이 포함된 문서가 열린 상태, **When** viewer가 렌더링되면, **Then** 문서 내 등장 순서대로 h1~h3 heading이 TOC에 표시된다.
2. **Given** TOC가 표시된 상태, **When** 사용자가 TOC 항목을 클릭하면, **Then** viewer가 해당 heading 블록 위치로 스크롤 이동한다.
3. **Given** h1~h6 heading이 섞여 있는 문서, **When** TOC가 표시되면, **Then** h4~h6 heading은 TOC에 포함되지 않는다.
4. **Given** 동일한 텍스트를 가진 heading이 문서에 여러 번 등장하는 경우, **When** 사용자가 그중 특정 TOC 항목을 클릭하면, **Then** 텍스트가 같은 다른 heading이 아니라 그 항목에 대응하는 heading 위치로 정확히 이동한다.

---

### User Story 2 - 계층 구조 시각화 (Priority: P2)

사용자가 TOC를 볼 때, heading level(h1/h2/h3)에 따라 들여쓰기가 적용된 계층 구조로 표시되어 문서의 전체 구조를 한눈에 파악할 수 있다.

**Why this priority**: 이동 기능(P1)이 있어도 평평한 목록으로는 문서 구조 파악이 어렵다. 계층 표현은 탐색 효율을 높이지만, 없어도 P1의 이동 기능 자체는 동작한다.

**Independent Test**: h1 아래 h2, h2 아래 h3가 있는 문서를 열어, TOC에서 level별 들여쓰기 깊이가 서로 다르게 표시되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** h1, h2, h3가 혼재된 문서, **When** TOC가 표시되면, **Then** h2 항목은 h1보다, h3 항목은 h2보다 더 깊게 들여쓰기되어 표시된다.
2. **Given** 상위 level 없이 h3부터 시작하는 문서(예: h1 없이 h3만 존재), **When** TOC가 표시되면, **Then** 항목이 누락되지 않고 각 level에 해당하는 들여쓰기로 표시된다.

---

### User Story 3 - heading 없는 문서에서 TOC 미표시 (Priority: P3)

사용자가 heading이 하나도 없는 문서(또는 h4~h6만 있는 문서)를 열었을 때, 빈 TOC 영역이 화면 공간을 차지하지 않는다.

**Why this priority**: 빈 상태 처리로, 기능의 완성도와 화면 공간 효율에 관련된다. P1/P2가 동작한 뒤 다듬는 성격의 요구사항이다.

**Independent Test**: heading이 전혀 없는 문서와 h4~h6만 있는 문서를 각각 열어, TOC 영역이 렌더되지 않는지 확인한다.

**Acceptance Scenarios**:

1. **Given** heading이 하나도 없는 문서, **When** viewer가 렌더링되면, **Then** TOC 영역이 렌더되지 않는다.
2. **Given** h4~h6 heading만 있는 문서, **When** viewer가 렌더링되면, **Then** TOC 영역이 렌더되지 않는다.

---

### Edge Cases

- heading 텍스트에 inline 서식(bold, code, link 등)이 포함된 경우, TOC에는 서식이 제거된 일반 텍스트로 표시된다.
- heading 텍스트가 빈 문자열이거나 공백만 있는 경우에도 TOC 항목은 문서 순서를 유지하며, 클릭 시 해당 블록으로 이동한다.
- 문서 맨 끝에 있는 heading을 클릭하면, viewer는 스크롤 가능한 최대 위치까지 이동한다 (heading을 화면 최상단에 고정할 수 없는 경우 포함).
- TOC 항목이 매우 많은 긴 문서에서는 TOC 자체가 독립적으로 스크롤 가능해야 하며, 본문 viewer의 스크롤과 간섭하지 않는다.
- 동일 문서에서 annotation 작업(텍스트 선택, 주석 생성) 중 TOC 클릭으로 스크롤이 이동해도 기존 annotation 앵커(line/offset) 데이터는 변경되지 않는다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 markdown 문서의 heading 블록 중 level 1~3 항목으로 TOC 항목 목록을 산출해야 한다. 각 TOC 항목은 대상 블록을 유일하게 식별할 수 있는 정보(블록 id), level, 표시 텍스트, 문서 내 시작 위치를 포함한다.
- **FR-002**: TOC 항목은 문서 내 등장 순서대로 표시되어야 한다.
- **FR-003**: TOC 항목은 heading level에 따라 시각적으로 구분되는 들여쓰기(h1 < h2 < h3 순으로 깊어짐)로 표시되어야 한다.
- **FR-004**: level 4~6 heading은 TOC에 포함되지 않아야 한다.
- **FR-005**: 사용자가 TOC 항목을 클릭하면 viewer가 해당 heading 블록 위치로 스크롤 이동해야 한다. 이동 대상은 heading 텍스트가 아닌 블록 고유 id로 결정되어, 동일 텍스트 heading이 여러 개여도 정확한 위치로 이동해야 한다.
- **FR-006**: TOC 대상 heading(h1~h3)이 하나도 없는 문서에서는 TOC 영역이 렌더되지 않아야 한다.
- **FR-007**: TOC 도입은 본문 렌더링 방식과 기존 annotation 앵커(line/offset) 동작을 변경하지 않아야 한다.
- **FR-008**: heading 텍스트에 inline 서식이 포함된 경우 TOC에는 서식 기호가 제거된 텍스트로 표시되어야 한다.

### Key Entities

- **TOC Entry**: TOC의 한 항목. 대상 heading 블록의 고유 식별자(blockId), heading level(1~3), 표시 텍스트(text), 문서 내 시작 라인(startLine)을 가진다. 기존 heading 블록에서 파생되는 읽기 전용 데이터로, 원본 문서나 annotation 데이터를 변경하지 않는다.
- **Heading Block** (기존): 파서가 산출하는 `type: "heading"` 블록. 고유 `id`, `level`, `startLine`을 이미 보유하며 TOC Entry 산출의 원천이 된다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: `packages/markdown-annotation-core`(TOC entry 추출 로직), `packages/markdown-annotation-react`(TOC 컴포넌트), `apps/markdown-annotator`(viewer 레이아웃 배치)가 범위. 앱 간 직접 import 없이 패키지를 통해 공유한다.
- **Frontend layering**: `apps/markdown-annotator`에서는 viewer 화면 레이아웃(FSD pages/features 레이어)에 TOC를 배치한다. 재사용 가능한 TOC UI는 앱이 아닌 `markdown-annotation-react` 패키지에 둔다.
- **Backend boundary**: 해당 없음 (Tauri/Rust 변경 없음, 프론트엔드 전용 기능).
- **Shared core vs UI**: 순수 core 우선 원칙에 따라 TOC entry 추출은 `markdown-annotation-core`에 순수 함수로 두고 fixture 기반 unit test로 검증한다. UI 컴포넌트는 이미 공유가 확립된 `markdown-annotation-react`에 추가하며, 앱 shell·라우팅·persistence에 의존하지 않는다.
- **Persistence and safety**: 해당 없음 (파일 쓰기, 세션, 권한 변경 없음. TOC는 파생 읽기 전용 데이터).
- **Documentation and Storybook**: `markdown-annotation-react`의 TOC 컴포넌트에 대한 Storybook 스토리 추가/갱신, 관련 unit test 추가.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: h1~h3 heading이 있는 문서를 열면 사용자가 별도 조작 없이 TOC를 즉시 확인할 수 있고, 100개 이상의 heading이 있는 문서에서도 TOC 표시가 체감 지연 없이 완료된다.
- **SC-002**: 임의의 TOC 항목 클릭 시 100%의 경우에 해당 heading(동일 텍스트 중복 포함)으로 정확히 스크롤 이동한다.
- **SC-003**: h4~h6 heading이 TOC에 나타나는 경우가 0건이다.
- **SC-004**: heading 없는 문서에서 TOC 영역이 표시되는 경우가 0건이다.
- **SC-005**: TOC 도입 전후로 기존 annotation 생성·표시(line/offset 앵커) 관련 기존 테스트가 모두 통과하며, 앵커 동작 회귀가 0건이다.

## Assumptions

- TOC는 viewer 화면에서 본문과 함께 항상 접근 가능한 형태(사이드 패널 등)로 배치하되, 구체적 배치(사이드 패널 vs 접이식)는 이슈에서 양쪽을 허용하므로 설계 단계에서 앱 레이아웃에 맞게 결정한다. 좁은 화면에서는 접거나 숨길 수 있는 형태를 기본으로 한다.
- 스크롤 이동은 부드러운(smooth) 스크롤을 기본으로 하되, 사용자 환경의 모션 감소 설정을 존중한다.
- 현재 문서 스크롤 위치에 따라 TOC에서 현재 섹션을 강조(active highlight)하는 기능은 이번 범위에 포함하지 않는다 (이슈 요구사항에 없음).
- TOC 항목 클릭은 URL/해시 변경 없이 화면 내 스크롤만 수행한다.
- 파서가 이미 heading 블록에 고유 id, level, 시작 라인을 제공하므로 파서 자체의 변경은 필요하지 않다.
