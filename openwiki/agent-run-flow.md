---
type: Runtime Workflow
title: Agent execution and orchestration flow
description: How Agentic Workbench launches ACP runs, streams run events, mediates permissions and MCP access, and coordinates read-only child tasks through the Main Coordinator.
tags: [agentic-workbench, acp, mcp, orchestration, runtime]
---

# 에이전트 실행 흐름

ACP 에이전트 실행의 시작부터 완료까지의 lifecycle입니다. 이 문서는 `apps/agentic-workbench/src-tauri/src/` 아래의 ACP engine, permission handling, MCP integration, orchestration-specific layer를 다룹니다. 기반이 되는 재사용 ACP runtime은 [공유 패키지와 크레이트](shared-packages.md)에 설명합니다.

## 실행 흐름 개요

```mermaid
sequenceDiagram
    participant UI as 프론트엔드 (AgentRunPanel)
    participant Cmd as tauri_commands.rs
    participant UC as StartAgentRunUseCase
    participant Reg as SessionRegistry (AppState)
    participant Runner as AcpAgentRunner
    participant Agent as 에이전트 서브프로세스
    participant Sink as TauriRunEventSink

    UI->>Cmd: start_agent_run(request)
    Cmd->>UC: execute(request, mcp_env)
    UC->>Reg: reserve_run(run_id)
    UC->>Runner: launch()
    Runner->>Agent: 서브프로세스 시작
    Runner->>Agent: ACP initialize
    Runner->>Agent: session/new 또는 session/load
    Runner->>Agent: 권한 모드 + 모델 설정
    UC->>Runner: run_to_completion()
    Runner->>Agent: session/prompt (초기 목표)
    loop session/update 이벤트
        Agent-->>Runner: 메시지/사고/툴/사용량
        Runner-->>Sink: RunEvent
        Sink-->>UI: agent-run-event emit
    end
    opt 권한 요청
        Agent-->>Runner: session/request_permission
        Runner-->>Sink: RunEvent::Permission
        Sink-->>UI: 권한 다이얼로그 표시
        UI-->>Cmd: respond_agent_permission
        Cmd-->>Runner: PermissionDecision
        Runner->>Agent: 권한 응답
    end
    Agent-->>Runner: 프로세스 종료
    Runner-->>Sink: RunEvent::Completed
    UC->>Reg: finish_run(run_id)
```

## 시작 (Start)

### 프론트엔드 → Tauri 명령

`start_agent_run` Tauri 명령 (`inbound/tauri_commands.rs`)이 진입점입니다:
1. 요청 정규화 (worktree 경로, 권한 모드, 모델 등)
2. MCP 실행 환경 변수 주입 (`AW_MCP_URL`, `AW_MCP_TOKEN`, `AW_MCP_RUN_ID`)
3. 명령어 오버라이드 해결 (프로필 env 병합)
4. `AcpAgentRunner` 생성
5. `StartAgentRunUseCase` 실행

### StartAgentRunUseCase

`application/start_agent_run.rs`:
1. `SessionRegistry::reserve_run(run_id)` — 중복 실행 방지 + 동시 실행 제한 (`ACP_WORKBENCH_MAX_RUNS`)
2. tokio 태스크 스폰:
   - `launcher.launch()` → `LaunchedSession { session, commander }`
   - `registry.attach_session(run_id, session)`
   - `commander.run_to_completion()` — 메인 실행 루프
   - `registry.finish_run(run_id)`

## ACP 엔진 상세

### AcpAgentRunner (`infrastructure/acp/runner.rs`)

`SessionLauncher` 트레이트를 구현하는 핵심 컴포넌트. `launch()` 단계:

1. **명령어 해결**: 카탈로그에서 에이전트 ID → 명령어 조회, 설정 오버라이드 적용
2. **환경 구성**: PATH 보강 (login shell PATH 병합), 프로필 환경변수 병합 (globalEnv ⊕ profile.env), MCP env 주입
3. **서브프로세스 시작**: `tokio::process::Command`로 에이전트 실행
4. **RPC 피어 생성**: `RpcPeer`로 stdin/stdout JSON-RPC 채널 구성
5. **ACP 초기화**: `initialize` 요청 전송
6. **세션 생성/복원**: `session/new` (새 세션) 또는 `session/load` (resume)
7. **설정 적용**: 권한 모드, 모델 구성 전송
8. **세션 기록**: `AcpSessionStore`에 세션 메타데이터 저장 (resume용)

### AcpRunCommander — 실행 루프

