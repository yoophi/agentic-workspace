# Contract: Hushline Agent Run 인터페이스

이 앱이 노출하는 인터페이스 계약. (1) hushline `src-tauri`가 등록하는 Tauri command,
(2) 프론트가 소비하는 `@yoophi/agent-client` TS 계약, (3) 스트리밍 이벤트 계약.
MCP 서빙 인터페이스는 본 기능 범위 밖(후속 스펙).

## 1. Tauri Commands (inbound, hushline src-tauri)

공유 코어 유스케이스를 감싸는 얇은 command. 입력 검증·위임만 담당.

| Command | 입력 | 출력 | 위임 대상(코어) |
|---|---|---|---|
| `start_agent_run` | `request: AgentRunRequest` | `AgentRun` | `StartAgentRunUseCase` |
| `send_prompt_to_run` | `runId: string, prompt: string` | `void` | `SendPromptUseCase` |
| `cancel_agent_run` | `runId: string` | `void` | `CancelAgentRunUseCase` |
| `respond_agent_permission` | `runId, permissionId, optionId` | `void` | permission broker |
| `set_run_permission_mode` | `runId, permissionMode` | `void` | `SetPermissionModeUseCase` |

- 모든 command는 `AppState`(코어 registry)를 Tauri state로 주입받는다.
- `start_agent_run`은 `HushlineAgentSink`(RunEventSink 구현)를 생성해 유스케이스에 전달.
- P2(정리)는 `start_agent_run`만으로 성립. P3(대화)는 `send_prompt_to_run` 반복.

### 검증 규칙
- `AgentRunRequest.goal` 비어있으면 거부.
- `AgentRunRequest.cwd`는 앱 관리 출력 폴더 하위여야 함(경로 검증).
- `runId` 소유자는 호출 세션/창과 일치해야 함(소유 범위 검증).
- 동시 run 수가 상한 초과면 거부(사용자 안내).

## 2. TS Client (packages/agent-client, 프론트 소비)

```text
startAgentRun(request: AgentRunRequest): Promise<AgentRun>
sendPromptToRun(runId: string, prompt: string): Promise<void>
cancelAgentRun(runId: string): Promise<void>
respondAgentPermission(runId, permissionId, optionId): Promise<void>
setRunPermissionMode(runId, permissionMode: PermissionMode): Promise<void>
listenRunEvents(callback: (env: RunEventEnvelope) => void): () => void   // unsubscribe 반환
```

- 타입(`AgentRunRequest`, `RunEventEnvelope`, `PermissionMode`, `AgentRun`)은 Rust domain과
  1:1(camelCase). 계약 픽스처 스냅샷 테스트로 드리프트 방지.

## 3. 스트리밍 이벤트 계약

- 채널: `agent-run-event` (Tauri emit). payload = `RunEventEnvelope { runId, event }`.
- hushline은 단일 창이므로 창-격리 emit 없이 전역 emit. 소비 측에서 `runId`로 필터.
- 정리(P2): `AgentMessage.text`를 누적 → OrganizedDocument.content로 저장.
- 대화(P3): 각 프롬프트 응답의 `AgentMessage`를 ChatSession.messages에 append.
- `Error`·`Lifecycle(Cancelled)` 수신 시 UI에 상태 표시, 자원 정리.

## 4. 계약 불변식 (테스트로 고정)

- CT-1: `RunEvent` 직렬화가 camelCase이고 TS `RunEventEnvelope`와 라운드트립 일치.
- CT-2: `start_agent_run`은 잘못된 `cwd`(관리 폴더 밖)를 거부한다.
- CT-3: 소유하지 않은 `runId`에 대한 `send_prompt_to_run`/`cancel_agent_run`은 거부.
- CT-4: 창/세션 종료 시 소유 run이 모두 취소되고 registry에 잔여 run이 없다.
- CT-5: 동시 run이 상한을 초과하지 않는다.
