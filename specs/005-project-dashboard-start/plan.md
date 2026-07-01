# Implementation Plan: 프로젝트 대시보드 시작화면

**Branch**: `102-issue` | **Date**: 2026-07-02 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/005-project-dashboard-start/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

AW 앱의 루트 시작화면을 기존 프로젝트 목록 중심 화면에서 프로젝트 대시보드로 개선한다. 구현은 AW 내부 FSD 경계를 유지하면서 대시보드 page를 추가하고, 프로젝트/세션/worktree 요약을 읽기 전용 summary model로 조합하며, 새 프로젝트 생성/기존 프로젝트 열기/최근 작업 재개 action을 첫 화면에 배치한다. backend는 기존 조회 command로 충족 가능한 범위를 우선 사용하고, 최근 세션 또는 상태 요약에 부족한 조회가 있을 때만 hexagonal 경계를 따라 확장한다.

## Technical Context

**Language/Version**: TypeScript 5.6, React 19, Rust/Tauri 2

**Primary Dependencies**: React Router 7, TanStack Query 5, Zustand, shadcn/ui local components, lucide-react, Tauri invoke bridge

**Storage**: Existing app-local JSON persistence for projects/session-related records, Git/worktree state from existing providers; no new storage file is planned for v1

**Testing**: Vitest for TypeScript model/UI helper tests, Storybook page/state stories, Cargo test/check if backend query behavior changes

**Target Platform**: Tauri desktop app for macOS and compatible desktop platforms supported by AW

**Project Type**: Desktop application in a pnpm/Turbo monorepo with a Tauri backend

**Performance Goals**: Initial dashboard should keep the first useful screen available within 2 seconds for typical project counts, and should not block main project actions while optional summaries are loading

**Constraints**: Operational, dense, scan-friendly UI; no marketing-style landing composition; keep app-specific code under `apps/agentic-workbench`; preserve route compatibility for project detail and session/worktree routes

**Scale/Scope**: AW start screen only; expected to summarize tens of projects and a small set of recent/resumable sessions or worktrees per project

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. Scope is limited to `apps/agentic-workbench` and spec docs under `specs/005-project-dashboard-start`; no app-to-app imports or new shared workspace packages are planned.
- **Feature-Sliced Frontend Architecture**: PASS. Screen-level dashboard UI belongs in `apps/agentic-workbench/src/pages/project-dashboard`; dashboard actions belong in `features`; project/session/worktree summary types and adapters belong in `entities`; reusable local UI atoms belong in `shared` or `components/ui`.
- **Hexagonal Tauri Backend Architecture**: PASS. Backend changes are optional. If needed, domain models/ports stay in `domain`, use cases in `application`, commands in `inbound`, and filesystem/Git/session reads in `infrastructure`.
- **Shared Core Before Shared UI**: PASS. No cross-app shared UI is proposed. Reuse remains AW-local unless at least two apps need the same headless core.
- **Atomic Cross-App Verification**: N/A. No `packages/*` or `crates/*` changes are planned.
- **Documentation and Storybook**: PASS. Page-level Storybook examples are required for recent projects, empty, loading, error, and long-content states. No separate `docs/*.md` is required unless design decisions grow beyond this plan.
- **Testing and Safety**: PASS. Pure summary/ranking helpers get unit tests. Any backend state lookup must retain project root/path validation and session owner scope, surfacing inaccessible data as dashboard status rather than crashing the screen.

## Project Structure

### Documentation (this feature)

```text
specs/005-project-dashboard-start/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── dashboard-ui-contract.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── app/
│   ├── App.tsx                         # route composition and root dashboard wiring
│   └── model/                          # route helpers if dashboard introduces route helpers
├── pages/
│   └── project-dashboard/
│       └── ui/
│           └── project-dashboard-page.tsx
├── features/
│   ├── project-form/ui/                # existing create/open project actions
│   ├── project-list/ui/                # existing table remains for full list/detail flows
│   └── project-dashboard/ui/           # dashboard-specific reusable action panels if needed
├── entities/
│   ├── project/
│   │   ├── api/                        # existing project/worktree adapters; optional summary adapter
│   │   ├── lib/                        # dashboard ranking/summary helpers and tests
│   │   └── model/                      # dashboard summary types if project-owned
│   └── agent-run/                      # existing session/run models if recent session summary is used
├── shared/
│   ├── storybook/sample-data.ts        # dashboard state fixtures
│   └── ui/                             # local reusable display primitives only if broadly useful
├── components/ui/                      # existing shadcn/ui primitives only
└── stories/
    └── pages.stories.tsx               # dashboard page states

apps/agentic-workbench/src-tauri/src/
├── domain/                             # optional dashboard summary domain structs/ports
├── application/                        # optional dashboard summary use case
├── inbound/tauri_commands.rs           # optional command delegation only
└── infrastructure/                     # optional project/session/worktree state readers
```

**Structure Decision**: Implement the first screen as an AW-local `pages/project-dashboard` page and wire `/` to it from `app/App.tsx`. Keep the existing `ProjectListPage`, project detail route, and worktree/session route as downstream workflows. Keep summary derivation in `entities/project/lib` with tests so the dashboard page remains presentation-oriented.

## Complexity Tracking

No constitution violations are planned.

## Phase 0: Research

Research is captured in [research.md](./research.md). The plan resolves the main design choices: dashboard composition, data source strategy, recent item ranking, status degradation, and validation approach.

## Phase 1: Design & Contracts

Design artifacts:

- [data-model.md](./data-model.md)
- [contracts/dashboard-ui-contract.md](./contracts/dashboard-ui-contract.md)
- [quickstart.md](./quickstart.md)

Agent context update: no `.specify/scripts/*/update-agent-context*` script exists in this repository, so there was no script to run for this step.

## Constitution Check (Post-Design)

- **Monorepo Boundary First**: PASS. Design artifacts keep implementation within `apps/agentic-workbench` and feature docs under `specs/005-project-dashboard-start`.
- **Feature-Sliced Frontend Architecture**: PASS. The UI contract separates page, feature actions, entities, shared UI, and shadcn/ui boundaries.
- **Hexagonal Tauri Backend Architecture**: PASS. The data model and contract allow a frontend-only composition using existing commands first; optional backend expansion follows domain/application/inbound/infrastructure separation.
- **Shared Core Before Shared UI**: PASS. No shared workspace UI package is introduced.
- **Atomic Cross-App Verification**: N/A. No shared packages or crates are planned.
- **Documentation and Storybook**: PASS. Quickstart requires Storybook page states for dashboard validation.
- **Testing and Safety**: PASS. Data model identifies validation and degraded-state handling for unavailable project/session/worktree data.
