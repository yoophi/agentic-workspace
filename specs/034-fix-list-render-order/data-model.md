# Phase 1 Data Model: Markdown 목록 렌더링 순서 보존

이 수정은 새 데이터 구조를 도입하지 않는다. 기존 core 블록 모델을 렌더 순서 판단에
사용할 뿐이며, 여기서는 관련 필드와 렌더 시 유도하는 개념(런)만 정리한다.

## 기존 엔티티: MarkdownBlock (재사용, 변경 없음)

`packages/markdown-annotation-core/src/types/markdown-block.ts`

| 필드 | 의미 | 순서 보존에서의 역할 |
|---|---|---|
| `id` | 블록 식별자(`block-N`) | 렌더 key, 주석 앵커 대상 |
| `type` | 블록 종류 | `list-item` 여부로 런 감지 |
| `order` | 문서 내 순서(0부터) | blocks 배열 순서 = 원문 순서의 근거 |
| `parentId` | 상위 목록 항목 id | `undefined`면 최상위 항목(런 대상) |
| `ordered` | 순서 목록 여부 | 런 내부 목록 종류 경계 분리 |
| `orderedStart` | 순서 목록 시작 번호 | `ol[start]` 렌더 |
| `level` | 중첩 깊이 | 중첩 표현(기존 로직) |
| `checked` | 작업 항목 완료 여부 | 작업 항목 렌더(기존) |
| `sourceRange` | 원문 offset/열 범위 | 주석 source 속성(`data-source-*`) 유지용 — **이 수정의 순서 판단에는 사용하지 않음** |

**규칙**: 렌더러는 `blocks` **배열 위치**를 원문 순서의 기준으로 삼는다(`order`는 배열
인덱스와 동일). 블록을 재정렬하거나 `order`를 재계산하지 않으며, 순서 판단에 `sourceRange`
를 쓰지 않는다.

## 유도 개념: 최상위 목록 런(Top-level List Run)

렌더 시점에만 존재하는 임시 개념(영속화하지 않음).

- **정의**: `blocks` 배열을 문서 순서로 순회할 때, 최상위 `list-item`
  (`parentId === undefined`)들이 **비-`list-item` 타입 블록에 끊기지 않고** 이어지는 최대
  구간. 런 사이에 나타나는 **중첩 `list-item`(`parentId` 있음)은 경계가 아니다.**
- **경계 규칙(정확)**:
  - 비-`list-item` 블록(heading/paragraph/code/table/blockquote/hr) → 런을 닫는다(**유일한
    경계**).
  - 중첩 `list-item` → 런을 시작하지도 끊지도 않는다(부모 재귀로 렌더).
  - 예: `- A\n  - A1\n- B` → `[A, (A1 중첩), B]`에서 A·B는 사이에 비-list 블록이 없으므로
    **하나의 런**이다. 반면 `- A\n\nPara\n\n- B`는 Para(문단)가 경계라 **두 런**이다.
- **불변식**:
  - 서로 다른 런은 사이에 최소 1개의 비-`list-item` 블록으로 분리된다.
  - 각 런은 자신의 첫 항목의 원문 위치(index)에서 렌더된다. 런의 나머지 항목과 중첩
    항목은 해당 index에서 `null`을 렌더한다.
  - 런 내부에서 `ordered` 값이 바뀌면 별도의 `ul`/`ol`로 분리된다.
  - 각 렌더된 항목은 `key`로 `MarkdownBlock.id`를 사용한다.
- **파서 전제(직접 확인)**: list item 내부의 문단/blockquote/code/table은 항목 `content`
  로 병합되어 별도 블록으로 방출되지 않는다. 따라서 같은 부모의 중첩 형제 항목은 항상
  연속이며, 순서 결함은 최상위에서만 발생한다.
- **상태 전이**: 없음(순수 파생값). 문서가 바뀌면 매 렌더에서 재계산된다.

## 관계

```text
MarkdownBlock[] (원문 순서, core 산출)
   └─ 렌더러가 순회하며
        ├─ 비-list-item 블록 → 제자리 렌더(런 경계)
        └─ 최상위 list-item  → 런 단위로 묶어 첫 항목 위치에서 렌더
                                └─ 런 내부: ordered 경계로 ul/ol 분리
                                     └─ 각 항목: 중첩 자식은 parentId 재귀 렌더
```

## 주석 앵커 불변식(회귀 방지)

- 순서 보존 이후에도 각 주석의 `anchor.blockId`는 동일한 `MarkdownBlock.id`에 연결된다.
- 존재하지 않는 블록을 가리키는 stale 앵커는 렌더에서 제외된다(기존
  `buildViewerAnnotationMaps` 동작 유지). 순서 변경이 앵커 매핑에 영향을 주지 않는다.
