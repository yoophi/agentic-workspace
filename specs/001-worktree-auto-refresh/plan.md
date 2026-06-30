# Implementation Plan: Worktree Auto Refresh

**Branch**: `001-worktree-auto-refresh` | **Date**: 2026-07-01 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-worktree-auto-refresh/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

`agentic-workbench`, `git-explorer`, `markdown-annotator`에서 active worktree/repository/markdown document의 파일 변경, Git commit/history/ref/dirty status 변경을 감지해 file tree, markdown preview, commit list/graph/detail을 자동 갱신한다. 구현 방향은 공통 refresh interval/stale selection/last-successful-data 정책을 reusable TypeScript package로 공유하고, 각 앱의 기존 Tauri command와 UI adapter에 연결하는 것이다.

## Technical Context

**Language/Version**: TypeScript 5.6, React 19, Rust 2024 edition

**Primary Dependencies**: Tauri 2, `@tanstack/react-query` 5, `react-resizable-panels`, `@yoophi/git-graph`, `@yoophi/git-ui`, `@yoophi/markdown-annotation-core`, `@yoophi/markdown-annotation-react`, Rust `git-core`, Git CLI adapters

**Storage**: N/A for new persistence. Feature reads active worktree/repository/document filesystem/Git state through existing Tauri commands or app-local file readers and keeps UI-only selection/refresh state in React state and React Query cache where available.

**Testing**: Vitest for shared refresh policy/selection reducer helpers; app checks for `@yoophi/agentic-workbench` and `@yoophi/markdown-annotator`; `git-explorer` typecheck plus Storybook build because it currently has no app-level test script; Rust unit tests for any changed Tauri backend; shared package tests for `packages/workspace-auto-refresh`.

**Target Platform**: Desktop Tauri app, local filesystem and Git worktrees.

**Project Type**: Cross-app monorepo desktop feature spanning three Tauri apps and one reusable TypeScript helper package.

**Performance Goals**: File and Git changes become visible within 3 seconds in the open workbench session, git-explorer repository, or markdown-annotator document; burst changes are coalesced so visible panes do not thrash or reset review context.

**Constraints**: Refresh scope is active `worktree.path`, selected repository id/path, or selected markdown file only; no app-to-app imports; shared behavior goes through `packages/*`; no new persistence; keep current root/path, size limit, and UTF-8 safeguards; preserve selected file/commit/document context when still valid; stale selections must be explicit when no longer valid.

**Scale/Scope**: `agentic-workbench` workspace panes (`git`, `files`, `markdown`), `git-explorer` repository changes panel, `markdown-annotator` active markdown document, and existing infinite commit/graph pagination.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. App-specific adapters stay under `apps/agentic-workbench`, `apps/git-explorer`, and `apps/markdown-annotator`; shared pure refresh policy/state helpers live under `packages/workspace-auto-refresh`; app-to-app imports are avoided.
- **Feature-Sliced Frontend Architecture**: PASS. `agentic-workbench` query keys/repositories remain in `entities/*`; refresh orchestration and pane state live in `features/worktree-workspace`; `git-explorer` and `markdown-annotator` keep their existing feature/entity boundaries.
- **Hexagonal Tauri Backend Architecture**: PASS. Current commands delegate to `application/*_service.rs` and infrastructure providers. Plan does not require persistence or direct command business logic. Any backend refresh helper must keep the same boundary.
- **Shared Core Before Shared UI**: PASS. A pure TypeScript helper package is proposed for refresh policy/stale state; shared UI is not required.
- **Atomic Cross-App Verification**: PASS. Shared package changes require package tests plus consuming app typecheck/test for workbench, git-explorer, and markdown-annotator.
- **Documentation and Storybook**: PASS. Update `docs/project-worktree-session-workspace-plan.md` and any app-local docs affected by cross-app auto reload. Add Storybook coverage only for reusable state UI if extracted.
- **Testing and Safety**: PASS. Unit tests are planned for query option/reducer logic; existing backend path safety remains in `worktree_file_service` and `FsWorktreeFileProvider`; worktree path is always query/command scoped.

## Project Structure

### Documentation (this feature)

