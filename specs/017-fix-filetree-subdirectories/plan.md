# Implementation Plan: AW Worktree Session Files 하위 디렉터리 조회 수정

**Branch**: `017-fix-filetree-subdirectories` | **Date**: 2026-07-08 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/017-fix-filetree-subdirectories/spec.md`

## Summary

릴리즈 버전의 AW worktree session page Files 섹션에서 하위 디렉터리 파일을 선택해도 내용 조회가 실패하거나 다른 상태에 머무르는 회귀를 수정한다. 구현은 `apps/agentic-workbench` 안에서 파일트리 lazy loading, 선택 경로 보존, preview query 상태 전환을 우선 검증하고, 필요한 경우 Tauri `read_worktree_text_file` 경로 검증/조회 경계를 보강한다. 루트 파일, 1단계 하위 파일, 다단계 하위 파일, 같은 이름의 다른 경로, 공백/한글 경로, 읽기 실패 상태를 fixture로 고정한다.

## Technical Context

**Language/Version**: TypeScript 5.x, React 19, Vite 기반 Tauri frontend; Rust 2024 edition Tauri backend

**Primary Dependencies**: TanStack Query, Tauri `invoke`, existing AW shadcn/Radix UI primitives, Rust `walkdir`, filesystem provider/service layers

**Storage**: 영구 저장소 변경 없음. worktree filesystem을 읽기 전용으로 조회하고 preview state는 ephemeral UI state다.

**Testing**: Vitest/React Testing Library 또는 기존 AW frontend test stack, Storybook state coverage, Rust `cargo test` for `apps/agentic-workbench/src-tauri`

**Target Platform**: macOS desktop Tauri app release/dev execution paths; resizable Worktree Session 화면

**Project Type**: pnpm/Turbo monorepo의 desktop app frontend plus Tauri backend

**Performance Goals**: 루트 목록은 기존 lazy loading 방식을 유지하고, 폴더 펼침 시 직계 항목만 조회한다. 파일 선택 후 preview는 일반 텍스트 파일에서 사용자 흐름을 끊지 않을 정도로 즉시 갱신되어야 한다.

**Constraints**: app-to-app import 금지, 신규 shared package 도입 없음, 파일 조회는 현재 worktree root 안으로 제한, 파일 수정/생성/삭제 기능 추가 없음, 릴리즈 빌드 또는 릴리즈에 준하는 실행 경로 검증 필요

**Scale/Scope**: `apps/agentic-workbench`의 Worktree Session Files 섹션 한 화면. 대표 fixture는 루트 파일, 1단계 하위 파일, 2단계 이상 하위 파일, 같은 basename의 다른 경로, 공백/한글 경로, 삭제/읽기 실패 파일을 포함한다.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS. 변경 범위는 `apps/agentic-workbench` 내부 frontend와 Tauri backend로 제한한다. `packages/*`, `crates/*`, 다른 앱 import는 계획하지 않는다.
- **Feature-Sliced Frontend Architecture**: PASS. Worktree Session 화면 조합은 `apps/agentic-workbench/src/pages/project-worktree-session`, Files 섹션 상호작용은 기존 `features/worktree-workspace`, 파일 entity adapter/type은 `entities/worktree-file` 경계를 따른다.
- **Hexagonal Tauri Backend Architecture**: PASS. 필요한 backend 수정은 `domain/worktree_file*`, `application/worktree_file_service.rs`, `inbound/tauri_commands.rs`, `infrastructure/fs_worktree_file_provider.rs` 경계를 유지한다. Tauri command는 service 위임만 유지한다.
- **Shared Core Before Shared UI**: PASS. AW 한 화면의 회귀 수정이므로 신규 shared UI/core를 만들지 않는다. 재사용 필요가 확인되면 먼저 headless 경로/선택 helper를 AW 내부에서 검증한다.
- **Atomic Cross-App Verification**: N/A. `packages/*` 또는 `crates/*` 변경을 계획하지 않는다.
- **Documentation and Storybook**: PASS. 재사용 가능한 Files tree/viewer 상태가 변경되면 AW Storybook에 하위 파일, 같은 이름 파일, 오류 상태를 추가한다. 별도 장기 문서는 필요하지 않다.
- **Testing and Safety**: PASS. frontend fixture tests와 backend path/root validation tests를 계획한다. 파일 조회는 worktree root 안으로 제한하고, escape path와 읽기 실패를 검증한다.

## Project Structure

### Documentation (this feature)

```text
specs/017-fix-filetree-subdirectories/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── worktree-files-ui.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── pages/project-worktree-session/ui/project-worktree-session-page.tsx
├── features/worktree-workspace/ui/worktree-workspace-panel.tsx
├── features/worktree-workspace/ui/*.test.tsx
├── entities/worktree-file/api/worktree-file-repository.ts
├── entities/worktree-file/api/query-keys.ts
├── entities/worktree-file/model/types.ts
├── shared/storybook/sample-data.ts
└── stories/*.stories.tsx

apps/agentic-workbench/src-tauri/src/
├── domain/worktree_file.rs
├── domain/worktree_file_provider.rs
├── application/worktree_file_service.rs
├── inbound/tauri_commands.rs
└── infrastructure/fs_worktree_file_provider.rs
```

**Structure Decision**: 사용자-visible 회귀는 `features/worktree-workspace/ui/worktree-workspace-panel.tsx`의 Files tab에서 드러나므로 selection, expanded folder, query state 테스트를 이 경계에 둔다. Tauri path safety와 nested file read behavior는 기존 `worktree_file_service`와 `FsWorktreeFileProvider` 테스트로 고정한다. `entities/worktree-file`은 invoke contract와 query key가 변경될 때만 갱신한다.

## Complexity Tracking

No constitution violations.

## Phase 0 Research

See [research.md](./research.md).

## Phase 1 Design

See [data-model.md](./data-model.md), [contracts/worktree-files-ui.md](./contracts/worktree-files-ui.md), and [quickstart.md](./quickstart.md).

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. Design artifacts keep the feature inside `apps/agentic-workbench` and avoid cross-app sharing.
- **Feature-Sliced Frontend Architecture**: PASS. Data model and contract place page composition, Files interaction, and worktree-file entity concerns in their existing FSD layers.
- **Hexagonal Tauri Backend Architecture**: PASS. Contract preserves Tauri command delegation and keeps filesystem read/validation inside application/infrastructure boundaries.
- **Shared Core Before Shared UI**: PASS. No shared UI package is introduced; Storybook updates remain app-local.
- **Atomic Cross-App Verification**: N/A. No shared workspace package/crate changes are planned.
- **Documentation and Storybook**: PASS. Quickstart includes Storybook/fixture expectations for reusable or changed Files tree states.
- **Testing and Safety**: PASS. Quickstart and contract include release-path validation, nested path fixtures, duplicate basename cases, UTF-8/special path cases, read failure state, and root escape rejection.

## Agent Context Update

No agent-context update script is present in this Spec Kit installation; no context files were generated or modified.