`commander.run_to_completion()` (`infrastructure/acp/runner.rs`):
1. 초기 목표 프롬프트 전송 (`session/prompt`)
2. Ralph Loop가 활성화된 경우: 목표 달성 시까지 후속 프롬프트 반복 전송 (최대 100회, 설정 가능 지연, 오류 시 중단)
3. 프로세스 종료 대기
4. `RunEvent::Completed` emit

### 이벤트 매핑

`session_update_mapper.rs`가 ACP `session/update` 페이로드를 `RunEvent`로 변환:
- `agent_message_chunk` → 메시지 누적
- `thought_chunk` → 사고 표시
- `plan` → 계획 렌더링
- `tool_call` → 툴 호출 표시
- `usage_update` → 토큰 사용량 업데이트

## Steer (실행 중 프롬프트 개입)

에이전트가 실행(active turn) 중일 때 사용자가 새 프롬프트를 주입하거나, 현재 프롬프트를 취소하고 새 프롬프트를 보낼 수 있습니다. 두 유스케이스 모두 `application/` 계층에 추가되었습니다.

| 유스케이스 | 파일 | Tauri 명령 | 동작 |
|-----------|------|-----------|------|
| `SteerPromptUseCase` | `application/steer_prompt.rs` | `steer_prompt_to_run` | 활성 세션에 steer 프롬프트 전달. `RunEvent::Lifecycle { SteerPending }` emit 후 세션에 위임 |
| `CancelPromptAndSendUseCase` | `application/cancel_prompt_and_send.rs` | `cancel_current_prompt_and_send_to_run` | 현재 프롬프트 취소 + 새 프롬프트 전송을 세션에 위임 |

`SessionHandle` 포트 (`ports/session_handle.rs`)에 `steer_prompt`와 `cancel_current_prompt_and_send` 메서드가 추가되어, `AcpSession`이 ACP `session/prompt`의 interrupt/steer 기능으로 구현합니다. 세션이 활성 상태가 아니면 각각 `SteerError::RunNotActive` / `SendPromptError::RunNotActive`를 반환합니다.

### Steer 라이프사이클 상태

steer 요청은 다음 `LifecycleStatus` 변형을 emit합니다 (`domain/events.rs`):

| 상태 | 의미 |
|------|------|
| `SteerPending` | steer 프롬프트가 제출됨 |
| `SteerAccepted` | 에이전트가 steer를 수락 |
| `SteerRejected` | 에이전트가 steer를 거부 (예: 비활성 턴) |

→ 프론트엔드에서 `run-panel-state.ts`가 이 상태들을 처리하여 UI에 steer 결과를 반영합니다.

## Main Coordinator 오케스트레이션

Agentic Workbench는 기존 단일 ACP run 위에 창·worktree 범위의 `OrchestrationSession`을 얹습니다. `main-agent-run`은 안정적인 Main Coordinator panel identity이고, ACP run은 교체 가능한 Coordinator generation입니다. Child work는 최대 8개 노드이며 첫 버전의 `AccessPolicy`는 `ReadOnly`입니다. 즉 병렬 조사·검토를 지원하지만 동시 worktree 쓰기나 손자 에이전트는 지원하지 않습니다. UI와 persistence 경계는 [Agentic Workbench](agentic-workbench.md)에 설명합니다.

```mermaid
sequenceDiagram
    participant User
    participant UI as WorktreeAgentRunArea
    participant Cmd as TauriCommands
    participant Service as OrchestrationService
    participant Main as MainCoordinator
    participant Child as ChildWorker
    participant MCP as MCPServer

    UI->>Cmd: bootstrap worktree workspace
    Cmd->>Service: create or recover session
    UI->>Cmd: bind main run
    Cmd->>MCP: bind coordinator capability
    User->>UI: delegate goal
    UI->>Cmd: delegate orchestration goal
    Cmd->>Main: send goal to active run
    Main->>MCP: create and assign child task
    MCP->>Child: launch read only task
    Child->>MCP: report progress or result
    MCP->>Service: persist task and report
    Service-->>UI: orchestration workspace update
    MCP->>Main: deliver coordinator notification
    Main-->>User: synthesize results
```

이 다이어그램은 worktree 목표 위임, capability로 인증된 MCP task tool, UI update 경로를 보여 줍니다.

### 작업과 결과 수명

