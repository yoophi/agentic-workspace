---
type: Application Reference
title: Agentic Workbench (AW)
description: Agentic Workbench is the primary Tauri desktop application for Git worktree sessions, ACP agent runs, and Main Coordinator-led read-only agent orchestration.
tags: [agentic-workbench, tauri, acp, orchestration]
---

# Agentic Workbench (AW)

`apps/agentic-workbench` — 메인 데스크톱 워크벤치 앱. 프로젝트 관리, Git worktree 운영, ACP 에이전트 세션 실행을 통합 제공합니다.

## 프론트엔드 구조

소스 경로: `apps/agentic-workbench/src/`

### 라우팅

HashRouter 기반. `app/App.tsx`에서 라우트를 정의합니다.

| 라우트 | 페이지 | 용도 |
|--------|--------|------|
| `/` | `ProjectDashboardPage` | 프로젝트 대시보드 |
| `/projects/:id` | `ProjectDetailPage` | 단일 프로젝트 상세 + worktree 목록 |
| `/session/:projectId?worktreePath=...` | `ProjectWorktreeSessionPage` | **메인 세션 페이지** — 에이전트 실행 + 워크스페이스 |
| `/settings-window` | `SettingsPage` | 에이전트 프로필/명령어 오버라이드 설정 |

세션 라우트는 `worktreePath`를 쿼리스트링으로 전달하여 공백, 한글, 특수문자를 안전하게 처리합니다.

### 주요 페이지: 세션 페이지

`pages/project-worktree-session/ui/project-worktree-session-page.tsx`

The session uses a resizable layout.
- **Left**: `WorktreeAgentRunArea` — the non-removable `main-agent-run`, tab/tile projection, and a worktree-session-window-scoped orchestration workspace.
- **Right auxiliary area**: `WorktreeWorkspacePanel` — file tree, Markdown/SpecKit preview, change review, and Mermaid. Users can select Git, Files, Markdown, Speckit, or hide the whole area; outer and selected-panel inner widths restore per worktree.

At startup, `WorktreeAgentRunArea` recovers or creates the worktree orchestration session and applies the latest `orchestration-workspace-updated` snapshot. The stable `main-agent-run` binds to the Main Coordinator. The unified composer sends directly to focused, selected, or all panels, or delegates a goal to the Coordinator. Delegated child tasks expose background, attention, and panel presentation states plus retry/cancel controls in `TaskActivityRail`. [Agent execution and orchestration flow](agent-run-flow.md) explains runtime collaboration and result reporting.

### 주요 기능 (features/)

| 기능 | 핵심 컴포넌트 | 설명 |
|------|--------------|------|
| `agent-run` | `AgentRunPanel`, `AgentRunPanelTabs`, `WorktreeAgentRunArea`, `WorkspacePromptComposer`, `TaskActivityRail`, `AgentRunRuntimeHost`, `CoordinatorHandoffDialog`, `PermissionRequestDialog`, `AgentRunMarkdown`, `PromptCommandAutocomplete`, `AgentRunMinimap` | Agent chat/event UI; multi-run tabs and tiles; Main Coordinator goal delegation and target-specific prompt dispatch; child-task activity, promotion, and handoff; permission dialog; Markdown rendering; command completion; and run minimap. |
| `worktree-workspace` | `WorktreeWorkspacePanel`, `WorkspacePanelSelector`, `MarkdownPreviewToc`, `MarkdownViewerComponents`, `SpeckitFilesPanel`, `MarkdownAnnotationWorkspace`, `useMarkdownAnnotationWorkspace`, `TasksKanbanPanel` | File tree, Markdown preview, TOC, Mermaid, SpecKit exploration, Tasks/Kanban, and SDD controls. The annotation auxiliary area in ordinary Markdown and SpecKit previews can be hidden; annotations are sent as agent prompts. [Shared packages and crates](shared-packages.md) documents the shared renderer's list-order and anchor rules. |
| `font-size-adjustment` | `FontSizeSlider`, `keyboard-shortcut.ts` | Adjusts typography in every AW WebView from `-2` to `2`; spacing, icons, images, and WebView zoom do not change. |
| `agent-command-override` | `AgentCommandOverrideEditor`, `EnvVarEditor` | 에이전트 프로필/환경변수 설정 UI |
| `project-form` | `ProjectFormDialog` | 프로젝트 생성/편집 다이얼로그 |
| `project-worktree` | `ProjectWorktreeCard` | worktree 카드 — 열기 모드 (현재 창 / 새 창 / 탭) |
| `saved-prompt` | `SavedPromptToolbar` | 저장 프롬프트 관리 |
| `worktree-change-review` | `WorktreeChangesPanel` | 변경사항 diff 리뷰 |

