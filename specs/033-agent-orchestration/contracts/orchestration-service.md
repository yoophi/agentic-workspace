# Interface Contract: Orchestration Service

## 공통 규칙

- Tauri command 이름은 snake_case, frontend wrapper는 camelCase를 사용한다.
- `windowLabel`은 invoking window에서 주입하며 client payload를 신뢰하지 않는다.
- bootstrap 때 canonical worktree와 안정적인 `workspaceId`를 묶고 이후 명령은 그
  immutable binding을 사용한다.
- mutating command는 `requestId`와 필요한 경우 `expectedRevision`을 받는다.
- 동일 request ID와 동일 payload는 저장된 결과를 반환한다.
- 동일 request ID와 다른 payload는 `duplicateConflict`다.
- durable 저장 성공 후 event를 emit한다. event 유실은 snapshot 재조회로 복구한다.
- 오류는 `{ "code": "...", "message": "...", "details": {} }` JSON으로 직렬화한다.

## Frontend → Tauri commands

### `bootstrap_orchestration_workspace`

Request:

```json
{
  "worktreePath": "/canonical/worktree",
  "resumeWorkspaceId": null
}
```

Response: `OrchestrationWorkspaceSnapshot`.

Rules:

- 경로를 canonicalize하고 directory/root 정책을 검증한다.
- 새 창은 기본적으로 새 `workspaceId`를 발급한다.
- `resumeWorkspaceId`는 같은 canonical worktree의 unbound/recoverable workspace를
  사용자가 명시적으로 선택한 경우에만 허용한다.
- 같은 workspace가 다른 live window에 bound되어 있으면 `workspaceAlreadyBound`다.
- Main Node `main-agent-run`을 정확히 하나 생성한다.

### `get_orchestration_workspace`

Request 없음. invoking window에 bound된 최신 snapshot을 반환한다.

Response excerpt:

```json
{
  "workspaceId": "workspace-uuid",
  "worktreePath": "/canonical/worktree",
  "revision": 12,
  "mainNodeId": "main-agent-run",
  "activeCoordinatorGenerationId": "generation-uuid",
  "nodes": [],
  "tasks": [],
  "dispatches": []
}
```

### `bind_main_coordinator_run`

Main panel run 시작/종료를 orchestration generation과 연결한다.

Request:

```json
{
  "requestId": "uuid",
  "panelId": "main-agent-run",
  "runId": "run-main-2",
  "state": "active",
  "expectedRevision": 12
}
```

`state`는 `active | ended`다.

Rules:

- panel ID는 `main-agent-run`이어야 하고 run의 active owner가 invoking window여야 한다.
- 새 active run은 새 `CoordinatorGeneration`을 만든다.
- 이전 generation의 비종료 task는 `awaitingHandoff`가 된다.
- `ended`는 task를 완료 또는 새 generation에 자동 귀속하지 않는다.

### `delegate_orchestration_goal`

공용 Composer의 Coordinator 모드 진입점이다.

Request:

```json
{
  "requestId": "uuid",
  "goal": "현재 구조의 위험을 세 관점에서 조사하고 종합해 주세요.",
  "expectedRevision": 13
}
```

Response:

```json
{
  "rootTaskId": "task-root",
  "generationId": "generation-uuid",
  "dispatchId": "dispatch-uuid",
  "status": "accepted"
}
```

Rules:

- active Coordinator Generation이 없으면 `inactiveCoordinator`다.
- root task와 delegate dispatch를 한 aggregate mutation으로 저장한다.
- 저장 후 Main run에 Coordinator prompt를 보낸다.
- Main은 MCP의 child task 도구로 하위 작업을 생성한다.

### `dispatch_workspace_prompt`

직접 panel 명령과 batch 결과를 기록한다.

Request:

```json
{
  "dispatchId": "uuid",
  "intent": "direct",
  "targetMode": "selected",
  "targetPanelIds": ["extra-agent-run-1", "extra-agent-run-2"],
  "message": "현재 결과를 요약해 주세요.",
  "delivery": "queue",
  "expectedRevision": 14
}
```

Response:

```json
{
  "dispatchId": "uuid",
  "targets": [
    {
      "panelId": "extra-agent-run-1",
      "requestId": "uuid-1",
      "status": "accepted",
      "failureCode": null,
      "failureReason": null
    },
    {
      "panelId": "extra-agent-run-2",
      "requestId": "uuid-2",
      "status": "rejected",
      "failureCode": "staleTargetRun",
      "failureReason": "Target run changed."
    }
  ]
}
```

Rules:

- `focused`, `selected`, `all`은 `intent=direct`다.
- `coordinator`는 이 command가 아니라 `delegate_orchestration_goal`을 사용한다.
- `all`은 Main을 포함한 현재 open/panel Node 전체를 submit 시 snapshot으로 고정한다.
- 공통 payload 오류만 전체 command를 실패시킨다.
- target 오류는 target별 terminal result로 저장하고 다른 성공을 rollback하지 않는다.
- visible panel은 기존 exchange의 `send | queue | draft`를 사용한다.
- background worker의 `send | queue`는 worker port를 사용한다.
- background target의 `draft`는 panel별 workspace draft slot에 저장한다.