`OrchestrationService`는 session을 window label에 결합하고 revision을 확인해 stale UI mutation을 거부합니다. 작업은 `pending → ready → running`에서 `input_required`, `blocked`, `completed`, `failed`, `cancelled`로 전이할 수 있습니다. 완료는 ACP process 종료만으로 판단하지 않고 Child가 제출한 명시적 structured result report로 판정합니다. report에는 summary, evidence, artifact reference, unresolved item, confidence가 포함될 수 있습니다.

MCP에서 Coordinator는 `aw_create_child_task`, `aw_assign_child_task`, `aw_list_child_tasks`, `aw_send_child_message`, `aw_wait_child_tasks`, `aw_collect_child_results`와 child-control tool을 받습니다. Child는 인증된 자기 task만 조회할 수 있고 `aw_get_own_task`, `aw_report_progress`, `aw_report_result`, `aw_request_parent_input`, `aw_report_blocked`, `aw_send_parent_message`만 받습니다. `CapabilityRegistry`는 run-scoped token을 Coordinator 또는 Child principal에 결합하고 run/generation 종료 때 제거하므로 raw run ID는 orchestration authority가 아닙니다.

Task, execution, presentation state는 분리됩니다. `TaskActivityRail`의 `background`, `attentionRequired`, `panel`, `detached`는 작업을 취소하지 않으며 panel을 닫아도 task는 계속됩니다. Coordinator run이 바뀌면 `handoff_orchestration_coordinator`는 새 generation으로 open work를 옮기기 전에 summary와 confirmation을 요구합니다. 창을 잃은 session은 JSON repository에서 복구 가능하지만 in-memory runtime event journal 전체를 보장하지 않으며, live run이 없는 active Child는 재시도 가능한 `runtimeLost` 상태로 조정됩니다.

## 권한 처리

### 권한 모드 (`domain/run.rs`의 `PermissionMode`)

| 모드 | 동작 |
|------|------|
| `Default` | 모든 권한 요청에 사용자 응답 대기 |
| `Auto` | 자동 승인 |
| `ReadOnly` | 읽기 전용 |
| `Plan` | 계획 모드 |
| `AcceptEdits` | 편집 자동 승인 |
| `DangerouslySkipAllPermissions` | 모든 권한 스킵 (확인 다이얼로그 거쳐야 선택 가능) |

### 권한 요청 흐름 (`infrastructure/acp/permission_flow.rs`)

1. 에이전트가 `session/request_permission` 전송
2. 자동 허용 설정이 있으면 즉시 승인
3. 그렇지 않으면 `PermissionBroker::create_waiter(run_id, permission_id)`로 대기 채널 생성
4. `RunEvent::Permission` emit → 프론트엔드에 `PermissionRequestDialog` 표시
5. 사용자 응답 → `respond_agent_permission` Tauri 명령 → `PermissionBroker::respond_for_run`
6. 응답이 run_id와 permission_id로 검증됨 (잘못된 run/창의 대기자 해제 방지)
7. 에이전트에게 권한 응답 전송

### 권한 브로커 (`infrastructure/permission_broker.rs`)

`PermissionDecisionPort` 구현. `HashMap<permission_id, {run_id, oneshot::Sender}>`로 대기자를 관리합니다. `clear_run(run_id)`으로 세션 종료 시 만료된 대기자를 정리합니다.

## MCP 통합

### 로컬 MCP 서버 (`infrastructure/mcp/mod.rs`)

`McpServerState`가 localhost에서 Axum HTTP 서버를 시작합니다 (랜덤 포트). 에이전트 실행 시 MCP 서버 설정이 run-scoped MCP 서버로 전달됩니다.

