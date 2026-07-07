# Implementation Plan: AW Git Commit 상세 한글 파일명 표시 수정

**Branch**: `[no branch]` | **Date**: 2026-07-07 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/015-fix-korean-filenames/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

AW의 Git 커밋 상세 화면에서 한글 파일명이 `\\355\\202\\244\\354\\230\\244` 같은 백슬래시+8진수 바이트 표기로 보이는 문제를 수정한다. 기술 접근은 공유 Rust Git core에서 커밋 파일 경로를 사용자 표시 가능한 UTF-8 경로로 안정화하고, AW가 사용하는 공통 Git UI가 그 값을 목록, 선택 상태, diff 표시에서 일관되게 사용하도록 fixture와 UI 검증을 추가하는 것이다.

## Technical Context

**Language/Version**: Rust 2021/2024 workspace, TypeScript 5, React 19

**Primary Dependencies**: Rust `std::process::Command`, shared `git-core`, Tauri 2, `@yoophi/git-graph`, `@yoophi/git-ui`, React Query, Vitest

**Storage**: N/A. 읽기 전용 Git commit detail 표시 개선이며 repository 파일, Git 이력, 앱 persistence를 변경하지 않는다.

**Testing**: `cargo test -p git-core`, `cargo check -p agentic-workbench`, `pnpm --filter @yoophi/git-ui test`, `pnpm --filter @yoophi/git-ui check-types`, `pnpm --filter @yoophi/agentic-workbench test`, `pnpm --filter @yoophi/agentic-workbench check-types`

**Target Platform**: Tauri desktop app, primary local development target macOS; Git CLI가 설치된 로컬 repository

**Project Type**: pnpm/Turbo monorepo + Rust Cargo workspace desktop app with shared Git core and shared React Git UI

**Performance Goals**: 커밋 상세 파일 목록 파싱은 기존 커밋 상세 로딩 체감 시간을 악화시키지 않아야 하며, 대표 커밋 10개에서 한글 파일명을 100% 사람이 읽을 수 있게 표시해야 한다.

**Constraints**: 경로 표시 보정은 파일명, Git 이력, 작업트리 상태를 변경하지 않는다. 기존 영문/숫자 경로와 파일 선택 흐름의 회귀가 0건이어야 한다. `packages/*` 또는 `crates/*` 변경 시 소비 앱 검증을 함께 수행한다.

**Scale/Scope**: AW Git commit detail 화면의 changed files list, selected file path, diff file header display. 필요 시 `crates/git-core`와 `packages/git-ui`까지 포함하되 cross-app UI 변경은 최소화한다.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. AW app integration remains under `apps/agentic-workbench`; shared Git parsing/display contracts remain in `crates/git-core`, `packages/git-graph`, and `packages/git-ui`.
- **Feature-Sliced Frontend Architecture**: PASS. AW screen integration stays in `apps/agentic-workbench/src/features/worktree-workspace` and `apps/agentic-workbench/src/entities/worktree-git`; reusable commit detail UI stays in `packages/git-ui`.
- **Hexagonal Tauri Backend Architecture**: PASS. Tauri commands in `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs` continue delegating to `application/worktree_git_service.rs`, which delegates to `infrastructure/git_cli_worktree_git_provider.rs` and shared `git-core`.
- **Shared Core Before Shared UI**: PASS. Path normalization/parsing belongs in pure `crates/git-core` first. UI only renders provided displayable paths and gets fixture/story coverage.
- **Atomic Cross-App Verification**: PASS. Because `crates/git-core` and `packages/git-ui` may change, verification includes `git-core` tests, AW Rust check, shared package type/test checks, and AW frontend checks.
- **Documentation and Storybook**: PASS. No project-level `docs/*.md` update is required for this narrow bug fix. Storybook/sample data should include a commit detail state with Korean paths or octal-quoted path regression data.
- **Testing and Safety**: PASS. Plan includes Rust fixture tests for quoted octal paths, parser behavior for normal paths and rename paths, and UI tests/story fixtures for long Korean path rendering. No filesystem write/persistence path is introduced.

## Project Structure

### Documentation (this feature)

```text
specs/015-fix-korean-filenames/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── commit-detail-path-display.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/git-core/src/
├── git_cli.rs            # Git commit detail/file diff CLI calls and path parsing fixtures
├── domain.rs             # GitCommitFileChange / GitFileDiff shared Rust domain shape
└── ports.rs              # GitHistoryReader contract

packages/git-graph/src/
└── types.ts              # TypeScript mirror of GitCommitDetail and GitFileDiff

packages/git-ui/src/
├── ui/commit-detail-view.tsx
└── model/file-tree.ts

apps/agentic-workbench/src/
├── entities/worktree-git/api/worktree-git-repository.ts
└── features/worktree-workspace/ui/worktree-workspace-panel.tsx

apps/agentic-workbench/src-tauri/src/
├── application/worktree_git_service.rs
├── infrastructure/git_cli_worktree_git_provider.rs
└── inbound/tauri_commands.rs
```

**Structure Decision**: The primary fix should be made at the shared Git data boundary in `crates/git-core` so AW receives displayable path strings before they reach Tauri serialization or React UI. `packages/git-ui` should remain presentation-focused and only add fixtures/tests/stories if needed to prove display behavior. AW-specific files should only need integration verification unless the selected file/diff flow requires path handling changes.

## Phase 0: Research Summary

See [research.md](./research.md). All technical unknowns are resolved.

## Phase 1: Design Summary

See [data-model.md](./data-model.md), [contracts/commit-detail-path-display.md](./contracts/commit-detail-path-display.md), and [quickstart.md](./quickstart.md).

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. The design keeps shared Git path semantics in `crates/git-core` and shared UI rendering in `packages/git-ui`, with AW consuming through existing ports.
- **Feature-Sliced Frontend Architecture**: PASS. No app-level composition or routing changes are planned; AW feature/entity boundaries remain intact.
- **Hexagonal Tauri Backend Architecture**: PASS. Existing command-service-provider layering is preserved.
- **Shared Core Before Shared UI**: PASS. Displayable path behavior is defined as data contract/core behavior before UI presentation.
- **Atomic Cross-App Verification**: PASS. Verification matrix covers shared Rust, shared TypeScript packages, and AW consumers.
- **Documentation and Storybook**: PASS. A Korean path Storybook/sample state is planned for reusable Git UI; no standalone docs are needed.
- **Testing and Safety**: PASS. Fixture tests cover normal UTF-8, quoted octal, mixed ASCII/Korean, and rename paths without introducing persistence or file writes.

## Complexity Tracking

No constitution violations.
