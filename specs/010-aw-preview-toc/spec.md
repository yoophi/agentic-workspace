# Feature Specification: AW workspace markdown preview heading 기반 Table of Contents

**Feature Branch**: `issue-127-aw-toc`

**Created**: 2026-07-02

**Status**: Draft

**Input**: User description: "https://github.com/yoophi/agentic-workspace/issues/127 이슈 진행 — AW: workspace markdown preview에 h1~h3 heading 기반 table of contents 제공"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - TOC로 preview 문서 내 빠른 이동 (Priority: P1)

사용자가 AW(agentic-workbench)의 worktree 세션 workspace 패널에서 긴 markdown 문서(계획/설계 문서 등)를 preview로 열었을 때, 문서의 h1~h3 heading 목차(TOC)를 열어 항목을 클릭하면 preview가 해당 heading 위치로 스크롤 이동한다. agent가 생성한 긴 문서를 검토하며 원하는 섹션으로 즉시 이동할 수 있다.

**Why this priority**: 이 기능의 존재 이유다. workspace preview는 agent 작업 결과(긴 계획/설계 문서)를 검토하는 핵심 화면이며, 섹션 탐색 수단이 없어 스크롤 비용이 크다. 이 스토리만으로 사용자 가치가 완결된다.

**Independent Test**: h1~h3 heading이 여러 개(동일 텍스트 중복 포함) 있는 markdown 파일을 workspace preview로 열어 TOC를 펼치고, 임의 항목 클릭 시 해당 heading으로 preview가 스크롤되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** h1~h3 heading이 포함된 markdown 파일이 preview에 열린 상태, **When** 사용자가 TOC를 펼치면, **Then** 문서 내 등장 순서대로 h1~h3 heading이 level별 들여쓰기와 함께 표시된다.
2. **Given** TOC가 펼쳐진 상태, **When** 사용자가 TOC 항목을 클릭하면, **Then** preview가 해당 heading 블록 위치로 스크롤 이동한다.
3. **Given** h1~h6 heading이 섞여 있는 문서, **When** TOC가 표시되면, **Then** h4~h6 heading은 TOC에 포함되지 않는다.
4. **Given** 동일한 텍스트의 heading이 여러 번 등장하는 문서, **When** 사용자가 그중 특정 TOC 항목을 클릭하면, **Then** 텍스트가 같은 다른 heading이 아니라 그 항목에 대응하는 heading 위치로 정확히 이동한다.

---

### User Story 2 - 밀도 높은 workspace 화면과의 공존 (Priority: P2)

사용자가 TOC를 사용하지 않을 때는 preview의 본문 가독성과 annotation 작업 공간이 침해되지 않는다. TOC는 기본적으로 접혀 있고, 필요할 때만 펼쳐 사용하며, 다시 접을 수 있다.

**Why this priority**: workspace 패널은 파일 트리·preview·annotation 열이 병행되는 밀도 높은 화면이고 preview 폭은 사용자가 조절할 수 있어, TOC가 상시 공간을 차지하면 기존 워크플로를 해친다. 다만 P1(이동 기능)이 있어야 의미가 있는 보조 요구사항이다.

**Independent Test**: preview를 연 직후 TOC가 접힌 상태로 본문이 기존과 동일한 폭을 사용하는지 확인하고, 펼침/접힘 토글이 동작하는지, preview 폭을 좁혀도 본문 읽기와 텍스트 선택(annotation)이 가능한지 확인한다.

**Acceptance Scenarios**:

1. **Given** markdown 파일을 preview로 연 직후, **When** 사용자가 아무 조작도 하지 않으면, **Then** TOC는 접힌 상태이며 본문 표시 영역은 TOC 도입 전과 실질적으로 동일하다.
2. **Given** TOC가 펼쳐진 상태, **When** 사용자가 접기 조작을 하면, **Then** TOC가 접히고 본문 영역이 복원된다.
3. **Given** preview 패널 폭을 좁게 조절한 상태, **When** TOC를 펼치면, **Then** 본문과 TOC가 겹치거나 잘리지 않으며 annotation 텍스트 선택이 계속 가능하다.
4. **Given** TOC 항목이 많은 긴 문서에서 TOC를 펼친 상태, **When** TOC 목록이 표시 영역을 초과하면, **Then** TOC는 자체적으로 스크롤되고 preview 본문 스크롤과 간섭하지 않는다.

