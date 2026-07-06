# Implementation Plan: Main and Extra Agent Run Panels

**Branch**: `013-main-extra-agent-panels` | **Date**: 2026-07-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/013-main-extra-agent-panels/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Worktree Session의 agent 영역을 단일 `AgentRunPanel`에서 main/extra 탭 구조로 확장한다. main 패널은 항상 존재하고 goal continuation과 settings persistence를 유지하며, extra 패널은 세션 생명주기 동안 수동 prompt 실행, active-panel annotation prompt routing, 독립 timeline/queue/run 상태, 실행 중 close confirmation과 cleanup을 제공한다. 기술 접근은 `apps/agentic-workbench` 내부에서 page composition을 `WorktreeAgentRunArea`로 분리하고, 기존 `AgentRunPanel`에 panel identity, run-state callback, goal/settings feature flags, panel-scoped resizable ids를 추가하는 것이다. backend는 기존 다중 run registry/cancel 흐름을 우선 사용하되, extra close cleanup과 late-event discard가 검증되도록 tests와 오류 메시지를 보강한다.

## Technical Context

**Language/Version**: TypeScript 5.6, React 19, Rust/Tauri 2

**Primary Dependencies**: Vite, TanStack Query, react-resizable-panels, lucide-react, shadcn/ui local components, Tauri command bridge, existing ACP agent-run backend

**Storage**: Existing JSON-backed agent run settings and goal repositories. No new persistent storage for extra panel layout in MVP.

**Testing**: Vitest for frontend model/UI behavior; `pnpm --filter @yoophi/agentic-workbench check-types`; `pnpm --filter @yoophi/agentic-workbench test`; Rust `cargo test` or targeted `cargo test -p agentic-workbench` equivalent from `apps/agentic-workbench/src-tauri` if backend cleanup code changes.

**Target Platform**: `agentic-workbench` desktop app running inside Tauri on local worktrees.

**Project Type**: Desktop app with React frontend and Tauri/Rust backend.

**Performance Goals**: Creating/switching an extra tab should feel immediate and meet spec target of usable prompt input within 2 seconds under normal local conditions. Multiple panel event filtering must not visibly slow timeline rendering for active panel use.

**Constraints**: Main panel remains non-removable. Extra panel state is session-local. Goal continuation and persisted settings must remain main-only. Existing owner-window cancellation behavior must continue. Close cleanup must be idempotent and must not leave active run, permission, prompt queue, or event listener state for closed extra panels.

**Scale/Scope**: One Worktree Session page, one required main panel, several user-created extra panels in the same React tree and same Tauri window. Scope excludes cross-window panels, persisted layout restore, automatic conversation merge, and panel-specific worktree change views.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. Scope stays under `apps/agentic-workbench`; no `packages/*`, `crates/*`, or app-to-app imports are planned.
- **Feature-Sliced Frontend Architecture**: PASS. Page-level split stays in `pages/project-worktree-session`; agent panel orchestration belongs in `features/agent-run`; agent API/model types stay in `entities/agent-run`; reusable primitives remain in `components/ui` or `shared` only when already established.
- **Hexagonal Tauri Backend Architecture**: PASS. Backend changes, if needed, are limited to application use cases (`cancel_agent_run`, error mapping) and infrastructure registry tests. Tauri commands continue delegating to application services.
- **Shared Core Before Shared UI**: PASS. No cross-app shared UI is introduced. Panel state logic remains app-local unless later consumed by another app.
- **Atomic Cross-App Verification**: N/A. No shared package or crate changes are planned.
- **Documentation and Storybook**: PASS. New reusable agent tab/area UI requires Storybook coverage in `apps/agentic-workbench/src/stories/organisms.stories.tsx` or a colocated story pattern if introduced. Korean docs update is only needed if delivered behavior diverges from `docs/main-extra-agent-run-panels-design.md`.
- **Testing and Safety**: PASS. Plan includes unit tests for panel reducer/state transitions, UI tests for tabs/routing/close confirmation, and backend tests for run cancellation/permission cleanup if touched.

## Project Structure

### Documentation (this feature)

```text
specs/013-main-extra-agent-panels/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── agent-panel-ui-contract.md
│   └── extra-panel-cleanup-contract.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── pages/project-worktree-session/ui/
│   └── project-worktree-session-page.tsx
├── features/agent-run/
│   ├── model/
│   │   ├── agent-run-panel-slots.ts
│   │   └── agent-run-panel-slots.test.ts
│   └── ui/
│       ├── agent-run-panel.tsx
│       ├── agent-run-panel-tabs.tsx
│       └── worktree-agent-run-area.tsx
├── features/worktree-workspace/ui/
│   └── worktree-workspace-panel.tsx
├── entities/agent-run/
│   ├── api/agent-run-repository.ts
│   └── model/
└── stories/
    └── organisms.stories.tsx

apps/agentic-workbench/src-tauri/src/
├── application/
│   ├── cancel_agent_run.rs
│   ├── send_prompt.rs
│   └── start_agent_run.rs
├── inbound/
│   └── tauri_commands.rs
├── infrastructure/
│   ├── agent_session_registry.rs
│   └── permission_broker.rs
└── ports/
    └── session_registry.rs
```

**Structure Decision**: 구현은 `apps/agentic-workbench` 내부 변경으로 제한한다. `ProjectWorktreeSessionPage`는 좌우 split과 worktree 전달만 유지하고, agent 영역의 탭/slot/prompt routing은 `features/agent-run/ui/worktree-agent-run-area.tsx`로 이동한다. 순수 slot 전환/삭제/cleanup 결정 로직은 `features/agent-run/model/agent-run-panel-slots.ts`에 두어 UI 없이 테스트한다. backend는 기존 cancel/registry 흐름이 요구사항을 만족하면 테스트만 보강하고, 부족한 경우 application/infrastructure 경계를 지켜 수정한다.

## Phase 0: Research

See [research.md](./research.md).

## Phase 1: Design & Contracts

See [data-model.md](./data-model.md), [agent-panel-ui-contract.md](./contracts/agent-panel-ui-contract.md), [extra-panel-cleanup-contract.md](./contracts/extra-panel-cleanup-contract.md), and [quickstart.md](./quickstart.md).

## Constitution Check - Post Design

- **Monorepo Boundary First**: PASS. All design artifacts keep changes under `apps/agentic-workbench`.
- **Feature-Sliced Frontend Architecture**: PASS. Data model assigns orchestration to `features/agent-run`, page composition to `pages`, and API interactions to `entities/agent-run`.
- **Hexagonal Tauri Backend Architecture**: PASS. Cleanup contract uses existing command/use-case/registry separation and does not put registry logic into Tauri commands.
- **Shared Core Before Shared UI**: PASS. No shared UI/package extraction is proposed.
- **Atomic Cross-App Verification**: N/A. No shared package or crate changes.
- **Documentation and Storybook**: PASS. Quickstart includes Storybook/build-storybook validation for the reusable agent area/tabs story.
- **Testing and Safety**: PASS. Quickstart and contracts require run owner/cancel cleanup, late event discard, and panel-local queue/permission cleanup validation.

## Complexity Tracking

No constitution violations.
