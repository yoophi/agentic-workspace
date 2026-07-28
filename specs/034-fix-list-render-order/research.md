# Phase 0 Research: Markdown 목록 렌더링 순서 보존

## R1. 순서 결함의 근본 원인

- **Decision**: 결함은 렌더러(`MarkdownViewer.tsx`)의 렌더 순서 로직에 있으며 core
  파서에는 없다. 파서는 각 블록에 올바른 `order`, `parentId`, `sourceRange`를 부여한다.
- **Rationale**: 현재 렌더 루프는 최상위 `list-item`을 만나면 "문서상 첫 최상위 목록
  항목"일 때만 `renderList()`를 호출하고, `renderList()`는 `blocksByParent.get(undefined)`
  로 문서 전체의 최상위 항목을 모아 한 위치에서 그린다. 따라서 서로 떨어진 목록이 하나로
  병합되고, 첫 목록 이후의 비목록 블록보다 앞서 렌더된다. `parseMarkdownToBlocks`로
  얻은 블록 배열 자체는 원문 순서(`Apple`, `paragraph`, `Banana`)를 유지함을 재현으로
  확인했다.
- **Alternatives considered**: 파서에서 인접 목록을 하나의 list 블록으로 합치는 방안 —
  블록 모델(항목 단위 주석 앵커)과 충돌하고 순서 문제를 해결하지 못해 기각.

## R2. 렌더 순서 보존 전략과 런 경계 규칙

- **Decision**: 문서 순서(`blocks` 배열)를 단일 순회하며, 최상위 `list-item` 런(run)의
  시작 지점에서만 그 런을 제자리 렌더한다. **런 경계는 오직 비-`list-item` 타입 블록으로만
  정의한다.** 중첩 `list-item`(`parentId` 있음)은 런을 시작하지도 끊지도 않는다.
- **Rationale**: blocks 배열이 이미 원문 순서이므로 "런 시작에서 렌더" 규칙만으로 순서와
  인접 목록 병합이 정확해진다. O(n) 단일 순회이며 기존 `findIndex` 이중 탐색을 제거한다.
- **재검증으로 정정한 함정**: 초안의 "직전 블록이 최상위 목록 항목이 아니면 새 런"
  규칙은 틀렸다. 실제 파서 출력을 확인한 결과 `- A\n  - A1\n- B`는 다음과 같다.
  ```
  block-0 list-item parent=undefined  (A, 최상위)
  block-1 list-item parent=block-0    (A1, 중첩)
  block-2 list-item parent=undefined  (B, 최상위)
  ```
  A와 B 사이에 있는 것은 중첩 항목 A1뿐이라 비-`list-item` 블록이 없으므로 A·B는 **같은
  런**(하나의 `ul`)이어야 한다. 직전 블록(A1)이 최상위 항목이 아니라는 이유로 B를 새 런
  으로 보면 오분리된다. 또한 list item 내부의 blockquote/code/table/문단은 파서가 항목
  `content`로 병합하여 별도 블록으로 방출하지 않으므로(직접 확인) 최상위 항목 사이에
  끼는 별도 블록은 "중첩 항목(경계 아님)" 또는 "진짜 비-list 블록(경계)" 둘뿐이다.
- **Alternatives considered**: (a) 각 항목을 개별 `ul`/`ol`로 감싸 제자리 렌더 —
  인접 항목이 별개 목록이 되어 시맨틱·번호 매김이 깨짐. (b) 블록별 원문 offset 정렬 후
  재배치 — 이미 배열이 정렬돼 있어 불필요한 복잡성.

## R3. 런 내부 순서/비순서 그룹화

- **Decision**: 런 내부에서 `ordered` 값이 바뀌는 경계마다 별도 `MarkdownList`로 분리
  하고, 순서 목록은 `orderedStart`를 `start`로 적용한다(기존 그룹화 로직 재사용).
