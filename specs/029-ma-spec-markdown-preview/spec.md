# Feature Specification: MA Spec Markdown Preview

**Feature Branch**: `029`

**Created**: 2026-07-12

**Status**: Draft

**Input**: User description: "markdown preview 기능으로 spec 문서를 편리하게 표시하여 작업하기 원할하게 한다"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Spec 문서를 읽기 좋은 Preview로 표시 (Priority: P1)

사용자는 MA에서 spec Markdown 문서를 열고 원문 표식에 방해받지 않는 읽기 중심 Preview로 요구사항과 작업 맥락을 확인한다.

**Why this priority**: spec의 구조와 내용을 빠르게 파악하는 것이 이후 검토와 주석 작업의 출발점이며 기능의 핵심 가치다.

**Independent Test**: 제목, 문단, 목록, 표, 체크리스트, 인용문, 코드 블록을 포함한 spec 문서를 열어 모든 요소가 구분 가능한 읽기 형태로 표시되는지 확인할 수 있다.

**Acceptance Scenarios**:

1. **Given** 사용자가 유효한 spec Markdown 문서를 선택했을 때, **When** Preview가 열리면, **Then** 문서 제목과 본문 구조가 원래 순서대로 읽기 좋은 형태로 표시된다.
2. **Given** spec 문서에 표, 체크리스트, 코드 블록이 포함되어 있을 때, **When** 사용자가 Preview를 확인하면, **Then** 각 요소는 내용 손실 없이 서로 구분되어 표시된다.
3. **Given** Preview가 표시된 상태일 때, **When** 사용자가 스크롤하면, **Then** 긴 문서의 시작부터 끝까지 콘텐츠가 겹치거나 잘리지 않고 탐색된다.
4. **Given** 사용자가 다른 Markdown 문서를 열었을 때, **When** 로딩이 완료되면, **Then** 이전 문서 내용이 남지 않고 새 문서의 Preview가 표시된다.
5. **Given** 문서에 `- [ ]`, `- [x]`, `- [X]` task 항목이 있을 때, **When** Preview가 표시되면, **Then** 미완료와 완료 상태는 서로 다른 아이콘과 시각적 강조로 즉시 구분된다.
6. **Given** H1 chapter 안에 하나 이상의 task가 있을 때, **When** Preview가 표시되면, **Then** 해당 H1 바로 아래에서 그 chapter에 속한 완료 task 수와 미완료 task 수를 확인할 수 있다.
7. **Given** 여러 H1 chapter가 있는 문서에서 일부 chapter에만 task가 있을 때, **When** Preview가 표시되면, **Then** task가 있는 각 chapter만 자체 요약을 표시하고 다른 chapter의 task를 개수에 포함하지 않는다.
8. **Given** H1 chapter에 하나 이상의 task가 있을 때, **When** Table of Contents가 표시되면, **Then** 해당 H1 항목에서 완료 task 수와 미완료 task 수를 아이콘과 함께 확인할 수 있다.
9. **Given** 문서 본문에 `<!-- ... -->` 형식의 HTML5 주석이 있을 때, **When** Preview가 표시되면, **Then** 주석의 표식과 내용은 화면에 표시되지 않고 나머지 문서 흐름만 유지된다.

---

### User Story 2 - 문서 구조를 이용해 빠르게 이동 (Priority: P2)

사용자는 Preview에서 spec의 제목 구조와 현재 위치를 확인하고 원하는 섹션으로 바로 이동한다.

**Why this priority**: 긴 spec에서 요구사항, 사용자 시나리오, 성공 기준을 반복적으로 오갈 때 탐색 시간을 줄여 작업 흐름을 유지한다.

**Independent Test**: 여러 단계의 제목이 있는 문서를 열어 구조 목록에서 각 항목을 선택하고 대응하는 본문 위치로 이동하는지 확인할 수 있다.

**Acceptance Scenarios**:

