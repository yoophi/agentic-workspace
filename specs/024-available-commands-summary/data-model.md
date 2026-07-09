# Data Model: 사용 가능한 명령 요약과 조회

## AvailableCommandMetadata

현재 visible session에서 사용할 수 있는 command 목록을 나타내는 session-level metadata다.

| Field | Type | Required | Validation | Notes |
|---|---|---:|---|---|
| `sessionUpdate` | `"available_commands_update"` | Yes | exact match | metadata 식별자 |
| `commands` | `CommandDetailItem[]` | Yes | 유효 command만 포함 | empty 가능 |
| `updatedAt` | `number \| null` | No | UI 수신 시각 또는 event metadata | user-facing freshness가 필요할 때만 |

Rules:

- timeline message로 변환하지 않는다.
- raw payload 전체는 UI에 직접 표시하지 않는다.
- 새 update가 도착하면 현재 visible run/session의 command metadata를 최신 목록으로 교체한다.

## CommandDetailItem

사용자에게 상세 조회로 보여줄 command 항목이다.

| Field | Type | Required | Validation | Notes |
|---|---|---:|---|---|
| `id` | `string` | Yes | 같은 목록 안에서 stable/unique | name + index 또는 provider id |
| `name` | `string` | Yes | trim 후 non-empty | `$skill`, slash, 일반 이름 모두 허용 |
| `description` | `string \| null` | No | trim 후 empty면 null | 긴 설명은 UI에서 clamp 가능 |
| `inputHint` | `string \| null` | No | 사람이 읽을 수 있는 텍스트만 | object input의 `hint` 우선 |
| `source` | `"extension" \| "appCommand" \| "sessionTool"` | No | name/payload에서 추론 | autocomplete와 공유 가능 |

Rules:

- `name`이 없거나 문자열이 아니면 항목을 제외한다.
- `description`이 없거나 문자열이 아니면 생략한다.
- `input`이 `null`이면 hint 없음으로 표시한다.
- `input.hint`가 문자열이면 `inputHint`로 보존한다.
- 예상하지 못한 `input` 구조는 raw JSON 대신 짧은 fallback 또는 생략을 사용한다.

## CommandSummary

header/status 영역에 compact하게 표시하는 파생 모델이다.

| Field | Type | Description |
|---|---|---|
| `count` | `number` | 표시 가능한 command 수 |
| `label` | `string` | 예: `3 commands available`, `No commands available` |
| `hasDetails` | `boolean` | detail list를 열 수 있는지 여부 |

Rules:

- count는 유효한 `CommandDetailItem` 수를 기준으로 한다.
- count가 0이어도 UI는 깨지지 않고 empty summary를 표시할 수 있다.

## TimelineContent

timeline에 렌더링되는 사용자-facing event content다.

Rules:

- `available_commands_update` 전체 raw payload는 `TimelineContent`가 아니다.
- 일반 user message, agent message, tool activity, lifecycle event, non-command raw diagnostic content는 기존 behavior를 유지한다.

## Relationships

- `AvailableCommandMetadata` has many `CommandDetailItem`.
- `CommandSummary` derives from `AvailableCommandMetadata.commands`.
- Prompt autocomplete candidates may derive from the same command list, but autocomplete behavior is not the primary contract of this feature.
