# Implementation Plan: Markdown Quick Prompt

**Branch**: `115-issue` | **Date**: 2026-07-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/005-markdown-quick-prompt/spec.md`

**Note**: `/speckit-plan` setup copied the plan template. `.specify/scripts` does not include a separate agent-context update script in this repository, so that step is recorded as not available.

## Summary

Markdown 문서 검토 중 선택 영역, 단일 Markdown 블럭, 전체 문서를 quick prompt 컨텍스트로 만들고, 사용자가 전송 전 prompt를 작성/수정한 뒤 활성 agent 대상에 보낼 수 있게 한다. 선택 영역 흐름은 텍스트 선택 후 표시되는 selection toolbar 안에 quick annotate 번개 버튼을 제공하고, 해당 버튼이 prompt 작성 다이얼로그를 연 뒤 사용자가 입력한 prompt를 agent-run으로 전송하는 것을 핵심 계약으로 둔다. 구현 접근은 Markdown 컨텍스트 추출/포맷팅을 `packages/markdown-annotation-core`의 순수 로직으로 추가하고, `packages/markdown-annotation-react`의 viewer에 quick prompt 액션 슬롯을 확장한 뒤, `apps/markdown-annotator`와 기존 AW Markdown workspace의 agent 전송 경계가 같은 계약을 사용하도록 정렬한다.

## Technical Context

**Language/Version**: TypeScript 5.6, React 19, Rust/Tauri 2는 기존 앱 shell 범위에서만 사용

**Primary Dependencies**: `@yoophi/markdown-annotation-core`, `@yoophi/markdown-annotation-react`, existing shadcn-compatible UI primitives, lucide icon set

**Storage**: N/A for quick prompt drafts; no new persistence. Existing document loading and annotation state remain unchanged.

**Testing**: Vitest for core/react/app tests; Storybook for MA visual states; existing app type checks through pnpm/Turbo

**Target Platform**: Desktop/web frontends already supported by `apps/markdown-annotator` and `apps/agentic-workbench`

**Project Type**: Monorepo desktop/web application with shared TypeScript packages

**Performance Goals**: Quick prompt context preview opens within 300 ms for representative documents up to 100 Markdown blocks; long-context limit calculation is immediate from already-loaded text.

**Constraints**: Must not break existing selection toolbar positioning, selection, annotation, Mermaid, code block, auto reload, or AW prompt send flows. Selection toolbar quick annotate must use the current non-empty selection and must not send until the user submits dialog-entered prompt text. Must preserve app-to-app isolation and avoid adding shared UI before interaction contracts converge.

**Scale/Scope**: Three context scopes: selection, block, document. Primary UI target is MA; shared viewer/core changes must remain compatible with AW Markdown workspace.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. App-specific composition stays under `apps/markdown-annotator/src` and `apps/agentic-workbench/src`; reusable TypeScript contracts and pure context logic stay under `packages/markdown-annotation-core` and `packages/markdown-annotation-react`. No app-to-app imports are planned.
- **Feature-Sliced Frontend Architecture**: PASS. MA composition remains in `pages/annotator`; quick prompt user actions belong in `features/quick-prompt`; context/domain types belong in `entities` or shared core; reusable app-local UI belongs in `shared` or existing `components/ui`.
- **Hexagonal Tauri Backend Architecture**: N/A for the core feature. No new persistence or Tauri command is required. If a later agent delivery command is introduced, it must validate session/workspace owner scope at the inbound boundary and delegate to application services.
- **Shared Core Before Shared UI**: PASS. The plan shares context modeling, extraction, and prompt formatting first. Shared React viewer receives an optional action slot; app-specific composition UI stays local.
- **Atomic Cross-App Verification**: PASS. Because `packages/markdown-annotation-core` and `packages/markdown-annotation-react` may change, verification includes package tests/type checks plus MA and AW type checks/tests.
- **Documentation and Storybook**: PASS. Storybook coverage is required for quick prompt controls and compose states in MA. No `docs/*.md` update is required unless implementation introduces a new cross-app agent delivery protocol.
- **Testing and Safety**: PASS. Pure context formatting gets fixture tests. Agent target delivery must validate target availability and avoid silently sending content to unrelated sessions. Existing file/root safety is preserved because the feature uses already-loaded document content.

## Project Structure

### Documentation (this feature)

```text
specs/005-markdown-quick-prompt/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── quick-prompt-ui.md
└── tasks.md
```

### Source Code (repository root)

```text
packages/markdown-annotation-core/src/
├── quick-prompt/
│   ├── build-quick-prompt-context.ts
│   ├── format-quick-prompt-for-agent.ts
│   └── *.test.ts
└── types/
    └── quick-prompt.ts

packages/markdown-annotation-react/src/
├── MarkdownViewer.tsx
├── QuickPromptDialog.tsx
├── types.ts
└── *.test.tsx

apps/markdown-annotator/src/
├── pages/annotator/
│   ├── AnnotatorPage.tsx
│   └── *.test.tsx
├── features/quick-prompt/
│   └── ui/
├── entities/
│   ├── document/
│   └── markdown-block/
├── shared/ui/
└── stories/
    ├── molecules/
    └── organisms/

apps/agentic-workbench/src/
└── features/worktree-workspace/ui/
    └── worktree-workspace-panel.tsx
```

**Structure Decision**: Use shared core for scope extraction and final agent prompt formatting, shared React only for viewer action slots and optional dialog primitives, and keep app-level target availability/send behavior local. AW integration is limited to adapting the existing Markdown workspace `onSendAnnotationPrompt` path to the new quick prompt formatter when needed.

## Complexity Tracking

No constitution violations are planned.

## Phase 0: Research

Research is recorded in [research.md](./research.md). All planning unknowns are resolved there:

- Agent target contract and unavailable state
- Context extraction source of truth for selection/block/document scopes
- Long-context policy
- Shared core vs shared UI boundary
- Accessibility and Storybook coverage

## Phase 1: Design & Contracts

Design artifacts:

- [data-model.md](./data-model.md)
- [contracts/quick-prompt-ui.md](./contracts/quick-prompt-ui.md), including the selection toolbar quick annotate surface contract
- [quickstart.md](./quickstart.md)

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. Contracts keep app-local delivery adapters under apps and shared logic under packages.
- **Feature-Sliced Frontend Architecture**: PASS. Data model separates Quick Prompt Context, Prompt Draft, Agent Target, and UI state so tasks can map cleanly to FSD layers.
- **Hexagonal Tauri Backend Architecture**: N/A. No backend change is required by the design artifacts.
- **Shared Core Before Shared UI**: PASS. `data-model.md` and contracts define headless context/formatter behavior before any visual component.
- **Atomic Cross-App Verification**: PASS. `quickstart.md` lists package checks plus MA/AW app checks.
- **Documentation and Storybook**: PASS. `quickstart.md` requires Storybook states for selection, block, document, disabled/unavailable, long-context, and keyboard-visible states.
- **Testing and Safety**: PASS. Contracts include invalid send prevention and target availability; quickstart includes regression checks for annotation and rendering workflows.
