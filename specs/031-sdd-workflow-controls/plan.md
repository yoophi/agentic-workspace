# Implementation Plan: SDD 워크플로 단계 표시 및 제어

**Branch**: `worktree-18c533ee42f34570` | **Date**: 2026-07-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/031-sdd-workflow-controls/spec.md`

## Summary

AW Worktree Session의 Speckit 탭에 현재 활성 SDD 기능과 `specify → plan → tasks → implement` 진행 상태를 표시하고, 사용자가 다음 작업을 기존 에이전트 작업 흐름으로 요청할 수 있게 한다. 활성 기능은 `.specify/feature.json`의 `feature_directory`만을 최우선 출처로 판별하고 해당 항목을 Speckit 목록에서 하이라이트한다. 파일이 없거나 유효하지 않을 때는 다른 기능으로 자동 대체하지 않고, 사용자가 편집·취소·명시적 전송할 수 있는 초기 SDD 프롬프트를 에이전트 입력 영역에 채운다.

구현은 기존 읽기 전용 worktree-file 조회, watcher invalidation, workspace-to-agent prompt routing을 재사용한다. 순수 SDD 상태/명령 프롬프트 모델과 UI를 AW frontend에 추가하며, 초안 주입과 즉시 전송을 구분하는 prompt request contract를 확장한다. 새로운 Tauri command나 영구 저장소는 만들지 않는다.

## Technical Context

**Language/Version**: TypeScript 5.x, React 19, Vite; 기존 Tauri 2 desktop shell

**Primary Dependencies**: TanStack Query, existing Tauri `invoke` worktree-file repository, lucide-react, existing shadcn/Radix UI primitives, existing agent run panel prompt routing

**Storage**: `.specify/feature.json` 및 `specs/<feature>` 산출물은 read-only로 조회한다. 단계 확인과 prompt draft 상태는 현재 session의 ephemeral UI state로만 유지한다.

**Testing**: Vitest for pure SDD state/prompt builders and agent prompt routing; React component tests for panel states and dialogs; Storybook organism state coverage; Rust test는 기존 file read contract를 바꾸지 않으므로 N/A

**Target Platform**: macOS desktop Tauri app의 AW Worktree Session, Speckit workspace tab 및 agent run panel

**Project Type**: pnpm/Turbo monorepo 안의 단일 desktop app frontend 변경

**Performance Goals**: Speckit 탭을 열거나 파일 변경이 갱신된 뒤 활성 기능 및 4단계 상태를 1초 이내 표시한다. 일반적인 feature 목록에서 상태 계산은 사용자 조작을 차단하지 않는다.

**Constraints**: `.specify/feature.json`만 활성 기능 출처로 사용; 유효하지 않은 pointer를 선택 feature로 대체하지 않음; 초안은 자동 전송 금지; 승인 게이트와 재실행은 사용자 확인 필요; app-to-app import 및 새 shared package 금지; worktree root 밖 경로 접근 금지

**Scale/Scope**: `apps/agentic-workbench`의 Worktree Session 페이지, `features/worktree-workspace`, `features/agent-run`, `entities/worktree-file` query adapter. SDD 단계는 specify, plan, tasks, implement 4개와 spec/plan 검토 확인으로 제한한다.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. 모든 런타임 변경은 `apps/agentic-workbench` 안에 두며, `specs/031-sdd-workflow-controls`는 설계 산출물만 가진다. 다른 앱 import나 새 package/crate는 없다.
- **Feature-Sliced Frontend Architecture**: PASS. 순수 활성 기능/단계/prompt 모델은 `features/worktree-workspace/model`, Speckit 제어 UI는 `features/worktree-workspace/ui`, session 간 prompt 요청 조합은 `pages/project-worktree-session`, prompt request 처리 변화는 `features/agent-run`에 둔다. filesystem invoke adapter는 기존 `entities/worktree-file`을 재사용한다.
- **Hexagonal Tauri Backend Architecture**: N/A. 기존 `read_worktree_text_file` command가 root-relative `.specify/feature.json`을 이미 안전하게 읽으므로 backend 변경은 필요 없다.
- **Shared Core Before Shared UI**: PASS. AW 단일 앱 기능이다. 먼저 app-local pure state reducer/builder를 만들고 UI는 기존 panel과 prompt input을 조합한다.
- **Atomic Cross-App Verification**: N/A. `packages/*`와 `crates/*` 변경이 없다.
- **Documentation and Storybook**: PASS. 재사용 가능한 SDD control organism은 AW Storybook에 active, unavailable, approval, loading/error 상태로 등록한다. 별도 `docs/` 문서는 새 아키텍처가 아닌 기능 구현이므로 필요하지 않다.
- **Testing and Safety**: PASS. JSON pointer parsing과 feature-path validation, 단계 전이, prompt builders, active-panel routing, invalid/missing pointer, stale refresh, approval/re-run confirmation을 focused test로 고정한다. 기존 worktree-file root/path validation을 그대로 사용한다.

## Project Structure

### Documentation (this feature)

```text
specs/031-sdd-workflow-controls/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── sdd-workflow-controls-ui.md
├── checklists/
│   └── requirements.md
└── tasks.md                         # /speckit-tasks 단계에서 생성
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── pages/project-worktree-session/ui/
│   └── project-worktree-session-page.tsx
├── features/
│   ├── agent-run/
│   │   ├── model/agent-run-panel-slots.ts
│   │   ├── model/agent-run-panel-slots.test.ts
│   │   └── ui/agent-run-panel.tsx
│   └── worktree-workspace/
│       ├── model/sdd-workflow.ts              # new pure model
│       ├── model/sdd-workflow.test.ts          # new
│       ├── ui/sdd-workflow-controls.tsx        # new organism
│       ├── ui/sdd-workflow-controls.test.tsx   # new
│       ├── ui/speckit-files-panel.tsx
│       ├── ui/speckit-files-panel.test.tsx
│       └── ui/worktree-workspace-panel.tsx
├── entities/worktree-file/
│   └── api/query-keys.ts
├── shared/storybook/sample-data.ts
└── stories/organisms.stories.tsx
```

**Structure Decision**: `.specify/feature.json`은 기존 `readWorktreeTextFile` query로 읽는다. SDD pointer parsing, feature-path validation, stage derivation, button availability, prompt text creation은 `features/worktree-workspace/model/sdd-workflow.ts`의 side-effect 없는 함수로 둔다. Speckit tab container가 pointer query와 document list를 결합해 controls와 file panel에 `activeFeaturePath`를 전달한다. 페이지는 workspace callback을 `AgentPromptRequest`로 변환한다. `AgentPromptRequest`에는 즉시 실행 요청과 편집 가능한 draft 주입을 구분하는 delivery mode를 추가하고, agent run panel은 draft를 textarea에만 설정한다.

## Complexity Tracking

No constitution violations.

## Phase 0 Research

See [research.md](./research.md).

## Phase 1 Design

See [data-model.md](./data-model.md), [contracts/sdd-workflow-controls-ui.md](./contracts/sdd-workflow-controls-ui.md), and [quickstart.md](./quickstart.md).

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. Final design remains inside AW and does not add cross-app dependencies.
- **Feature-Sliced Frontend Architecture**: PASS. File query adapter remains entity-owned; pure SDD model and controls belong to the existing worktree workspace feature; agent prompt delivery stays in agent-run; page only composes callbacks.
- **Hexagonal Tauri Backend Architecture**: N/A. The existing root-relative text-file command provides the required checked file read with no boundary changes.
- **Shared Core Before Shared UI**: PASS. SDD stage and prompt derivation are pure app-local logic, covered before UI integration; no shared UI package is planned.
- **Atomic Cross-App Verification**: N/A. No shared package/crate changes.
- **Documentation and Storybook**: PASS. Storybook coverage is specified for the new SDD controls and highlighted panel state.
- **Testing and Safety**: PASS. The model rejects malformed, absolute, traversal, and non-`specs/` pointers; UI does not auto-send drafts; confirmation states and file refresh are covered.

## Agent Context Update

No agent-context update script is present in this Spec Kit installation; no context files were generated or modified.

## Kanban and Needed-Tasks Extension

선택된 `tasks.md`의 content를 기존 Speckit feature model과 분리된 순수 parser로 해석한다. parser는 checkbox 작업, 원본 순서, 완료 여부, 가장 가까운 상위 heading, heading 아래의 비작업 문맥을 반환한다. UI는 Markdown preview를 보존하면서 `Preview`, `Kanban`, `작업 필요` 보기를 전환한다. Kanban은 미완료/완료 열과 상태 필터를 제공하고, 작업 필요 보기는 미완료 작업을 하나 이상 포함한 section만 그 heading과 문맥을 포함해 렌더한다. 완료 행과 완료-only section은 절대 렌더하지 않는다.
