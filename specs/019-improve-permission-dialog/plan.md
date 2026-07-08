# Implementation Plan: 긴 Permission 다이얼로그 레이아웃 개선

**Branch**: `main` | **Date**: 2026-07-09 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/019-improve-permission-dialog/spec.md`

## Summary

AW permission 요청 다이얼로그가 긴 command, payload, 승인 option label을 표시할 때 화면 안에 유지되고, 메시지/상세/버튼 영역이 겹치지 않도록 개선한다. 구현은 기존 `PermissionRequestDialog`를 `features/agent-run/ui`의 독립 컴포넌트로 분리하고, `entities/agent-run/model`에 permission 표시용 view model과 option label 요약 규칙을 추가해 긴 콘텐츠 상태를 테스트 가능하게 만든다.

## Technical Context

**Language/Version**: TypeScript 5.6, React 19

**Primary Dependencies**: Existing Radix/shadcn dialog and button components, Tailwind CSS 4 utility styling, Vitest, Storybook 10

**Storage**: N/A. Permission 요청은 runtime event에서 표시되며 영구 저장하지 않는다.

**Testing**: `pnpm --dir apps/agentic-workbench check-types`, `pnpm --dir apps/agentic-workbench test`, targeted Vitest tests for permission display model and dialog rendering, Storybook stories for long-content states

**Target Platform**: AW Tauri desktop app frontend, including narrow desktop windows down to 360px width

**Project Type**: Desktop app frontend feature

**Performance Goals**: 5,000자 이상의 permission input을 표시해도 dialog open/interaction이 사용자에게 즉시 반응하는 수준을 유지하고, 긴 텍스트 렌더링이 전체 app layout을 resize하지 않아야 한다.

**Constraints**: Permission 원문 command/input/cwd/승인 범위는 사용자가 확인 가능해야 한다. Button label은 짧게 요약하되 실제 승인 option id/kind/name 의미를 바꾸면 안 된다. 좁은 창에서도 action controls는 화면 밖으로 나가거나 서로 겹치면 안 된다.

**Scale/Scope**: `apps/agentic-workbench`의 agent run permission dialog 한 곳. Permission 정책, backend 승인 처리, agent protocol, Tauri command contract 변경은 범위 밖이다.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. 변경 범위는 `apps/agentic-workbench` 내부 frontend에 한정하고, `packages/*` 또는 다른 app import를 만들지 않는다.
- **Feature-Sliced Frontend Architecture**: PASS. Permission 표시 모델은 `apps/agentic-workbench/src/entities/agent-run/model`, 승인 interaction UI는 `apps/agentic-workbench/src/features/agent-run/ui`, Storybook 사례는 `apps/agentic-workbench/src/stories`에 둔다.
- **Hexagonal Tauri Backend Architecture**: N/A. Backend permission 처리나 Tauri command는 변경하지 않는다.
- **Shared Core Before Shared UI**: PASS. AW 전용 UX이므로 cross-app shared UI를 만들지 않는다. 재사용은 먼저 headless permission display helper로 제한한다.
- **Atomic Cross-App Verification**: N/A. `packages/*`와 `crates/*` 변경이 없다.
- **Documentation and Storybook**: PASS. 긴 command, JSON payload, 긴 approval label, 좁은 화면 상태를 Storybook organism 사례로 등록한다. 별도 `docs/*.md` 변경은 필요하지 않다.
- **Testing and Safety**: PASS. Permission 승인 의미는 바꾸지 않고 UI 표시/요약만 다룬다. 순수 요약 규칙은 unit test로 검증하고, dialog render는 accessibility-oriented DOM assertions와 Storybook 상태로 검증한다.

## Project Structure

### Documentation (this feature)

```text
specs/019-improve-permission-dialog/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── permission-dialog-ui.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── entities/agent-run/model/
│   ├── permission-display.ts           # permission input/option 표시 모델과 label 요약 규칙
│   ├── permission-display.test.ts      # 긴 command, JSON, prefix, fallback 규칙 검증
│   └── index.ts                        # 필요한 model export 추가
├── features/agent-run/ui/
│   ├── permission-request-dialog.tsx   # 분리된 permission dialog UI
│   ├── permission-request-dialog.test.tsx
│   └── agent-run-panel.tsx             # dialog import와 wiring만 유지
└── stories/
    └── organisms.stories.tsx           # PermissionRequestDialog long-content states 추가
```

**Structure Decision**: permission event type은 이미 `entities/agent-run/model/types.ts`에 존재한다. 승인 요청을 어떻게 요약하고 표시할지는 domain-facing 표시 모델이므로 `entities/agent-run/model`에 순수 helper로 둔다. 사용자의 승인/거절 interaction과 dialog composition은 `features/agent-run/ui`에 유지한다. 기존 `agent-run-panel.tsx` 안의 inline dialog는 분리해 테스트와 Storybook 등록을 가능하게 한다.

## Phase 0: Research

Research completed in [research.md](./research.md).

Key decisions:

- Display model helper를 만들어 긴 option label과 input detail rendering policy를 UI에서 분리한다.
- Dialog content는 viewport-bounded layout으로 제한하고, detail 영역만 scroll 가능하게 한다.
- Button labels는 원문 대신 짧은 action summary를 사용하되, full option text는 detail/accessible text에서 확인 가능하게 한다.
- Backend permission semantics는 변경하지 않는다.

## Phase 1: Design & Contracts

Design artifacts:

- [data-model.md](./data-model.md): Permission request display model, command detail, approval option summary, dialog layout region.
- [contracts/permission-dialog-ui.md](./contracts/permission-dialog-ui.md): 사용자-visible UI contract and acceptance states.
- [quickstart.md](./quickstart.md): automated and manual validation scenarios.

Agent context update:

- No `.specify/scripts/bash/update-agent-context.sh` or equivalent script exists in this repository, so there is no agent context update command to run for this plan.

## Constitution Check (Post-Design)

- **Monorepo Boundary First**: PASS. Design artifacts keep source changes inside `apps/agentic-workbench`.
- **Feature-Sliced Frontend Architecture**: PASS. Data model and UI contract map to `entities/agent-run/model` and `features/agent-run/ui` respectively.
- **Hexagonal Tauri Backend Architecture**: N/A. No backend changes are designed.
- **Shared Core Before Shared UI**: PASS. No shared UI package is introduced; reusable logic remains headless and app-local.
- **Atomic Cross-App Verification**: N/A. No cross-app package/crate changes.
- **Documentation and Storybook**: PASS. Storybook states are required in quickstart and project structure.
- **Testing and Safety**: PASS. Tests cover long-content rendering, summarized labels, preserved original detail access, and narrow viewport behavior without changing approval semantics.

## Complexity Tracking

No constitution violations require justification.
