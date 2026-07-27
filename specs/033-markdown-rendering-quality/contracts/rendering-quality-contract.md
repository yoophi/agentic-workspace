# 렌더링 및 주석 UI 계약

## 입력 계약

`MarkdownViewer`는 원문 Markdown과 AST에서 유도한 `RenderedDocumentBlock[]`을 같은
문서 버전으로 받아야 한다. 호출자는 블록 배열을 임의로 재정렬하거나 각 block의
`content`를 별도 Markdown 문서처럼 렌더링해서는 안 된다.

## 출력 계약

| 입력 의미 | 필수 HTML 관계 | 주석 대상 |
|---|---|---|
| 비순서 목록 | `ul > li` | `li` 또는 그 안의 의미 블록 |
| 순서 목록 | `ol[start] > li` | `li` 또는 그 안의 의미 블록 |
| 여러 문단 목록 항목 | `li > p + p` | 각 문단 및 목록 항목 |
| 중첩 목록 | `li > ul/ol` | 중첩 `li`와 부모 `li` |
| fenced code | `pre > code` | `pre`/코드 블록 |
| 표 | `table > thead/tbody` | 표 블록 |

각 주석 가능 요소는 `data-block-id`, `data-block-type`, source line 범위를 제공한다.
이 속성은 실제 의미론적 요소에 부착하며 `ul`, `ol`, `table`의 직접 자식 규칙을
깨는 wrapper를 삽입하지 않는다.

## 주석 행동 계약

- 부분 선택 annotation은 해당 `blockId`의 렌더 텍스트 offset을 사용한다.
- block annotation은 선택한 의미 블록 하나에만 적용한다.
- edit/cancel은 지정한 annotation id만 변경한다.
- 문서 갱신으로 앵커 검증에 실패하면 stale을 보고하며, 인접 또는 같은 텍스트의
  다른 블록으로 자동 이동하지 않는다.

## 안전 계약

- raw HTML은 실행 가능한 DOM으로 승격하지 않는다.
- `javascript:` 등 활성 URL은 새로 활성화하지 않는다.
- 문법 오류와 지원하지 않는 확장은 미리보기를 중단하지 않고 안전한 평문/기본
  렌더링으로 남긴다.

## AW 주석 영역 전환 계약

| 상태 | 본문 열 | 보조 열 | 전환 제어 |
|---|---|---|---|
| 표시 | 본문과 보조 열이 함께 배치됨 | 주석 목록, agent prompt, TOC를 표시 | `주석 영역 숨기기` |
| 숨김 | 사용 가능한 가로 폭을 사용 | 렌더링하지 않음 | `주석 영역 보이기` |

- 전환 제어는 버튼으로 키보드 접근이 가능하고, 현재 상태 및 실행할 동작을 이름으로
  제공한다.
- 숨김 상태에서도 MarkdownViewer의 block action과 선택 annotation 흐름은 사용할 수
  있다.
- 표시 상태를 바꿔도 열린 파일, annotation 수와 내용, selection highlight,
  draft/편집 상태 및 전송할 annotation prompt는 변하지 않는다.
