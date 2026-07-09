# 구현 계획: 세션 정보 메타데이터 표시

**Branch**: `main` | **Date**: 2026-07-09 | **Spec**: [spec.md](./spec.md)

**Input**: `/specs/023-session-info-metadata/spec.md`

## Summary

#148은 ACP `session_info_update`가 전달하는 `title`과 `updatedAt`을 raw JSON timeline item 없이 AW UI에 표시한다. `title`은 기존 Worktree Session window title 흐름의 primary target으로 연결하고, `updatedAt`은 run/session header의 보조 metadata로 표시한다. #145에서 만든 typed `sessionInfo` event, raw suppression, active/idle status indicator는 유지한다.

## Technical Context

**Language/Version**: TypeScript 5.x, React 19, Vite, Rust 2024(Tauri backend은 계약 확인 또는 최소 보정 시에만)

**Primary Dependencies**: `@tauri-apps/api/window`, 기존 `listenRunEvents`, `entities/agent-run` formatter/types, `features/agent-run` panel state/UI, shadcn/ui badge primitives

**Storage**: N/A. 세션 metadata는 현재 visible run/session의 live UI 상태로만 유지하며 영구 저장하지 않는다.

**Testing**: Vitest, React Testing Library, `pnpm --filter agentic-workbench check-types`, backend 변경 시 `cargo test -p agentic-workbench session_update_mapper`

**Target Platform**: Agentic Workbench Tauri desktop app(macOS 우선, Tauri window API 기반)

**Project Type**: Desktop app frontend feature with existing Tauri event contract

**Performance Goals**: `session_info_update` 수신 후 window title과 header metadata가 1초 이내 갱신된다. 해당 update는 timeline item 0건을 추가한다.

**Constraints**: app-to-app import 금지, shared package 추가 금지, persistence 변경 금지, #113의 active/idle window-title prefix/suffix 정책은 범위 밖으로 유지한다. 빈 title, control character, 과도하게 긴 title, malformed `updatedAt`은 UI를 깨뜨리지 않고 무시하거나 fallback해야 한다.

**Scale/Scope**: `apps/agentic-workbench`의 현재 worktree session 화면 1개와 `specs/023-session-info-metadata` 산출물에 한정한다.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. app-specific 구현은 `apps/agentic-workbench`, feature 문서는 `specs/023-session-info-metadata`에만 둔다. `packages/*` 또는 `crates/*` 추가 없음.
- **Feature-Sliced Frontend Architecture**: PASS. metadata parsing/formatting은 `entities/agent-run/model`, run-panel state는 `features/agent-run/model`, visible rendering과 event bridge는 `features/agent-run/ui` 및 기존 `app/App.tsx` window title composition에 둔다.
- **Hexagonal Tauri Backend Architecture**: PASS. 기존 `src-tauri/src/domain/events.rs`와 `infrastructure/acp/session_update_mapper.rs`는 이미 `title/updatedAt`을 typed event로 보존한다. 변경이 필요해도 mapper contract 보정과 테스트에 한정한다.
- **Shared Core Before Shared UI**: PASS. AW 단일 화면 기능이므로 shared UI/package를 만들지 않는다.
- **Atomic Cross-App Verification**: N/A. `packages/*` 또는 `crates/*` 변경이 없다.
- **Documentation and Storybook**: PASS. 새 reusable UI component를 만들지 않고 기존 panel/header 내부 표시를 확장한다. Storybook은 새 reusable component 도입 시에만 추가한다.
- **Testing and Safety**: PASS. formatter/state/UI focused tests와 app typecheck/test를 계획한다. filesystem, persistence, permission scope 변경 없음.

## Project Structure

### Documentation (this feature)

```text
specs/023-session-info-metadata/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── session-info-metadata-ui.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── app/
│   ├── App.tsx
│   └── App.test.tsx
├── entities/
│   ├── agent-run/model/
│   │   ├── format.ts
│   │   ├── format.test.ts
│   │   └── types.ts
│   └── project/lib/
│       ├── worktree-window-title.ts
│       └── worktree-window-title.test.ts
└── features/
    └── agent-run/
        ├── model/
        │   ├── run-panel-state.ts
        │   └── run-panel-state.test.ts
        └── ui/
            ├── agent-run-panel.tsx
            └── agent-run-panel.test.tsx

apps/agentic-workbench/src-tauri/src/
├── domain/events.rs
└── infrastructure/acp/session_update_mapper.rs
```

**Structure Decision**: 기존 window title 제어는 `app/App.tsx`와 `entities/project/lib/worktree-window-title.ts`에 이미 존재한다. `AgentRunPanel`은 `session_info_update`를 감지하는 지점에서 metadata를 읽고, title update를 기존 window-title event/fallback 흐름으로 전달한다. `updatedAt` formatting은 agent-run domain helper 또는 panel-local helper로 두고, 반복 사용이 필요해질 때만 `entities/agent-run/model`로 승격한다.

## Complexity Tracking

헌법 위반이 없으므로 추가 복잡도 예외는 없다.