### 엔티티 (entities/)

| 엔티티 | API 모듈 | 모델 |
|--------|---------|------|
| `agent-run` | `agent-run-repository.ts`, `goal-repository.ts`, `worktree-changes-repository.ts`, `query-keys.ts`, `query-options.ts` | `types.ts`, `format.ts`, `permission-display.ts`, `prompt-autocomplete.ts` |
| `agent-orchestration` | `orchestration-repository.ts`, `query-keys.ts` | `types.ts`, `task-state.ts`, `relationships.ts`, `task-communication.ts`, `prompt-dispatch.ts`, `failure-guidance.ts` — worktree 범위 세션, Coordinator 세대, 최대 8개 노드, 작업/보고/명령/대상별 dispatch 계약 |
| `appearance-preferences` | `appearance-preferences-repository.ts` | `types.ts`, `font-size-step.ts` — 앱 전체 글꼴 크기 단계 |
| `project` | `project-repository.ts`, `git-worktree-repository.ts`, `git-branch-repository.ts`, `git-remote-repository.ts` | `types.ts`, `git-worktree.ts`, `dashboard.ts` |
| `saved-prompt` | 저장 프롬프트 API | — |
| `settings-window` | `settings-window-repository.ts` | 설정 창 열기 |
| `worktree-file` | `worktree-file-repository.ts` | 파일 엔티티 |
| `worktree-git` | `worktree-git-repository.ts` | Git 히스토리/그래프 |

모든 Tauri 명령 호출은 `entities/*/api/*-repository.ts` 파일의 래퍼 함수를 통해 이루어집니다. TanStack Query로 캐싱과 무효화를 관리합니다.

### 모델: `agent-run/model/run-panel-state.ts`

에이전트 실행 이벤트를 누적하여 UI 상태를 계산하는 핵심 로직. RunEvent 스트림 → 메시지 그룹, 라이프사이클 상태, 툴 호출 시퀀스를 구성합니다.

### Appearance preferences

At startup, `AppearancePreferencesProvider` reads `fontSizeStep` from the Rust repository and subscribes to `app://appearance-preferences-changed`. It applies the value to `<html>` as `data-font-size-step`; the Settings slider and keyboard shortcut call `set_font_size_step` or `adjust_font_size_step`. Steps range from `-2` through `2`, while out-of-range stored values normalize to the default. The JSON-backed preference is broadcast to every window.

## 백엔드 구조

소스 경로: `apps/agentic-workbench/src-tauri/src/`

헥사고날 아키텍처의 5계층으로 구성됩니다. 자세한 아키텍처 원칙은 [아키텍처](architecture.md)를 참조.

> **ACP 코어 추출**: agent 실행 관련 domain · ports · application · `infrastructure/acp/*` 계층은 공유 crate `crates/acp-agent-core`로 추출되었습니다. AW의 각 `mod.rs`는 `acp_agent_core` 모듈을 re-export하므로, 기존 경로(`domain::run`, `application::start_agent_run`, `infrastructure::acp::runner` 등)는 그대로 동작합니다. AW 고유의 계층(프로젝트, 목표, worktree, 설정, MCP 등)은 로컬에 유지됩니다.

### 도메인 계층 (domain/)

순수 도메인 모델. 외부 의존성 없음.

