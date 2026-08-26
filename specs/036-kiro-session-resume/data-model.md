# Phase 1 Data Model: Kiro CLI Session Resume

**Feature**: `036-kiro-session-resume` | **Date**: 2026-08-25

이 기능은 새로운 도메인 엔티티를 만들지 않는다. 기존 `ProviderSession`에 Kiro 세션을 **매핑**하는 것이 전부다. 아래는 그 매핑 규칙과 판별 규칙을 정의한다.

---

## 1. 기존 엔티티 (변경 없음)

### `ProviderSession` — `domain/provider_session.rs`

provider가 디스크에 남긴 세션 한 건의 요약. 프론트로 camelCase 직렬화된다.

| 필드 | 타입 | Kiro에서의 값 |
|---|---|---|
| `agent_id` | `String` | `"kiro-cli"` |
| `id` | `String` | 메타의 `session_id` |
| `cwd` | `Option<String>` | 메타의 `cwd` |
| `title` | `Option<String>` | 메타의 `title`, 없으면 첫 `Prompt`에서 발췌 |
| `file` | `String` | `<uuid>.jsonl` 경로 |
| `message_count` | `usize` | `Prompt` + `AssistantMessage` 이벤트 수 (상한 있음, §4) |
| `created_at` | `Option<String>` | 메타의 `created_at` (RFC3339) |
| `updated_at` | `Option<String>` | 메타의 `updated_at` (RFC3339) |
| `model` | `Option<String>` | `session_state.rts_model_state.model_info.model_id` |
| `branch` | `Option<String>` | `None` (Kiro 메타에 git 정보 없음) |
| `source` | `Option<String>` | `None` (§5 참조) |

### `SessionScope` — 변경 없음

`All` 또는 `Path(PathBuf)`. Kiro도 기존 `matches_scope()`를 그대로 통과한다 — `cwd`를 채우기만 하면 된다.

---

## 2. 변경되는 타입

### `ProviderKind` — 배리언트 추가

```
ProviderKind ::= Claude | Codex | Pi | Kiro
                                      ^^^^ 추가
```

### `provider_kind_for(agent_id)` — 매핑 추가

| `agent_id` | `ProviderKind` |
|---|---|
| `"claude-code"` | `Claude` |
| `"codex"` | `Codex` |
| `"pi-coding-agent"` | `Pi` |
| **`"kiro-cli"`** | **`Kiro`** (추가) |
| 그 외 (`"opencode"` 등) | `None` |

### `SessionRoots` — 필드 추가

`kiro: Option<PathBuf>` 추가. 테스트에서 임시 디렉토리를 주입하기 위한 것으로, 기존 3개 provider와 동일한 역할이다.

---

## 3. 외부 데이터 형태 (읽기 전용)

AW가 소유하지 않는 Kiro의 저장 형식이다. 파싱 대상이며 절대 쓰지 않는다.

### 3.1 메타 파일 `<uuid>.json`

```jsonc
{
  "session_id": "0c45dab2-69e0-48df-9c7d-9a1c12f9fbf7",
  "cwd": "/Users/yoophi/project/mermaid-live",
  "title": "현재 프로젝트 아키텍처 및 제공 기능 조사",  // 없을 수 있음
  "created_at": "2026-08-21T08:15:59.452811Z",
  "updated_at": "2026-08-21T08:20:52.971217Z",
  "session_created_reason": "subagent",   // 사용하지 않음 (research R3)
  "session_state": {
    "agent_name": "kiro_default",          // 사용하지 않음
    "rts_model_state": {
      "model_info": { "model_id": "auto" } // → ProviderSession.model
    }
  }
}
```

**파싱 정책**: 모든 필드는 없을 수 있다고 가정한다. `session_id`가 없으면 파일 stem을 id로 쓴다(기존 `file_stem_id()` 패턴). 그 외 필드는 없으면 `None`.

### 3.2 대화 로그 `<uuid>.jsonl`

한 줄에 이벤트 하나:

```jsonc
{"kind": "Prompt",           "version": "v1", "data": {"content": "...", "message_id": "...", "meta": {}}}
{"kind": "AssistantMessage", "version": "v1", "data": {...}}
{"kind": "ToolResults",      "version": "v1", "data": {...}}
{"kind": "Compaction",       "version": "v1", "data": {...}}
```

관측된 `kind` 값은 위 4종이다. 모르는 `kind`는 무시한다(전방 호환).

---

## 4. 파생 규칙

### 4.1 `message_count`

```
message_count = |{ e ∈ events : e.kind ∈ {Prompt, AssistantMessage} }|
```

`ToolResults`와 `Compaction`은 세지 않는다. 기존 `parse_codex`가 도구 호출을 제외하고 메시지만 세는 것과 같은 의미론이다.

**상한**: 로그를 최대 200줄까지만 읽는다(`read_json_lines(path, 200)`, Codex와 동일). 실측 최대 세션이 1255줄이므로 그런 세션의 `message_count`는 **실제보다 작게 나온다**.

이는 의도된 트레이드오프다. 5MB 로그를 매 조회마다 전부 읽으면 SC-003(30건 이상에서 2초 이내)을 지킬 수 없다. `message_count`는 목록의 보조 표시값이지 정확성이 요구되는 값이 아니다.

### 4.2 빈 세션 판별

```
is_empty(session) ⟺ 로그에 유효 이벤트가 하나도 없음
```

빈 세션은 목록에서 제외한다(FR-007). 판별은 **로그 기준**이며 제목 유무로 하지 않는다 — 대화 중이라 제목이 아직 없는 세션을 잘못 제외하지 않기 위해서다(research R4).

### 4.3 제목 대체

```
title = meta.title
      | 첫 Prompt 이벤트의 data.content 발췌
      | None
```

빈 세션은 §4.2로 이미 걸러지므로, 세 번째 경우(둘 다 없음)는 실질적으로 도달하지 않는다.

---

## 5. 의도적으로 채우지 않는 필드

| 필드 | 이유 |
|---|---|
| `branch` | Kiro 메타에 git 정보가 없다. Codex는 `payload.git.branch`가 있지만 Kiro에는 대응물이 없다. |
| `source` | 후보였던 `session_created_reason`은 전 세션이 동일 값이라 정보량이 없고(research R3), `agent_name`도 2종뿐이다. 잘못된 의미를 부여하느니 비워둔다. |

---

## 6. 불변식

1. **읽기 전용**: Kiro 세션 파일을 생성·수정·삭제하지 않는다.
2. **경로 제한**: 세션 루트 밖의 경로를 읽지 않는다. 심볼릭 링크를 따라가지 않는다(기존 `WalkDir::follow_links(false)`가 보장).
3. **부분 실패 격리**: 세션 하나의 파싱 실패가 목록 전체를 실패시키지 않는다(FR-006).
4. **범위 격리**: `SessionScope::Path`가 주어지면 그 디렉토리에서 진행된 세션만 반환한다 — 다른 프로젝트의 대화가 노출되지 않는다.