1. **Given** 문서에 하나 이상의 제목이 있을 때, **When** Preview가 표시되면, **Then** 사용자는 제목 계층과 순서를 반영한 문서 구조 목록을 볼 수 있다.
2. **Given** 문서 구조 목록이 표시되어 있을 때, **When** 사용자가 제목 항목을 선택하면, **Then** Preview는 해당 제목 위치로 이동하고 목적 위치를 식별할 수 있게 한다.
3. **Given** 동일한 제목 문구가 여러 번 나타날 때, **When** 사용자가 각 항목을 선택하면, **Then** 각 항목은 문서 순서에 맞는 고유한 위치로 이동한다.
4. **Given** 문서에 제목이 없을 때, **When** Preview가 표시되면, **Then** 문서 구조 영역은 탐색을 방해하지 않는 빈 상태를 알리고 본문은 정상 표시한다.

---

### User Story 3 - Wikilink로 연결된 문서 이동 (Priority: P2)

사용자는 Preview에 표시된 wikilink를 선택하여 현재 문서와 같은 디렉터리의 연결 문서로 이동하고, 별칭이 있는 링크는 이해하기 쉬운 문구로 확인한다.

**Why this priority**: spec, plan, tasks와 관련 설계 문서가 서로 연결되어 있을 때 파일을 다시 찾지 않고 문맥을 이어서 검토할 수 있다.

**Independent Test**: `[[link]]`와 `[[link | 링크]]`를 포함한 문서를 열고 각 링크의 표시 문구와 클릭 후 `./link.md` 이동 결과를 확인할 수 있다.

**Acceptance Scenarios**:

1. **Given** 현재 문서에 `[[link]]`가 있을 때, **When** Preview가 표시되면, **Then** 해당 문구는 클릭 가능한 `link`로 표시되고 대상은 현재 문서 기준 `./link.md`가 된다.
2. **Given** 현재 문서에 `[[link | 링크]]`가 있을 때, **When** Preview가 표시되면, **Then** 해당 문구는 `링크`로 표시되고 대상은 현재 문서 기준 `./link.md`가 된다.
3. **Given** 유효한 wikilink가 표시되어 있을 때, **When** 사용자가 링크를 선택하면, **Then** MA는 대상 Markdown 문서를 현재 Preview에 열고 문서 정보와 구조 목록을 대상 기준으로 갱신한다.
4. **Given** 사용자가 wikilink로 이동한 문서에 또 다른 wikilink가 있을 때, **When** 연속해서 링크를 선택하면, **Then** 각 대상은 현재 열린 문서 위치를 기준으로 해석되어 문서 간 이동이 이어진다.
5. **Given** wikilink 대상 문서가 없거나 읽을 수 없을 때, **When** 사용자가 링크를 선택하면, **Then** 현재 문서는 유지되고 대상 이동 실패와 원인을 확인할 수 있다.

---

### User Story 4 - 변경되는 Spec을 안정적으로 검토 (Priority: P3)

사용자는 외부 도구에서 spec이 수정되어도 최신 내용을 다시 확인하고, 지원하기 어려운 요소가 있어도 나머지 문서를 계속 읽는다.

**Why this priority**: spec은 작성 과정에서 자주 바뀌므로 오래된 내용을 기준으로 작업하는 실수를 줄이고 예외 콘텐츠 때문에 검토 전체가 중단되지 않게 한다.

**Independent Test**: 열린 spec 파일을 외부에서 수정하고, 잘못된 다이어그램과 매우 긴 행을 포함시켜 최신 본문과 안전한 대체 표시가 유지되는지 확인할 수 있다.

**Acceptance Scenarios**:

1. **Given** Preview로 열린 spec 파일이 외부에서 변경되었을 때, **When** 변경이 감지되면, **Then** 사용자는 문서가 변경되었음을 알고 최신 내용으로 갱신할 수 있다.
2. **Given** 문서에 유효한 다이어그램 정의가 있을 때, **When** Preview가 표시되면, **Then** 사용자는 본문 흐름 안에서 다이어그램을 확인하고 필요하면 더 크게 볼 수 있다.
3. **Given** 문서에 표시할 수 없는 다이어그램이나 지원되지 않는 요소가 있을 때, **When** Preview가 처리하면, **Then** 오류가 해당 요소에 한정되어 안내되고 나머지 문서는 계속 읽을 수 있다.
4. **Given** 읽을 수 없거나 사라진 파일을 열려고 할 때, **When** 로딩에 실패하면, **Then** 사용자는 원인을 이해할 수 있는 오류와 다시 파일을 선택할 수 있는 경로를 확인한다.

---