- **Rationale**: 033에서 확립한 시맨틱 계약(`ul`/`ol` 직접 자식은 항상 `li`)과 번호
  시작 표현을 유지하며, 런 단위로 스코프만 좁히면 된다.
- **Alternatives considered**: 런 전체를 항상 한 목록으로 — 비순서/순서 혼합 시 잘못된
  마크업을 생성해 기각.

## R4. 중첩 목록 처리

- **Decision**: 각 항목의 중첩 자식은 기존 `renderList(item.id)` 재귀를 그대로 사용한다.
- **Rationale**: 같은 부모(`parentId`)를 가진 자식 항목은 항상 연속이며 사이에 별도
  블록이 끼지 않으므로(문단 등은 항목 content로 병합됨) 최상위에서 발생한 순서 문제가
  중첩 수준에는 없다. 재사용으로 회귀 위험을 최소화한다.
- **Alternatives considered**: 중첩에도 런 감지 적용 — 불필요(자식은 이미 연속), 복잡성만
  증가.

## R5. 파생 표시(TaskSummary 등) 위치 보존

- **Decision**: 런 렌더는 각 항목의 원래 인덱스 순서를 보존하고, 인덱스 기반으로 삽입되는
  TaskSummary/preamble 표시가 올바른 위치에 남도록 한다.
- **Rationale**: 현재 TaskSummary는 `blocks.map`의 index/H1 챕터 기준으로 삽입된다.
  런을 제자리에 렌더하면 항목 인덱스 순서가 유지되어 파생 표시 위치도 자연히 정확해진다.
- **Alternatives considered**: 파생 표시 로직 재작성 — 범위 밖, 회귀 위험. 순서 보존만
  하면 기존 로직이 정상 동작.

## R6. 회귀 테스트 및 성능 검증

- **Decision**: `MarkdownViewer.test.tsx`에 (a) 문단으로 분리된 두 목록, (b) 제목으로
  구분된 목록, (c) 비순서→순서 전환, (d) 중첩 유지 케이스를 렌더 순서 단언으로 추가.
  기존 2,000블록 성능 예산 테스트를 목록 다수 케이스로도 유지한다.
- **Rationale**: spec의 SC-001~SC-004를 직접 검증한다. `react-dom/server`로 정적 렌더
  후 텍스트 출현 순서를 비교하는 기존 테스트 패턴을 재사용한다.
- **Alternatives considered**: DOM 통합 테스트(jsdom 상호작용) — 순서 검증에는 과함.
  정적 렌더 문자열 순서 비교로 충분.

## R7. React `key` 누락(재검증 중 발견한 부수 결함)

- **Decision**: 런 렌더 시 각 목록 항목에 `key={item.id}`를 부여한다.
- **Rationale**: 현재 `itemsInList.map((item) => renderBlock(item, ...))`는 key 없이
  요소 배열을 만들어 "Each child in a list should have a unique key" 경고가 발생하고,
  항목 추가/삭제 시 잘못된 재조정 위험이 있다(재현으로 확인). 순서 수정과 함께 정리한다.
- **Alternatives considered**: 경고 무시 — 재조정 안정성 저하로 기각.

## R8. 항목 내 다중 하위 목록 병합(범위 밖)

- **Decision**: 한 항목 안에서 문단으로 분리된 여러 하위 목록이 하나로 합쳐지고 문단이
  부모 content로 hoisting되는 현상은 본 수정의 범위 밖으로 둔다.
- **Rationale**: 이는 파서(033)의 항목 content 병합 방식에서 비롯되며, 최상위 목록 순서
  결함과 독립적이다. 실제 파서 출력으로 이 동작을 확인했다. 034는 최상위 순서 보존에
  집중하고, 이 한계는 spec의 범위 밖 항목으로 명시한다.
- **Alternatives considered**: 파서에서 하위 목록 분리 보존 — 블록 모델·주석 앵커 변경을
  수반해 범위·위험이 커지므로 별도 과제로 분리.

## 미해결 항목

- 없음. 모든 NEEDS CLARIFICATION 해소.
