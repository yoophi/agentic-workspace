# Interface Contract: Orchestration MCP

## 인증과 Principal

각 agent run은 서로 다른 opaque bearer capability를 받는다.

```text
bearer capability
  → source runId
  → active owner window 검증
  → workspace/node binding 조회
  → actor kind, task, generation을 server-side 도출
```

MCP tool argument의 `runId`, `nodeId`, `generationId`를 actor 인증 근거로 사용하지 않는다.
기존 exchange tool에 `runId`가 남아 있으면 capability source와 exact match해야 한다.

Principal validation:

- `AppState.active_owner_of(runId) == workspace.boundWindowLabel`
- Node의 `currentRunId == runId`
- Coordinator는 active generation의 Main run
- Child는 assigned task의 current attempt run
- terminal/cancelled/old generation capability는 mutation 불가

Transport는 기존과 같이 HTTP 200 tool result를 반환하되 domain/application 실패는
`isError: true`와 typed `structuredContent`로 제공한다.

## Coordinator-only tools

### `aw_create_child_task`

새 direct Child 또는 기존 수동 Child에 task를 생성·배정하고 scheduling한다.

Arguments:

```json
{
  "requestId": "uuid",
  "title": "현재 구조 조사",
  "role": {
    "name": "Researcher",
    "responsibility": "현재 코드와 대안을 조사",
    "expectedOutput": "근거가 연결된 결정 목록"
  },
  "objective": "현재 오케스트레이션 경계와 재사용 지점을 조사한다.",
  "constraints": ["read-only", "same-worktree"],
  "expectedResult": "summary, findings, artifacts, unresolved",
  "dependencyTaskIds": [],
  "preferredNodeId": null
}
```

Result:

```json
{
  "taskId": "task-uuid",
  "nodeId": "extra-agent-run-3",
  "status": "ready",
  "executionStatus": "starting"
}
```

Rules:

- Node 생성, task 생성, assignment와 idempotency record는 한 aggregate mutation이다.
- `preferredNodeId`는 같은 Main의 기존 idle direct Child만 허용한다.
- child count/capacity limit이면 task는 `ready`로 남고 이유를 반환한다.
- caller가 active Main generation이 아니면 `forbiddenActor` 또는
  `staleCoordinatorGeneration`이다.

### `aw_assign_child_task`

미할당 task를 기존 direct Child에 배정한다. 이미 terminal이거나 다른 generation의
task는 explicit handoff 없이 배정할 수 없다.

### `aw_list_child_tasks`

Arguments:

```json
{
  "statuses": ["running", "inputRequired", "completed"],
  "includeReports": false
}
```

현재 generation이 관리할 수 있는 direct child task만 반환한다.

### `aw_send_child_message`

Arguments:

```json
{
  "requestId": "uuid",
  "taskId": "task-uuid",
  "message": "권한 위험을 우선 검토해 주세요.",
  "delivery": "queue"
}
```

active Child worker에만 전달한다. 다른 generation/형제 task를 지정할 수 없다.
handler는 UI와 동일한 application command service를 사용하며, result의 `accepted`는
worker runtime 수락을 의미한다. request/payload가 같은 재시도는 Child에 두 번
전달하지 않는다.

### `aw_wait_child_tasks`

Arguments:

```json
{
  "taskIds": ["task-a", "task-b", "task-c"],
  "afterRevision": 18,
  "timeoutMs": 30000
}
```

Result:

```json
{
  "timedOut": false,
  "workspaceRevision": 21,
  "tasks": [
    {
      "taskId": "task-a",
      "status": "completed",
      "latestReportId": "report-a"
    }
  ]
}
```

Rules:

- `timeoutMs`는 0..30000 범위로 clamp한다.
- task 변화 또는 timeout 중 먼저 발생한 시점에 반환한다.
- timeout은 task 상태를 바꾸지 않는다.
- HTTP connection이 끊겨도 task를 취소하지 않는다.

### `aw_collect_child_results`

Arguments:

```json
{
  "taskIds": ["task-a", "task-b", "task-c"],
  "includePartial": true
}
```