**제공 툴**:
- `set_window_title` (`title_tool.rs`) — 에이전트가 세션 창 제목을 변경할 수 있음. 검증: 최대 80자, 제어문자 없음. 오리진 허용 목록 (tauri://localhost, 127.0.0.1, localhost)
- 오케스트레이션 도구 (`orchestration_tool.rs`) — capability로 식별된 Coordinator에만 자식 생성·할당·목록·메시지·대기·결과 수집·interrupt/cancel/retry/reassign tool을, Child에만 자기 task 조회·진행/결과/차단 보고·부모 입력 요청/메시지 tool을 공개합니다. 권한과 generation/task 범위 검증은 [Agentic Workbench](agentic-workbench.md)의 `capability_registry.rs` 설명을 따릅니다.

**에이전트에게 주입되는 컨텍스트**: MCP env와 함께 에이전트 지시문이 프롬프트에 주입되어, 에이전트가 worktree 요약, 목표, 세션 정보를 자연어로 쿼리할 수 있습니다.

### MCP 제목 변경 → 창 제목 동기화

```text
에이전트 → MCP tools/call(set_window_title)
  → McpServerState 처리
  → workspace://mcp-window-title 이벤트 emit
  → App.tsx에서 수신 → Tauri window.set_title()
```

## 세션 관리

### SessionRegistry (`infrastructure/agent_session_registry.rs`의 `AppState`)

`SessionRegistry` 트레이트 구현. 핵심 역할:
- `reserve_run(run_id)` — 중복 및 동시 실행 제한 검사
- `attach_run_handle` / `attach_session` — 런타임 핸들 부착
- `finish_run` / `cancel_run` — 실행 정리
- **창 소유권 추적**: 각 run_id를 세션 창 label과 연결. 창이 닫히면(`WindowEvent::Destroyed`) 소유한 모든 run 취소
- `TitleControlRegistry` + `AgentToolCandidateRegistry`도 함께 구현

### 세션 Resume

`AcpSessionStore`가 세션 메타데이터를 JSON 파일에 저장합니다. `ResumePolicy` (Fresh/ResumeIfAvailable/ResumeRequired)에 따라 기존 세션을 재개할 수 있습니다.

### 멀티 창 격리

각 worktree 세션은 별도 창(`session-{uuid}`)에서 열립니다. run 이벤트는 소유 창 label로 `emit_to(label, "agent-run-event", ...)` 전송되어 창 간 이벤트 섞임을 방지합니다.

## 세션 라이프사이클 상태 메시지

`domain/events.rs`의 `LifecycleStatus`는 세션 전체 라이프사이클을 추적합니다:

```text
Started → Initialized → SessionCreated → SessionIdle
  → PromptSent → PromptCompleted → Completed
  (steer: SteerPending → SteerAccepted | SteerRejected)
  (취소: Cancelled)
```

`sessionCreated` / `sessionIdle` 상태는 `SessionLifecycleStatusMessage` (`entities/agent-run/model/types.ts`)로 변환되어 UI에 정보성 상태 메시지로 표시됩니다. `run-panel-state.ts`가 이 메시지를 중복 제거(dedupeKey 기반)하여 표시합니다.

`SessionInfoUpdateMetadata`는 ACP `session_info_update`에서 스레드 상태(`AgentThreadStatus`), 세션 제목, 갱신 시각을 전달하며, 에이전트 상태 아이콘과 창 제목에 반영됩니다.

## 프롬프트 자동완성

`application/agent_tool_candidate_service.rs` + `entities/agent-run/model/prompt-autocomplete.ts`가 세션/앱/확장 소스에서 삽입 가능한 툴 후보를 해결합니다. 프론트엔드의 `PromptCommandAutocomplete` 컴포넌트가 `/` 명령어 자동완성을 제공합니다.

## 관련 소스

| 영역 | 경로 |
|------|------|
| 실행 유스케이스 | `src-tauri/src/application/start_agent_run.rs` |
| Steer 유스케이스 | `src-tauri/src/application/steer_prompt.rs` |
| Cancel-and-send 유스케이스 | `src-tauri/src/application/cancel_prompt_and_send.rs` |
| ACP 러너 | `src-tauri/src/infrastructure/acp/runner.rs` |
| 권한 흐름 | `src-tauri/src/infrastructure/acp/permission_flow.rs` |
| 권한 브로커 | `src-tauri/src/infrastructure/permission_broker.rs` |
| 세션 레지스트리 | `src-tauri/src/infrastructure/agent_session_registry.rs` |
| MCP 서버 | `src-tauri/src/infrastructure/mcp/mod.rs` |
| 오케스트레이션 도메인/서비스 | `src-tauri/src/domain/agent_orchestration.rs`, `src-tauri/src/application/orchestration_service.rs` |
| 오케스트레이션 MCP/권한 | `src-tauri/src/infrastructure/mcp/orchestration_tool.rs`, `src-tauri/src/infrastructure/mcp/capability_registry.rs` |
| 이벤트 싱크 | `src-tauri/src/infrastructure/tauri_run_event_sink.rs`, `src-tauri/src/infrastructure/tauri_orchestration_event_sink.rs` |
| 도메인 이벤트 | `src-tauri/src/domain/events.rs` |
| 프론트엔드 패널 | `src/features/agent-run/ui/agent-run-panel.tsx` |
| 프론트엔드 상태 | `src/features/agent-run/model/run-panel-state.ts` |
