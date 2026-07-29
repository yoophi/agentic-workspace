# Implementation Plan: Main Coordinator 기반 에이전트 오케스트레이션

**Branch**: `033-agent-orchestration` | **Date**: 2026-07-27 | **Last Updated**: 2026-07-29 | **Spec**: [spec.md](./spec.md)

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
in-memory event journal을 사용하며 run별 최근 512 event를 보존한다(FR-043).

**Testing**: Vitest/React Testing Library for pure reducers, controller and UI contracts;
Storybook build/a11y states; Rust unit/application/infrastructure tests for state machines,
repository recovery, scheduler, capability authorization, ownership and adapter mapping;
AW TypeScript check/test/build와 AW Tauri cargo test/check

**Target Platform**: macOS desktop Tauri Worktree Session WebView. 기존 cross-platform
Tauri command/domain 계약은 유지하되 1차 수동 검증은 macOS AW 개발 앱에서 수행한다.

**Project Type**: pnpm/Turbo + Cargo monorepo의 desktop app frontend/backend feature

**Performance Goals**: local Composer/Activity Rail interaction 200ms 이내; task/report
변경 후 1초 이내 UI 반영; 목표 submit 후 30초 이내 세 Child의 생성·배정 확인; 8 Node와
4 active run 대표 시나리오에서 각 상호작용 200ms 이내(SC-016); bounded MCP wait 최대 30초;
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
UI와 MCP runtime command parity; Node 상한 8개(FR-045)와 run별 event 보존 512개(FR-043);
승격 정책은 역할별 시스템 배정이며 사용자 편집 없음(FR-046); artifact는 workspace 상대
경로만 허용(FR-047); 모든 거부는 구분 가능한 사유와 `retryable`을 반환하고 상태를 바꾸지
않음(FR-048)

**Scale/Scope**: Worktree Session workspace당 Node 최대 8개, 대표 active run Main+Child
3개, 작은 task DAG와 구조화 reports; `apps/agentic-workbench`와 `docs/specs`만 변경하며
`packages/*`, `crates/*`, 다른 앱은 변경하지 않음

**Non-scope**: spec.md의 `Out of Scope` 절을 그대로 따른다. 특히 다단계 자식 계층, 창 간
오케스트레이션, 자동 Child의 파일 쓰기, 사용자 편집 승격 정책, 공급자 내장 하위 에이전트
노출, crash 이후 전체 timeline 복원, 실패 시 자동 revert는 이 계획의 설계 대상이 아니다.

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
  repository/worker/event journal port는 최상위 `ports` 모듈, JSON/ACP/Tauri event/MCP는
  `infrastructure`, Tauri command는 `inbound`에서 service에 위임한다. 헌법 III(v1.0.1)은
  전용 `ports` 모듈을 허용하며 이 앱은 해당 위치를 일관되게 사용하고 `ports`에는 adapter를
  두지 않는다.
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
│   ├── project-worktree-session-page.test.tsx
│   └── project-worktree-session-page.stories.tsx
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
│       ├── prompt-dispatch.test.ts
│       ├── task-communication.ts
│       └── task-communication.test.ts
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
│       ├── agent-run-panel.test.tsx
│       ├── agent-run-runtime-host.tsx
│       ├── agent-run-runtime-host.test.tsx
│       ├── worktree-agent-run-area.tsx
│       ├── worktree-agent-run-area.test.tsx
│       ├── workspace-prompt-composer.tsx
│       ├── workspace-prompt-composer.test.tsx
│       ├── workspace-prompt-composer.stories.tsx
│       ├── prompt-dispatch-status.tsx
│       ├── task-activity-rail.tsx
│       ├── task-activity-item.tsx
│       ├── task-activity-rail.test.tsx
│       ├── task-activity-rail.stories.tsx
│       ├── coordinator-handoff-dialog.tsx
│       ├── coordinator-handoff-dialog.test.tsx
│       └── orchestration-workspace.stories.tsx
└── shared/storybook/
    ├── agent-orchestration-sample-data.ts
    ├── agent-orchestration-atoms.stories.tsx
    └── agent-orchestration-molecules.stories.tsx

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

Storybook 배치는 두 위치로 나눈다. 단일 컴포넌트에 종속된 organism/page 사례는 해당
컴포넌트 옆(`features/agent-run/ui/*.stories.tsx`,
`pages/project-worktree-session/ui/*.stories.tsx`)에 두고, 여러 컴포넌트가 공유하는
atom/molecule 사례와 결정적 fixture는 `shared/storybook/agent-orchestration-*`에 둔다.
기존 앱 전역 `src/stories/*`는 이 기능에서 변경하지 않는다.

## Complexity Tracking

현재 미해결 위반이나 편차가 없다.

### 해소된 항목

| ID | 편차였던 내용 | 해소 방법 |
| --- | --- | --- |
| C1 | 헌법 III v1.0.0은 "순수 도메인 모델과 **port**는 `domain`에 둔다"고 규정했으나, 이 기능의 `OrchestrationRepositoryPort`, `AgentWorkerPort`, `CoordinatorNotificationPort`, `OrchestrationEventSinkPort`, `RuntimeEventJournalPort`는 앱 전역 관례에 따라 최상위 `apps/agentic-workbench/src-tauri/src/ports/`에 있었다. | 2026-07-29 헌법 **v1.0.1** PATCH 개정으로 해소. 개정된 원칙 III은 port를 `domain` 또는 전용 `ports` 모듈에 둘 수 있게 하고(앱별로 한 위치를 일관되게 사용), `ports`에는 port 정의만 두며 `domain`·`ports` 모두 Tauri·파일시스템·영속화·UI에 의존하지 않아야 한다는 제약을 유지한다. 이 기능의 배치는 개정된 문구를 그대로 충족한다. |

