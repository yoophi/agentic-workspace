# Agent 작업 상태 MCP 인터페이스 설계

이 문서는 Agent가 진행 중인 작업 요약, 체크리스트, 발견 이슈, 사용자 확인
필요 항목을 앱에 구조화된 상태로 전달하기 위한 MCP 인터페이스를 정의한다.

목표는 일반 timeline/log와 별도로 최신 작업 상태를 안정적으로 표시하는 것이다.
Agent가 임의 HTML이나 script를 전달하는 방식은 허용하지 않고, 앱이 렌더링 가능한
structured data만 받는다.

## 문제 정의

현재 ACP timeline은 agent message, tool call, permission request, diagnostic event를
시간순으로 보여준다. 이 방식은 세부 기록을 따라가기에는 적합하지만, 사용자가
현재 작업의 요약 상태를 빠르게 파악하기에는 어렵다.

새 인터페이스는 다음 질문에 바로 답할 수 있어야 한다.

- Agent가 지금 무엇을 하는 중인가?
- 어떤 단계가 끝났고 어떤 단계가 남았는가?
- 발견한 위험, 실패, TODO는 무엇인가?
- 사용자가 확인하거나 결정해야 하는 항목이 있는가?
- 다음 action은 무엇이며 agent가 대기 중인지 실행 중인지 완료됐는가?

## Scope

상태는 `run`에 귀속한다.

`run` scope를 기본으로 잡는 이유는 다음과 같다.

- 같은 worktree에서 여러 run이 열릴 수 있다.
- 같은 session을 재사용하더라도 steer나 재시작으로 run lifecycle이 달라질 수 있다.
- UI의 표시 영역은 worktree 작업 창 안에 있지만, 충돌 방지는 active run id를
  기준으로 하는 것이 가장 명확하다.

MVP에서는 `runId`가 필수이다. 후속 확장에서 같은 모델에 `worktreePath`,
`sessionId`, `projectId`를 보조 scope로 추가할 수 있다.

## MCP Surface

MCP server는 앱 쪽에서 제공한다. Agent는 tool call로 상태를 갱신하고, 앱은 해당
상태를 active run UI에 표시한다.

### Tools

#### `app_status.replace`

현재 run의 상태 전체를 교체한다.

부분 업데이트 병합 규칙을 단순화하기 위해 MVP의 기본 갱신 방식은 full
replacement로 한다. Agent는 자신이 알고 있는 최신 전체 상태를 보낸다.

입력 schema:

```json
{
  "type": "object",
  "required": ["runId", "status"],
  "additionalProperties": false,
  "properties": {
    "runId": { "type": "string", "minLength": 1 },
    "status": { "$ref": "#/$defs/AgentWorkStatus" }
  }
}
```

#### `app_status.patch`

현재 run 상태의 일부 필드만 바꾼다.

MVP 이후에 추가한다. patch는 top-level field 단위 replacement만 허용하고,
JSON Patch처럼 임의 path를 수정하는 기능은 제공하지 않는다.

입력 schema:

```json
{
  "type": "object",
  "required": ["runId", "patch"],
  "additionalProperties": false,
  "properties": {
    "runId": { "type": "string", "minLength": 1 },
    "patch": { "$ref": "#/$defs/AgentWorkStatusPatch" }
  }
}
```

#### `app_status.clear`

현재 run의 status panel을 비운다.

입력 schema:

```json
{
  "type": "object",
  "required": ["runId"],
  "additionalProperties": false,
  "properties": {
    "runId": { "type": "string", "minLength": 1 }
  }
}
```

#### `app_status.complete`

현재 run 상태를 완료 상태로 전환한다. `summary`, `issues`, `nextAction`을 함께
보낼 수 있다.

입력 schema:

```json
{
  "type": "object",
  "required": ["runId"],
  "additionalProperties": false,
  "properties": {
    "runId": { "type": "string", "minLength": 1 },
    "summary": { "type": "string", "maxLength": 4000 },
    "issues": {
      "type": "array",
      "maxItems": 50,
      "items": { "$ref": "#/$defs/AgentWorkIssue" }
    },
    "nextAction": { "type": "string", "maxLength": 1000 }
  }
}
```

### Resources

#### `app_status://runs/{runId}`

해당 run의 최신 상태를 읽는다.

MVP에서는 UI가 Tauri event/store를 통해 상태를 받으므로 resource read는
필수 구현이 아니다. 하지만 agent가 자기 상태를 재조회하거나, 여러 도구가 같은
상태를 공유해야 할 때를 위해 resource 이름을 예약한다.

#### `app_status://worktrees/{encodedWorktreePath}/latest`

해당 worktree에서 마지막으로 갱신된 run 상태를 읽는다. 후속 확장 범위이다.

## Data Model

### `AgentWorkStatus`

```json
{
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "summary": { "type": "string", "maxLength": 4000 },
    "phase": { "type": "string", "maxLength": 120 },
    "state": {
      "type": "string",
      "enum": ["running", "waitingForUser", "blocked", "complete"]
    },
    "items": {
      "type": "array",
      "maxItems": 100,
      "items": { "$ref": "#/$defs/AgentWorkItem" }
    },
    "issues": {
      "type": "array",
      "maxItems": 50,
      "items": { "$ref": "#/$defs/AgentWorkIssue" }
    },
    "needsUserInput": {
      "type": "array",
      "maxItems": 20,
      "items": { "$ref": "#/$defs/AgentUserInputRequest" }
    },
    "nextAction": { "type": "string", "maxLength": 1000 },
    "updatedAt": { "type": "string", "format": "date-time" },
    "revision": { "type": "integer", "minimum": 1 }
  }
}
```

`revision`은 run 안에서 단조 증가해야 한다. 같은 run에서 낮은 revision이 늦게
도착하면 앱은 무시한다. revision이 없으면 앱이 수신 순서대로 revision을 부여할
수 있지만, agent가 제공하는 것을 권장한다.

### `AgentWorkItem`

```json
{
  "type": "object",
  "required": ["id", "label", "status"],
  "additionalProperties": false,
  "properties": {
    "id": { "type": "string", "minLength": 1, "maxLength": 120 },
    "label": { "type": "string", "minLength": 1, "maxLength": 500 },
    "status": {
      "type": "string",
      "enum": ["pending", "inProgress", "done", "skipped", "blocked"]
    },
    "detail": { "type": "string", "maxLength": 2000 }
  }
}
```

### `AgentWorkIssue`

```json
{
  "type": "object",
  "required": ["id", "severity", "title"],
  "additionalProperties": false,
  "properties": {
    "id": { "type": "string", "minLength": 1, "maxLength": 120 },
    "severity": {
      "type": "string",
      "enum": ["info", "warning", "error"]
    },
    "title": { "type": "string", "minLength": 1, "maxLength": 300 },
    "detail": { "type": "string", "maxLength": 2000 },
    "filePath": { "type": "string", "maxLength": 1000 },
    "line": { "type": "integer", "minimum": 1 }
  }
}
```

### `AgentUserInputRequest`

```json
{
  "type": "object",
  "required": ["id", "prompt"],
  "additionalProperties": false,
  "properties": {
    "id": { "type": "string", "minLength": 1, "maxLength": 120 },
    "prompt": { "type": "string", "minLength": 1, "maxLength": 1000 },
    "options": {
      "type": "array",
      "maxItems": 8,
      "items": {
        "type": "object",
        "required": ["id", "label"],
        "additionalProperties": false,
        "properties": {
          "id": { "type": "string", "minLength": 1, "maxLength": 120 },
          "label": { "type": "string", "minLength": 1, "maxLength": 120 }
        }
      }
    }
  }
}
```

## Update Lifecycle

