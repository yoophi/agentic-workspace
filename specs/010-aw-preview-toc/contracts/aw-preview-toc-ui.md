# UI Contract: AW markdown preview TOC 조립

**Feature**: 010-aw-preview-toc | **Date**: 2026-07-02

공유 패키지 API는 [specs/009-ma-heading-toc/contracts/toc-api.md](../../009-ma-heading-toc/contracts/toc-api.md)의 계약(C1~C15)을 그대로 따르며 **변경하지 않는다**. 이 문서는 AW 앱 조립 계약만 정의한다.

## 1. `MarkdownPreviewToc` (앱 로컬 컴포넌트)

`apps/agentic-workbench/src/features/worktree-workspace/ui/markdown-preview-toc.tsx`

```ts
export type MarkdownPreviewTocProps = {
  entries: TocEntry[];
  onEntrySelect?: (entry: TocEntry) => void;
  defaultOpen?: boolean; // 기본 false. 테스트/스토리에서 펼침 상태 정적 렌더용
  className?: string;
};

export function MarkdownPreviewToc(props: MarkdownPreviewTocProps): ReactNode;
```

**동작 계약**:

| # | Given | Then |
|---|-------|------|
| A1 | `entries.length === 0` | `null` 반환 — 토글 행 포함 아무것도 렌더하지 않음 (FR-005) |
| A2 | entries 존재, 접힘(기본) | 한 줄 토글 행만 렌더 ("Contents" 라벨 + 펼침 아이콘, `aria-expanded="false"`). TOC 목록(`nav`) 미렌더 |
| A3 | 펼침 상태 (`defaultOpen` 또는 토글 클릭) | 토글 행(`aria-expanded="true"`) 아래에 공유 `MarkdownToc` 목록 렌더. 목록은 `max-height` 제한 + 자체 세로 스크롤 (FR-009) |
| A4 | 펼침 상태에서 토글 클릭 | 접힘으로 복귀, 목록 미렌더 (FR-004) |
| A5 | 항목 클릭 | `onEntrySelect(entry)` 위임 (공유 `MarkdownToc` 계약 C11 승계) — 컴포넌트 스스로 스크롤/URL 변경 없음 |
| A6 | 모든 상태 | annotation aside 하단 sticky 배치(`sticky bottom-4 mt-auto`, 호출부 className) — 본문 markdown 열과 겹치지 않으며, 접힘 토글 행이 스크롤 위치와 무관하게 우측 하단에 상시 접근 가능 *(사용자 피드백으로 상단 in-flow에서 개정)* |

**비의존성 계약**: Tauri API, route 상태, persistence, query client에 의존하지 않는 presentational 컴포넌트. shadcn `components/ui`와 공유 `MarkdownToc`만 사용.

## 2. panel 조립 계약 (`worktree-workspace-panel.tsx`)

| # | 항목 | 계약 |
|---|------|------|
| A7 | entries 산출 | `const tocEntries = useMemo(() => extractTocEntries(blocks), [blocks])` — 기존 `blocks` memo 연동으로 파일 전환·자동 새로고침 시 자동 갱신 (FR-006) |
| A8 | 렌더 위치 | preview pane 데이터 분기(`previewQuery.data` 존재) 내부, annotation aside(`flex flex-col`)의 마지막 자식으로 배치. 로딩/오류/미선택 분기에서는 렌더되지 않음 (FR-005) *(사용자 피드백으로 개정)* |
| A9 | 리셋 | `key={selectedFilePath}`로 파일 전환 시 기본 접힘 재마운트 |
| A10 | 스크롤 연결 | `onEntrySelect` → `scrollToBlock(previewPaneRef.current, entry.blockId)` |
| A11 | 무변경 보장 | preview header, 본문 grid 구조, `MarkdownViewer` props, 선택 하이라이트·annotation 로직은 변경하지 않음 (FR-008). `packages/*` diff 없음 |