### User Story 5 - 다양한 SpecKit 산출물 예제로 Preview 확인 (Priority: P3)

사용자는 로컬 파일을 준비하지 않고도 MA의 예제 목록에서 대표적인 SpecKit 산출물을 선택하여 문서 구조와 Preview 기능을 확인한다.

**Why this priority**: 서로 다른 SpecKit 문서의 구조와 연결 관계를 즉시 체험할 수 있어 MA의 Preview 기능을 학습하고 회귀 상태를 점검하는 시간을 줄인다.

**Independent Test**: 예제 목록에서 feature specification, implementation plan, data model, tasks, requirements checklist를 각각 열어 문서별 콘텐츠와 Preview 요소가 정상 표시되는지 확인할 수 있다.

**Acceptance Scenarios**:

1. **Given** 사용자가 MA를 열었을 때, **When** 예제 목록을 펼치면, **Then** feature specification, implementation plan, data model, tasks, requirements checklist 예제를 서로 구분되는 이름으로 확인할 수 있다.
2. **Given** 사용자가 SpecKit 예제 하나를 선택했을 때, **When** 문서를 불러오면, **Then** 선택한 산출물의 파일 이름, 본문과 Table of Contents가 표시된다.
3. **Given** tasks 또는 requirements checklist 예제를 선택했을 때, **When** Preview와 Table of Contents를 확인하면, **Then** 완료·미완료 task 상태와 chapter별 집계가 예제 원문과 일치한다.
4. **Given** SpecKit 예제에 다른 산출물을 가리키는 wikilink가 있을 때, **When** 사용자가 해당 링크를 선택하면, **Then** 연결된 예제 문서로 이동하여 산출물 간 관계를 이어서 확인할 수 있다.
5. **Given** plan 또는 data model 예제에 표, 코드 블록이나 Mermaid 다이어그램이 있을 때, **When** Preview가 표시되면, **Then** 각 요소가 내용 손실 없이 읽기 좋은 형태로 표시된다.

---

### User Story 6 - AW에서도 동일한 Markdown Preview 사용 (Priority: P2)

사용자는 AW의 Markdown Preview에서도 MA에 반영된 공용 Markdown 표시와 탐색 기능을 동일한 방식으로 사용한다.

**Why this priority**: 같은 문서를 MA와 AW에서 번갈아 검토할 때 task, 문서 구조와 특수 문법의 표시 결과가 달라지면 요구사항을 오해하거나 작업 상태를 잘못 판단할 수 있다.

**Independent Test**: 동일한 기준 Markdown 문서를 MA와 AW의 Preview에 각각 표시하여 공용 지원 요소의 본문, Table of Contents와 task 집계 결과가 일치하는지 확인할 수 있다.

**Acceptance Scenarios**:

1. **Given** 동일한 Markdown 문서가 MA와 AW에 열렸을 때, **When** 두 Preview를 비교하면, **Then** 제목, GFM 요소, task 상태, H1 chapter 요약과 HTML5 주석 처리 결과가 일치한다.
2. **Given** task가 있는 H1 chapter가 AW에 표시될 때, **When** 사용자가 본문과 Table of Contents를 확인하면, **Then** 완료·미완료 개수가 MA와 동일한 범위와 상태별 아이콘으로 표시된다.
3. **Given** 문서에 wikilink가 있을 때, **When** AW Preview가 표시되면, **Then** MA와 동일한 표시 문구와 상대 Markdown 대상이 제공되며 AW의 문서 탐색 경계 안에서 활성화된다.
4. **Given** 공용 Markdown Preview 동작이 변경되었을 때, **When** 기능 검증을 수행하면, **Then** MA와 AW 소비 화면의 회귀 검증이 모두 통과해야 한다.
5. **Given** 사용자가 AW의 Speckit Preview panel에서 문서를 열었을 때, **When** 본문 블록 또는 선택 영역에 annotation을 추가하면, **Then** 일반 Markdown Preview와 동일하게 annotation을 확인하고 편집하거나 삭제할 수 있다.
6. **Given** AW의 Speckit Preview panel에 제목이 있는 문서가 열렸을 때, **When** Table of Contents 항목을 선택하면, **Then** 해당 제목 위치로 이동하고 task가 있는 H1 항목에는 완료·미완료 개수가 표시된다.

