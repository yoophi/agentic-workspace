# Data Model: 세션 정보 메타데이터 표시

## SessionInfoMetadata

`session_info_update`에서 도착하는 session-level metadata다. timeline message가 아니다.

| Field | Type | Required | Validation | Notes |
|---|---|---:|---|---|
| `threadStatus` | `AgentThreadStatus \| null` | No | `active`, `idle`, otherwise `unknown` | #145 status indicator 유지 |
| `title` | `string \| null` | No | trim 후 비어 있지 않고 control character가 없으며 최대 길이 이내 | window title primary input |
| `updatedAt` | `string \| null` | No | `Date`로 파싱 가능한 값만 표시 | header freshness input |

## SessionDisplayTitle

현재 visible session을 식별하기 위한 제목 상태다.

| Field | Type | Description |
|---|---|---|
| `defaultWindowTitle` | `string` | project/worktree 기반 fallback title |
| `agentWindowTitle` | `string \| null` | live session title에서 normalize된 값 |
| `effectiveWindowTitle` | `string` | `agentWindowTitle ?? defaultWindowTitle` |

Lifecycle:

- project 또는 worktree route가 바뀌면 `agentWindowTitle`을 `null`로 reset한다.
- 의미 있는 새 `title` update를 받으면 `agentWindowTitle`을 갱신한다.
- 빈 title 또는 invalid title은 기존 title을 덮어쓰지 않는다.

## SessionFreshness

`updatedAt`을 user-facing 보조 metadata로 표시하기 위한 파생 상태다.

| Field | Type | Description |
|---|---|---|
| `rawUpdatedAt` | `string \| null` | event에서 받은 원본 값 |
| `parsedUpdatedAt` | `Date \| null` | 유효한 날짜일 때만 사용 |
| `displayLabel` | `string \| null` | header에 표시할 읽기 쉬운 값 |

Validation:

- missing 또는 malformed 값은 `displayLabel`을 만들지 않는다.
- 더 새로운 유효 값이 도착하면 표시를 갱신한다.
- metadata-only update는 기존 `AgentThreadStatus`를 제거하지 않는다.

## AgentThreadStatus

#145에서 도입된 active/idle 상태 표시다.

| Field | Type | Description |
|---|---|---|
| `type` | `"active" \| "idle" \| "unknown"` | 표시할 thread 상태 |
| `activeFlags` | `string[] \| undefined` | active 상태의 부가 flag |

Rule:

- `sessionInfo` update에 status가 없으면 기존 status를 유지한다.
- status가 있으면 metadata 표시와 독립적으로 갱신한다.

## TimelineContent

timeline에 렌더링되는 event content다.

Rule:

- `session_info_update`는 `TimelineContent`가 아니다.
- typed `sessionInfo` event와 raw fallback session_info_update 모두 timeline item을 만들지 않는다.
- 일반 message, tool, lifecycle, diagnostic raw event는 기존 동작을 유지한다.