| 파일 | 엔티티/용도 |
|------|------------|
| `run.rs` | `AgentRunRequest`, `AgentRun`, `PermissionMode`, `ContextSizePreset`, `ResumePolicy`, `RalphLoopRequest`, `AgentMcpServerConfig` |
| `events.rs` | `RunEvent` 열거형 — 에이전트 실행 전체 이벤트 어휘 (Lifecycle, AgentMessage, Thought, Plan, Tool, Usage, Permission, FileSystem, Terminal, Diagnostic, RalphLoop, Raw, Error) |
| `agent.rs` | `AgentDescriptor` (id, label, command, models, context_sizes) |
| `agent_run_settings.rs` | `AgentRunSettings`, `AgentCommandOverrides`, `AgentProfile` (내장: codex, claude-code, opencode, pi-coding-agent) |
| `goal.rs` | `ThreadGoal` — 상태 (Active/Paused/Blocked/UsageLimited/BudgetLimited/Complete), 토큰 예산 추적 |
| `project.rs` | `Project` (id, name, working_directory, description) |
| `git_worktree.rs` | `GitWorktree`, `GitWorktreeStatus`, `GitWorktreeCreateDraft` |
| `acp_session.rs` | `AcpSessionRecord` — 이력 세션 메타데이터 (resume용) |
| `mcp_title_control.rs` | `TitleChangeRequest`, `ValidatedWindowTitle` |
| `agent_orchestration.rs` | 창 범위 `OrchestrationSession`, 고정 Main/Child 노드, Coordinator 세대, 의존 작업, 보고/명령/알림/프롬프트 전송. 자식은 읽기 전용이며 `MAX_ORCHESTRATION_NODES`는 8 |
| `appearance_preferences.rs` | `AppearancePreferences`, 범위가 검증된 `FontSizeStep` (`-2..=2`) |
| `window_menu.rs` | `AwWindow`, `WindowKind`, `WindowMenuEntry` |

### 애플리케이션 계층 (application/)

유스케이스 오케스트레이터. 포트와 도메인에만 의존.

**에이전트 실행 라이프사이클**:

| 파일 | 유스케이스 |
|------|-----------|
| `start_agent_run.rs` | `StartAgentRunUseCase` — 레지스트리에 run ID 예약 → tokio 태스크에서 launcher.launch() → 세션 부착 → commander.run_to_completion() → finish_run |
| `send_prompt.rs` | `SendPromptUseCase` — 활성 세션 조회 후 프롬프트 전달 |
| `steer_prompt.rs` | `SteerPromptUseCase` — 실행 중인 세션에 steer 프롬프트 주입 (`SteerPending` → `SteerAccepted`/`SteerRejected`) |
| `cancel_prompt_and_send.rs` | `CancelPromptAndSendUseCase` — 현재 프롬프트 취소 후 새 프롬프트 전송 |
| `cancel_agent_run.rs` | `CancelAgentRunUseCase` — 항상 Cancelled 라이프사이클 이벤트 emit |
| `set_permission_mode.rs` | `SetPermissionModeUseCase` — 세션 재시작 없이 권한 모드 변경 |

**오케스트레이션 서비스**: `orchestration_service.rs`는 창 범위 세션을 부트스트랩·복구하고 Main 실행을 세대에 결합하며, 자식 작업 생성/할당/보고/재시도/취소/재배정/인계를 변경 리비전과 함께 처리합니다. `orchestration_command_service.rs`는 자식 명령 전달과 결과 수집을 담당하고, `orchestration_scheduler.rs`는 중복 작업 실행을 방지합니다. `json_orchestration_repository.rs`가 세션을 JSON으로 저장하므로 잃어버린 창의 작업은 복구 가능한 상태로 조정됩니다.

**지원 서비스**: `agent_run_settings_service.rs` (설정 CRUD, 명령어 해결 우선순위: 프로필 → 글로벌 → 기본값), `appearance_preferences_service.rs` (글꼴 크기 설정 검증·저장), `goal_service.rs`, `project_service.rs`, `saved_prompt_service.rs`, `git_worktree_service.rs`, `worktree_file_service.rs`, `worktree_git_service.rs`, `mcp_title_control_service.rs`, `window_menu_service.rs`, `agent_tool_candidate_service.rs`

