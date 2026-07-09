# Implementation Plan: Session Lifecycle Status

**Branch**: `026-session-lifecycle-status` | **Date**: 2026-07-10 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/026-session-lifecycle-status/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Agentic Workbench의 agent run 화면에서 새 session 시작과 idle 진입을 짧고 낮은 강조도의 lifecycle/status 메시지로 표시한다. 기존 `session_info_update` raw JSON suppression, header thread status badge, available commands summary는 유지하고, session status 전환을 run-local dedupe 규칙으로 timeline/status 흐름에 통합한다. Backend event contract와 persistence는 변경하지 않는다.

## Technical Context

**Language/Version**: TypeScript 5.6, React 19, Vite 7

**Primary Dependencies**: Existing React UI, existing agent-run model helpers, existing Storybook/Vitest setup

**Storage**: N/A. Status messages are transient run-local UI state.

**Testing**: Vitest unit/source regression tests, Storybook state coverage for lifecycle/status display, Agentic Workbench type checks and tests

**Target Platform**: Agentic Workbench desktop app frontend

**Project Type**: Tauri desktop application with React frontend

**Performance Goals**: Status transition handling should be constant-time relative to current run state and should not add duplicate timeline items under repeated status updates.

**Constraints**: Preserve raw `session_info_update` suppression, preserve current header status badge and command summary behavior, avoid backend/persistence changes, and avoid mixing status messages between runs.

**Scale/Scope**: One Agentic Workbench run/session UI flow with repeated `active`/`idle` session updates and existing lifecycle timeline items.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. Planned code changes stay under `apps/agentic-workbench/src` plus `specs/026-session-lifecycle-status`. No cross-app imports or shared packages are required.
- **Feature-Sliced Frontend Architecture**: PASS. UI/status rendering stays in `features/agent-run`; parsing/formatting/dedupe helpers stay in `entities/agent-run/model`.
- **Hexagonal Tauri Backend Architecture**: N/A. No Tauri command, backend port, filesystem, ACP transport, or persistence change is planned.
- **Shared Core Before Shared UI**: PASS. Any reusable behavior is pure model logic first. No shared UI extraction is proposed.
- **Atomic Cross-App Verification**: N/A. No `packages/*` or `crates/*` changes are planned.
- **Documentation and Storybook**: PASS. Feature docs are under `specs/026-session-lifecycle-status`; Storybook or rendering test coverage is planned for visible status states.
- **Testing and Safety**: PASS. Model tests are planned for status transition/dedupe and UI/source tests for suppression and rendering. Run scope is explicitly preserved; no file/persistence/session-owner mutation is introduced.

## Project Structure

### Documentation (this feature)

```text
specs/026-session-lifecycle-status/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── session-lifecycle-status.md
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
│           ├── format.ts
│           ├── format.test.ts
│           ├── types.ts
│           └── index.ts
├── features/
│   └── agent-run/
│       ├── model/
│       │   ├── run-panel-state.ts
│       │   └── run-panel-state.test.ts
│       └── ui/
│           ├── agent-run-panel.tsx
│           └── agent-run-panel.test.tsx
└── stories/
    └── organisms.stories.tsx
```

**Structure Decision**: Status parsing and dedupe should live in the existing `entities/agent-run/model` boundary when pure. Panel state integration belongs in `features/agent-run/model/run-panel-state.ts` and the live UI listener in `features/agent-run/ui/agent-run-panel.tsx`. Visual examples belong in existing organism Storybook stories because the agent run panel/status timeline is an organism-level work surface.

## Phase 0: Research

See [research.md](./research.md).

Resolved decisions:

- Convert meaningful session status transitions into concise lifecycle/status messages rather than raw timeline JSON.
- Keep the current header thread status badge as the current-state indicator and use timeline/status messages only for meaningful transitions.
- Dedupe repeated identical statuses per run.
- Do not re-implement available command summary/detail UI.
- Keep backend contract unchanged.

## Phase 1: Design & Contracts

See:

- [data-model.md](./data-model.md)
- [contracts/session-lifecycle-status.md](./contracts/session-lifecycle-status.md)
- [quickstart.md](./quickstart.md)

Agent context update: no `.specify/scripts` agent-context update script exists in this repository, so there is no generated agent context file to update for this plan.

## Constitution Check (Post-Design)

- **Monorepo Boundary First**: PASS. Design keeps changes app-local to `apps/agentic-workbench`.
- **Feature-Sliced Frontend Architecture**: PASS. Pure status transition logic is in `entities`; state integration and UI rendering are in `features`.
- **Hexagonal Tauri Backend Architecture**: N/A. Backend remains unchanged.
- **Shared Core Before Shared UI**: PASS. No shared UI package is introduced.
- **Atomic Cross-App Verification**: N/A. No shared package/crate change is planned.
- **Documentation and Storybook**: PASS. Storybook/rendering verification is identified for visible status states.
- **Testing and Safety**: PASS. Tests cover run-local dedupe, raw suppression, malformed updates, and UI coexistence with command summary.

## Complexity Tracking

No constitution violations.
