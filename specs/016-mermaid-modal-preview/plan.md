# Implementation Plan: Mermaid Chart Expanded Modal for Markdown Preview

**Branch**: `` | **Date**: 2026-07-07 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/016-mermaid-modal-preview/spec.md`

## Summary

MA의 Mermaid chart와 AW worktree session page markdown preview 영역에 agent-run 패널의 Mermaid 큰 modal 보기 경험을 적용한다. 기존 `packages/markdown-annotation-react`의 `MermaidDiagram`과 `MarkdownViewer`가 MA와 AW preview 양쪽에서 이미 공유되고 있으므로, 확대 modal의 headless/view contract와 fit body를 공유 패키지에 추가하고, Base UI(MA)와 Radix(AW)의 dialog/tooltip/button 차이는 기존 component injection 방식으로 흡수한다. AW agent-run은 같은 shared expanded viewer를 소비하도록 정리해 세 화면의 열기/닫기, zoom/fit, 실패 제외 규칙을 맞춘다.

## Technical Context

**Language/Version**: TypeScript 5.x, React 19, Vite 기반 Tauri desktop frontend

**Primary Dependencies**: `@yoophi/markdown-annotation-react`, `@yoophi/markdown-annotation-core`, `mermaid` 11.16, `lucide-react`, MA Base UI shadcn dialog/tooltip, AW Radix shadcn dialog/tooltip

**Storage**: N/A. Modal open/zoom state is ephemeral UI state only.

**Testing**: Vitest + React static rendering/unit tests; app-level `check-types`; Storybook stories for reusable visual states.

**Target Platform**: macOS desktop Tauri apps through browser WebView; responsive behavior must work inside resizable app panels.

**Project Type**: pnpm/Turbo monorepo desktop app frontend plus shared React workspace package.

**Performance Goals**: Opening or closing a rendered Mermaid modal must not reflow surrounding document panels perceptibly; large diagrams must remain contained in the modal and scroll locally.

**Constraints**: No direct imports between `apps/markdown-annotator` and `apps/agentic-workbench`; no new backend or persistence; Mermaid rendering must keep `securityLevel: "strict"` and existing failure fallback behavior; failed/empty diagrams must not expose an expand trigger.

**Scale/Scope**: Three consumer surfaces: MA `MarkdownViewer`, AW worktree workspace `MarkdownViewer`, and existing AW agent-run Mermaid modal. Representative fixtures cover success, large diagram, failed diagram, empty source, ordinary code block, reload/file switch workflows.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. Shared behavior is planned under `packages/markdown-annotation-react`; app-specific dialog adapters stay under `apps/markdown-annotator/src` and `apps/agentic-workbench/src`. App-to-app imports are avoided.
- **Feature-Sliced Frontend Architecture**: PASS. AW markdown preview composition remains in `apps/agentic-workbench/src/features/worktree-workspace`; AW agent-run composition remains in `apps/agentic-workbench/src/features/agent-run`; MA page composition remains in `apps/markdown-annotator/src/pages/annotator` with shared UI adapters under `apps/markdown-annotator/src/shared/ui`.
- **Hexagonal Tauri Backend Architecture**: N/A. No Tauri backend behavior, commands, ports, filesystem, or persistence changes are required.
- **Shared Core Before Shared UI**: PASS. Mermaid render state already lives in shared package. This feature adds a shared UI contract only because MA and AW markdown preview already consume the same shared `MarkdownViewer`, and AW agent-run already consumes the same `MermaidDiagram`.
- **Atomic Cross-App Verification**: PASS. `packages/markdown-annotation-react` changes require package tests plus MA and AW type checks/tests for affected consumer apps.
- **Documentation and Storybook**: PASS. Storybook updates are required for reusable Mermaid expanded modal states. A Korean `docs/*.md` architecture note is required only if implementation changes the shared Markdown/Mermaid contract beyond the plan described here.
- **Testing and Safety**: PASS. Unit/fixture tests are planned for rendered trigger, fallback exclusion, zoom/fit body, ordinary code preservation, reload/file-switch stability. No root/path/session-owner validation is introduced because no file or session mutation is added.

## Project Structure

### Documentation (this feature)

```text
specs/016-mermaid-modal-preview/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── mermaid-expanded-view-ui.md
└── tasks.md
```

### Source Code (repository root)

```text
packages/markdown-annotation-react/src/
├── MermaidDiagram.tsx
├── MermaidDiagram.test.tsx
├── MarkdownViewer.tsx
├── MarkdownViewer.test.tsx
├── types.ts
├── index.ts
└── styles.css

apps/markdown-annotator/src/
├── pages/annotator/AnnotatorPage.tsx
├── pages/annotator/*.test.tsx
├── shared/ui/markdown-viewer-components.tsx
├── shared/ui/annotation-dialog-components.tsx
└── stories/molecules/MarkdownViewer.stories.tsx

apps/agentic-workbench/src/
├── features/worktree-workspace/ui/worktree-workspace-panel.tsx
├── features/worktree-workspace/ui/markdown-viewer-components.tsx
├── features/worktree-workspace/ui/*.test.tsx
├── features/agent-run/ui/agent-run-markdown.tsx
├── features/agent-run/ui/agent-run-markdown-*.test.tsx
└── stories/organisms.stories.tsx
```

**Structure Decision**: 공유 가능한 Mermaid expanded view의 상태, body layout, zoom bounds, trigger contract는 `packages/markdown-annotation-react`에 둔다. MA와 AW의 Dialog/Tooltip/Button 합성 차이는 기존 `MarkdownViewerComponents` injection을 확장해 각 앱 adapter에서 처리한다. AW agent-run은 app-local Markdown parsing은 유지하되 shared expanded Mermaid component를 소비하도록 정리한다.

## Complexity Tracking

No constitution violations.

## Phase 0 Research

See [research.md](./research.md).

## Phase 1 Design

See [data-model.md](./data-model.md), [contracts/mermaid-expanded-view-ui.md](./contracts/mermaid-expanded-view-ui.md), and [quickstart.md](./quickstart.md).

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. Design artifacts keep shared React code in `packages/markdown-annotation-react` and app adapters in their owning apps.
- **Feature-Sliced Frontend Architecture**: PASS. AW worktree workspace remains in `features/worktree-workspace`, AW agent-run remains in `features/agent-run`, MA page-level wiring remains in `pages/annotator`, and reusable app primitives remain in `shared/ui` or `components/ui`.
- **Hexagonal Tauri Backend Architecture**: N/A. No backend work is designed.
- **Shared Core Before Shared UI**: PASS. The plan reuses existing shared Mermaid render state and introduces shared UI only for the converged expanded-view interaction across three consumers.
- **Atomic Cross-App Verification**: PASS. Quickstart lists package tests and both consuming app checks/tests.
- **Documentation and Storybook**: PASS. Storybook states are explicitly part of the validation path; docs are conditional on shared contract changes that need long-term architecture explanation.
- **Testing and Safety**: PASS. Contract and quickstart include fallback exclusion, ordinary code preservation, large diagram containment, and safety preservation through existing strict Mermaid rendering.

## Agent Context Update

No agent-context update script is present in this Spec Kit installation; no context files were generated or modified.