### 인프라 계층 (infrastructure/)

ACP 엔진과 MCP 서버가 이 계층의 핵심입니다. 상세한 실행 흐름은 [에이전트 실행 흐름](agent-run-flow.md)을 참조.

**ACP 엔진** (`infrastructure/acp/`):
- `runner.rs` — `AcpAgentRunner` (SessionLauncher 구현). 에이전트 서브프로세스 실행, RPC 피어 생성, 세션 초기화/resume, 권한 모드 적용, Ralph Loop 반복 실행
- `client.rs` — `AcpClient` — ACP JSON-RPC 메서드 래퍼
- `transport.rs` — `RpcPeer` — stdin/stdout JSON-RPC 통신
- `permission_flow.rs` — 권한 요청 처리 (자동 허용 또는 사용자 대기)
- `session_update_mapper.rs` — `session/update` → `RunEvent` 변환

**MCP 서버** (`infrastructure/mcp/`):
- `mod.rs` — `McpServerState` — localhost Axum HTTP 서버. `tools/list`, `tools/call` 처리. 실행 시 env(`AW_MCP_URL`, `AW_MCP_TOKEN`, `AW_MCP_RUN_ID`)과 에이전트 지시문을 주입
- `title_tool.rs` — `set_window_title` MCP 툴
- `orchestration_tool.rs` — Coordinator와 Child의 작업 생성·시작·보고·입력 요청·결과 수집 MCP 툴. `capability_registry.rs`가 run-scoped capability를 Coordinator/Child principal에 묶고, 세대 또는 실행 종료 시 이를 폐기합니다.

**세션 관리**:
- `agent_session_registry.rs` — `AppState` — `SessionRegistry` 구현. run_id → RunSlot 매핑, 동시 실행 제한 (env `ACP_WORKBENCH_MAX_RUNS`), 창 소유권 추적
- `permission_broker.rs` — `PermissionBroker` — 권한 대기/응답 관리
- `tauri_run_event_sink.rs` — `TauriRunEventSink` — 소유 창에 `agent-run-event` emit
- `window_manager.rs` — 세션 창 생성 (`session-{uuid}` 라벨), 설정 창 (싱글톤)
- `agent_catalog.rs` — `ConfigurableAgentCatalog` — env `ACP_AGENT_CATALOG_PATH` 또는 내장 4 에이전트

**영속성** (JSON 파일):
- `json_project_repository.rs`, `json_goal_repository.rs`, `json_saved_prompt_repository.rs`, `json_agent_run_settings_repository.rs`, `json_acp_session_store.rs`
- `json_orchestration_repository.rs` — worktree별 durable task/node/report/command snapshot. live 런타임 event journal은 메모리 전용이므로 crash 뒤에는 재시도 가능한 `runtimeLost` 상태로 구분됩니다.
- `json_worktree_workspace_layout_repository.rs` — worktree별 보조 workspace 선택/폭. 부분 갱신은 다른 패널 폭을 지우지 않습니다.
- `json_appearance_preferences_repository.rs` — app-wide `fontSizeStep`; 손상 파일은 backup 복구 후에도 실패하면 보존하고 기본값을 만듭니다.

**Git CLI 어댑터**:
- `git_cli_worktree_provider.rs`, `git_cli_worktree_change_provider.rs`, `git_cli_worktree_git_provider.rs` (커밋 히스토리/그래프는 `crates/git-core`에 위임)
- `fs_worktree_file_provider.rs`, `fs_worktree_watcher.rs` (500ms 디바운스, `notify` crate)

### 인바운드 계층 (inbound/)

`inbound/tauri_commands.rs` (~40개 Tauri 명령). 모든 명령은 얇은 래퍼로 application 서비스에 위임합니다.

