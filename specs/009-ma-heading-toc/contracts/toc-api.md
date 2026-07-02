# Public API Contract: TOC (core + react)

**Feature**: 009-ma-heading-toc | **Date**: 2026-07-02

패키지가 외부(소비 앱)에 노출하는 계약. 여기 명시된 시그니처와 동작이 구현·테스트의 기준이다.

## 1. `@yoophi/markdown-annotation-core`

### `extractTocEntries`

```ts
export type TocLevel = 1 | 2 | 3;

export type TocEntry = {
  blockId: string;
  level: TocLevel;
  text: string;
  startLine: number;
};

export function extractTocEntries(blocks: MarkdownBlock[]): TocEntry[];
```

**동작 계약**:

| # | Given | Then |
|---|-------|------|
| C1 | h1~h3 heading 블록 포함 | 문서 순서대로 entry 반환, `blockId`/`level`/`startLine`은 블록 값 그대로 |
| C2 | h4~h6 heading 블록 | 결과에 포함되지 않음 |
| C3 | heading 블록 없음 (또는 h4~h6만 존재) | `[]` 반환 |
| C4 | 동일 텍스트 heading 다수 | 각각 별도 entry, 서로 다른 `blockId` |
| C5 | heading `content`에 inline 서식(`**`, `_`, `` ` ``, `~~`, 링크, 이미지) 포함 | `text`는 서식 기호 제거 + trim된 plain text |
| C6 | 빈 입력 `[]` | `[]` 반환 |
| C7 | 임의 입력 | 입력 배열/블록을 변경하지 않음 (순수 함수) |

### `stripInlineMarkdown`

```ts
export function stripInlineMarkdown(text: string): string;
```

**동작 계약**: `![alt](url)` → `alt`, `[text](url)` → `text`, `**b**`/`__b__` → `b`, `*i*`/`_i_` → `i`, `` `code` `` → `code`, `~~s~~` → `s`. 결과는 trim. 매칭되지 않는 텍스트는 그대로 통과. 순수 함수.

## 2. `@yoophi/markdown-annotation-react`

### `MarkdownToc`

```ts
export type MarkdownTocProps = {
  entries: TocEntry[];
  onEntrySelect?: (entry: TocEntry) => void;
  className?: string;
};

export function MarkdownToc(props: MarkdownTocProps): ReactNode;
```

**동작 계약**:

| # | Given | Then |
|---|-------|------|
| C8 | `entries.length === 0` | `null` 반환 (DOM 미출력) |
| C9 | entries 존재 | `<nav>` 내부에 entry 순서대로 클릭 가능한 항목(button) 렌더 |
| C10 | entry `level` 1/2/3 | `(level - 1)` 비례 들여쓰기가 적용된 시각 구분 (h1 < h2 < h3 깊이) |
| C11 | 항목 클릭 | `onEntrySelect(entry)` 1회 호출. 컴포넌트 스스로 스크롤/URL 변경하지 않음 |
| C12 | 각 항목 | `data-toc-block-id={entry.blockId}` 노출 (테스트/디버깅 훅) |

**비의존성 계약**: 앱 shell, Tauri API, route 상태, persistence, `MarkdownViewerComponents` 주입에 의존하지 않는다. Tailwind 유틸 클래스 + `cn`만 사용한다.

### `scrollToBlock`

```ts
export type ScrollToBlockOptions = {
  behavior?: ScrollBehavior; // 미지정 시 prefers-reduced-motion에 따라 자동 결정
};

export function scrollToBlock(
  container: ParentNode | null,
  blockId: string,
  options?: ScrollToBlockOptions,
): boolean; // 대상 발견·스크롤 시 true, container null 또는 대상 없음이면 false
```

**동작 계약**:

| # | Given | Then |
|---|-------|------|
| C13 | container 내 `[data-block-id="<blockId>"]` 존재 | 해당 요소에 `scrollIntoView({ block: "start", behavior })` 호출, `true` 반환 |
| C14 | container가 `null` 또는 대상 미존재 | 아무 동작 없이 `false` 반환 (throw 금지) |
| C15 | `options.behavior` 미지정 | `prefers-reduced-motion: reduce`면 `"auto"`, 아니면 `"smooth"` |

## 3. 앱 조립 계약 (`apps/markdown-annotator`)

- AnnotatorPage는 `extractTocEntries` 결과가 비어 있으면 TOC 패널(열 포함)을 렌더하지 않는다 (FR-006).
- `onEntrySelect` → `scrollToBlock(documentPaneRef.current, entry.blockId)` 연결.
- TOC 패널은 자체 스크롤 영역을 가지며 본문 ScrollArea와 독립적으로 스크롤된다.
- 기존 `MarkdownViewer` props/DOM 계약(`data-block-id`, line/offset 앵커)은 변경하지 않는다 (FR-007).