### `respond_orchestration_input`

Request:

```json
{
  "requestId": "uuid",
  "taskId": "task-reviewer",
  "inputReportId": "report-input",
  "message": "권한 모델은 읽기 전용으로 가정하세요.",
  "expectedTaskRevision": 4
}
```

Rules:

- task가 `inputRequired`이고 report가 최신 unresolved input request여야 한다.
- current task attempt, assigned Node와 active run을 서버에서 다시 해석한다.
- 전체 응답 payload와 `inputReportId`를 포함한 `TaskCommand`를 idempotent하게 저장한다.
- 동일 worker run이 응답을 수락한 뒤에만 command를 `accepted`, task를 `running`으로
  전이한다.
- worker가 stopped이거나 전송이 실패하면 응답 command를 보존하고 task는
  `inputRequired`로 유지하며 `workerUnavailable`과 retry 선택지를 반환한다.
- 최신 unresolved input request가 바뀌면 stale response를 전달하지 않는다.

### `cancel_orchestration_task`

Request:

```json
{
  "requestId": "uuid",
  "taskId": "task-tester",
  "expectedTaskRevision": 3
}
```

Rules:

- UI와 MCP는 같은 application command use case를 사용한다.
- active worker가 있으면 durable cancel command를 저장하고 worker cancel을 요청한 뒤
  task를 terminal `cancelled`로 저장한다.
- 완료가 먼저 확정된 race에서는 `alreadyCompleted`를 반환한다.
- cancel 뒤 늦은 result는 partial report로 보존하지만 상태를 completed로 되돌리지 않는다.

### `retry_orchestration_task`

Request:

```json
{
  "requestId": "uuid",
  "taskId": "task-tester",
  "expectedTaskRevision": 4
}
```

Rules:

- `failed` 또는 해결 가능한 `blocked` task만 재시도한다.
- attempt를 증가시키고 과거 report/result는 보존한다.
- capacity가 없으면 `ready` queue에 둔다.
- capacity가 있으면 새 planned run/capability를 발급하고 worker를 실제 launch한다.
- 이전 attempt의 command/report는 새 attempt 상태를 변경할 수 없다.

### `reassign_orchestration_task`

Request:

```json
{
  "requestId": "uuid",
  "taskId": "task-reviewer",
  "targetNodeId": "extra-agent-run-2",
  "expectedTaskRevision": 5
}
```

Target은 같은 Main의 직접 자식이고 다른 active primary task가 없어야 한다.

Rules:

- 이전 worker가 active이면 cancel/interrupt하고 capability를 fence한다.
- ownership과 새 attempt를 durable 저장한 뒤 target worker를 scheduling·launch한다.
- crash/retry에도 두 active worker가 생기지 않도록 transfer lease를 사용한다.

### `send_orchestration_child_command`

UI direct send, Activity Rail action과 Coordinator MCP가 공유하는 application command다.

Request:

```json
{
  "requestId": "uuid",
  "taskId": "task-reviewer",
  "kind": "message",
  "message": "현재까지 확인한 사실을 먼저 보고해 주세요.",
  "delivery": "queue",
  "expectedTaskRevision": 5
}
```

Response는 `TaskCommand`다. `accepted`는 ACP worker가 command를 수락했다는 뜻이며
단순히 run ID가 존재한다는 뜻이 아니다.

Rules:

- command 생성 전에 current task/attempt/node/run/generation scope를 검증한다.
- idempotency lookup은 optimistic revision rejection보다 먼저 수행한다.
- 같은 request ID와 같은 전체 payload는 저장된 outcome을 반환하고 재전송하지 않는다.
- 같은 request ID와 다른 payload는 `duplicateConflict`다.
- stale run/attempt, terminal task 또는 다른 window 대상은 typed error다.
- `draft`는 background worker runtime command로 사용할 수 없다.

### Coordinator notification dispatch

Child report transaction은 `CoordinatorNotification`을 함께 생성한다.

- `inputRequest`, `blocked`, `result`는 active Main에 즉시 queue한다.
- `progress`, `message`는 durable Main inbox에 저장하고 설정에 따라 coalesce할 수 있다.
- notification body는 workspace/task/report ID와 report type만 포함한다.
- active Main이 없으면 pending으로 보존한다.
- handoff와 report가 경쟁하면 정확히 한 generation에만 귀속한다.
- Main 전송 실패는 report 저장을 rollback하지 않으며 outbox에서 재시도한다.

### `handoff_coordinator_generation`

Request:

```json
{
  "requestId": "uuid",
  "fromGenerationId": "generation-old",
  "toGenerationId": "generation-new",
  "taskIds": ["task-researcher", "task-reviewer"],
  "decision": "accept",
  "expectedRevision": 20
}
```