명령 카테고리:
- **프로젝트/목표/프롬프트 CRUD**: `list_projects`, `create_project`, `update_project`, `delete_project`, `get_goal`, `create_goal`, `update_goal`, `record_goal_progress` 등
- **설정/외관**: `get_agent_run_settings`, `save_agent_run_settings`, `get_appearance_preferences`, `set_font_size_step`, `adjust_font_size_step`
- **오케스트레이션**: `bootstrap_orchestration_workspace`, `list_recoverable_orchestration_workspaces`, `bind_main_coordinator_run`, `delegate_orchestration_goal` 및 작업·프롬프트·인계·복구 명령. 모든 요청은 현재 창과 worktree 범위의 세션을 대상으로 함
- **Git/Worktree**: `list_git_worktrees`, `create_git_worktree`, `delete_git_worktree`, `list_worktree_files`, `read_worktree_text_file`, `list_worktree_git_history`, `get_worktree_git_graph` 등
- **에이전트 실행**: `list_agents`, `start_agent_run`, `send_prompt_to_run`, `steer_prompt_to_run`, `cancel_current_prompt_and_send_to_run`, `cancel_agent_run`, `set_run_permission_mode`, `respond_agent_permission`, `list_agent_tool_command_candidates`
- **창 관리**: `open_worktree_window`, `open_settings_window`, `open_external_url`, `list_provider_sessions`

## 내장 에이전트 카탈로그

`ConfigurableAgentCatalog` (공유 crate `acp-agent-core`에 위치)가 4개 에이전트를 기본 제공합니다. codex와 claude-code는 ACP 버전을 고정(`runtimeVersion` 포함)하여 재현성을 보장합니다:

| 에이전트 ID | 명령어 | 고정 버전 |
|------------|--------|-----------|
| codex | `npx -y @agentclientprotocol/codex-acp@1.1.5` | 1.1.5 |
| claude-code | `npx -y @agentclientprotocol/claude-agent-acp@0.60.0` | 0.60.0 |
| pi-coding-agent | `npx -y pi-acp` | — |
| opencode | `npx -y opencode-ai acp` | — |

환경변수 `ACP_AGENT_CATALOG_PATH`로 외부 카탈로그 파일을 지정할 수 있습니다. 모델 정보는 models.dev API에서 가져오거나 캐시에서 로드합니다.

## 환경변수

| 변수 | 용도 | 기본값 |
|------|------|--------|
| `ACP_WORKBENCH_MAX_RUNS` | 최대 동시 에이전트 실행 수 | — |
| `ACP_AGENT_CATALOG_PATH` | 외부 에이전트 카탈로그 파일 경로 | 내장 카탈로그 사용 |
| `AW_MCP_URL` | MCP 서버 URL (에이전트에게 주입됨) | 런타임 할당 |
| `AW_MCP_TOKEN` | MCP 서버 인증 토큰 | 런타임 할당 |
| `AW_MCP_RUN_ID` | 현재 run ID (MCP 컨텍스트) | 런타임 할당 |

## 변경 시 주의사항

- **도메인 계층 순수성**: `domain/`에 Tauri, 파일시스템, JSON 저장 의존성을 추가하지 마세요.
- **Tauri 명령은 얇게**: `inbound/tauri_commands.rs`에서 비즈니스 로직을 직접 구현하지 말고 application 서비스에 위임하세요.
- **다중 창 이벤트 격리**: run 이벤트는 소유 창 label로 emit됩니다. 새 창 타입을 추가할 때 이벤트 라우팅을 확인하세요.
- **세션 창 정리**: 세션 창이 닫히면 해당 창이 소유한 live run이 취소됩니다 (`lib.rs`의 `WindowEvent::Destroyed` 핸들러). 오케스트레이션 자식은 완료로 오인하지 말고 durable snapshot에서 `runtimeLost`/재시도 가능 상태로 복구되는지 확인하세요.
- **오케스트레이션 경계**: Main/Child MCP tool은 run-scoped capability principal로 호출자를 결정합니다. 요청 payload의 run ID를 권한 근거로 사용하거나 Child가 형제를 직접 관리하게 만들지 마세요.
- **레이아웃과 외관 검증**: 패널 복원은 hydration 이후에만 적용하고 창 resize만으로 저장하지 마세요. 외관 변경은 모든 창의 provider 동기화, 저장 실패 시 기존 canonical 값 유지, WebView zoom 비변경을 테스트합니다.