### Edge Cases

- 비어 있는 spec 파일은 빈 문서임을 알리되 앱 전체 오류로 처리하지 않는다.
- 제목이 매우 길거나 계층이 깊어도 구조 목록과 본문 너비를 침범하지 않는다.
- 넓은 표와 공백 없는 긴 문자열은 문서 전체 레이아웃을 밀어내지 않고 해당 영역에서 확인할 수 있다.
- 완료 및 미완료 체크리스트 항목은 상태가 서로 구분되지만 Preview에서 실수로 변경되지 않는다.
- HTML 조각이나 외부 링크가 포함되어도 앱의 로컬 데이터나 시스템 기능에 임의로 접근하지 않는다.
- 문서가 빠르게 연속 변경되면 최종 파일 내용과 일치하는 Preview만 표시된다.
- Preview 도중 파일이 삭제되거나 접근 권한이 사라지면 마지막으로 확인한 내용을 새 문서처럼 오인하지 않도록 상태를 알린다.
- `[[target]]`, `[[target | label]]` 형식이 완성되지 않았거나 대상 이름이 비어 있으면 일반 텍스트로 안전하게 표시하고 임의의 파일을 열지 않는다.
- wikilink 대상에 공백이나 한글이 포함되어도 표시 문구와 대상 파일 이름을 손상하지 않는다.
- wikilink가 현재 문서 자신을 가리키거나 문서들이 순환 연결되어도 앱이 반복해서 자동 이동하지 않으며 사용자가 선택한 이동만 수행한다.
- wikilink가 현재 문서 디렉터리 밖을 가리키려 하면 허용된 문서 접근 범위를 벗어나지 않도록 처리한다.
- 예제 문서를 연 상태에서는 로컬 파일로 오인하여 저장, 변경 감지 또는 파일 오류 동작을 적용하지 않는다.
- SpecKit 예제 간 wikilink 대상이 예제 목록에 없으면 현재 예제를 유지하고 이동 실패를 알린다.
- 한 줄 또는 여러 줄의 `<!-- ... -->` HTML5 주석은 Preview에서 숨기되, fenced code block이나 inline code 안의 동일한 문자열은 코드 내용으로 보존한다.
- 닫히지 않은 HTML5 주석은 이후 문서 전체를 임의로 숨기지 않도록 안전한 일반 텍스트 또는 명확한 대체 상태로 처리한다.
- MA와 AW가 서로 다른 앱 상태나 문서 열기 방식을 사용하더라도 공유 Markdown 문법의 표시, task 집계와 TOC 결과는 일치해야 한다.
- MA 전용 예제 선택과 Tauri window 동작은 AW에 복제하지 않지만, AW Speckit Preview panel의 annotation은 AW 일반 Markdown Preview가 사용하는 기존 annotation 동작과 상태 모델을 재사용해야 한다.
- AW Speckit Preview panel에서 문서를 전환할 때 annotation과 TOC 선택 대상은 현재 Speckit 문서 기준으로 교체되어 다른 문서의 상태와 섞이지 않아야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: MA는 사용자가 연 spec Markdown 문서를 읽기 중심의 Preview로 표시해야 한다.
- **FR-002**: Preview는 제목, 문단, 강조, 링크, 순서 및 비순서 목록, 인용문, 표, 체크리스트, 인라인 코드와 코드 블록을 내용 순서와 의미를 유지하여 표시해야 한다.
- **FR-003**: Preview는 긴 문서에서도 본문이 지정된 작업 영역 안에서 독립적으로 스크롤되도록 해야 한다.
- **FR-004**: Preview는 현재 문서의 파일 이름과 경로를 식별할 수 있게 표시해야 한다.
- **FR-005**: MA는 문서 제목에서 계층적 구조 목록을 만들고 본문 순서와 일치하게 표시해야 한다.
- **FR-006**: 사용자는 구조 목록의 제목을 선택하여 대응하는 Preview 위치로 이동할 수 있어야 한다.
- **FR-007**: 동일하거나 특수문자가 포함된 제목도 각각 고유하고 안정적인 이동 대상으로 처리해야 한다.
- **FR-008**: Preview는 `- [ ]`를 미완료 task로, `- [x]`와 `- [X]`를 완료 task로 인식하고 각 상태를 서로 다른 아이콘 및 시각적 강조와 함께 표시하되 읽기 모드에서 상태 변경을 허용하지 않아야 한다.
- **FR-009**: Preview는 유효한 다이어그램을 본문에 표시하고 확대하여 확인할 수 있는 수단을 제공해야 한다.
- **FR-010**: 특정 다이어그램 또는 문서 요소의 표시가 실패해도 오류를 해당 요소에 한정하고 나머지 Preview를 유지해야 한다.
- **FR-011**: 열린 파일의 외부 변경이 감지되면 MA는 변경 상태를 알리고 최신 내용으로 갱신할 수 있게 해야 한다.
- **FR-012**: 새로운 문서를 열면 MA는 문서 내용, 구조 목록, 현재 위치와 오류 상태를 새 문서 기준으로 교체해야 한다.
- **FR-013**: 파일 읽기 실패, 파일 삭제, 접근 권한 상실 시 MA는 원인을 이해할 수 있는 오류와 복구 행동을 제공해야 한다.
- **FR-014**: Preview의 링크는 목적지를 식별할 수 있어야 하며, 사용자의 명시적 동작 없이 외부 위치를 열거나 실행하지 않아야 한다.
- **FR-015**: Preview는 사용자가 키보드만으로 본문, 구조 목록, 링크와 다이어그램 확대 조작에 접근할 수 있게 해야 한다.
- **FR-016**: Preview는 읽기 기능에 집중하며 Markdown 원문 편집, spec 생성, 요구사항 상태의 영구 변경을 포함하지 않아야 한다.
- **FR-017**: Preview는 `[[target]]` 형식을 표시 문구 `target`, 대상 `./target.md`인 클릭 가능한 문서 링크로 해석해야 한다.
- **FR-018**: Preview는 `[[target | label]]` 형식을 표시 문구 `label`, 대상 `./target.md`인 클릭 가능한 문서 링크로 해석해야 하며 구분자 주변 공백은 대상과 표시 문구에서 제거해야 한다.
- **FR-019**: wikilink 대상은 현재 열린 문서의 디렉터리를 기준으로 해석해야 하며, 사용자가 링크를 선택하면 대상 문서를 현재 Preview에 표시해야 한다.
- **FR-020**: wikilink로 문서를 이동하면 문서 정보, 본문, 구조 목록, 현재 위치, 변경 감지 대상과 오류 상태를 새 문서 기준으로 교체해야 한다.
- **FR-021**: 시스템은 비어 있거나 완성되지 않은 wikilink를 이동 가능한 링크로 만들지 않아야 한다.
- **FR-022**: wikilink 대상 파일이 없거나 읽을 수 없으면 현재 문서를 유지하고 이동 실패 원인과 대상 경로를 알려야 한다.
- **FR-023**: wikilink 이동은 사용자의 명시적인 클릭 또는 키보드 활성화에 의해서만 발생해야 하며 문서를 표시하는 것만으로 자동 이동하지 않아야 한다.
- **FR-024**: wikilink 대상 해석은 허용된 로컬 문서 접근 범위를 벗어나는 임의 경로 이동을 차단해야 한다.
- **FR-025**: 완료 task의 본문은 완료 상태를 나타내는 낮은 강조도와 취소선으로 표시하고, 미완료 task의 본문은 남은 작업임을 식별할 수 있는 강조를 제공해야 한다.
- **FR-026**: task 상태 표시는 라이트 및 다크 화면 모두에서 읽을 수 있어야 하며 색상만으로 완료 여부를 전달하지 않아야 한다.
- **FR-027**: 일반 bullet 목록과 코드 블록 내부의 task 형식 텍스트는 task 상태 스타일의 영향을 받지 않아야 한다.
- **FR-028**: 긴 task 본문, 중첩 task, 링크와 인라인 코드가 포함된 task에서도 아이콘과 본문 정렬 및 문서 레이아웃이 유지되어야 한다.
- **FR-029**: H1 chapter에 하나 이상의 task가 있으면 Preview는 해당 chapter의 완료 task 개수와 미완료 task 개수를 아이콘 및 텍스트와 함께 표시해야 한다.
- **FR-030**: chapter task 요약은 해당 H1 제목 바로 아래에 표시하고, 각 H1의 범위는 그 제목부터 다음 H1 직전까지로 정의해야 한다.
- **FR-031**: task가 없는 H1 chapter에는 task 요약을 표시하지 않아야 하며, H2 이하 하위 섹션의 task는 가장 가까운 선행 H1 chapter의 개수에 포함해야 한다.
- **FR-032**: H1이 없는 문서 또는 첫 H1 이전에 있는 task는 문서 서문 영역의 task로 집계하여 문서 본문 최상단에 요약해야 한다.
- **FR-033**: 중첩 수준과 관계없이 각 task는 자신이 속한 chapter 요약 개수에 정확히 한 번 포함되어야 한다.
- **FR-034**: Table of Contents는 task가 포함된 H1 항목에 해당 chapter의 완료 task 개수와 미완료 task 개수를 상태별 아이콘과 함께 표시하고, task가 없는 항목은 기존 제목 표시를 유지해야 한다.
- **FR-035**: MA의 예제 목록은 feature specification, implementation plan, data model, tasks, requirements checklist를 대표하는 SpecKit Markdown 예제를 제공해야 한다.
- **FR-036**: 각 SpecKit 예제는 산출물 유형을 구분할 수 있는 제목, 파일 이름과 설명을 제공하고 선택 시 해당 문서의 Preview와 Table of Contents를 표시해야 한다.
- **FR-037**: SpecKit 예제 모음은 wikilink, task와 chapter 요약, 표, 코드 블록, Mermaid 다이어그램을 포함하여 관련 Preview 동작을 확인할 수 있어야 한다.
- **FR-038**: 서로 관련된 SpecKit 예제 문서는 상대 wikilink로 연결되어야 하며, 사용자는 링크를 선택하여 예제 산출물 사이를 이동할 수 있어야 한다.
- **FR-039**: Preview는 한 줄 또는 여러 줄의 `<!-- ... -->` HTML5 주석 표식과 그 내부 내용을 화면에 표시하지 않아야 하며, 주석 제거로 생긴 불필요한 빈 콘텐츠가 문서 흐름을 방해하지 않도록 해야 한다.
- **FR-040**: fenced code block과 inline code 내부의 `<!-- ... -->` 문자열은 HTML5 주석으로 제거하지 않고 원본 코드 내용으로 표시해야 한다.
- **FR-041**: MA에 반영된 공용 Markdown Preview의 parsing, wikilink 표시, task 상태, H1 chapter 요약, Table of Contents task 정보와 HTML5 주석 처리 변경사항은 AW의 Markdown Preview에도 적용되어야 한다.
- **FR-042**: MA와 AW는 공용 Markdown 입력에 대해 동일한 block 순서, 표시 콘텐츠와 task 집계 결과를 사용해야 하며 앱별 문서 탐색 및 shell 동작만 각 앱 경계에서 조합해야 한다.
- **FR-043**: 공용 Markdown Preview package를 변경하면 MA와 AW의 관련 type check와 test를 모두 수행해야 하며 어느 한 소비 앱의 회귀가 남은 상태를 완료로 처리하지 않아야 한다.
- **FR-044**: AW의 Speckit Preview panel은 본문 블록 및 선택 영역 annotation 추가, annotation 유형과 comment 편집, 삭제와 agent prompt 전달을 AW 일반 Markdown Preview와 동일한 동작으로 지원해야 한다.
- **FR-045**: AW의 Speckit Preview panel은 H1~H3 Table of Contents를 표시하고 항목 선택 시 대응하는 본문 위치로 이동해야 하며, task가 포함된 H1에는 완료·미완료 개수를 표시해야 한다.
- **FR-046**: AW Speckit Preview의 annotation은 Speckit 문서 경로별로 분리하고, 문서 선택이 변경되면 Preview, annotation 목록, 선택 상태, Table of Contents와 agent prompt를 선택 문서 기준으로 갱신해야 한다.