---

### User Story 3 - heading 없는 문서에서 TOC 미표시 (Priority: P3)

사용자가 heading이 하나도 없는 문서(또는 h4~h6만 있는 문서)를 preview로 열었을 때, TOC 관련 UI(펼침 조작 포함)가 나타나지 않는다.

**Why this priority**: 빈 상태 처리로 완성도에 관련된다. P1/P2가 동작한 뒤 다듬는 성격이다.

**Independent Test**: heading이 전혀 없는 문서와 h4~h6만 있는 문서를 각각 preview로 열어, TOC 영역과 펼침 조작이 렌더되지 않는지 확인한다.

**Acceptance Scenarios**:

1. **Given** heading이 하나도 없는 문서, **When** preview가 렌더링되면, **Then** TOC 영역과 TOC 펼침 조작이 렌더되지 않는다.
2. **Given** h4~h6 heading만 있는 문서, **When** preview가 렌더링되면, **Then** TOC 영역과 TOC 펼침 조작이 렌더되지 않는다.

---

### Edge Cases

- 파일 트리에서 다른 markdown 파일로 전환하면 TOC 내용은 새 문서 기준으로 갱신된다. 새 문서에 h1~h3가 없으면 TOC UI가 사라진다.
- 문서가 외부(agent 실행 등)에서 변경되어 preview가 자동 갱신되면 TOC도 최신 heading 목록으로 갱신된다.
- 문서 맨 끝의 heading을 클릭하면 preview는 스크롤 가능한 최대 위치까지 이동한다.
- heading 텍스트에 inline 서식(bold, code, link 등)이 포함되어도 TOC에는 서식 기호가 제거된 텍스트로 표시된다.
- TOC 이동 후 텍스트 선택으로 annotation을 생성해도 기존 line/offset 앵커 데이터는 영향을 받지 않는다.
- preview가 로딩 중/오류/파일 미선택 상태일 때는 TOC UI가 나타나지 않는다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: workspace markdown preview는 문서의 heading 블록 중 level 1~3 항목으로 구성된 TOC를 제공해야 한다. 항목은 문서 등장 순서를 유지하고 level별 들여쓰기(h1 < h2 < h3 깊이)로 표시된다.
- **FR-002**: level 4~6 heading은 TOC에 포함되지 않아야 한다.
- **FR-003**: TOC 항목 클릭 시 preview가 해당 heading 블록 위치로 스크롤 이동해야 한다. 이동 대상은 heading 텍스트가 아닌 블록 고유 식별자로 결정되어, 동일 텍스트 heading이 여러 개여도 정확한 위치로 이동해야 한다.
- **FR-004**: TOC는 기본 접힘 상태로 제공되며, 사용자가 펼치고 접을 수 있어야 한다. 접힌 상태에서 본문 표시 영역은 TOC 도입 전과 실질적으로 동일해야 한다.
- **FR-005**: TOC 대상 heading(h1~h3)이 하나도 없는 문서, 또는 preview가 로딩/오류/미선택 상태일 때는 TOC UI(펼침 조작 포함)가 렌더되지 않아야 한다.
- **FR-006**: 표시 중인 문서가 바뀌거나(파일 전환) 내용이 갱신되면(자동 새로고침) TOC도 해당 문서의 최신 heading 목록으로 갱신되어야 한다.
- **FR-007**: heading 텍스트에 inline 서식이 포함된 경우 TOC에는 서식 기호가 제거된 텍스트로 표시되어야 한다.
- **FR-008**: TOC 도입은 preview 본문 렌더링 방식, 기존 annotation 앵커(line/offset), 텍스트 선택 하이라이트 동작을 변경하지 않아야 한다.
- **FR-009**: preview 패널 폭이 좁아져도 TOC와 본문이 겹치거나 잘리지 않아야 하며, TOC 목록이 길면 자체 스크롤되어야 한다.

