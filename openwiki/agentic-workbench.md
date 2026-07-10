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

크기 조절 가능한 2패널 분할 레이아웃:
- **좌측**: `WorktreeAgentRunArea` — 에이전트 실행 패널 + 다중 탭
- **우측**: `WorktreeWorkspacePanel` — 파일 트리, Markdown 미리보기, 변경사항 리뷰, Mermaid 다이어그램

### 주요 기능 (features/)

| 기능 | 핵심 컴포넌트 | 설명 |
|------|--------------|------|
| `agent-run` | `AgentRunPanel`, `AgentRunPanelTabs`, `WorktreeAgentRunArea`, `PermissionRequestDialog`, `AgentRunMarkdown`, `PromptCommandAutocomplete` | 에이전트 채팅/이벤트 UI, 다중 실행 탭, 권한 다이얼로그, Markdown 렌더링, 프롬프트 자동완성 |
| `worktree-workspace` | `WorktreeWorkspacePanel`, `MarkdownPreviewToc`, `MarkdownViewerComponents` | 파일 트리, Markdown 미리보기, TOC, Mermaid |
| `agent-command-override` | `AgentCommandOverrideEditor`, `EnvVarEditor` | 에이전트 프로필/환경변수 설정 UI |
| `project-form` | `ProjectFormDialog` | 프로젝트 생성/편집 다이얼로그 |
| `project-worktree` | `ProjectWorktreeCard` | worktree 카드 — 열기 모드 (현재 창 / 새 창 / 탭) |
| `saved-prompt` | `SavedPromptToolbar` | 저장 프롬프트 관리 |
| `worktree-change-review` | `WorktreeChangesPanel` | 변경사항 diff 리뷰 |

### 엔티티 (entities/)

| 엔티티 | API 모듈 | 모델 |
|--------|---------|------|
| `agent-run` | `agent-run-repository.ts`, `goal-repository.ts`, `worktree-changes-repository.ts`, `query-keys.ts`, `query-options.ts` | `types.ts`, `format.ts`, `permission-display.ts`, `prompt-autocomplete.ts` |
| `project` | `project-repository.ts`, `git-worktree-repository.ts`, `git-branch-repository.ts`, `git-remote-repository.ts` | `types.ts`, `git-worktree.ts`, `dashboard.ts` |
| `saved-prompt` | 저장 프롬프트 API | — |
| `settings-window` | `settings-window-repository.ts` | 설정 창 열기 |
| `worktree-file` | `worktree-file-repository.ts` | 파일 엔티티 |
| `worktree-git` | `worktree-git-repository.ts` | Git 히스토리/그래프 |

모든 Tauri 명령 호출은 `entities/*/api/*-repository.ts` 파일의 래퍼 함수를 통해 이루어집니다. TanStack Query로 캐싱과 무효화를 관리합니다.

### 모델: `agent-run/model/run-panel-state.ts`

에이전트 실행 이벤트를 누적하여 UI 상태를 계산하는 핵심 로직. RunEvent 스트림 → 메시지 그룹, 라이프사이클 상태, 툴 호출 시퀀스를 구성합니다.

## 백엔드 구조

소스 경로: `apps/agentic-workbench/src-tauri/src/`

헥사고날 아키텍처의 5계층으로 구성됩니다. 자세한 아키텍처 원칙은 [아키텍처](architecture.md)를 참조.

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
| `window_menu.rs` | `AwWindow`, `WindowKind`, `WindowMenuEntry` |

### 애플리케이션 계층 (application/)

유스케이스 오케스트레이터. 포트와 도메인에만 의존.

**에이전트 실행 라이프사이클**:

| 파일 | 유스케이스 |
|------|-----------|
| `start_agent_run.rs` | `StartAgentRunUseCase` — 레지스트리에 run ID 예약 → tokio 태스크에서 launcher.launch() → 세션 부착 → commander.run_to_completion() → finish_run |
| `send_prompt.rs` | `SendPromptUseCase` — 활성 세션 조회 후 프롬프트 전달 |
| `cancel_agent_run.rs` | `CancelAgentRunUseCase` — 항상 Cancelled 라이프사이클 이벤트 emit |
| `set_permission_mode.rs` | `SetPermissionModeUseCase` — 세션 재시작 없이 권한 모드 변경 |

**지원 서비스**: `agent_run_settings_service.rs` (설정 CRUD, 명령어 해결 우선순위: 프로필 → 글로벌 → 기본값), `goal_service.rs`, `project_service.rs`, `saved_prompt_service.rs`, `git_worktree_service.rs`, `worktree_file_service.rs`, `worktree_git_service.rs`, `mcp_title_control_service.rs`, `window_menu_service.rs`, `agent_tool_candidate_service.rs`

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

**세션 관리**:
- `agent_session_registry.rs` — `AppState` — `SessionRegistry` 구현. run_id → RunSlot 매핑, 동시 실행 제한 (env `ACP_WORKBENCH_MAX_RUNS`), 창 소유권 추적
- `permission_broker.rs` — `PermissionBroker` — 권한 대기/응답 관리
- `tauri_run_event_sink.rs` — `TauriRunEventSink` — 소유 창에 `agent-run-event` emit
- `window_manager.rs` — 세션 창 생성 (`session-{uuid}` 라벨), 설정 창 (싱글톤)
- `agent_catalog.rs` — `ConfigurableAgentCatalog` — env `ACP_AGENT_CATALOG_PATH` 또는 내장 4 에이전트

**영속성** (JSON 파일):
- `json_project_repository.rs`, `json_goal_repository.rs`, `json_saved_prompt_repository.rs`, `json_agent_run_settings_repository.rs`, `json_acp_session_store.rs`

**Git CLI 어댑터**:
- `git_cli_worktree_provider.rs`, `git_cli_worktree_change_provider.rs`, `git_cli_worktree_git_provider.rs` (커밋 히스토리/그래프는 `crates/git-core`에 위임)
- `fs_worktree_file_provider.rs`, `fs_worktree_watcher.rs` (500ms 디바운스, `notify` crate)

### 인바운드 계층 (inbound/)

`inbound/tauri_commands.rs` (~40개 Tauri 명령). 모든 명령은 얇은 래퍼로 application 서비스에 위임합니다.

명령 카테고리:
- **프로젝트/목표/프롬프트 CRUD**: `list_projects`, `create_project`, `update_project`, `delete_project`, `get_goal`, `create_goal`, `update_goal`, `record_goal_progress` 등
- **설정**: `get_agent_run_settings`, `save_agent_run_settings`
- **Git/Worktree**: `list_git_worktrees`, `create_git_worktree`, `delete_git_worktree`, `list_worktree_files`, `read_worktree_text_file`, `list_worktree_git_history`, `get_worktree_git_graph` 등
- **에이전트 실행**: `list_agents`, `start_agent_run`, `send_prompt_to_run`, `cancel_agent_run`, `set_run_permission_mode`, `respond_agent_permission`, `list_agent_tool_command_candidates`
- **창 관리**: `open_worktree_window`, `open_settings_window`, `open_external_url`, `list_provider_sessions`

## 내장 에이전트 카탈로그

`infrastructure/agent_catalog.rs`의 `StaticAgentCatalog`가 4개 에이전트를 기본 제공합니다:

| 에이전트 ID | 명령어 |
|------------|--------|
| codex | `npx -y @agentclientprotocol/codex-acp` |
| claude-code | `npx -y @agentclientprotocol/claude-agent-acp` |
| pi-coding-agent | `npx -y pi-acp` |
| opencode | `npx -y opencode-ai acp` |

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
- **세션 창 정리**: 세션 창이 닫히면 해당 창이 소유한 모든 진행 중 run이 취소됩니다 (`lib.rs`의 `WindowEvent::Destroyed` 핸들러).