Result는 task별 status, role, result reports, unresolved, artifact refs와 failure를
구분한다. 한 task 실패 때문에 다른 결과를 숨기지 않는다.

### `aw_interrupt_child_task`

현재 prompt turn을 중단하거나 steer 가능한 safe point를 요청한다. task 자체는 취소하지
않으며 결과 status를 반환한다.

### `aw_cancel_child_task`

task와 active worker를 취소한다. terminal result와 race가 발생하면 확정된 먼저 상태를
따르고 늦은 report는 partial result로 저장한다.

### `aw_retry_child_task`

retryable failed/blocked task의 attempt를 증가시키고 scheduling한다.

### `aw_reassign_child_task`

task를 다른 idle direct Child로 옮긴다. 기존 attempt와 report를 보존한다.

## Child-only tools

### `aw_get_own_task`

Arguments 없음. capability에 bind된 current task의 role, objective, constraints,
expected result, dependency summary와 parent Main 정보를 반환한다.

### `aw_report_progress`

Arguments:

```json
{
  "requestId": "uuid",
  "progressPercent": 40,
  "summary": "현재 교환 경계와 세션 소유권을 확인했습니다.",
  "findings": []
}
```

자기 task/current attempt에만 report를 추가한다.

### `aw_report_result`

Arguments:

```json
{
  "requestId": "uuid",
  "summary": "권장 구조는 별도 durable task registry입니다.",
  "findings": [
    {
      "title": "Exchange와 task 상태 분리",
      "detail": "delivered는 작업 완료가 아닙니다.",
      "evidence": ["apps/agentic-workbench/src-tauri/src/application/agent_exchange_service.rs"],
      "severity": "warning"
    }
  ],
  "artifactRefs": [],
  "unresolved": [],
  "confidence": 0.9
}
```

Rules:

- 이 tool의 성공 transaction만 task를 `completed`로 전이한다.
- file artifact는 canonical worktree root와 size/UTF-8 정책을 검증한다.
- report와 active Main 대상 `CoordinatorNotification`을 같은 aggregate transaction에서
  생성한다.
- Main notification 실패는 report/result 저장을 되돌리지 않는다.
- report idempotency fingerprint는 progress, summary, findings, artifactRefs, unresolved,
  confidence를 포함한 전체 normalized payload를 사용한다.
- terminal task의 동일 request 재시도는 기존 result를 반환한다.
- 취소 뒤 새로운 result는 partial report로 저장하고 status를 되돌리지 않는다.

### `aw_request_parent_input`

Arguments:

```json
{
  "requestId": "uuid",
  "summary": "두 정책 중 선택이 필요합니다.",
  "question": "읽기 전용 위반 시 즉시 중단할까요?",
  "options": ["즉시 중단", "보고 후 계속"]
}
```

task를 `inputRequired`, presentation을 `attentionRequired`로 바꾸고 Main/user에게 event를
보낸다. 자동 panel focus는 발생시키지 않는다.

### `aw_report_blocked`

Arguments:

```json
{
  "requestId": "uuid",
  "summary": "필요한 fixture가 없습니다.",
  "retryable": true,
  "partialFindings": []
}
```

### `aw_send_parent_message`

Task status를 바꾸지 않는 구조화되지 않은 짧은 메시지다. 형제 target은 허용하지 않는다.

## 금지 규칙

- Child의 child 생성, assign, cancel, reassign tool 호출
- Child의 sibling task/report 조회 또는 메시지
- old Main generation의 새 generation task 제어
- 다른 workspace/window/worktree ID 사용
- caller-supplied actor identity
- result report 없이 task 완료
- agent가 직접 panel focus 또는 layout 변경

## Tool result 오류 형식

```json
{
  "content": [
    {
      "type": "text",
      "text": "Only the active Main Coordinator can create child tasks."
    }
  ],
  "structuredContent": {
    "code": "forbiddenActor",
    "message": "Only the active Main Coordinator can create child tasks.",
    "retryable": false
  },
  "isError": true
}
```
