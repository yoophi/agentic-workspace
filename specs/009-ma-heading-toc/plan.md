# Implementation Plan: MA markdown viewer heading 기반 Table of Contents

**Branch**: `worktree-issue-124-ma-toc` | **Date**: 2026-07-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/009-ma-heading-toc/spec.md`

## Summary

MA(markdown-annotator)의 viewer에 h1~h3 heading 기반 TOC를 제공한다. 파서가 이미 산출하는 heading 블록(`type: "heading"`, `level`, `id`, `startLine`)에서 TOC entry를 추출하는 순수 함수를 `markdown-annotation-core`에 추가하고, presentational TOC 컴포넌트와 `data-block-id` 기반 스크롤 helper를 `markdown-annotation-react`에 추가하며, `apps/markdown-annotator`의 AnnotatorPage 문서 pane 좌측에 접이식 TOC 패널로 조립한다. 본문 렌더링 방식과 line/offset 앵커 동작은 변경하지 않는다.

## Technical Context

**Language/Version**: TypeScript 5.x, React 19

**Primary Dependencies**:
- `packages/markdown-annotation-core`: 의존성 없는 순수 TS (파서/모델/포매터)
- `packages/markdown-annotation-react`: React 19, Tailwind CSS v4 유틸 클래스 + `cn`, lucide-react (기존 의존성만 사용, 신규 의존성 없음)
- `apps/markdown-annotator`: Vite + Tauri, shadcn/ui (`components/ui`), Radix ScrollArea

**Storage**: N/A (TOC는 파생 읽기 전용 데이터, persistence 없음)

**Testing**: vitest. core는 순수 함수 fixture 테스트, react는 기존 관례대로 `renderToStaticMarkup` 기반 마크업 검증(jsdom 미사용) + scroll helper는 fake ParentNode 주입으로 검증. 앱은 `check-types` + `vitest run --passWithNoTests`.

**Target Platform**: Tauri desktop webview + 브라우저(Vite dev), Storybook(port 6007)

**Project Type**: pnpm/Turbo monorepo — 공유 패키지 2개 + 소비 앱 1개 (프론트엔드 전용, Rust 변경 없음)

**Performance Goals**: heading 100개 이상 문서에서도 TOC 표시 체감 지연 없음 (추출 O(n), `useMemo`로 blocks 변경 시에만 재계산)

**Constraints**:
- 본문 렌더링 방식 변경 금지 — line/offset 앵커 정합 유지 (ReactMarkdown 본문 도입 금지)
- TOC 클릭은 URL/해시 변경 없이 화면 내 스크롤만 수행
- `prefers-reduced-motion` 존중 (smooth ↔ auto)
- 공유 패키지 Tailwind 클래스는 소비 앱 `index.css`의 `@source`로 스캔됨 — `markdown-annotation-react` src는 이미 등록되어 있어 추가 설정 불필요 (`apps/markdown-annotator/src/index.css:8`)

**Scale/Scope**: 문서 수천 라인 / heading 수백 개 수준. 화면 1개(AnnotatorPage) 레이아웃 변경, 신규 컴포넌트 1개, 순수 함수 2개(추출 + 서식 제거)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS — 추출 로직은 `packages/markdown-annotation-core/src/toc/`, TOC UI는 `packages/markdown-annotation-react/src/`, 레이아웃 조립은 `apps/markdown-annotator/src/pages/annotator/`. 앱 간 import 없음.
- **Feature-Sliced Frontend Architecture**: PASS — 앱 변경은 `pages/annotator/AnnotatorPage.tsx`(화면 레이아웃)에 한정. shadcn 컴포넌트는 `components/ui`에서 import. 신규 앱 레이어 추가 없음.
- **Hexagonal Tauri Backend Architecture**: N/A — 백엔드/Rust 변경 없음.
- **Shared Core Before Shared UI**: PASS — 순수 core(`extractTocEntries`, `stripInlineMarkdown`)를 먼저 두고 fixture 테스트로 검증. 공유 UI(`MarkdownToc`)는 이미 공유가 확립된 `markdown-annotation-react`에 추가하며 앱 shell/Tauri/route/persistence에 의존하지 않음 (props로 entries와 콜백만 받는 presentational 컴포넌트).
- **Atomic Cross-App Verification**: PASS — `packages/markdown-annotation-*` 변경이므로 두 패키지의 `test`/`check-types`와 소비 앱 검증을 완료 조건에 포함 (quickstart.md 참조). 구현 단계 재확인 결과 `agentic-workbench`도 두 패키지를 소비하므로(`apps/agentic-workbench/package.json`) `agentic-workbench`의 `check-types`/`test`도 검증 대상에 포함했다. `git-explorer`는 미소비.
- **Documentation and Storybook**: PASS — `MarkdownToc` 스토리를 기존 관례대로 `apps/markdown-annotator/src/stories/molecules/`에 추가 (entries 있음/없음/긴 목록/중첩 level 상태 포함). 별도 `docs/*.md`는 신규 아키텍처 개념이 없어 불필요하나, 필요 시 tasks에서 재평가.
- **Testing and Safety**: PASS — 순수 로직(추출/서식 제거) unit test 선행, 컴포넌트 마크업 테스트, scroll helper fake 주입 테스트. 파일/persistence/세션/권한 변경 없어 root/path/owner 검증 대상 없음.

**Post-Phase 1 재평가**: 설계 산출물(data-model, contracts) 확인 결과 위반 없음 — 모든 게이트 PASS 유지. Complexity Tracking 불필요.

## Project Structure

### Documentation (this feature)

```text
specs/009-ma-heading-toc/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── toc-api.md       # Phase 1 output (core/react 공개 API 계약)
└── tasks.md             # Phase 2 output (/speckit-tasks — 이 명령에서 생성하지 않음)
```

### Source Code (repository root)

```text
packages/markdown-annotation-core/src/
├── toc/
│   ├── extract-toc-entries.ts        # 신규: heading 블록 → TocEntry[] 순수 함수
│   ├── extract-toc-entries.test.ts   # 신규: fixture 기반 unit test
│   ├── strip-inline-markdown.ts      # 신규: inline 서식 제거 순수 함수
│   └── strip-inline-markdown.test.ts # 신규
├── types/                            # TocEntry 타입 추가
└── index.ts                          # export 추가

packages/markdown-annotation-react/src/
├── MarkdownToc.tsx                   # 신규: presentational TOC 컴포넌트
├── MarkdownToc.test.tsx              # 신규: renderToStaticMarkup 마크업 검증
├── scroll-to-block.ts                # 신규: data-block-id 스크롤 helper
├── scroll-to-block.test.ts           # 신규: fake ParentNode 주입 검증
└── index.ts                          # export 추가

apps/markdown-annotator/src/
├── pages/annotator/AnnotatorPage.tsx # 수정: 문서 pane 좌측 접이식 TOC 패널 배치
└── stories/molecules/
    └── MarkdownToc.stories.tsx       # 신규: Storybook 스토리
```

**Structure Decision**: constitution 원칙 IV(Shared Core Before Shared UI)에 따라 3계층 분리 — core(순수 추출)에서 시작해 react(공유 presentational UI + DOM helper), app(레이아웃 조립·스크롤 컨테이너 연결) 순으로 의존 방향을 유지한다. 파서(`parse-markdown-to-blocks.ts`)는 변경하지 않는다.

## Complexity Tracking

> Constitution Check 위반 없음 — 해당 없음.