`decision`은 `accept | cancelOutstanding | leaveUnassigned`다.

Rules:

- `toGenerationId`는 현재 active Main generation이어야 한다.
- `accept`는 handoff summary와 부분 결과를 새 Main에 전달하고 선택 task의 관리 권한을
  이동한다.
- 사용자 선택 전에는 task owner generation을 바꾸지 않는다.
- stale old Main capability는 새 generation의 task를 제어할 수 없다.

### `promote_orchestration_task`

Request:

```json
{
  "requestId": "uuid",
  "taskId": "task-reviewer",
  "placement": "right",
  "anchorPanelId": "main-agent-run",
  "expectedRevision": 21
}
```

Rules:

- `placement`은 `right | below`다.
- Node와 run binding을 유지하고 presentation만 `promoting → panel`로 변경한다.
- frontend가 기존 tile reducer로 leaf를 연결한 뒤 acknowledge한다.
- 명시적 사용자 open이므로 성공 시 새 panel focus를 허용한다.

### `detach_orchestration_task_panel`

Request:

```json
{
  "requestId": "uuid",
  "taskId": "task-reviewer",
  "expectedRevision": 22
}
```

presentation과 layout leaf만 제거한다. worker cancel command를 호출하지 않는다.

## Backend → Frontend events

### `orchestration-workspace-updated`

최신 aggregate revision과 변경된 snapshot을 owner window에 보낸다.

```json
{
  "workspaceId": "workspace-uuid",
  "revision": 23,
  "reason": "taskReport"
}
```

Frontend는 revision gap이 있으면 `get_orchestration_workspace`로 full snapshot을 다시 읽는다.

### `orchestration-task-updated`

Activity Rail의 빠른 projection을 위한 task/node/report delta다. full snapshot을 대신하는
source of truth가 아니다.

### `orchestration-attention-requested`

```json
{
  "taskId": "task-reviewer",
  "reason": "inputRequired",
  "reportId": "report-input",
  "autoFocus": false
}
```

항상 `autoFocus=false`다. UI는 badge/rail highlight만 표시한다.

### `workspace-prompt-dispatch-updated`

대상별 accepted/delivered/rejected/failed/cancelled 변경을 전달한다.

### `orchestration-runtime-event`

기존 `agent-run-event` envelope에 run별 `sequence`와 orchestration binding metadata를
추가한 projection이다. frontend runtime controller가 background timeline을 유지한다.

### `orchestration-command-updated`

`TaskCommand`의 pending/dispatching/accepted/failed/cancelled 상태를 owner window에
전달한다. Activity Rail과 Composer는 이 event 또는 최신 workspace snapshot으로 실제
runtime delivery 결과를 표시한다.

### `orchestration-coordinator-notification-updated`

Child report에 대한 Main notification의 pending/accepted/failed/superseded 상태를
전달한다. UI attention event와 Main runtime notification은 독립적으로 실패/재시도한다.

### `replay_orchestration_runtime_events`

Input:

```json
{
  "runId": "run-child-1",
  "afterSequence": 0
}
```

Output:

```json
{
  "runId": "run-child-1",
  "events": [
    {
      "runId": "run-child-1",
      "sequence": 1,
      "event": {},
      "terminal": false
    }
  ],
  "lastSequence": 1,
  "terminal": false,
  "gapDetected": false
}
```

규칙:

- owner window에 속한 run만 replay할 수 있다.
- `afterSequence`보다 큰 event를 sequence 오름차순으로 반환한다.
- 요청 cursor가 bounded journal보다 오래되면 `gapDetected=true`다.
- replay 조회 자체는 task, execution 또는 presentation 상태를 변경하지 않는다.
- frontend는 반환 event를 live event와 동일한 reducer에 적용해야 한다.
- panel mount의 빈 state는 replay 또는 workspace snapshot의 run binding보다 우선할 수 없다.

### `orchestration-promotion-requested`

backend가 `promoting`을 저장한 뒤 frontend layout leaf 연결을 요청한다. frontend는
성공 또는 실패를 acknowledge하고 실패 시 presentation을 이전 상태로 되돌린다.

모든 event는 기존 Tauri emit과 DOM fallback convention을 사용하며 owner window
하나에만 전달한다.

## Error codes

- `workspaceNotBootstrapped`
- `workspaceAlreadyBound`
- `scopeMismatch`
- `revisionConflict`
- `unknownCoordinator`
- `inactiveCoordinator`
- `staleCoordinatorGeneration`
- `unknownNode`
- `unknownTask`
- `staleTaskRevision`
- `invalidTransition`
- `forbiddenActor`
- `staleSourceRun`
- `staleTargetRun`
- `readOnlyViolation`
- `concurrencyLimit`
- `duplicateConflict`
- `workerUnavailable`
- `alreadyCompleted`
- `invalidArtifact`
- `windowUnavailable`
- `persistenceFailed`