### Key Entities

- **Spec 문서**: 사용자가 연 Markdown 파일로, 파일 이름, 절대 경로, 원문 내용과 변경 상태를 가진다.
- **Preview 콘텐츠**: Spec 문서를 읽기 형태로 표시한 결과로, 문서 요소의 순서, 상태와 오류 대체 표시를 포함한다.
- **문서 구조 항목**: 제목의 표시 문구, 계층 수준, 문서 순서와 이동 대상을 나타낸다.
- **Preview 세션**: 현재 문서, 현재 탐색 위치, 갱신 필요 여부와 표시 오류 상태를 묶어 나타낸다.
- **Wikilink**: 원문 대상 이름, 선택적 표시 별칭, 현재 문서 기준으로 해석된 Markdown 파일 경로와 이동 가능 상태를 나타낸다.
- **예제 문서**: MA에 내장된 읽기 전용 Markdown 문서로, 고유 식별자, 파일 이름, 제목, 설명, 산출물 유형과 본문을 가진다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: MA 앱의 문서 Preview 경험이 범위이며, 기존 Markdown 해석 및 표시 역량을 공유 모듈에서 재사용한다. 다른 앱 전용 코드를 직접 가져오지 않는다.
- **Frontend layering**: 화면 조립은 pages, Preview 전환과 탐색 행동은 features, 문서 및 구조 모델은 entities, 범용 표시 어댑터는 shared에 둔다. 생성된 UI 기본 요소는 components/ui에 유지한다.
- **Backend boundary**: 로컬 spec 읽기와 변경 감지는 기존 문서 도메인 및 애플리케이션 서비스가 담당하고, 앱 명령은 해당 서비스에 위임하며 파일 접근 세부사항은 인프라 경계에 둔다.
- **Shared core vs UI**: Markdown 파싱, 제목 식별과 공통 표시 규칙은 기존 공유 core 및 React 모듈을 우선 사용하며, MA 고유 Preview 화면 조립만 앱 내부에 둔다.
- **Persistence and safety**: 사용자가 선택한 파일과 명시적으로 활성화한 wikilink 대상만 읽고, 기준 디렉터리와 허용 범위 검증, 파일 여부, 허용 형식, 크기와 문자 인코딩 오류를 경계에서 처리한다. Preview 상태는 영구 저장하지 않는다.
- **Documentation and Storybook**: Preview의 기본, 긴 spec, 제목 없음, 넓은 표, 체크리스트, 다이어그램 오류, 파일 오류와 대표 SpecKit 산출물 상태를 예제 또는 Storybook에서 확인한다. 사용자 실행 흐름이 바뀌면 영어 파일명의 한국어 문서를 갱신한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 대표 spec 문서를 처음 연 사용자의 95% 이상이 5초 안에 문서 제목과 첫 번째 요구사항 섹션을 식별한다.
- **SC-002**: 사용자의 95% 이상이 구조 목록을 이용해 목표 섹션으로 10초 안에 이동한다.
- **SC-003**: 지원 대상 Markdown 요소를 포함한 기준 문서 100%에서 내용 순서와 텍스트가 누락 없이 표시된다.
- **SC-004**: 2,000개 문서 요소 또는 1MB 크기 이하의 기준 spec에서 Preview가 2초 안에 읽을 수 있는 상태로 표시된다.
- **SC-005**: 외부 파일 변경 후 사용자가 최신 내용을 확인할 수 있는 상태가 2초 안에 제공된다.
- **SC-006**: 잘못된 다이어그램, 넓은 표, 긴 제목, 빈 파일과 읽기 실패를 포함한 경계 사례 100%에서 앱 전체가 중단되지 않고 본문 또는 복구 안내가 유지된다.
- **SC-007**: 키보드 기반 검증 시 사용자의 100%가 구조 이동과 모든 Preview 대화형 요소의 조작을 완료한다.
- **SC-008**: 기준 문서의 유효한 `[[target]]` 및 `[[target | label]]` 사례 100%가 올바른 표시 문구와 현재 문서 기준 `./target.md` 대상으로 변환된다.
- **SC-009**: 사용자의 95% 이상이 wikilink를 이용해 연결 문서로 5초 안에 이동하고 대상 문서 제목을 식별한다.
- **SC-010**: 누락 대상, 잘못된 형식, 자기 참조, 순환 연결과 범위 밖 경로를 포함한 wikilink 경계 사례 100%에서 자동 이동이나 앱 중단 없이 현재 문서 또는 명확한 오류 상태가 유지된다.
- **SC-011**: `- [ ]`, `- [x]`, `- [X]` 기준 사례 100%에서 상태에 맞는 아이콘과 스타일이 표시되고 완료·미완료 상태를 색상 없이도 구분할 수 있다.
- **SC-012**: 일반 bullet, 코드 블록 문구, 긴 본문, 3단계 중첩, 링크와 인라인 코드를 포함한 task 표시 검증 사례 100%에서 콘텐츠 손실이나 레이아웃 깨짐이 없다.
- **SC-013**: task가 포함된 기준 문서 100%에서 각 H1 chapter의 완료·미완료 요약 개수가 해당 chapter의 실제 task 상태와 일치하고, task가 없는 chapter에는 요약이 표시되지 않는다.
- **SC-014**: H1 없음, 첫 H1 이전 task, H2 이하 task와 여러 H1 chapter를 포함한 위치 검증 사례 100%에서 각 요약이 요구된 위치와 범위로 표시된다.
- **SC-015**: task가 포함된 H1 chapter 기준 사례 100%에서 Table of Contents의 완료·미완료 개수가 본문 chapter 요약과 일치하며, task가 없는 TOC 항목에는 task 정보가 표시되지 않는다.
- **SC-016**: 제공되는 SpecKit 예제 5종 모두 예제 목록에서 선택 가능하고, 각 문서의 Preview, Table of Contents와 포함된 wikilink, task, 표, 코드 또는 다이어그램이 콘텐츠 손실이나 앱 중단 없이 표시된다.
- **SC-017**: 한 줄·여러 줄 HTML5 주석과 code 내부의 주석 유사 문자열을 포함한 기준 사례 100%에서 실제 주석은 화면에 표시되지 않고 code 내용과 주석 밖의 본문은 손실 없이 표시된다.
- **SC-018**: 동일한 기준 Markdown fixture를 MA와 AW에 적용한 교차 앱 검증 사례 100%에서 본문 요소, wikilink 출력, task 상태, H1 chapter 및 TOC 집계, HTML5 주석 처리 결과가 일치한다.
- **SC-019**: 공용 Markdown Preview 변경에 대한 release 검증에서 MA와 AW의 관련 type check 및 test 명령이 모두 성공한다.
- **SC-020**: AW Speckit Preview 기준 annotation 검증 사례 100%에서 block 및 선택 영역 annotation의 생성·편집·삭제와 agent prompt 반영이 일반 Markdown Preview와 동일하게 동작하고 다른 Speckit 문서의 annotation과 섞이지 않는다.
- **SC-021**: H1~H3, 중복 제목과 task가 포함된 AW Speckit 문서 기준 사례 100%에서 Table of Contents의 순서, 이동 대상과 H1 task 개수가 본문과 일치한다.

