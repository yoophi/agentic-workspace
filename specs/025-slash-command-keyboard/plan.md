# Implementation Plan: Slash Command Keyboard Navigation

**Branch**: `025-slash-command-keyboard` | **Date**: 2026-07-09 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/025-slash-command-keyboard/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Agentic Workbench의 slash command prompt 자동 완성에서 기존 prompt autocomplete 흐름을 유지하면서 keyboard-only 탐색을 안정화한다. 핵심 접근은 후보 목록을 multi-select나 command palette로 전면 교체하지 않고, 현재 `PromptCommandAutocomplete`와 `AgentRunPanel`의 highlight/selection 흐름을 보강해 `ArrowUp`/`ArrowDown` 이동, `Enter`/`Tab` 적용, `Escape` 취소, 그리고 강조 후보의 visible 영역 유지가 항상 동작하도록 만드는 것이다.

## Technical Context

**Language/Version**: TypeScript 5.6, React 19, Vite 7

**Primary Dependencies**: React, Radix/shadcn UI primitives, existing `cmdk` dependency only as reference; no new runtime dependency planned

**Storage**: N/A. This feature changes transient prompt UI state only.

**Testing**: Vitest, React static rendering tests, source-level regression tests where DOM focus/scroll behavior is difficult to assert in jsdom

**Target Platform**: Agentic Workbench desktop app frontend

**Project Type**: Tauri desktop application with React frontend

**Performance Goals**: Keyboard movement should feel immediate for typical command lists and must not introduce expensive work per key press beyond updating highlight and visible list state.

**Constraints**: Preserve existing prompt input, trigger detection, command filtering, single-candidate insertion, prompt history navigation, and mouse selection behavior. Do not convert the UI to multi-select chips or a separate command palette.

**Scale/Scope**: One Agentic Workbench prompt autocomplete surface, including long command lists of at least 20 candidates.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. Planned changes stay under `apps/agentic-workbench/src` and `specs/025-slash-command-keyboard`. No `packages/*`, `crates/*`, or app-to-app imports are required.
- **Feature-Sliced Frontend Architecture**: PASS. Interaction UI remains in `apps/agentic-workbench/src/features/agent-run/ui`. Existing command candidate model/filtering helpers remain in `apps/agentic-workbench/src/entities/agent-run/model`.
- **Hexagonal Tauri Backend Architecture**: N/A. No backend command, persistence, filesystem, ACP transport, or Tauri adapter change is planned.
- **Shared Core Before Shared UI**: PASS. No shared UI package is introduced. If pure helper changes are needed, they stay in the existing agent-run model boundary.
- **Atomic Cross-App Verification**: N/A. No `packages/*` or `crates/*` changes are planned.
- **Documentation and Storybook**: PASS. Feature documentation is under `specs/025-slash-command-keyboard`. Storybook is not required unless the autocomplete is promoted to a reusable component beyond the existing feature-local UI.
- **Testing and Safety**: PASS. Unit/source regression tests are planned for keyboard handling, list visibility behavior, no multi-select conversion, long text containment, and unchanged prompt insertion. No root/path/session-owner validation is involved.

## Project Structure

### Documentation (this feature)

```text
specs/025-slash-command-keyboard/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── prompt-command-autocomplete.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── entities/
│   └── agent-run/
│       └── model/
│           ├── prompt-autocomplete.ts
│           └── prompt-autocomplete.test.ts
└── features/
    └── agent-run/
        └── ui/
            ├── agent-run-panel.tsx
            ├── agent-run-panel.test.tsx
            ├── prompt-command-autocomplete.tsx
            └── prompt-command-autocomplete.test.tsx
```

**Structure Decision**: Implementation is feature-local to Agentic Workbench. `PromptCommandAutocomplete` owns candidate list rendering and visible highlighted item behavior. `AgentRunPanel` owns prompt textarea key routing, highlight state, candidate selection, and conflict with prompt history navigation. Existing `entities/agent-run/model` helpers remain the boundary for trigger, filtering, replacement, and highlight clamping.

## Phase 0: Research

See [research.md](./research.md).

Resolved decisions:

- Keep existing autocomplete component and do not replace it with multi-select or command palette UI.
- Guarantee highlighted candidate visibility by connecting highlighted item identity to list item refs and using nearest scrolling behavior when highlight changes.
- Keep keyboard event ownership in the prompt textarea because the textarea remains the focused editing control.
- Preserve prompt history arrow-key behavior when autocomplete is closed.

## Phase 1: Design & Contracts

See:

- [data-model.md](./data-model.md)
- [contracts/prompt-command-autocomplete.md](./contracts/prompt-command-autocomplete.md)
- [quickstart.md](./quickstart.md)

Agent context update: no `.specify/scripts` agent-context update script exists in this repository, so there is no generated agent context file to update for this plan.

## Constitution Check (Post-Design)

- **Monorepo Boundary First**: PASS. Design artifacts keep code changes under `apps/agentic-workbench/src` and do not introduce cross-app imports.
- **Feature-Sliced Frontend Architecture**: PASS. UI rendering/interaction remains in `features/agent-run/ui`; model helpers remain in `entities/agent-run/model`.
- **Hexagonal Tauri Backend Architecture**: N/A. Design does not require backend changes.
- **Shared Core Before Shared UI**: PASS. No shared UI extraction is proposed. Any reusable logic remains pure and local to the existing agent-run model boundary.
- **Atomic Cross-App Verification**: N/A. No shared package or crate changes are planned.
- **Documentation and Storybook**: PASS. Spec-kit documentation is complete. Storybook is optional because the component remains feature-local; tests are the required verification.
- **Testing and Safety**: PASS. Quickstart and contracts define focused UI/model tests plus manual keyboard validation. No filesystem, persistence, agent session owner, or permission safety change is involved.

## Complexity Tracking

No constitution violations.