### Key Entities

- **TOC Entry** (기존): 대상 heading 블록의 고유 식별자(blockId), heading level(1~3), 서식 제거된 표시 텍스트(text), 문서 내 시작 라인(startLine). specs/009(MA TOC)에서 공유 패키지에 정의된 파생 읽기 전용 데이터를 그대로 사용하며, 이 기능에서 신규 데이터 모델은 도입하지 않는다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 범위는 `apps/agentic-workbench`의 앱 조립뿐이다. TOC 추출·컴포넌트·스크롤 helper는 specs/009에서 `packages/markdown-annotation-core`/`packages/markdown-annotation-react`에 이미 구현되어 있어 **공유 패키지 변경 없음**을 원칙으로 한다. 앱 간 직접 import 없음.
- **Frontend layering**: 변경은 `apps/agentic-workbench/src/features/worktree-workspace`(preview UI가 속한 features 레이어)에 한정한다. shadcn 컴포넌트는 `components/ui`에서 import.
- **Backend boundary**: 해당 없음 (Tauri/Rust 변경 없음).
- **Shared core vs UI**: 순수 core와 공유 UI는 이미 존재(specs/009). 이 기능은 소비 앱 조립만 수행한다. 조립 과정에서 공유 컴포넌트 변경이 필요해지면 MA(정본)와의 호환을 유지하는 방향으로 최소화한다.
- **Persistence and safety**: 해당 없음 (TOC 펼침 상태는 세션 내 휘발, 파일/세션/권한 변경 없음).
- **Documentation and Storybook**: AW 조립 상태(접힘/펼침/빈 상태)를 보여주는 Storybook 스토리 갱신, 관련 unit test 추가.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: h1~h3 heading이 있는 문서에서 사용자가 한 번의 조작(TOC 펼치기)으로 목차를 확인하고, 항목 클릭 시 100%의 경우에 해당 heading(동일 텍스트 중복 포함)으로 정확히 이동한다.
- **SC-002**: h4~h6 heading이 TOC에 나타나는 경우가 0건이다.
- **SC-003**: heading 없는 문서·로딩/오류/미선택 상태에서 TOC UI가 표시되는 경우가 0건이다.
- **SC-004**: TOC가 접힌 기본 상태에서 preview 본문 표시 영역이 TOC 도입 전 대비 실질적으로 동일하다 (본문 폭 감소 없음).
- **SC-005**: 파일 전환·자동 새로고침 후 TOC가 이전 문서의 heading을 표시하는 경우가 0건이다.
- **SC-006**: TOC 도입 전후로 기존 annotation(line/offset 앵커)·선택 하이라이트 관련 기존 테스트가 모두 통과하며 회귀가 0건이다.

## Assumptions

- TOC는 preview 영역 내 접이식 형태로 배치하고 **기본 접힘**을 원칙으로 한다 (이슈 구현 메모의 우선 검토안). 구체 배치(헤더 토글 + 오버레이/인라인 패널 등)는 설계 단계에서 preview 레이아웃(가변 폭, xl에서 annotation 열 병행)에 맞게 확정한다.
- TOC 펼침/접힘 상태는 저장하지 않는다 (파일 전환·재방문 시 기본 접힘으로 시작).
- 스크롤 이동은 부드러운(smooth) 스크롤을 기본으로 하되 모션 감소 설정을 존중한다 (공유 helper의 기존 동작).
- 현재 스크롤 위치 기반 현재 섹션 강조(active highlight)는 범위에 포함하지 않는다 (이슈 요구사항에 없음).
- agent run 대화 타임라인의 markdown 렌더(짧은 메시지 위주)는 범위에 포함하지 않는다. 대상은 workspace 패널의 markdown preview뿐이다.
- specs/009에서 공유 패키지에 구현된 TOC 추출·컴포넌트·스크롤 helper를 그대로 사용하며 파서와 본문 렌더링은 변경하지 않는다.
