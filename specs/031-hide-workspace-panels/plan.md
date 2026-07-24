# Implementation Plan: Hide Workspace Panels

**Branch**: `현재 worktree` | **Date**: 2026-07-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/031-hide-workspace-panels/spec.md`

## Summary

Worktree Session의 Git, Files, Markdown, Speckit 보조 패널을 화면 가장 오른쪽의 세로 제어 영역에서 선택·해제한다. 선택 없음이면 보조 패널과 분할 핸들을 렌더링하지 않아 작업 영역을 완전히 사용한다. 바깥 분할과 각 패널 내부 분할은 `A:B = *:1`로 바꾸며, B(오른쪽 보조 영역) 폭만 Worktree별로 저장하고 A는 항상 남은 공간을 사용한다.

전용 `WorkspaceLayoutSettings` 도메인 모델·포트·애플리케이션 서비스·JSON 어댑터·Tauri 명령을 추가한다. 프런트엔드는 전용 entity API와 query key를 통해 레이아웃을 초기화·저장한다. Agent Run 설정과 레이아웃 설정은 수명과 책임이 다르므로 저장소를 공유하지 않는다.

## Technical Context

<!--
  ACTION REQUIRED: Replace the content in this section with the technical details
  for the project. The structure here is presented in advisory capacity to guide
  the iteration process.
-->

**Language/Version**: TypeScript 5.x / React 19.x, Rust 2024 edition

**Primary Dependencies**: Tauri 2, TanStack Query, `react-resizable-panels` 4.11.2, Tailwind CSS 4, shadcn/ui 스타일 프리미티브

**Storage**: 앱 데이터 디렉터리의 별도 JSON 파일. Worktree의 정규화된 경로를 키로 하는 레이아웃 설정 목록

**Testing**: Vitest/React Testing Library, Rust 단위 테스트, `pnpm --dir apps/agentic-workbench check-types`, `pnpm --dir apps/agentic-workbench test`, `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml`

**Target Platform**: Tauri 데스크톱 앱이 지원하는 macOS, Windows, Linux

**Project Type**: 모노레포 내 Agentic Workbench 데스크톱 앱

**Performance Goals**: 패널 선택·해제와 크기 반영은 즉시 체감 가능해야 하며, 드래그 중 저장 작업이 UI 반응성을 저해하지 않아야 한다.

**Constraints**: B 폭만 저장하고 A 폭은 저장하지 않는다. 저장된 B 폭이 현재 컨테이너에서 불가능하면 표시할 때만 안전 범위로 제한하며, 사용자의 원래 선호 값을 축소된 화면 때문에 덮어쓰지 않는다. 선택 없음 상태는 `tablist` 의미론을 사용하지 않고 접근 가능한 버튼 그룹으로 제공한다.

**Scale/Scope**: Worktree Session 외부 분할 1개와 Git, Files, Markdown, Speckit 내부 분할 최대 4개, Worktree별 레이아웃 설정 1개

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Answer each gate with PASS/FAIL/N/A and cite the concrete files or plan
sections that justify the answer.

- **Monorepo Boundary First — PASS**: 모든 변경은 `apps/agentic-workbench`에 한정한다. 두 번째 소비자가 없으므로 `packages/*` 또는 `crates/*` 승격은 하지 않는다.
- **Feature-Sliced Frontend Architecture — PASS**: 페이지는 바깥 분할 조립을 맡고, `features/worktree-workspace`는 패널 제어·내부 레이아웃을 맡는다. 새 레이아웃 모델과 Tauri API는 `entities/worktree-workspace-layout`에 둔다.
- **Hexagonal Tauri Backend Architecture — PASS**: 순수 `WorkspaceLayoutSettings`와 repository port는 domain, 저장·조회·정규화는 application, Tauri command는 inbound, JSON 파일은 infrastructure에 둔다.
- **Shared Core Before Shared UI — PASS**: AW 전용 Worktree Session 레이아웃이며 공용 UI 또는 코어 추출의 두 번째 소비자가 없다.
- **Atomic Cross-App Verification — N/A**: 공용 package/crate를 변경하지 않는다.
- **Documentation and Storybook — PASS**: 화면 전용 조립은 Storybook 필수가 아니다. 독립 재사용 제어 컴포넌트를 추출하는 경우에만 atomic 분류 Storybook을 추가한다. 기존 사용자 문서와 동작이 달라질 경우에만 `docs/*.md`를 갱신한다.
- **Testing and Safety — PASS**: 순수 폭 정규화·upsert·Worktree 격리 Rust 단위 테스트, 선택 토글·저장 요청·복원 렌더 프런트엔드 테스트, 접근성 버튼 및 빈 상태 컴포넌트 테스트를 추가한다. 경로 키는 비어 있지 않은 정규화된 Worktree 경로만 허용한다.

## Project Structure

### Documentation (this feature)

```text
specs/031-hide-workspace-panels/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)
<!--
  ACTION REQUIRED: Replace the placeholder tree below with the concrete layout
  for this feature. Delete unused options and expand the chosen structure with
  real paths (e.g., apps/admin, packages/something). The delivered plan must
  not include Option labels.
-->

```text
apps/agentic-workbench/
├── src/
│   ├── pages/project-worktree-session/ui/project-worktree-session-page.tsx
│   ├── features/worktree-workspace/
│   │   ├── model/workspace-layout.ts
│   │   └── ui/worktree-workspace-panel.tsx
│   └── entities/worktree-workspace-layout/
│       ├── api/query-keys.ts
│       ├── api/worktree-workspace-layout-repository.ts
│       └── model/types.ts
└── src-tauri/src/
    ├── domain/worktree_workspace_layout.rs
    ├── domain/worktree_workspace_layout_repository.rs
    ├── application/worktree_workspace_layout_service.rs
    ├── inbound/tauri_commands.rs
    ├── infrastructure/json_worktree_workspace_layout_repository.rs
    └── lib.rs
```

**Structure Decision**: Worktree Session에만 종속된 UI는 `pages`와 `features/worktree-workspace`에 남긴다. Worktree별 저장 레이아웃은 실행 설정과 독립된 도메인·repository port를 사용한다. Tauri 호출 경계는 entity API로 제한한다.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| 해당 없음 | — | — |
