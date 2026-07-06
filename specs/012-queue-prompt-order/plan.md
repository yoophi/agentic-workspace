# Implementation Plan: Queue Prompt Order

**Branch**: `012-queue-prompt-order` | **Date**: 2026-07-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/012-queue-prompt-order/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Worktree Session Page의 최초 agent 실행 프롬프트 표시 순서를 정리한다. 현재는 첫 프롬프트가 agent-run timeline에 먼저 사용자 메시지처럼 표시된 뒤 agent 실행 메시지와 응답이 이어진다. 변경 후에는 첫 프롬프트를 기존 queued prompt와 같은 대기 상태로 먼저 보여주고, agent 세션이 준비되어 프롬프트를 실제로 처리할 때 실행 상태로 전환한 뒤 출력을 표시한다.

기술 접근은 `apps/agentic-workbench` 내부의 agent-run 상태 모델과 패널 조합을 조정하는 것이다. queue 상태 전환 규칙은 `features/agent-run/model/run-panel-state.ts`에 순수 함수로 두고, `features/agent-run/ui/agent-run-panel.tsx`는 첫 실행, saved prompt, external prompt, 일반 queue 경로가 같은 순서 규칙을 쓰도록 연결한다.

## Technical Context

**Language/Version**: TypeScript/React frontend inside the existing pnpm workspace; Rust/Tauri backend is only touched if prompt dispatch sequencing proves to require backend state alignment.

**Primary Dependencies**: React, TanStack Query, Tauri invoke/event bridge, existing shadcn/ui-based component primitives, Vitest, Storybook.

**Storage**: No new durable storage. Existing run events and prompt state remain runtime UI/session state.

**Testing**: Vitest for pure state transitions and UI behavior; Storybook story update for queued prompt first-run visual state if reusable/observable UI state changes; app type check via `pnpm --dir apps/agentic-workbench check-types`.

**Target Platform**: Agentic Workbench desktop app, specifically Worktree Session Page.

**Project Type**: Desktop app frontend feature with possible Tauri command integration checks.

**Performance Goals**: First prompt should appear as queued feedback immediately after Return, without perceptible delay; timeline order must remain stable during normal event streaming.

**Constraints**: Do not duplicate the first prompt in the timeline. Do not dispatch prompts to the wrong run. Preserve existing queue edit/move/remove behavior for prompts that are waiting.

**Scale/Scope**: One app (`apps/agentic-workbench`), one main surface (`ProjectWorktreeSessionPage` through `AgentRunPanel`), existing agent-run entity/model surface.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. Scope is app-specific and stays under `apps/agentic-workbench`; no `packages/*`, `crates/*`, or app-to-app imports are planned.
- **Feature-Sliced Frontend Architecture**: PASS. Page composition remains in `pages/project-worktree-session`; prompt queue actions and UI state remain in `features/agent-run`; run event types remain in `entities/agent-run`; reusable UI primitives are only used from existing locations.
- **Hexagonal Tauri Backend Architecture**: PASS/N/A. The primary plan is frontend state/timeline ordering. If backend sequencing is needed, Tauri commands remain inbound adapters and dispatch/session readiness rules must live in application-level services, not directly in commands.
- **Shared Core Before Shared UI**: PASS. No shared UI extraction is planned. Shared logic, if any, is limited to pure queue/timeline state helpers in the local feature model.
- **Atomic Cross-App Verification**: N/A. No `packages/*` or `crates/*` changes are planned.
- **Documentation and Storybook**: PASS. Storybook should be updated if the queued first-run state is exposed as a reusable state. `docs/*.md` is optional because this is a narrow UI flow unless implementation reveals broader agent startup flow documentation.
- **Testing and Safety**: PASS. Unit tests are planned for pure queue state; UI tests/Storybook coverage are planned for first-run ordering. Session/run owner safety is preserved by using the active run ID and existing agent-run repository calls.

## Project Structure

### Documentation (this feature)

```text
specs/012-queue-prompt-order/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── agent-run-prompt-order.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── pages/project-worktree-session/
│   └── ui/project-worktree-session-page.tsx
├── features/agent-run/
│   ├── model/run-panel-state.ts
│   ├── model/run-panel-state.test.ts
│   └── ui/agent-run-panel.tsx
├── entities/agent-run/
│   ├── api/agent-run-repository.ts
│   └── model/types.ts
└── stories/
    └── organisms.stories.tsx
```

**Structure Decision**: 구현은 `features/agent-run`에 집중한다. `ProjectWorktreeSessionPage`는 `AgentRunPanel`을 포함하는 화면 조합만 유지한다. prompt queue 상태 전환은 `model/run-panel-state.ts`에서 테스트 가능한 순수 함수로 정리하고, 실제 입력/표시는 `ui/agent-run-panel.tsx`에서 연결한다. `entities/agent-run`은 기존 run event 타입과 repository 호출을 재사용하며, 새 외부 API 계약은 만들지 않는다.

## Complexity Tracking

헌법 위반이 없으므로 별도 complexity exception은 없다.

## Phase 0: Research

See [research.md](./research.md).

## Phase 1: Design & Contracts

See [data-model.md](./data-model.md), [contracts/agent-run-prompt-order.md](./contracts/agent-run-prompt-order.md), and [quickstart.md](./quickstart.md).

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. Design artifacts keep work under `apps/agentic-workbench`.
- **Feature-Sliced Frontend Architecture**: PASS. Data model assigns queue transition helpers to feature model and run event types to existing entity model.
- **Hexagonal Tauri Backend Architecture**: PASS/N/A. The UI contract avoids backend-specific changes; any later backend adjustment must keep commands as adapters.
- **Shared Core Before Shared UI**: PASS. No shared UI package is introduced.
- **Atomic Cross-App Verification**: N/A. No shared packages/crates are involved.
- **Documentation and Storybook**: PASS. Quickstart identifies Storybook update only if the implementation changes reusable queued prompt states.
- **Testing and Safety**: PASS. Quickstart requires unit tests for ordering and manual/E2E validation for duplicate-free first-run prompt display.

## Agent Context Update

No agent context update script exists in `.specify/scripts/bash/` for this repository. This step is recorded as skipped with no generated context changes.
