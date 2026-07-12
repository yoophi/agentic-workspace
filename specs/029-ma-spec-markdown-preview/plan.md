# Implementation Plan: MA Spec Markdown Preview

**Branch**: `029` | **Date**: 2026-07-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/029-ma-spec-markdown-preview/spec.md`

## Summary

MA에서 SpecKit Markdown 문서를 읽기 중심 Preview로 표시하고, H1~H3 Table of Contents 이동, 상대 wikilink 문서 이동, task 상태와 H1 chapter별 완료·미완료 집계, HTML5 주석 비표시, Mermaid 확대 및 예외 격리를 제공한다. Markdown block 파싱과 재사용 가능한 렌더링은 기존 `markdown-annotation-core`와 `markdown-annotation-react`를 확장하고, 예제 선택·파일 이동·변경 감지는 MA의 FSD 경계에서 조합한다. AW Speckit Preview panel은 AW 일반 Markdown workspace의 annotation 상태·선택 도구·dialog·agent prompt와 TOC 조합을 재사용하여 Speckit 문서에서도 동일한 annotation과 구조 탐색을 제공한다. Feature specification, implementation plan, data model, tasks, requirements checklist 예제는 `examples/markdown-annotator`에서 읽기 전용 fixture로 제공한다.

## Technical Context

**Language/Version**: TypeScript 5.6+, React 19; Rust 2021/Tauri 2는 기존 파일 읽기·감시 경로에 한정

**Primary Dependencies**: Vite 7, react-markdown 10, remark-gfm 4, Mermaid 11, lucide-react, Tauri 2, Vitest 4, Storybook 10

**Storage**: 읽기 전용 로컬 UTF-8 Markdown 파일과 번들된 `examples/markdown-annotator/*.md`; 신규 영속 저장소 없음

**Testing**: Vitest unit/fixture tests, React static markup tests, MA/AW consumer tests와 type checks, MA Storybook 및 Vite build

**Target Platform**: MA Tauri desktop app 및 Vite browser development surface

**Project Type**: pnpm/Turbo monorepo의 공유 TypeScript packages + React/Tauri desktop app

**Performance Goals**: 2,000개 Markdown block 또는 1MB 이하 문서를 2초 안에 읽을 수 있는 Preview 상태로 표시

**Constraints**: 오프라인 동작, Preview에서 원문/task 상태 변경 금지, 명시적 사용자 동작만 문서 이동, 현재 문서 디렉터리와 허용된 Markdown 범위 밖 경로 차단, 요소별 렌더 오류 격리

**Scale/Scope**: 공유 packages 2개, MA 소비 화면 1개, AW 일반 Markdown/Speckit Preview panel 2개, SpecKit 예제 5종, H1~H3 TOC와 H1 chapter task 집계, AW 문서별 annotation lifecycle

## Constitution Check

*GATE: Phase 0 전 PASS, Phase 1 설계 후 재검토 결과도 동일하게 PASS.*

- **Monorepo Boundary First — PASS**: 순수 파싱·TOC·타입은 `packages/markdown-annotation-core`, 재사용 렌더링은 `packages/markdown-annotation-react`, MA 전용 선택·탐색은 `apps/markdown-annotator/src`, fixture는 `examples/markdown-annotator`에 둔다. 앱 간 직접 import가 없다.
- **Feature-Sliced Frontend Architecture — PASS**: MA 화면 조합은 `pages/annotator`, 문서 모델/API는 `entities/document`, 문서 열기 동작은 `features/open-document`, 앱별 UI adapter는 `shared/ui`에 둔다. AW annotation/TOC 조합은 `features/worktree-workspace` 내부의 재사용 가능한 model/UI로 두고 Speckit 및 일반 Markdown panel이 공유한다.
- **Hexagonal Tauri Backend Architecture — PASS**: 신규 backend 변경은 계획하지 않는다. 기존 파일 읽기/감시는 inbound command → application service → domain port/infrastructure adapter 경계를 그대로 사용한다.
- **Shared Core Before Shared UI — PASS**: Markdown block, TOC, wikilink와 task 집계 규칙을 pure core에서 정의한 후 app-shell 독립 React renderer가 소비한다. MA의 Tauri/route 상태는 공유 UI로 이동하지 않는다.
- **Atomic Cross-App Verification — PASS**: core/React package별 `check-types`와 `test`, 소비 앱 MA와 AW의 `check-types`와 `test`를 실행하고 MA build/Storybook 및 AW Speckit annotation/TOC integration tests를 추가 검증한다.
- **Documentation and Storybook — PASS**: reusable TOC/task/Markdown 상태를 `apps/markdown-annotator/src/stories/molecules`에 등록하고, 실행 방법 변경 시 `docs/*.md` 또는 OpenWiki를 갱신한다. 본 feature의 설계 문서는 `specs/029-ma-spec-markdown-preview`에 둔다.
- **Testing and Safety — PASS**: parser/집계/wikilink는 fixture 기반 unit test, renderer는 접근 가능한 markup test, 파일 이동은 상대 경로 정규화·확장자·root 범위·누락/권한 오류 test를 계획한다. 신규 persistence/session 범위는 없다.

## Project Structure

### Documentation (this feature)

```text
specs/029-ma-spec-markdown-preview/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── markdown-preview-contract.md
├── checklists/
│   └── requirements.md
└── tasks.md                       # $speckit-tasks output
```

### Source Code (repository root)

```text
apps/markdown-annotator/src/
├── pages/annotator/               # Preview/TOC 화면 조합과 문서 상태
├── features/open-document/        # 파일 선택 및 wikilink 문서 이동
├── entities/document/             # 문서 API, example metadata/model
├── shared/ui/                     # 공유 renderer의 MA UI adapter
├── components/ui/                 # shadcn/ui primitives
└── stories/molecules/             # MarkdownViewer/MarkdownToc stories

apps/agentic-workbench/src/features/worktree-workspace/
├── model/                          # 문서별 annotation/selection workspace 상태
└── ui/
    ├── worktree-workspace-panel.tsx # 일반 Markdown/Speckit panel 조합
    ├── markdown-annotation-workspace.tsx
    └── markdown-preview-toc.tsx

apps/markdown-annotator/src-tauri/src/
├── domain/                        # 문서 port/model
├── application/                   # 문서 읽기/감시 use case
├── inbound/                       # Tauri commands
└── infrastructure/                # filesystem reader/watcher

packages/markdown-annotation-core/src/
├── parse/                         # Markdown block와 wikilink parsing
├── toc/                           # heading 추출과 chapter task 집계
└── types/                         # block, TOC, task summary contracts

packages/markdown-annotation-react/src/
├── MarkdownViewer.tsx             # block/task/Mermaid rendering
├── MarkdownToc.tsx                # heading navigation/task summary UI
└── *.test.tsx                     # renderer contract tests

examples/markdown-annotator/
├── speckit-spec.md
├── speckit-plan.md
├── speckit-data-model.md
├── speckit-tasks.md
└── speckit-checklist.md
```

**Structure Decision**: Markdown 의미 해석과 소비 앱 공통 표현은 기존 두 공유 package를 확장한다. MA만 아는 문서 선택, Tauri filesystem, example lifecycle은 MA의 FSD/hexagonal 경계에 남긴다. AW의 annotation 상태와 interaction은 app-specific이므로 공유 package로 이동하지 않고 `features/worktree-workspace` 안에서 일반 Markdown과 Speckit panel이 재사용한다. 샘플은 source module이 아니라 실제 Markdown fixture로 유지한다.

## Phase Plan

### Phase 0 - Research

`research.md`의 결정을 기준으로 GFM task parsing, H1 chapter 범위, relative wikilink 안전성, shared renderer 접근성, 예제 fixture 구성을 확정한다. 외부 의존성을 추가하지 않고 기존 parser와 renderer 확장을 우선한다.

### Phase 1 - Domain and Contracts

`data-model.md`의 MarkdownDocument, MarkdownBlock, TocEntry, TaskSummary, WikilinkTarget, ExampleMarkdownDocument 관계를 구현 계약으로 사용한다. `contracts/markdown-preview-contract.md`에서 입력 문법, TOC/task 출력, link activation, 오류/접근성 계약을 고정한다.

### Phase 2 - Implementation Sequence

1. core parser/type에 wikilink와 H1 chapter task summary 규칙 및 fixture tests를 추가한다.
2. React viewer와 TOC에 task icon/summary, wikilink 출력, Mermaid/error 상태 및 접근성 contract tests를 추가한다.
3. MA에서 안전한 wikilink 문서 이동, 문서 상태 교체, 예제 문서 선택과 읽기 전용 lifecycle을 연결한다.
4. SpecKit 예제 5종과 MarkdownViewer/MarkdownToc Storybook 사례를 등록한다.
5. package → MA/AW 순서로 type/test/build/Storybook 회귀 검증을 수행한다.
6. AW 일반 Markdown workspace의 annotation/selection/dialog/prompt 조합을 `features/worktree-workspace` 내부 재사용 단위로 추출한다.
7. Speckit Preview panel에 문서 경로별 annotation 상태, selection toolbar, annotation 목록과 agent prompt를 연결한다.
8. Speckit Preview panel에 `extractTocEntries`와 `MarkdownPreviewToc`을 연결하고 heading 이동 및 H1 task count를 검증한다.

## Post-Design Constitution Re-check

**PASS**. 설계는 app/package 경계, FSD, 기존 Tauri hexagonal 경계를 유지한다. AW annotation interaction은 AW feature 내부에서 재사용하고 app shell 의존성을 공유 package로 유출하지 않는다. 공유 package는 pure core 우선이며 MA/AW consumer verification을 포함한다. 신규 예외나 constitution 위반이 없어 Complexity Tracking은 필요하지 않다.

## Complexity Tracking

해당 없음.
