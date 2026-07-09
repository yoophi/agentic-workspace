# Implementation Plan: AW Active-Turn Steer

**Branch**: `020-aw-active-turn-steer` | **Date**: 2026-07-09 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/020-aw-active-turn-steer/spec.md`

## Summary

AW의 steer 동작을 현재의 "실행 중인 run 취소 후 새 run 시작" 중심 흐름에서, 지원 가능한 provider/session에서는 현재 작업에 pending steer 입력을 추가하는 흐름으로 개선한다. 계획은 먼저 frontend prompt dispatch 상태를 queue와 steer가 분리된 순수 모델로 정리하고, backend에는 active session에 대한 steer capability/command/use case 경계를 추가한다. provider가 cancel-free steer를 지원하지 않는 경우에는 입력을 보존한 채 queue 이동 또는 명시적 restart-with-steering으로 fallback한다.

## Technical Context

**Language/Version**: TypeScript 5.6, React 19, Rust edition 2024

**Primary Dependencies**: Vite, Vitest, Tauri 2, agent-client-protocol 0.15.0, Tokio

**Storage**: v1은 영구 저장 없음. prompt/steer/queue 상태는 active AW panel/run 상태로 관리한다. provider session record는 기존 ACP session store를 그대로 사용한다.

**Testing**: Vitest for frontend state/model tests, Storybook for reusable UI states, Cargo test/check for Tauri application/domain/infrastructure changes

**Target Platform**: macOS desktop Tauri app, Agentic Workbench worktree session UI

**Project Type**: desktop app with React frontend and Tauri/Rust backend

**Performance Goals**: 실행 중 steer/queue 상태 변경은 사용자 입력 후 100ms 이내에 UI에 반영된다. queue/steer 목록 50개까지 재정렬, 표시, 상태 전환이 지연 없이 동작한다.

**Constraints**: 사용자가 명시적으로 restart를 선택하지 않는 한 steer 제출이 `cancel_agent_run`으로 이어지면 안 된다. 늦게 도착한 이전 run 이벤트는 현재 run의 queue/pending steer 상태를 변경하면 안 된다. session owner/run identity 검증을 유지해야 한다.

**Scale/Scope**: `apps/agentic-workbench`의 단일 worktree session panel 입력 흐름. cross-app package/crate 추출은 범위 밖이다.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. 모든 소스 변경은 `apps/agentic-workbench`와 `docs/aw-active-turn-steer.md`에 한정한다. `packages/*`, `crates/*` 신규 공유 모듈은 만들지 않는다.
- **Feature-Sliced Frontend Architecture**: PASS. prompt dispatch 순수 상태와 helper는 `apps/agentic-workbench/src/features/agent-run/model`, Tauri invoke adapter는 `apps/agentic-workbench/src/entities/agent-run/api`, 화면 조합은 `apps/agentic-workbench/src/features/agent-run/ui`에 둔다.
- **Hexagonal Tauri Backend Architecture**: PASS. `SessionHandle` capability/steer port는 `ports`/`domain` 경계에 두고, use case는 `application`, Tauri command는 `inbound`, ACP 세부 구현은 `infrastructure/acp`에 둔다.
- **Shared Core Before Shared UI**: PASS. 먼저 feature-local 순수 reducer/state transition을 검증한다. UI 공유 패키지는 만들지 않는다.
- **Atomic Cross-App Verification**: N/A. `packages/*` 또는 `crates/*` 변경 계획이 없다.
- **Documentation and Storybook**: PASS. `docs/aw-active-turn-steer.md`를 추가하고, pending steer/queued prompt UI가 재사용 컴포넌트로 분리되면 `apps/agentic-workbench/src/stories/organisms.stories.tsx`에 상태별 story를 추가한다.
- **Testing and Safety**: PASS. `run-panel-state.test.ts`에 순수 상태 전이 테스트를 추가하고, Rust use case/port tests에서 empty prompt, inactive run, unsupported capability, owner/run identity event handling을 검증한다.

## Project Structure

### Documentation (this feature)

```text
specs/020-aw-active-turn-steer/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── frontend-state.md
│   └── tauri-commands.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── entities/agent-run/
│   ├── api/agent-run-repository.ts
│   └── model/types.ts
├── features/agent-run/
│   ├── model/run-panel-state.ts
│   ├── model/run-panel-state.test.ts
│   └── ui/agent-run-panel.tsx
└── stories/organisms.stories.tsx

apps/agentic-workbench/src-tauri/src/
├── domain/events.rs
├── application/send_prompt.rs
├── application/steer_prompt.rs
├── inbound/tauri_commands.rs
├── infrastructure/acp/runner.rs
└── ports/session_handle.rs

docs/
└── aw-active-turn-steer.md
```

**Structure Decision**: 기존 AW 입력 흐름이 이미 `features/agent-run/model`과 `features/agent-run/ui`에 있으므로, state/reducer 확장은 feature layer에 둔다. backend는 기존 `send_prompt`와 같은 use case pattern을 따르되, steer의 의미가 send/cancel/restart와 다르므로 별도 `steer_prompt` use case와 `SessionHandle` capability method를 둔다. provider별 실제 steer 구현은 ACP runner adapter 내부에 캡슐화한다.

## Phase 0: Research

Research output: [research.md](./research.md)

Resolved questions:

- active-turn steer를 지원하지 않는 ACP provider의 기본 동작
- queue와 steer의 state model 분리 기준
- 늦은 lifecycle event가 새 run 상태를 지우는 race 방지 방식
- backend command/use case/port 경계
- Storybook 및 documentation 범위

## Phase 1: Design & Contracts

Design outputs:

- [data-model.md](./data-model.md)
- [contracts/frontend-state.md](./contracts/frontend-state.md)
- [contracts/tauri-commands.md](./contracts/tauri-commands.md)
- [quickstart.md](./quickstart.md)

Agent context update: `.specify/scripts/bash/update-agent-context.sh` is absent in this repository, so no agent context update script was run.

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. Design artifacts keep implementation inside `apps/agentic-workbench` plus `docs/`.
- **Feature-Sliced Frontend Architecture**: PASS. Contracts assign pure state to `features`, Tauri invoke types/adapters to `entities`, and screen rendering to `features/ui`.
- **Hexagonal Tauri Backend Architecture**: PASS. Contract separates inbound command, application use case, `SessionHandle` port, and ACP infrastructure.
- **Shared Core Before Shared UI**: PASS. No shared UI package. Reusable UI work is app-local Storybook only.
- **Atomic Cross-App Verification**: N/A. No workspace package/crate changes.
- **Documentation and Storybook**: PASS. Quickstart includes docs and Storybook validation when UI component extraction occurs.
- **Testing and Safety**: PASS. Data model and contracts include run/session target checks, unsupported capability handling, and late event isolation.

## Complexity Tracking

No constitution violations.
