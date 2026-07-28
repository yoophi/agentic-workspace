# Implementation Plan: Main Coordinator 기반 에이전트 오케스트레이션

**Branch**: `main` | **Date**: 2026-07-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/033-agent-orchestration/spec.md`

## Summary

AW Worktree Session의 `main-agent-run`을 안정적인 Main Coordinator Node로 두고, 이후
생성되는 모든 Node를 Main의 직접 Child로 관리한다. 사용자 목표는 durable task aggregate에
저장하고 Main이 run-scoped MCP 도구로 역할별 Child task를 생성·대기·수집한다. 실제
Worker는 기존 ACP 실행 primitive를 감싼 AW app-local adapter가 읽기 전용으로 실행한다.
명시적 구조화 결과만 task 완료로 인정하며 prompt/process/file 신호는 보조 상태로만
사용한다.

Frontend는 기존 탭·타일 projection과 안정적인 panel identity를 유지하면서
`AgentRunPanel`의 prompt/run/timeline controller를 표시 UI에서 분리한다. 워크스페이스
단일 Composer가 focused/selected/all/coordinator 대상을 라우팅하고, Activity Rail이
background task를 표시한다. 승격/분리는 presentation과 tile leaf만 변경하여 같은 run과
timeline을 유지한다.

현재 vertical slice에서 발견된 승격 결함을 해결하기 위해 runtime controller의 소유권을
panel component에서 Worktree Session workspace로 올린다. background observer와 visible
panel은 Node/run별 동일 controller를 사용하며, panel 승격 시 `Node.currentRunId`로
journal snapshot을 timeline reducer에 적용하고 last sequence 이후 live event를 이어
받는다. panel의 빈 mount state는 기존 run binding을 갱신할 수 없다.

양방향 통신은 durable `TaskCommand` outbox와 `CoordinatorNotification` inbox/outbox로
완성한다. 사용자 입력 응답, UI direct command와 Coordinator MCP command는 동일한
application command service가 current task attempt/run binding을 검증한 뒤 worker port로
전달한다. Child report는 저장 transaction에서 Main notification을 만들며, active Main은
notification을 받고 MCP로 구조화된 report 원문을 조회한다. runtime 수락 전에는 task를
앞서 전이하지 않으며 retry/reassign도 실제 scheduling과 새 worker launch까지 수행한다.

## Technical Context

**Language/Version**: TypeScript 5.6+, React 19, Rust 2024 edition, Tauri 2

**Primary Dependencies**: 기존 React/Vite/TanStack Query/Zustand/shadcn UI,
`react-resizable-panels`, Tauri invoke/listen, Axum local MCP server, Tokio,
Serde/serde_json, existing `acp-agent-core` run/session primitives, existing
`AgentExchangeService`와 `AgentRunPanel` reducers

**Storage**: app data의 atomic JSON `orchestration-sessions.json`에 workspace aggregate,
revision, tasks, reports, TaskCommand outbox, CoordinatorNotification inbox/outbox,
dispatch와 idempotency 결과를 저장한다. active run timeline rehydration은 window-scoped
bounded in-memory event journal을 사용한다.

**Testing**: Vitest/React Testing Library for pure reducers, controller and UI contracts;
Storybook build/a11y states; Rust unit/application/infrastructure tests for state machines,
repository recovery, scheduler, capability authorization, ownership and adapter mapping;
AW TypeScript check/test/build와 AW Tauri cargo test/check

**Target Platform**: macOS desktop Tauri Worktree Session WebView. 기존 cross-platform
Tauri command/domain 계약은 유지하되 1차 수동 검증은 macOS AW 개발 앱에서 수행한다.

**Project Type**: pnpm/Turbo + Cargo monorepo의 desktop app frontend/backend feature

**Performance Goals**: local Composer/Activity Rail interaction 200ms 이내; task/report
변경 후 1초 이내 UI 반영; 목표 submit 후 30초 이내 세 Child의 생성·배정 확인; 8 Node와
4 active run 대표 시나리오에서 responsive UI 유지; bounded MCP wait 최대 30초;
panel 승격 후 journal 보존 범위의 timeline을 1초 이내 표시; Child
input/result/blocked notification을 active Main과 UI에 1초 이내 반영

**Constraints**: Main Node 정확히 하나; Child는 Main의 직접 자식 한 단계; 동일
orchestration workspace/window/canonical worktree scope; automatic Child는 read-only
capable profile만 사용; prompt 16KiB 상한; mutating request idempotency와 expected
revision; 명시적 result만 completion; Main generation 자동 인계 금지; 기존 tab/tile,
타일 진입 `1:1:1`, panel limit 8/depth 4와 peer exchange 회귀 금지; 동일 Child run에
runtime controller 하나; panel mount state는 authoritative run
binding을 덮어쓸 수 없음; snapshot/live event는 sequence로 exact-once 적용; runtime
delivery accepted 전 task 상태 선행 전이 금지; full-payload idempotency; attempt/run fence;
UI와 MCP runtime command parity

**Scale/Scope**: Worktree Session workspace당 Node 최대 8개, 대표 active run Main+Child
3개, 작은 task DAG와 구조화 reports; `apps/agentic-workbench`와 `docs/specs`만 변경하며
`packages/*`, `crates/*`, 다른 앱은 변경하지 않음

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. 오케스트레이션은 Main panel/worktree/UI에 종속된
  AW app-local 기능으로 `apps/agentic-workbench`에 둔다. 기존 `acp-agent-core`와
  `packages/agent-client`는 공개 primitive를 소비만 하며 app-to-app import가 없다.
- **Feature-Sliced Frontend Architecture**: PASS. 순수 orchestration 타입/API는
  `entities/agent-orchestration`, workspace action/controller/UI는 기존
  `features/agent-run`, 화면 조립은 `pages/project-worktree-session`, 범용 primitive는
  `components/ui` 경계를 따른다.
- **Hexagonal Tauri Backend Architecture**: PASS. 관계·상태 전이는
  `domain/agent_orchestration.rs`, scheduler/권한/use case는 `application`,
  repository/worker/event journal은 `ports`, JSON/ACP/Tauri event/MCP는
  `infrastructure`, Tauri command는 `inbound`에서 service에 위임한다.
- **Shared Core Before Shared UI**: PASS. task state machine, relationship validation,
  dispatch reducer와 runtime mapping을 pure app-local core로 먼저 구현한다. 두 번째
  소비자가 없으므로 새 shared package/UI는 만들지 않는다.
- **Atomic Cross-App Verification**: N/A. `packages/*`와 `crates/*`를 변경하지 않는다.
  구현 중 공유 코어 변경이 필요해지면 해당 consumer 검증을 tasks에 추가하기 전에는
  범위를 넓히지 않는다.
- **Documentation and Storybook**: PASS. `docs/agent-orchestration-workspace.md`를
  유지하고 status badge, target selector, dispatch status, Activity item, Composer,
  Rail, handoff와 대표 page를 atomic category별 Storybook에 추가한다.
- **Testing and Safety**: PASS. pure state transitions와 idempotency는 단위 테스트,
  JSON atomic/backup 복구는 fixture 테스트, canonical path/window/run/node/task/generation
  owner 검증은 application/infrastructure 테스트로 계획했다. 읽기 전용 profile,
  mutation deny와 change violation 감지를 검증한다.

## Project Structure

### Documentation (this feature)

```text
specs/033-agent-orchestration/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── orchestration-service.md
│   ├── orchestration-mcp.md
│   ├── worker-runtime.md
│   └── orchestration-workspace-ui.md
├── checklists/
│   └── requirements.md
└── tasks.md                         # /speckit-tasks 단계에서 생성
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── pages/project-worktree-session/ui/
│   ├── project-worktree-session-page.tsx
│   └── project-worktree-session-page.test.tsx
├── entities/agent-orchestration/
│   ├── api/
│   │   ├── orchestration-repository.ts
│   │   ├── orchestration-repository.test.ts
│   │   └── query-keys.ts
│   └── model/
│       ├── types.ts
│       ├── task-state.ts
│       ├── task-state.test.ts
│       ├── relationships.ts
│       ├── relationships.test.ts
│       ├── prompt-dispatch.ts
│       └── prompt-dispatch.test.ts
├── entities/agent-run/
│   └── model/
│       ├── agent-run-workspace.ts
│       ├── agent-run-workspace.test.ts
│       ├── tile-layout.ts
│       └── tile-layout.test.ts
├── features/agent-run/
│   ├── model/
│   │   ├── agent-run-controller.ts
│   │   ├── agent-run-controller.test.ts
│   │   ├── orchestration-workspace.ts
│   │   ├── orchestration-workspace.test.ts
│   │   ├── prompt-target-selection.ts
│   │   └── prompt-target-selection.test.ts
│   └── ui/
│       ├── agent-run-panel.tsx
│       ├── agent-run-runtime-host.tsx
│       ├── worktree-agent-run-area.tsx
│       ├── workspace-prompt-composer.tsx
│       ├── workspace-prompt-composer.test.tsx
│       ├── prompt-dispatch-status.tsx
│       ├── task-activity-rail.tsx
│       ├── task-activity-item.tsx
│       ├── task-activity-rail.test.tsx
│       ├── coordinator-handoff-dialog.tsx
│       └── *.test.tsx
└── stories/
│   ├── atoms.stories.tsx
│   ├── molecules.stories.tsx
│   ├── organisms.stories.tsx
│   └── pages.stories.tsx

apps/agentic-workbench/scripts/
├── acp-orchestration-smoke-agent.mjs
└── acp-orchestration-smoke-agents.json

apps/agentic-workbench/src-tauri/src/
├── domain/
│   ├── agent_orchestration.rs
│   └── mod.rs
├── application/
│   ├── orchestration_service.rs
│   ├── orchestration_command_service.rs
│   ├── coordinator_notification_dispatcher.rs
│   ├── orchestration_scheduler.rs
│   ├── orchestration_event_projector.rs
│   └── mod.rs
├── ports/
│   ├── orchestration_repository.rs
│   ├── agent_worker.rs
│   ├── coordinator_notification.rs
│   ├── orchestration_event_sink.rs
│   ├── runtime_event_journal.rs
│   └── mod.rs
├── inbound/
│   └── tauri_commands.rs
├── infrastructure/
│   ├── json_orchestration_repository.rs
│   ├── acp_agent_worker_adapter.rs
│   ├── acp_agent_launch_factory.rs
│   ├── in_memory_runtime_event_journal.rs
│   ├── tauri_orchestration_event_sink.rs
│   ├── json_store.rs
│   └── mcp/
│       ├── mod.rs
│       ├── capability_registry.rs
│       └── orchestration_tool.rs
└── lib.rs

docs/
└── agent-orchestration-workspace.md
```

**Structure Decision**: durable orchestration은 `agent-exchange`와 별도 app-local aggregate로
구현한다. Backend application service가 repository, scheduler, worker port와 event sink를
조정하고 Tauri/MCP adapter는 같은 service를 호출한다. `AcpAgentWorkerAdapter`는 기존
공유 ACP use case를 조합하되 AW task/panel 타입을 공유 crate에 넣지 않는다. Frontend는
기존 `features/agent-run` workspace를 점진적으로 확장하고, 오케스트레이션 도메인만
`entities/agent-orchestration`에 둔다. runtime controller와 bounded event journal이
background/panel projection 사이에서 동일 run/timeline을 유지한다.
Tauri UI와 MCP inbound adapter는 동일한 `orchestration_command_service`를 호출한다.
command/notification dispatcher는 repository lock 밖에서 runtime port를 호출하고 durable
outbox receipt로 crash와 retry를 조정한다.

## Complexity Tracking

No constitution violations.

## Phase 0 Research

[research.md](./research.md)에 다음 결정을 확정했다.

- session이 아닌 durable task를 제어 평면의 기준으로 사용
- live exchange와 durable orchestration repository 분리
- Main 직접 Child의 별 모양 topology
- backend-owned ACP worker adapter와 runtime event journal
- workspace-owned Node/run runtime controller와 panel view 재수화
- durable TaskCommand outbox와 full-payload idempotent runtime delivery
- Child report 기반 CoordinatorNotification과 generation-safe Main wake-up
- UI/MCP command parity, retry/reassign actual worker lifecycle과 attempt fencing
- 명시적 구조화 result를 완료 primary signal로 사용
- run-scoped MCP capability와 역할별 tool allow-list
- 단일 Composer의 direct/delegate 분리 및 target별 partial result
- task/execution/presentation 상태 분리
- read-only capable profile + permission deny + change violation 감지
- FIFO capacity scheduling과 explicit Coordinator generation handoff
- provider-native orchestration은 후속 adapter로 격리

모든 기술적 unknown은 해결되었으며 `NEEDS CLARIFICATION` 항목이 없다.

## Phase 1 Design

- [data-model.md](./data-model.md): aggregate, Node, Generation, Task, Report, Dispatch,
  TaskCommand, CoordinatorNotification, runtime journal, Runtime View Binding과 세 상태 축
- [contracts/orchestration-service.md](./contracts/orchestration-service.md): Tauri
  command/event, command outbox, Main notification, idempotency, handoff와 typed errors
- [contracts/orchestration-mcp.md](./contracts/orchestration-mcp.md): run-scoped principal과
  Coordinator/Child 도구
- [contracts/worker-runtime.md](./contracts/worker-runtime.md): ACP worker adapter,
  command/notification dispatcher, scheduler, event mapping과 read-only enforcement
- [contracts/orchestration-workspace-ui.md](./contracts/orchestration-workspace-ui.md):
  단일 Composer, Activity Rail, 승격/분리, runtime rehydration과 tab/tile 회귀
- [quickstart.md](./quickstart.md): deterministic smoke Worker와 end-to-end 검증 시나리오

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. 최종 설계의 source 변경은 AW app-local이며
  `packages/*`/`crates/*` 변경이 없다.
- **Feature-Sliced Frontend Architecture**: PASS. durable types/API, workspace interaction,
  screen composition과 UI primitive의 FSD 책임이 source tree와 UI contract에 명시됐다.
- **Hexagonal Tauri Backend Architecture**: PASS. domain/application/ports/inbound/
  infrastructure 경계와 Tauri/MCP가 공유할 command service, worker/notification port가
  네 계약에 명시됐다.
- **Shared Core Before Shared UI**: PASS. 관계, task/dispatch state machine과 controller를
  pure core로 정의했고 UI는 AW에 유지한다.
- **Atomic Cross-App Verification**: N/A. 공유 package/crate 변경이 없다.
- **Documentation and Storybook**: PASS. 한국어 설계 문서, Mermaid 흐름과 atomic
  Storybook 상태가 계획/계약/quickstart에 포함됐다.
- **Testing and Safety**: PASS. repository atomic recovery, capability principal,
  canonical worktree/window/run/node/task/generation scope, artifact path/UTF-8/size,
  read-only 위반, terminal race, runtime snapshot/live sequence dedupe, panel mount의
  stale binding overwrite 방지, full-payload idempotency, attempt/run fencing,
  UI/MCP command parity와 outbox crash recovery 검증이 계획됐다.

## Agent Context Update

현재 SpecKit 설치에는 `.specify/scripts/bash/update-agent-context.sh`가 없다. 사용 가능한
scripts를 확인했으며 agent context 파일을 임의 생성하거나 수정하지 않는다. 구현 컨텍스트는
본 plan과 `AGENTS.md`를 기준으로 한다.
