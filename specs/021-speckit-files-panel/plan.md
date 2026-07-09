# Implementation Plan: Speckit Files Panel

**Branch**: `main` | **Date**: 2026-07-09 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/021-speckit-files-panel/spec.md`

## Summary

AW worktree session 화면에 Speckit 전용 패널을 추가해 현재 worktree의 `specs/*` 기능 폴더, 주요 Speckit 문서, `tasks.md` 진행 요약을 한 곳에서 탐색하고 기존 markdown 검토 흐름으로 열 수 있게 한다. 구현은 `apps/agentic-workbench` 내부에 한정하며, 기존 worktree file 조회/감시 경계를 재사용하고 Speckit 문서 분류와 tasks 진행 계산은 frontend entity/model의 순수 로직으로 검증한다. 파일 접근이 새 backend 명령을 필요로 하는 경우에도 기존 hexagonal worktree file 경계를 확장하는 방식으로 제한한다.

## Technical Context

**Language/Version**: TypeScript 5.x, React 19, Vite 기반 Tauri frontend; Rust 2024 edition Tauri backend when file command extension is needed

**Primary Dependencies**: TanStack Query, Tauri `invoke`, existing AW shadcn/Radix UI primitives, lucide-react icons, existing worktree file watcher and markdown viewer flow

**Storage**: 영구 저장소 변경 없음. 현재 worktree filesystem을 읽기 전용으로 조회하고 Speckit panel selection/filter state는 ephemeral UI state로 유지한다.

**Testing**: Vitest/React Testing Library for frontend model and UI, Storybook state coverage, Rust `cargo test` for `apps/agentic-workbench/src-tauri` only if backend file command/provider behavior changes

**Target Platform**: macOS desktop Tauri app; AW Worktree Session 화면의 resizable panel 환경

**Project Type**: pnpm/Turbo monorepo의 desktop app frontend plus existing Tauri backend boundary

**Performance Goals**: Speckit 구조가 있는 worktree에서 패널 진입 후 5초 이내 목록 표시. 대표 `tasks.md` 20개에 대해 진행 요약 계산이 사용자 조작을 막지 않아야 한다. 문서 추가/수정/삭제 후 3초 이내 또는 명시적 갱신 1회 이내 최신 상태 확인.

**Constraints**: app-to-app import 금지, 신규 shared package 도입 없음, 파일 접근은 worktree root 안으로 제한, 문서 편집/생성/삭제 기능은 제외, `contracts/*`와 `checklists/*` 하위 문서는 기능별로 구분 표시

**Scale/Scope**: `apps/agentic-workbench` Worktree Session의 workspace 패널 한 영역. 대상 데이터는 `specs/*` 기능 디렉터리와 `spec.md`, `plan.md`, `tasks.md`, `research.md`, `data-model.md`, `quickstart.md`, `contracts/*`, `checklists/*` markdown 문서다.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. 변경 범위는 `apps/agentic-workbench`와 `specs/021-speckit-files-panel` 산출물에 한정한다. `packages/*`, `crates/*`, 다른 앱 import는 계획하지 않는다.
- **Feature-Sliced Frontend Architecture**: PASS. Worktree Session 화면 조합은 `pages/project-worktree-session`, Speckit 패널 상호작용은 `features/worktree-workspace` 또는 별도 `features/worktree-speckit`, Speckit feature/document/progress 모델과 query adapter는 `entities`, 작은 공용 UI는 기존 `shared`/`components/ui` 경계를 따른다.
- **Hexagonal Tauri Backend Architecture**: PASS. backend 변경이 필요하면 `domain/worktree_file*`, `application/worktree_file_service.rs`, `inbound/tauri_commands.rs`, `infrastructure/fs_worktree_file_provider.rs` 경계를 유지하고 Tauri command는 service 위임만 수행한다.
- **Shared Core Before Shared UI**: PASS. AW 단일 화면 기능이므로 신규 shared UI package를 만들지 않는다. 문서 분류와 tasks 진행 계산은 app-local pure model로 먼저 검증한다.
- **Atomic Cross-App Verification**: N/A. `packages/*` 또는 `crates/*` 변경을 계획하지 않는다.
- **Documentation and Storybook**: PASS. 패널 UI는 AW Storybook organisms/pages에 기본, 빈 상태, 오류, 긴 목록, tasks 진행 요약 상태로 등록한다. 별도 프로젝트 문서가 필요해지면 `docs/*.md` English filename/Korean content 규칙을 따른다.
- **Testing and Safety**: PASS. Speckit 문서 분류와 tasks progress parser는 fixture unit test로 검증한다. 파일 접근은 기존 worktree root/path validation을 유지하고, backend 확장 시 escape path, UTF-8, size limit, read failure를 Rust test로 고정한다.

## Project Structure

### Documentation (this feature)

```text
specs/021-speckit-files-panel/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── speckit-panel-ui.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── pages/project-worktree-session/ui/project-worktree-session-page.tsx
├── features/worktree-workspace/
│   ├── model/
│   │   ├── speckit-files.ts
│   │   └── speckit-files.test.ts
│   └── ui/
│       ├── speckit-files-panel.tsx
│       ├── speckit-files-panel.test.tsx
│       └── worktree-workspace-panel.tsx
├── entities/worktree-file/
│   ├── api/worktree-file-repository.ts
│   ├── api/query-keys.ts
│   └── model/types.ts
├── shared/storybook/sample-data.ts
└── stories/
    ├── organisms.stories.tsx
    └── pages.stories.tsx

apps/agentic-workbench/src-tauri/src/
├── domain/worktree_file.rs
├── domain/worktree_file_provider.rs
├── application/worktree_file_service.rs
├── inbound/tauri_commands.rs
└── infrastructure/fs_worktree_file_provider.rs
```

**Structure Decision**: 사용자-visible UI는 기존 `WorktreeWorkspacePanel`의 Git/Files/Markdown tab 구성에 Speckit tab을 추가하는 방식으로 시작한다. Speckit feature/document/progress 계산은 `features/worktree-workspace/model`에 app-local pure helper로 둔다. 기존 `entities/worktree-file` invoke contract로 필요한 파일 목록과 markdown content를 조회할 수 있으면 backend 변경을 피하고, 부족한 경우에만 기존 worktree file service/provider 경계를 확장한다.

## Complexity Tracking

No constitution violations.

## Phase 0 Research

See [research.md](./research.md).

## Phase 1 Design

See [data-model.md](./data-model.md), [contracts/speckit-panel-ui.md](./contracts/speckit-panel-ui.md), and [quickstart.md](./quickstart.md).

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. Design artifacts keep implementation inside `apps/agentic-workbench`; no cross-app sharing or app-to-app imports are introduced.
- **Feature-Sliced Frontend Architecture**: PASS. The UI contract maps tab composition, Speckit interaction, and document/progress models to existing page/feature/entity boundaries.
- **Hexagonal Tauri Backend Architecture**: PASS. Design prefers existing worktree file commands; any backend expansion remains behind application service/provider boundaries with Tauri commands delegating.
- **Shared Core Before Shared UI**: PASS. Pure Speckit classification/progress logic is planned before UI reuse. No shared UI package is introduced.
- **Atomic Cross-App Verification**: N/A. No shared workspace package/crate changes are planned.
- **Documentation and Storybook**: PASS. Storybook coverage is explicitly required for default, empty, error, long-list, and progress-summary states.
- **Testing and Safety**: PASS. Research, data model, contract, and quickstart cover fixture-based parser tests, UI tests, worktree root/path safety, read failure, UTF-8/large file behavior, and watcher invalidation.

## Agent Context Update

No agent-context update script is present in this Spec Kit installation; no context files were generated or modified.
