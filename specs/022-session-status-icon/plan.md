# 구현 계획: 세션 상태 아이콘

**Branch**: `022` | **Date**: 2026-07-09 | **Spec**: [spec.md](./spec.md)

**Input**: `specs/022-session-status-icon/spec.md`의 기능 명세

## 요약

Agentic Workbench의 agent timeline에서 `session_info_update` raw payload가 메시지로 보이지 않게 하고, 그 안의 agent `threadStatus`를 compact한 상태 표시로 노출한다. 구현은 `apps/agentic-workbench` 내부의 agent-run event model과 panel UI에 한정하며, raw event filtering과 active/idle 상태 추출은 순수 helper로 검증한 뒤 기존 run panel 상태 흐름에 연결한다.

## 기술 맥락

**Language/Version**: TypeScript 5.x, React 19, Vite 기반 Tauri frontend

**Primary Dependencies**: 기존 Agentic Workbench agent-run model/UI, shadcn/ui primitive, lucide-react icon, Vitest, React Testing Library

**Storage**: 영구 저장소 변경 없음. 최신 thread status는 visible run/session UI state로만 유지한다.

**Testing**: event classification/state update는 Vitest unit test로 검증하고, status indicator rendering과 raw payload suppression은 React Testing Library 또는 기존 source-level UI test로 검증한다.

**Target Platform**: macOS desktop Tauri app, Agentic Workbench run panel 및 worktree session surface

**Project Type**: pnpm/Turbo monorepo의 desktop app frontend

**Performance Goals**: session status update 수신 후 1초 이내 visible status를 반영한다. repeated status update는 timeline item을 0개 추가해야 한다.

**Constraints**: app-to-app import 금지, 신규 shared package 도입 없음, raw diagnostics 화면의 의도된 raw group 기능은 유지하되 `session_info_update`만 user timeline에서 제외한다. unknown/partial status는 UI를 깨뜨리지 않고 neutral 상태로 처리한다.

**Scale/Scope**: `apps/agentic-workbench`의 agent-run event pipeline, run panel state, run panel header/agent identity/status area. backend command, persistence, cross-app package는 범위 밖이다.

## Constitution Check

*GATE: Phase 0 research 전 통과해야 하며, Phase 1 design 후 다시 확인한다.*

- **Monorepo Boundary First**: PASS. 변경 범위는 `apps/agentic-workbench`와 `specs/022-session-status-icon` 산출물에 한정한다. `packages/*`, `crates/*`, 다른 앱 import는 계획하지 않는다.
- **Feature-Sliced Frontend Architecture**: PASS. Event type/classification은 `entities/agent-run/model`, run panel state와 user interaction은 `features/agent-run/model`, status indicator UI는 `features/agent-run/ui`에 둔다. shadcn primitive는 기존 `components/ui`만 사용한다.
- **Hexagonal Tauri Backend Architecture**: N/A. backend, filesystem, persistence, Tauri command 변경이 필요하지 않다.
- **Shared Core Before Shared UI**: PASS. AW 단일 기능이므로 신규 shared UI package를 만들지 않는다. 재사용 가능성이 있는 status extraction은 app-local pure model로 먼저 둔다.
- **Atomic Cross-App Verification**: N/A. `packages/*` 또는 `crates/*` 변경을 계획하지 않는다.
- **Documentation and Storybook**: PASS. 새 reusable component가 생기면 Storybook state를 추가한다. 단순 run panel 내부 표시라면 targeted UI test와 quickstart 수동 검증으로 충분하다. 프로젝트 문서 변경은 필요하지 않다.
- **Testing and Safety**: PASS. Raw metadata filtering, active/idle/unknown extraction, timeline preservation, UI indicator rendering을 unit/UI test로 계획한다. session ownership/security boundary 변경은 없다.

## 프로젝트 구조

### 문서 산출물

```text
specs/022-session-status-icon/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── session-status-ui.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### 소스 코드

```text
apps/agentic-workbench/src/
├── entities/agent-run/model/
│   ├── types.ts
│   ├── format.ts
│   └── format.test.ts
└── features/agent-run/
    ├── model/
    │   ├── run-panel-state.ts
    │   └── run-panel-state.test.ts
    └── ui/
        ├── agent-run-panel.tsx
        └── agent-run-panel.test.tsx
```

**구조 결정**: `session_info_update`는 agent-run domain event의 한 종류로 분류되므로 raw payload 판단과 status extraction은 `entities/agent-run/model`에 둔다. Run panel의 awaiting/running state와 latest agent status 갱신은 `features/agent-run/model`에 반영한다. 실제 시각 표시와 accessibility label은 기존 `AgentRunPanel` UI에서 처리한다. Backend와 shared package는 변경하지 않는다.

## Complexity Tracking

Constitution 위반 없음.

## Phase 0 Research

[research.md](./research.md)를 참조한다.

## Phase 1 Design

[data-model.md](./data-model.md), [contracts/session-status-ui.md](./contracts/session-status-ui.md), [quickstart.md](./quickstart.md)를 참조한다.

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. Design artifact는 구현을 `apps/agentic-workbench` 내부로 제한하며 cross-app sharing 또는 app-to-app import를 추가하지 않는다.
- **Feature-Sliced Frontend Architecture**: PASS. 설계는 raw event classification을 `entities`, run state transition을 `features/model`, visible status rendering을 `features/ui`에 매핑한다.
- **Hexagonal Tauri Backend Architecture**: N/A. backend 또는 Tauri command 변경을 계획하지 않는다.
- **Shared Core Before Shared UI**: PASS. Status extraction은 app-local pure logic으로 유지하며 shared UI package를 도입하지 않는다.
- **Atomic Cross-App Verification**: N/A. shared workspace package/crate 변경이 없다.
- **Documentation and Storybook**: PASS. reusable indicator를 도입하는 경우에만 Storybook이 필요하다. 그렇지 않으면 test와 quickstart로 검증한다.
- **Testing and Safety**: PASS. Data model, contract, quickstart는 raw filtering, active/idle display, unknown/partial tolerance, timeline preservation을 다룬다.

## Agent Context Update

현재 Spec Kit 설치에는 agent-context update script가 없어서 context file을 생성하거나 수정하지 않았다.
