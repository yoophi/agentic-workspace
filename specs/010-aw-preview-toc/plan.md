# Implementation Plan: AW workspace markdown preview heading 기반 Table of Contents

**Branch**: `issue-127-aw-toc` | **Date**: 2026-07-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/010-aw-preview-toc/spec.md`

## Summary

AW(agentic-workbench) worktree 세션 workspace 패널의 markdown preview에 h1~h3 heading TOC를 추가한다. specs/009에서 공유 패키지에 구현된 `extractTocEntries`(core), `MarkdownToc`·`scrollToBlock`(react)을 그대로 사용하고, **공유 패키지 변경 없이** 앱 조립만 수행한다. preview pane 상단에 기본 접힘 상태의 접이식 TOC 섹션(`MarkdownPreviewToc`, 앱 로컬 컴포넌트)을 배치하고, 항목 클릭 시 `scrollToBlock(previewPaneRef.current, blockId)`로 이동한다. 본문 렌더링과 annotation 앵커·선택 하이라이트는 변경하지 않는다.

## Technical Context

**Language/Version**: TypeScript 5.x, React 19

**Primary Dependencies**:
- `@yoophi/markdown-annotation-core`: `extractTocEntries`, `TocEntry` (기존, 무변경)
- `@yoophi/markdown-annotation-react`: `MarkdownToc`, `scrollToBlock` (기존, 무변경)
- `apps/agentic-workbench`: Vite + Tauri, shadcn/ui, TanStack Query(preview 조회), ResizablePanel 레이아웃. 신규 의존성 없음
- Tailwind `@source`: AW `index.css:8`에 `markdown-annotation-react/src`가 이미 등록되어 있어 추가 설정 불필요

**Storage**: N/A (TOC 펼침 상태는 컴포넌트 로컬 휘발 상태, 파일 전환 시 기본 접힘으로 리셋)

**Testing**: vitest. AW 기존 관례(`agent-run-markdown.test.tsx`)대로 `renderToStaticMarkup` 기반 마크업 검증(jsdom 미사용). 정적 렌더로 펼침 상태를 검증할 수 있도록 컴포넌트에 `defaultOpen` prop 제공

**Target Platform**: Tauri desktop webview (agentic-workbench), Storybook

**Project Type**: pnpm/Turbo monorepo — 앱 1개(`agentic-workbench`)의 features 레이어 조립 (공유 패키지·Rust 무변경)

**Performance Goals**: `extractTocEntries`는 O(n) `useMemo`로 blocks 변경 시에만 재계산 (preview 자동 새로고침이 내용 불변이면 blocks memo가 유지되어 TOC 재계산 없음)

**Constraints**:
- 공유 패키지(`packages/markdown-annotation-*`) 변경 금지 — MA(정본)와의 호환 유지
- preview 본문 렌더링·annotation 앵커·선택 하이라이트 무변경 (FR-008)
- TOC 기본 접힘, 접힘 시 본문 표시 영역 실질 동일 (FR-004, SC-004)
- 펼침 시 본문과 겹침/잘림 금지 — overlay가 아닌 in-flow 배치 (FR-009)

**Scale/Scope**: 화면 1개(worktree workspace panel, 1,829줄)의 preview 영역 조립, 앱 로컬 컴포넌트 1개 신규, Storybook 스토리 1건 추가

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS — 변경은 `apps/agentic-workbench` 내부로 한정. 공유 로직은 workspace 패키지 import(`@yoophi/markdown-annotation-*`)로만 소비. 앱 간 import 없음.
- **Feature-Sliced Frontend Architecture**: PASS — 신규 컴포넌트와 조립 수정 모두 `apps/agentic-workbench/src/features/worktree-workspace/ui/`(preview UI가 속한 features 레이어). shadcn 컴포넌트는 `components/ui`에서 import.
- **Hexagonal Tauri Backend Architecture**: N/A — 백엔드/Rust 변경 없음.
- **Shared Core Before Shared UI**: PASS — 순수 core·공유 UI는 specs/009에서 이미 공유됨. 이번에 추가하는 `MarkdownPreviewToc`는 AW preview 레이아웃 전용(접이식 컨테이너) wrapper로 앱 로컬에 둔다. MA에도 동일한 접이식 요구가 수렴하면 그때 공유 승격을 검토한다(성급한 공유 금지 원칙).
- **Atomic Cross-App Verification**: PASS — `packages/*`·`crates/*` 변경이 없으므로 교차 앱 검증 의무는 발생하지 않는다. `agentic-workbench`의 `check-types`/`test`를 완료 조건으로 하고, 공유 패키지 무변경을 diff로 확인한다(quickstart 참조).
- **Documentation and Storybook**: PASS — `MarkdownPreviewToc` 스토리를 AW 관례(`src/stories/molecules.stories.tsx`, atomic 카테고리 단일 파일)에 추가: 접힘(기본)/펼침/긴 목록/빈 entries 상태. 신규 아키텍처 개념이 없어 `docs/*.md` 신규 문서는 불필요.
- **Testing and Safety**: PASS — `MarkdownPreviewToc` 마크업 unit test(`renderToStaticMarkup`, `defaultOpen`으로 펼침 상태 정적 검증). 신규 순수 로직 없음(추출 로직은 specs/009에서 테스트 완료). 파일/persistence/세션/권한 변경 없어 안전 경계 검증 대상 없음.

**Post-Phase 1 재평가**: 설계 산출물(data-model, contracts) 기준 위반 없음 — 전 게이트 PASS 유지, Complexity Tracking 불필요.

## Project Structure

### Documentation (this feature)

```text
specs/010-aw-preview-toc/
├── plan.md                  # This file
├── research.md              # Phase 0 output
├── data-model.md            # Phase 1 output
├── quickstart.md            # Phase 1 output
├── contracts/
│   └── aw-preview-toc-ui.md # Phase 1 output (앱 조립 UI 계약)
└── tasks.md                 # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── features/worktree-workspace/ui/
│   ├── markdown-preview-toc.tsx        # 신규: 접이식 TOC 섹션 (앱 로컬 wrapper)
│   ├── markdown-preview-toc.test.tsx   # 신규: renderToStaticMarkup 마크업 검증
│   └── worktree-workspace-panel.tsx    # 수정: preview pane 상단에 조립, scrollToBlock 연결
└── stories/
    └── molecules.stories.tsx           # 수정: MarkdownPreviewToc 스토리 추가

# 공유 패키지: 변경 없음 (specs/009 산출물 소비만)
packages/markdown-annotation-core/src/   # extractTocEntries, TocEntry (기존)
packages/markdown-annotation-react/src/  # MarkdownToc, scrollToBlock (기존)
```

**Structure Decision**: 공유 계층은 specs/009 산출물을 그대로 소비하고, AW 전용 접이식 컨테이너(`MarkdownPreviewToc`)만 features 레이어에 신규 추가한다. 1,829줄 panel 파일의 비대화를 피하고 마크업 테스트·Storybook 등록이 가능하도록 별도 파일로 분리한다.

## Complexity Tracking

> Constitution Check 위반 없음 — 해당 없음.