```text
specs/001-worktree-auto-refresh/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── worktree-auto-refresh.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── pages/project-worktree-session/
│   └── ui/project-worktree-session-page.tsx
├── features/worktree-workspace/
│   ├── model/
│   │   ├── auto-refresh-options.ts
│   │   ├── selection-staleness.ts
│   │   └── *.test.ts
│   └── ui/
│       ├── worktree-workspace-panel.tsx
│       └── refresh-status-indicator.tsx
├── entities/worktree-file/
│   ├── api/query-keys.ts
│   └── api/worktree-file-repository.ts
├── entities/worktree-git/
│   ├── api/query-keys.ts
│   └── api/worktree-git-repository.ts
└── entities/project/
    ├── api/query-options.ts
    └── api/query-keys.ts

apps/agentic-workbench/src-tauri/src/
├── application/
│   ├── worktree_file_service.rs
│   └── worktree_git_service.rs
├── inbound/tauri_commands.rs
└── infrastructure/
    ├── fs_worktree_file_provider.rs
    └── git_cli_worktree_git_provider.rs

apps/git-explorer/src/
├── app/providers/query.tsx
├── entities/repository/api.ts
└── widgets/changes-panel/ui/ChangesPanel.tsx

apps/markdown-annotator/src/
├── entities/document/api/documentApi.ts
├── features/open-document/openMarkdownDocument.ts
└── pages/annotator/AnnotatorPage.tsx

packages/workspace-auto-refresh/src/
├── index.ts
├── refresh-options.ts
├── selection-staleness.ts
└── *.test.ts

docs/
└── project-worktree-session-workspace-plan.md
```

**Structure Decision**: 세 앱이 같은 auto reload 정책을 사용하므로 순수 refresh option/stale state 로직은 `packages/workspace-auto-refresh`에 둔다. 각 앱은 기존 Tauri command, repository API, document reader, UI state를 유지하고 shared helper를 adapter 형태로 소비한다. Backend 변경은 필요할 때만 각 앱의 기존 Tauri service/provider 경계 안에서 수행한다.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|

## Phase 0: Research

Research output: [research.md](./research.md)

핵심 결정:

- React Query `refetchInterval`, `refetchOnWindowFocus`, targeted `invalidateQueries`/`refetchQueries`를 pane별로 조합한다. React Query가 없는 markdown-annotator는 같은 interval policy를 local document reload adapter로 소비한다.
- 파일 목록과 Git 상태는 active worktree path, repository id/path, markdown file path key에만 묶는다.
- infinite query는 자동 갱신 시 첫 페이지 기준으로 최신 head를 갱신하고, 필요 시 loaded pages를 reset/refetch하는 정책을 명시한다.
- OS filesystem watcher는 1차 구현에서 도입하지 않는다. Tauri watcher는 burst/권한/platform 차이를 줄이기 위한 후속 최적화 후보로 남긴다.

## Phase 1: Design & Contracts

Design outputs:

- [data-model.md](./data-model.md)
- [contracts/worktree-auto-refresh.md](./contracts/worktree-auto-refresh.md)
- [quickstart.md](./quickstart.md)

Agent context update:

- `.specify/scripts` 아래에 agent context update script가 없어서 실행할 항목이 없다.

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. 산출물은 `apps/agentic-workbench`, `apps/git-explorer`, `apps/markdown-annotator`, `packages/workspace-auto-refresh`, `docs/`, `specs/`를 대상으로 한다.
- **Feature-Sliced Frontend Architecture**: PASS. Data model과 contract는 query key/adapter/entity와 feature orchestration 경계를 분리한다.
- **Hexagonal Tauri Backend Architecture**: PASS. Contract는 기존 Tauri command 사용을 전제로 하며, 새 command가 필요해도 inbound → application → provider 구조를 요구한다.
- **Shared Core Before Shared UI**: PASS. pure helper만 공유하고 UI 승격 없음.
- **Atomic Cross-App Verification**: PASS. 새 shared package와 세 소비 앱 검증이 필요하다.
- **Documentation and Storybook**: PASS. 기존 설계 문서 갱신과 선택적 Storybook 상태 보강이 quickstart/contract에 포함된다.
- **Testing and Safety**: PASS. Quickstart는 frontend test/check와 backend 변경 시 Rust test를 포함한다.