1. run 시작 시 앱은 해당 `runId`의 status state를 빈 상태로 만든다.
2. Agent가 `app_status.replace`를 호출하면 앱은 schema를 검증한다.
3. `runId`가 active run이 아니면 저장은 할 수 있지만 현재 창에는 표시하지 않는다.
4. 같은 `runId`에서 새 revision이 오면 기존 상태를 교체한다.
5. 낮은 revision이나 동일 revision의 중복 update는 무시한다.
6. `app_status.clear`는 해당 run의 상태를 제거한다.
7. run lifecycle이 `completed` 또는 `cancelled`가 되면 상태는 읽기 전용으로 남긴다.
8. 새 run이 같은 worktree에서 시작되면 이전 run 상태와 섞지 않는다.

## UI Placement

MVP UI는 worktree session screen의 `AgentRunPanel` 안에서 timeline 위쪽 또는
오른쪽 side panel에 둔다.

우선순위:

1. `needsUserInput`이 있으면 가장 눈에 띄는 위치에 표시한다.
2. `state`와 `phase`를 compact header로 표시한다.
3. `summary`를 2-4줄 preview로 표시하고 길면 접는다.
4. `items`는 checklist 형태로 표시한다.
5. `issues`는 severity badge와 함께 별도 목록으로 표시한다.

timeline은 원본 이벤트 기록을 유지하고, status panel은 최신 snapshot만 보여준다.
따라서 status panel은 log를 대체하지 않는다.

## Concurrency Strategy

동시 run 충돌은 다음 규칙으로 피한다.

- store key는 `runId`이다.
- UI는 현재 창의 `activeRunId`와 같은 status만 자동 표시한다.
- 같은 worktree의 최신 상태를 보여주는 secondary view가 필요하면 `updatedAt`과
  `runId`를 함께 표시한다.
- 같은 run에서 update 순서는 `revision`으로 결정한다.
- agent가 `runId` 없이 update하는 것은 거부한다.
- 오래된 run이 늦게 event를 보내도 active run panel을 덮어쓰지 않는다.

## Backend Architecture

Tauri backend는 hexagonal architecture를 따른다.

- `domain`: `AgentWorkStatus`, item, issue, input request model
- `ports`: `AgentWorkStatusStore`
- `application`: status replace, patch, clear, complete use case
- `inbound`: MCP tool handler 또는 Tauri command adapter
- `infrastructure`: in-memory store MVP, 이후 JSON persistence adapter

MVP는 in-memory store로 충분하다. 앱 재시작 후 상태 복원이 필요해지면 JSON
persistence를 추가한다.

## Frontend Architecture

프론트엔드는 Feature-Sliced Design을 따른다.

- `entities/agent-work-status`: 타입, API adapter, query key
- `features/agent-work-status-panel`: status panel UI와 사용자 input 표시
- `features/agent-run`: active run과 status panel 배치
- `shared`: badge, compact checklist 등 cross-domain primitive가 필요할 때만 사용

## MVP Scope

MVP에서 구현할 범위:

- `app_status.replace`
- `app_status.clear`
- `AgentWorkStatus` 기본 모델
- run-scoped in-memory store
- active run UI에 latest status snapshot 표시
- revision 기반 stale update 무시
- schema validation 및 length/item count 제한

## Follow-up Scope

후속 확장 범위:

- `app_status.patch`
- `app_status.complete`
- worktree latest resource
- JSON persistence
- user input request를 앱 action이나 steer prompt와 연결
- issue item에서 파일/diff panel로 이동
- 여러 run 상태를 비교하는 history view

## Security and Rendering Rules

- Markdown은 plain text로 저장하고, 렌더링 시 허용된 markdown subset만 사용한다.
- HTML, script, inline event handler는 허용하지 않는다.
- file path는 workspace 밖 파일을 링크로 열기 전에 normalize와 scope check를 거친다.
- 큰 payload는 schema limit에서 거부한다.
- status update 실패는 agent run 자체를 실패시키지 않고 diagnostic event로 남긴다.
