# Data Model: MA heading TOC

**Feature**: 009-ma-heading-toc | **Date**: 2026-07-02

## 개요

TOC는 파서 출력(`MarkdownBlock[]`)에서 파생되는 읽기 전용 데이터다. 신규 persistence 없음, 기존 엔티티 변경 없음.

```mermaid
flowchart LR
    MD[markdownText] -->|parseMarkdownToBlocks<br/>기존, 변경 없음| B["MarkdownBlock[]"]
    B -->|extractTocEntries<br/>신규| T["TocEntry[]"]
    T --> UI[MarkdownToc 컴포넌트]
    UI -->|onEntrySelect → scrollToBlock| V["MarkdownViewer DOM<br/>(data-block-id)"]
```

## TocEntry (신규)

`packages/markdown-annotation-core/src/types`에 추가.

| 필드 | 타입 | 설명 |
|------|------|------|
| `blockId` | `string` | 원본 heading 블록의 고유 `id`. 스크롤 타겟(`data-block-id`) 결정에 사용 |
| `level` | `1 \| 2 \| 3` | heading level. 들여쓰기 깊이 결정 |
| `text` | `string` | inline 서식 제거(trim 포함) 후 표시 텍스트 |
| `startLine` | `number` | 원본 문서 내 시작 라인 (블록의 `startLine` 그대로) |

### 파생 규칙 (extractTocEntries)

1. 입력 `MarkdownBlock[]` 중 `type === "heading"`이고 `level`이 1~3인 블록만 선택한다.
2. 입력 배열 순서(= 문서 등장 순서)를 그대로 유지한다. 정렬/그룹핑하지 않는다.
3. `text = stripInlineMarkdown(block.content)`. 결과가 빈 문자열이어도 entry는 유지한다 (spec Edge Case — 문서 순서 보존).
4. level 4~6 heading, heading 외 블록은 제외한다.
5. 입력을 변경(mutate)하지 않는 순수 함수다.

### 불변 조건

- `entries.length === 0` ⇔ TOC UI 미렌더 (FR-006)
- 모든 `blockId`는 입력 blocks 내에서 유일 (파서가 보장하는 `block-N` id를 그대로 사용)
- entry 간 `startLine`은 단조 증가 (파서의 순차 산출 특성에서 유도)

## MarkdownBlock (기존, 변경 없음)

TOC가 소비하는 기존 필드: `id`, `type`, `level`, `content`, `startLine`. 파서(`parse-markdown-to-blocks.ts`)와 타입 정의는 수정하지 않는다.

## 상태 (app 전용, 비영속)

| 상태 | 위치 | 설명 |
|------|------|------|
| `tocEntries` | AnnotatorPage `useMemo` | `extractTocEntries(blocks)` 결과. blocks 변경 시에만 재계산 |
| `isTocOpen` | AnnotatorPage `useState` | 접이식 패널 열림 여부. 저장하지 않음(세션 내 휘발) |
