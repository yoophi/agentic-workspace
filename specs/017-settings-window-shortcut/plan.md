# Implementation Plan: 설정 별도 창과 단축어 실행

**Branch**: `` | **Date**: 2026-07-08 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/017-settings-window-shortcut/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Agentic Workbench의 현재 `/settings` 화면을 메인 작업 창 내 라우트가 아니라 앱 전역 단일 설정 창으로 연다. 기존 설정 UI와 저장 모델은 유지하되, 설정 창 전용 URL/route를 만들고 Tauri window manager에 `open_settings_window` 명령을 추가한다. macOS 앱 메뉴에는 `Preferences...` 항목과 `Cmd+,` accelerator를 연결하고, 기존 화면 내 설정 버튼과 agent-run 오류 진입점은 라우팅 대신 동일한 창 열기 명령을 호출한다.

## Technical Context

**Language/Version**: TypeScript 5.x, React 19, Rust 2024 edition

**Primary Dependencies**: Tauri 2, `@tauri-apps/api`, React Router HashRouter, TanStack Query, shadcn/Radix UI, `lucide-react`

**Storage**: 기존 agent run settings JSON 저장소 유지. 새 persistent storage 없음.

**Testing**: Vitest + React static/unit tests, `pnpm --dir apps/agentic-workbench check-types`, Rust `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml`, targeted rustfmt check, Storybook page stories

**Target Platform**: macOS desktop Tauri app. 기존 non-macOS app menu behavior는 회귀 없이 유지.

**Project Type**: Desktop app in pnpm/Turbo monorepo with Tauri backend

**Performance Goals**: `Cmd+,` 또는 기존 진입점 실행 후 1초 이내 설정 창 표시/포커스. 반복 실행 5회에도 설정 창 1개 유지.

**Constraints**: 메인 창의 현재 route/session/input 상태를 변경하지 않음. 설정 창 label은 고정값으로 단일 재사용. 세션 창의 owner-window cleanup 규칙은 `session-` label에만 계속 적용. 설정 저장 의미와 기존 query key 유지.

**Scale/Scope**: `apps/agentic-workbench` 단일 앱. 설정 창 1개, 메인 창/세션 창 여러 개에서 동일 설정 창을 열 수 있음. 공유 packages/crates 변경 없음.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: Does the plan keep app-specific code under
  `apps/*`, reusable TypeScript under `packages/*`, and reusable Rust under
  `crates/*`? Are app-to-app imports avoided?
  PASS. 모든 변경은 `apps/agentic-workbench`와 이 feature의 `specs/017-settings-window-shortcut`에 한정한다. `packages/*`, `crates/*`, 다른 앱 import 없음.
- **Feature-Sliced Frontend Architecture**: For frontend work, are changes
  placed in the correct `app`, `pages`, `features`, `entities`, `shared`, or
  `components/ui` layer?
  PASS. 앱 라우팅/창 모드 판별은 `apps/agentic-workbench/src/app`, 설정 화면은 `pages/settings`, 설정 창 열기 사용자 동작은 `features` 또는 app-level adapter, Tauri invoke wrapper는 `entities` API 경계에 둔다. shadcn 컴포넌트는 기존 `components/ui`만 사용한다.
- **Hexagonal Tauri Backend Architecture**: For backend work, are domain,
  application, inbound, infrastructure, and ports kept separate? Do Tauri
  commands delegate to application services?
  PASS. 창 생성은 OS/app shell adapter 성격이므로 `infrastructure/window_manager.rs`에 둔다. `inbound/tauri_commands.rs`는 `open_settings_window` command로 input 없는 위임만 수행한다. 설정 저장 도메인/application/repository는 변경하지 않는다.
- **Shared Core Before Shared UI**: If sharing is proposed, is pure core shared
  before UI? If UI is shared, is it independent of app shell/Tauri APIs?
  N/A. 공유 UI 또는 공유 core 승격 없음.
- **Atomic Cross-App Verification**: If `packages/*` or `crates/*` changes,
  does the plan list verification for all affected consumer apps?
  N/A. 공유 package/crate 변경 없음.
- **Documentation and Storybook**: Are required `docs/*.md` updates and
  Storybook stories identified for reusable UI?
  PASS. 설정 페이지의 별도 창 형태, 로딩/오류/긴 내용 상태를 `apps/agentic-workbench/src/stories/pages.stories.tsx`에 등록한다. 사용자 문서가 설정 접근 방식을 설명하는 경우에만 `docs/*.md` 한국어 문서를 갱신한다.
- **Testing and Safety**: Are unit/fixture tests planned for pure logic and
  root/path/session-owner validation planned for filesystem, persistence,
  agent, session, permission, or exchange changes?
  PASS. window manager 단일 label/URL/중복 방지 Rust tests와 frontend open-settings adapter tests를 계획한다. 새 filesystem/session/permission mutation이 없으며, 세션 cleanup은 `session-` label 한정 조건을 유지한다.

## Project Structure

### Documentation (this feature)

```text
specs/017-settings-window-shortcut/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── settings-window-ui.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── app/App.tsx
├── pages/settings/ui/settings-page.tsx
├── entities/settings-window/api/settings-window-repository.ts
├── features/agent-run/ui/agent-run-panel.tsx
├── features/agent-run/ui/worktree-agent-run-area.tsx
├── pages/project-worktree-session/ui/project-worktree-session-page.tsx
└── stories/pages.stories.tsx

apps/agentic-workbench/src-tauri/src/
├── inbound/tauri_commands.rs
├── infrastructure/window_manager.rs
└── lib.rs

apps/agentic-workbench/src-tauri/
└── capabilities/default.json
```

**Structure Decision**: 설정 창은 Agentic Workbench app-shell UX이므로 `apps/agentic-workbench` 내부에 둔다. UI 재사용은 기존 `SettingsPage`를 창 모드에서도 렌더링하도록 props를 조정하는 수준으로 제한한다. 창 열기 invoke wrapper는 frontend `entities/settings-window/api`에 두어 app/feature에서 Tauri command 문자열을 직접 알지 않게 한다. backend는 기존 `window_manager.rs`가 세션 창을 담당하므로 같은 infrastructure adapter에 설정 창 단일 label/URL/포커스 로직을 추가한다.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No constitution violations.

## Phase 0 Research

See [research.md](./research.md).

## Phase 1 Design

See [data-model.md](./data-model.md), [contracts/settings-window-ui.md](./contracts/settings-window-ui.md), and [quickstart.md](./quickstart.md).

## Post-Design Constitution Check

- **Monorepo Boundary First**: PASS. Design artifacts keep all source changes under `apps/agentic-workbench`.
- **Feature-Sliced Frontend Architecture**: PASS. App routing remains in `app`, screen composition in `pages/settings`, setting-window invoke boundary in `entities/settings-window/api`, user-facing entrypoints in existing page/feature components.
- **Hexagonal Tauri Backend Architecture**: PASS. Tauri command delegates to infrastructure window manager; existing settings domain/application/persistence boundaries remain unchanged.
- **Shared Core Before Shared UI**: N/A. No cross-app sharing is designed.
- **Atomic Cross-App Verification**: N/A. No `packages/*` or `crates/*` changes.
- **Documentation and Storybook**: PASS. Storybook page states are part of quickstart; docs update is conditional on existing user-facing docs mentioning settings navigation.
- **Testing and Safety**: PASS. Rust and frontend tests cover single-window reuse, route URL, repeated opening, settings save errors, and session state preservation.

## Agent Context Update

No agent-context update script is present in this Spec Kit installation; no context files were generated or modified.