## Assumptions

- 대상 spec은 로컬에 저장된 UTF-8 Markdown 파일이며 사용자가 파일 열기 또는 기존 MA 실행 경로를 통해 선택한다.
- Preview는 MA의 기존 주석 작업 흐름과 함께 제공되지만, 이번 기능은 읽기와 탐색 경험 개선을 우선한다.
- 기존 Markdown 표시 기능이 지원하는 표, 체크리스트, 코드와 다이어그램 역량을 확장하거나 조합해 사용한다.
- 외부 변경은 자동으로 감지하되 사용자의 읽기 위치를 불필요하게 잃지 않는 방식으로 최신 내용 확인을 제공한다.
- 확장자가 생략된 wikilink 대상에는 `.md`를 붙이며, 기본 대상은 현재 문서와 같은 디렉터리의 파일이다.
- wikilink 별칭은 화면 표시와 접근 가능한 링크 이름에 사용하고 대상 파일 해석에는 영향을 주지 않는다.
- 원격 URL의 Markdown 가져오기, 여러 spec 비교, 원문 편집과 SpecKit 명령 실행은 이번 범위에 포함하지 않는다.
- SpecKit 예제는 기능 학습과 표시 검증을 위한 읽기 전용 샘플이며 실제 프로젝트 산출물을 생성하거나 수정하지 않는다.
- AW는 MA의 예제 catalog나 Tauri window workflow를 복제할 필요가 없다. Speckit Preview annotation은 AW 일반 Markdown Preview의 기존 annotation 상태와 상호작용을 재사용하고, Markdown 표시와 TOC는 `markdown-annotation-core`와 `markdown-annotation-react`의 공용 동작을 소비한다.
