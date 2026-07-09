# 구현 계획: 사용 가능한 명령 요약과 조회

**Branch**: `main` | **Date**: 2026-07-09 | **Spec**: [spec.md](./spec.md)

**Input**: `/specs/024-available-commands-summary/spec.md`

## Summary

#150은 `session/update`의 `available_commands_update`를 raw timeline noise가 아닌 session-level metadata로 처리한다. 기존 prompt autocomplete용 command 추출 흐름을 확장해 command count summary와 상세 조회 모델을 만들고, run panel header 또는 세션 정보 영역에서 이름, 설명, 입력 힌트를 compact하게 확인할 수 있게 한다. 일반 raw event, user/agent message, tool activity의 기존 timeline 동작은 유지한다.

## Technical Context

**Language/Version**: TypeScript 5.x, React 19, Vite, Rust 2024(Tauri backend은 typed event가 필요할 때만 최소 변경)

**Primary Dependencies**: 기존 `listenRunEvents`, `entities/agent-run/model/prompt-autocomplete`, `entities/agent-run/model/format`, `features/agent-run/model/run-panel-state`, `features/agent-run/ui/agent-run-panel`, shadcn/ui primitives

**Storage**: N/A. command metadata는 현재 visible run/session의 live UI 상태로만 유지하며 영구 저장하지 않는다.

**Testing**: Vitest, React Testing Library/source-level regression tests, `pnpm --filter agentic-workbench check-types`, `pnpm --filter agentic-workbench test`, backend mapper 변경 시 `cargo test -p agentic-workbench session_update_mapper`

**Target Platform**: Agentic Workbench Tauri desktop app

**Project Type**: Desktop app frontend feature with optional ACP mapper contract refinement

**Performance Goals**: 50개 이상의 command update도 1초 이내에 summary/detail 상태로 반영된다. 해당 update는 full raw JSON timeline item 0건을 추가한다.

**Constraints**: app-to-app import 금지, shared package 추가 금지, persistence 변경 금지, schema 전체 렌더링 범위 밖, malformed command는 UI를 깨뜨리지 않고 유효 항목만 표시하거나 fallback한다.

**Scale/Scope**: `apps/agentic-workbench`의 agent run panel 1개 화면과 `specs/024-available-commands-summary` 산출물에 한정한다.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. app-specific 구현은 `apps/agentic-workbench`, feature 문서는 `specs/024-available-commands-summary`에만 둔다. `packages/*` 또는 `crates/*` 추가 없음.
- **Feature-Sliced Frontend Architecture**: PASS. command metadata parsing/formatting은 `entities/agent-run/model`, reducer/state는 `features/agent-run/model`, visible summary/detail UI는 `features/agent-run/ui`에 둔다.
- **Hexagonal Tauri Backend Architecture**: PASS. backend 변경이 필요해도 `src-tauri/src/domain/events.rs`와 `infrastructure/acp/session_update_mapper.rs`의 ACP event contract 보존에 한정한다. Tauri commands, persistence, application services는 변경하지 않는다.
- **Shared Core Before Shared UI**: PASS. AW 단일 화면 기능이며 shared UI/package 추출 근거가 없다.
- **Atomic Cross-App Verification**: N/A. `packages/*` 또는 `crates/*` 변경이 없다.
- **Documentation and Storybook**: PASS. 새 reusable UI component를 만들지 않고 existing run panel 내부 표시를 확장한다. reusable component로 승격할 경우에만 Storybook을 추가한다.
- **Testing and Safety**: PASS. pure parser/formatter와 reducer tests를 계획한다. filesystem, persistence, permission, exchange scope 변경 없음.

## Project Structure

### Documentation (this feature)

```text
specs/024-available-commands-summary/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── available-commands-ui.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── entities/
│   └── agent-run/model/
│       ├── prompt-autocomplete.ts
│       ├── prompt-autocomplete.test.ts
│       ├── format.ts
│       ├── format.test.ts
│       └── types.ts
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

**Structure Decision**: 기존 `availableCommandCandidatesFromSessionUpdate`는 command 이름/설명을 autocomplete 후보로 추출한다. 이 feature는 같은 parsing source를 확장해 `AvailableCommandMetadata`와 `CommandDetailItem`을 만들고, AgentRunPanel의 live event listener/reducer에서 timeline append 전에 metadata로 처리한다. backend typed event는 raw suppression과 detail 표시를 frontend에서 충분히 해결할 수 없을 때만 추가한다.

## Complexity Tracking

헌법 위반이 없으므로 추가 복잡도 예외는 없다.