C1은 이 기능만의 선택이 아니라 앱 전역 관례와 헌법 문구의 불일치였기 때문에, 코드 이동이
아니라 헌법 명확화로 해결했다. `packages/*`·`crates/*`와 다른 앱은 변경하지 않았다.

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

모든 기술적 unknown은 해결되었으며 `NEEDS CLARIFICATION` 항목이 없다. 2026-07-29 spec 보강은
새로운 기술적 unknown을 만들지 않았다. FR-043–FR-048은 이미 결정·구현된 경계값과 검증 규칙을
명세로 끌어올린 것이므로 추가 research 항목이 없다.

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

### 2026-07-29 재평가 (spec 보강 반영)

spec의 FR-043–FR-048, SC-016·SC-017과 신규 Key Entity 5개를 기준으로 Phase 1 산출물을
재검토했고 다음을 확인·수정했다.

- `data-model.md`는 `TaskCommand`, `CoordinatorNotification`, `RuntimeEventJournal`,
  `RuntimeViewBinding`, `IdempotencyRecord`를 이미 정의하고 있어 신규 Key Entity와 일치한다.
  `PromotionPolicy`만 FR-046의 역할별 배정 규칙을 반영해 수정했다.
- `contracts/orchestration-service.md`의 두 가지 구현 불일치를 정정했다. (1) 별도
  `promote_orchestration_task`/`detach_orchestration_task_panel`이 아니라 Node 대상의 단일
  `set_orchestration_presentation`이 실제 계약이다. (2) 세분화된 error code 23개 나열을
  실제 `{ code, message, retryable }` 구조와 12개 분류로 교체하고 FR별 대표 상황을 매핑했다.
- 새 요구사항 중 설계 변경이 필요한 항목은 없다. FR-043–FR-047은 이미 구현된 경계값·검증
  로직의 명세화이고, FR-022의 사유 세분화와 FR-044·FR-048의 사용자 안내만 보완이 필요해
  `tasks.md` Phase 14(T150–T162)로 분리했다.
- SC-016·SC-017은 새 계약을 요구하지 않으며 측정·거부 시나리오 검증으로 처리한다.

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. 최종 설계의 source 변경은 AW app-local이며
  `packages/*`/`crates/*` 변경이 없다.
- **Feature-Sliced Frontend Architecture**: PASS. durable types/API, workspace interaction,
  screen composition과 UI primitive의 FSD 책임이 source tree와 UI contract에 명시됐다.
- **Hexagonal Tauri Backend Architecture**: PASS. domain/application/ports/inbound/
  infrastructure 경계와 Tauri/MCP가 공유할 command service, worker/notification port가
  네 계약에 명시됐다. port 모듈 위치는 헌법 III(v1.0.1)이 허용하는 전용 `ports` 모듈이다.
- **Shared Core Before Shared UI**: PASS. 관계, task/dispatch state machine과 controller를
  pure core로 정의했고 UI는 AW에 유지한다.
- **Atomic Cross-App Verification**: N/A. 공유 package/crate 변경이 없다.
- **Documentation and Storybook**: PASS. 한국어 설계 문서, Mermaid 흐름과 atomic
  Storybook 상태가 계획/계약/quickstart에 포함됐다.
- **Testing and Safety**: PASS. repository atomic recovery, capability principal,
  canonical worktree/window/run/node/task/generation scope, artifact path/UTF-8/size,
  read-only 위반, terminal race, runtime snapshot/live sequence dedupe, panel mount의
  stale binding overwrite 방지, full-payload idempotency, attempt/run fencing,
  UI/MCP command parity와 outbox crash recovery 검증이 계획됐다. 2026-07-29 재평가에서
  FR-043–FR-048과 SC-016·SC-017의 경계값·거부·측정 검증을 tasks.md Phase 14로 추가했다.

### 2026-07-29 헌법 재점검 결과

spec 보강 이후 다시 평가했고 새로운 위반은 없다. 유일하게 남아 있던 C1(port 모듈 위치)은
같은 날 헌법 v1.0.1 PATCH 개정으로 해소되어 현재 미해결 편차가 없다. 신규 FR-047(artifact
경로 정규화·경계 이탈 거부)은 Engineering Standards의 파일 접근 검증 요구를 명시적으로
강화하는 방향이므로 새 편차를 만들지 않는다. `packages/*`와 `crates/*` 무변경 원칙도 그대로
유지된다. 이 계획은 헌법 **v1.0.1**을 기준으로 점검했다.

## Agent Context Update

현재 SpecKit 설치에는 `.specify/scripts/bash/update-agent-context.sh`가 없다(2026-07-29
재확인: `check-prerequisites.sh`, `common.sh`, `create-new-feature.sh`, `setup-plan.sh`,
`setup-tasks.sh`만 존재). agent context 파일을 임의 생성하거나 수정하지 않는다. 구현
컨텍스트는 본 plan과 `AGENTS.md`를 기준으로 한다.
