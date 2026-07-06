# Implementation Plan: MCP Session Title Control

**Branch**: `main` | **Date**: 2026-07-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/011-mcp-session-ui-control/spec.md`

## Summary

AW(agentic-workbench)가 agent 실행 중 사용할 수 있는 로컬 MCP HTTP 서비스를 제공하고, 초기 capability는 Worktree Session 윈도우 타이틀 변경 하나로 제한한다. agent 실행 시 MCP endpoint, run id, 인증 토큰을 agent 환경에 주입하고, MCP tool 호출은 활성 run/session 소유권과 제목 유효성을 검증한 뒤 해당 세션 윈도우에만 title 변경 이벤트를 전달한다. unstaged 변경 표시, 파일/Markdown preview 제어, 파일 수정 기능은 이번 범위에서 제외한다.

## Technical Context

**Language/Version**: Rust 2024 edition, Tauri 2, Tokio 1.x, TypeScript 5.6, React 19

**Primary Dependencies**:
- `apps/agentic-workbench/src-tauri`: 기존 Tauri backend, `tokio`, `serde`, `serde_json`, `uuid`, `anyhow`
- 신규 Rust HTTP server dependency: `axum` + `tower-http` 또는 동등한 Tokio 기반 HTTP stack (research R2 결정)
- `apps/agentic-workbench/src`: Tauri JS API `getCurrentWindow().setTitle`, React route/page composition
- MCP transport contract: Streamable HTTP compatible single MCP endpoint, tools/list, tools/call 최소 지원

**Storage**: N/A. agent 제공 title은 runtime window state이며 영속 저장하지 않는다.

**Testing**:
- Rust: `cargo test -p agentic-workbench` 또는 `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml`
- TypeScript: `pnpm --filter @yoophi/agentic-workbench check-types`, `pnpm --filter @yoophi/agentic-workbench test`
- Manual/dev validation: Worktree Session에서 agent run 시작 후 MCP `set_window_title` 호출로 단일 window title 변경 확인

**Target Platform**: Tauri desktop app webview, local agent subprocess

**Project Type**: pnpm/Turbo monorepo + Rust Cargo workspace; app-specific desktop application feature

**Performance Goals**:
- 열린 Worktree Session에서 valid title 요청의 95%가 1초 이내 title 반영
- title validation과 session-owner lookup은 in-memory lookup 기준으로 사용자 체감 지연 없음

**Constraints**:
- MCP service는 localhost 전용으로 bind하고 인증 토큰을 요구한다.
- initial MCP capability는 window title 변경만 노출한다.
- run/session owner scope 검증 전에는 어떤 window title도 변경하지 않는다.
- user-provided/profile env와 충돌하지 않도록 내부 MCP env key는 예약 prefix를 사용한다.
- agent conversation, permission state, workspace navigation state를 변경하지 않는다.

**Scale/Scope**:
- 앱 1개: `apps/agentic-workbench`
- backend modules: MCP service, title validation/use case, agent launch env injection, app state/session ownership integration
- frontend touchpoints: Worktree Session route/page title override listener and title application helper/test
- docs: Korean architecture document 1개 under `docs/*.md`
- no `packages/*` or `crates/*` changes

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Monorepo Boundary First**: PASS — 변경 범위는 `apps/agentic-workbench`와 `docs/`에 한정한다. 공유 TypeScript package나 Rust crate를 만들지 않으며 app-to-app import가 없다.
- **Feature-Sliced Frontend Architecture**: PASS — session route composition은 `app/pages`, title-control interaction helper/listener는 필요 시 `features/agent-run` 또는 `features/worktree-workspace`에 둔다. 기존 project title formatter는 `entities/project/lib`에 유지한다.
- **Hexagonal Tauri Backend Architecture**: PASS — title validation/domain model은 domain 또는 application pure helper, session ownership/title command use case는 application, MCP HTTP transport와 window event delivery는 infrastructure/inbound에 둔다. Tauri command나 HTTP handler가 직접 business rule을 소유하지 않게 한다.
- **Shared Core Before Shared UI**: PASS — 공유 UI 없음. 재사용이 필요한 경우에도 title validation/result modeling 같은 headless core만 앱 내부에서 분리한다.
- **Atomic Cross-App Verification**: PASS — `packages/*`와 `crates/*` 변경 없음. AW app TypeScript/Rust 검증만 필요하다.
- **Documentation and Storybook**: PASS — `docs/mcp-session-title-control.md`를 한국어로 추가하고 Mermaid flow를 포함한다. 신규 reusable UI component가 없으므로 Storybook은 불필요하다.
- **Testing and Safety**: PASS — pure title validation, session-owner authorization, token validation, capability list 제한, frontend title application을 테스트한다. 파일 접근/경로 검증은 이번 범위에서 제외되며 파일 capability 자체를 노출하지 않는 테스트로 안전 경계를 확인한다.

**Post-Phase 1 재평가**: data model과 contract 기준 Constitution 위반 없음. 앱 내부 변경만 포함하며 shared package/crate 변경이 없다. Complexity Tracking 불필요.

**Agent context update**: `.specify/scripts/bash/update-agent-context.sh`가 이 프로젝트에 없어 실행을 건너뛰었다. 별도 agent context 파일 갱신은 발생하지 않았다.

## Project Structure

### Documentation (this feature)

```text
specs/011-mcp-session-ui-control/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── mcp-title-control.md
└── tasks.md                 # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
apps/agentic-workbench/src/
├── app/
│   └── App.tsx                              # Worktree Session route title override 적용
├── pages/project-worktree-session/ui/
│   └── project-worktree-session-page.tsx    # 필요한 경우 title-control listener 조립
├── features/agent-run/
│   └── model/                               # agent run request/env 조립 테스트 필요 시
└── entities/project/lib/
    ├── worktree-window-title.ts             # default/custom title formatting rule 확장 가능
    └── worktree-window-title.test.ts

apps/agentic-workbench/src-tauri/src/
├── domain/
│   ├── mcp_title_control.rs                 # title request/result/value validation
│   └── mod.rs
├── application/
│   ├── mcp_title_control_service.rs         # owner scope + title command application contract
│   └── mod.rs
├── infrastructure/
│   ├── mcp/
│   │   ├── mod.rs                           # MCP HTTP service bootstrap/router
│   │   ├── protocol.rs                      # minimal JSON-RPC/MCP request/response shapes
│   │   └── title_tool.rs                    # tools/list + tools/call routing
│   ├── agent_session_registry.rs            # active run owner lookup 재사용/확장
│   └── mod.rs
├── inbound/
│   └── tauri_commands.rs                    # start_agent_run env injection integration
└── lib.rs                                   # MCP service managed state/bootstrap

docs/
└── mcp-session-title-control.md             # 한국어 설계 문서 + Mermaid flow
```

**Structure Decision**: MCP HTTP와 JSON-RPC parsing은 Tauri 외부 입력 어댑터이므로 `infrastructure/mcp`에 둔다. 세션 소유권 검증과 title validation은 테스트 가능한 순수 application/domain 단위로 분리한다. Frontend는 기존 Worktree Session route의 `windowTitle` 계산 위에 agent-provided runtime title override만 얹는다.

## Complexity Tracking

> Constitution Check 위반 없음 — 해당 없음.
