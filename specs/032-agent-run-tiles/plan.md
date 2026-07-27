# Implementation Plan: Agent Run 탭·타일 워크스페이스

**Branch**: `main` | **Date**: 2026-07-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/032-agent-run-tiles/spec.md`

## Summary

AW Worktree Session의 기존 `WorktreeAgentRunArea`를 패널 수명과 표시 레이아웃이 분리된 agent workspace로 확장한다. 기존 탭 모드는 그대로 유지하고, 동일한 패널 인스턴스를 중첩 분할 트리에 투영하는 타일 모드를 추가한다. 현재 패널 기준 오른쪽·아래 분할, 경계 크기 조정, 닫기 후 트리 축약, 탭/타일 전환 시 상태 보존을 순수 reducer로 관리한다.

런 간 통신은 같은 Tauri 창과 canonical worktree에 등록된 panel/run endpoint만 대상으로 한다. Frontend가 workspace snapshot을 backend registry와 동기화하고, 사용자 또는 MCP agent가 만든 exchange를 application service가 소유권·대상·중복·크기를 검증한 뒤 소유 창에 전달한다. Frontend는 기존 `AgentPromptRequest.delivery`의 `send | queue | draft` 계약으로 정확한 패널에 적용하고 결과를 acknowledge한다. MCP는 피어 조회, 메시지 전송, 교환 상태 조회를 제공한다.

## Technical Context

**Language/Version**: TypeScript 5.x, React 19, Rust 2024 edition, Tauri 2

**Primary Dependencies**: React, `react-resizable-panels`, TanStack Query, existing shadcn/ui primitives, Tauri invoke/listen, Axum-based local MCP server, existing `SessionRegistry` and `AgentRunPanel`

**Storage**: 영구 저장 없음. 타일 레이아웃, panel UI 상태, workspace endpoint, exchange 및 중복 제거 기록은 세션 창 수명 동안 메모리에 유지

**Testing**: Vitest for layout/workspace/exchange reducers and React contracts; Storybook for workspace states; Rust unit tests for domain validation/application ownership; Tauri command and MCP tool contract tests; consuming AW frontend check-types/test and AW Tauri cargo test/check

**Target Platform**: macOS desktop Tauri app의 AW Worktree Session WebView

**Project Type**: pnpm/Turbo + Cargo monorepo의 desktop app frontend/backend 기능

**Performance Goals**: 보기 전환·분할 명령은 200ms 이내 조작 가능; 8 panel/4 split depth에서 지속적인 타일 조작; exchange 요청은 1초 이내 accepted/rejected 상태 제공

**Constraints**: 기존 panel React state와 active run을 remount하지 않음; Main panel 삭제 금지; 현재 창·canonical worktree 범위 밖 통신 금지; agent source run 소유권 검증; 16KiB message 상한; request ID idempotency; v1 영속화와 창 간 통신 제외

**Scale/Scope**: Worktree Session 한 창당 panel 최대 8개, split depth 최대 4, exchange 최근 기록 최대 500개; `apps/agentic-workbench`만 변경하고 `packages/*`, `crates/*`는 변경하지 않음

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. 모든 runtime 변경은 `apps/agentic-workbench`에 한정하며 다른 앱 import와 새 package/crate가 없다. 기존 `acp-agent-core`의 session port는 소비만 한다.
- **Feature-Sliced Frontend Architecture**: PASS. 화면 조합은 `pages/project-worktree-session`, workspace/tile/교환 사용자 동작은 `features/agent-run`, 순수 layout/exchange 모델과 Tauri adapter는 `entities/agent-run`, 범용 UI primitive는 기존 `components/ui`를 사용한다.
- **Hexagonal Tauri Backend Architecture**: PASS. `domain/agent_exchange.rs`가 순수 모델·검증, `application/agent_exchange_service.rs`가 피어 조회/전달/ack 유스케이스, `ports/agent_workspace_registry.rs`가 저장·라우팅 port, `inbound/tauri_commands.rs`와 MCP tool이 입력 adapter, `infrastructure/in_memory_agent_workspace_registry.rs`가 메모리 구현과 창 event sink를 담당한다.
- **Shared Core Before Shared UI**: PASS. layout reducer와 exchange state transition을 app-local 순수 모듈로 먼저 구현한다. AW 전용 작업 화면이므로 공유 UI로 승격하지 않는다.
- **Atomic Cross-App Verification**: N/A. `packages/*`와 `crates/*` 변경이 없다.
- **Documentation and Storybook**: PASS. `docs/agent-run-tile-workspace.md`에 한국어 Mermaid 구조/흐름을 추가하고 toolbar, tile header, split layout, peer exchange dialog의 atoms/molecules/organisms/pages 스토리를 추가한다.
- **Testing and Safety**: PASS. split invariant, close collapse, focus fallback, idempotency, message limits, stale run, owner window, canonical worktree scope를 순수 및 adapter 테스트로 고정한다. MCP와 Tauri 경계는 application service에 위임한다.

## Project Structure

### Documentation (this feature)

```text
specs/032-agent-run-tiles/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── agent-run-workspace-ui.md
│   └── agent-exchange.md
├── checklists/
│   └── requirements.md
└── tasks.md                         # /speckit-tasks 단계에서 생성
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── pages/project-worktree-session/ui/
│   └── project-worktree-session-page.tsx
├── entities/agent-run/
│   ├── api/
│   │   ├── agent-exchange-repository.ts
│   │   └── agent-exchange-repository.test.ts
│   └── model/
│       ├── agent-run-workspace.ts
│       ├── agent-run-workspace.test.ts
│       ├── tile-layout.ts
│       ├── tile-layout.test.ts
│       └── agent-exchange.ts
├── features/agent-run/
│   ├── model/
│   │   ├── agent-run-panel-slots.ts
│   │   └── agent-run-panel-slots.test.ts
│   └── ui/
│       ├── worktree-agent-run-area.tsx
│       ├── agent-run-panel-tabs.tsx
│       ├── agent-run-workspace-toolbar.tsx
│       ├── agent-run-tile-layout.tsx
│       ├── agent-run-tile.tsx
│       ├── agent-peer-message-dialog.tsx
│       └── *.test.tsx
├── shared/storybook/sample-data.ts
└── stories/
    ├── molecules.stories.tsx
    ├── organisms.stories.tsx
    └── pages.stories.tsx

apps/agentic-workbench/src-tauri/src/
├── domain/
│   ├── agent_exchange.rs
│   └── mod.rs
├── application/
│   ├── agent_exchange_service.rs
│   └── mod.rs
├── ports/
│   ├── agent_workspace_registry.rs
│   └── mod.rs
├── inbound/
│   └── tauri_commands.rs
├── infrastructure/
│   ├── in_memory_agent_workspace_registry.rs
│   ├── mod.rs
│   └── mcp/
│       ├── mod.rs
│       └── agent_exchange_tool.rs
└── lib.rs

docs/
└── agent-run-tile-workspace.md
```

**Structure Decision**: `AgentRunPanel`의 내부 state는 유지하고 workspace가 panel slot과 layout tree를 소유한다. `tile-layout.ts`는 leaf/split 생성·분할·삭제·비율 변경·flatten·검증만 담당하고 React를 참조하지 않는다. `agent-run-workspace.ts`는 view mode, focused panel, slots와 layout action을 결합한다. Backend는 frontend snapshot을 신뢰하지 않고 active run의 owner window를 기존 `AppState`로 재검증하며, exchange application service만 registry와 window event port를 사용한다. MCP와 Tauri command는 동일 service를 호출한다.

## Complexity Tracking

No constitution violations.

## Phase 0 Research

See [research.md](./research.md).

## Phase 1 Design

See [data-model.md](./data-model.md), [contracts/agent-run-workspace-ui.md](./contracts/agent-run-workspace-ui.md), [contracts/agent-exchange.md](./contracts/agent-exchange.md), and [quickstart.md](./quickstart.md).

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. 최종 설계는 AW app-local frontend/backend만 변경한다.
- **Feature-Sliced Frontend Architecture**: PASS. page는 callback 조합만 하고 entity model/API, feature interaction, UI primitive 책임이 분리됐다.
- **Hexagonal Tauri Backend Architecture**: PASS. domain, application, port, inbound, infrastructure 책임과 동일 service를 공유하는 Tauri/MCP adapter가 계약에 명시됐다.
- **Shared Core Before Shared UI**: PASS. layout/exchange pure state가 UI보다 먼저 정의되며 공유 package는 만들지 않는다.
- **Atomic Cross-App Verification**: N/A. 공용 package/crate 변경이 없다.
- **Documentation and Storybook**: PASS. 한국어 Mermaid 문서와 atomic category별 상태 스토리가 산출물에 포함됐다.
- **Testing and Safety**: PASS. 창·worktree·source run·target run scope, 16KiB 제한, idempotency, stale snapshot, close race 및 reducer invariant 검증이 quickstart와 계약에 포함됐다.

## Agent Context Update

Spec Kit 설치에 agent-context update script가 없으므로 context 파일은 생성하거나 수정하지 않는다.
