# Phase 0 Research: Kiro CLI Session Resume

**Feature**: `036-kiro-session-resume` | **Date**: 2026-08-25

조사는 로컬에 실재하는 Kiro 세션 저장소(11개 세션)를 직접 읽어서 수행했다. 추정이 아니라 실측이며, 수치는 조사 시점(2026-08-25) 기준이다.

---

## R1. 세션 저장소 레이아웃

**결정**: 세션 루트는 `~/.kiro/sessions/cli/`이며, 세션 한 건은 `<uuid>.json`(메타)과 `<uuid>.jsonl`(대화 로그) 한 쌍으로 저장된다.

**실측**:

```
~/.kiro/sessions/cli/
├── <uuid>.json      # 메타 (11개)
├── <uuid>.jsonl     # 대화 로그 (11개)
├── <uuid>.history   # 슬래시 커맨드 입력 이력 (5개, 목록에 불필요)
└── <uuid>/tasks/    # 태스크 산출물 (*.json, 목록에 불필요)
```

`.json`과 `.jsonl`이 11쌍으로 정확히 짝을 이룬다. `.jsonl` 없는 세션은 없었다.

**영향**: 기존 `scan_agent()`가 `.jsonl` 확장자만 순회하는데, 이 전제가 Kiro에도 성립한다. 즉 **`scan_agent`를 그대로 재사용할 수 있다**.

**Alternatives considered**:
- `.json`을 순회하는 별도 스캔 함수: 세션 디렉토리 하위 `tasks/*.json`이 함께 걸리므로 깊이 제한이 추가로 필요하다. `.jsonl` 순회가 이 문제를 자연히 피한다. 기각.
- `.history` 사용: 슬래시 커맨드 입력만 담겨 있어 세션 식별에 쓸 수 없다. 기각.

**리스크**: `.jsonl` 없이 `.json`만 남는 세션이 생기면 그 세션은 목록에서 누락된다. 현재 데이터에는 없지만 Kiro 구현이 바뀌면 발생할 수 있다.

---

## R2. 메타 파일 스키마

**결정**: 목록 표시에 필요한 정보는 전부 `<uuid>.json`에서 얻는다. 대화 로그를 읽지 않고도 채울 수 있다(FR-010 충족).

**실측 스키마** (최상위 키):

| 키 | 예시 | ProviderSession 매핑 |
|---|---|---|
| `session_id` | `"0c45dab2-69e0-..."` | `id` |
| `cwd` | `"/Users/yoophi/project/mermaid-live"` | `cwd` |
| `title` | `"현재 프로젝트 아키텍처 및 제공 기능 조사"` | `title` |
| `created_at` | `"2026-08-21T08:15:59.452811Z"` | `created_at` |
| `updated_at` | `"2026-08-21T08:20:52.971217Z"` | `updated_at` |
| `session_created_reason` | `"subagent"` | (R3 참조 — 사용하지 않음) |
| `session_state` | 객체 | 하위에서 `model` 추출 |

`session_state` 하위: `agent_name`, `conversation_metadata`, `goal`, `permissions`, `rts_model_state`, `version`.

모델은 `session_state.rts_model_state.model_info.model_id`에 있다:

```json
{"conversation_id":"0c45...","model_info":{"model_id":"auto","context_window_tokens":200000},
 "context_usage_percentage":10.6764}
```

타임스탬프는 RFC3339(UTC, 마이크로초)라 `ProviderSession`의 RFC3339 문자열 규약과 그대로 맞는다.

**Alternatives considered**:
- `agent_name`을 `source`로 노출: 값 분포가 `kiro_default` 6건 / 없음 5건이라 정보량이 낮다. `source`는 `None`으로 둔다.
- `branch`: 메타에 git 정보가 없다. Codex는 `payload.git.branch`가 있지만 Kiro에는 없으므로 `None`.

---

## R3. `session_created_reason`은 필터 기준이 될 수 없다 ⚠️

**결정**: 이 필드를 "사람이 시작한 세션" 판별에 사용하지 않는다. **spec의 초기 가정이 틀렸음을 확인하고 수정했다.**

**실측**: 11개 세션 전부 `session_created_reason == "subagent"`.

```
11 subagent
```

여기에는 사용자가 터미널에서 직접 타이핑한 대화도 포함된다:

```
2026-08-21T04:52:28  subagent  /Users/yoophi/payhereinc  "kiro 로그인하려면 어떻게 해야해?"
```

이 값으로 필터링하면 **목록이 100% 비게 된다**. 필드의 실제 의미는 알 수 없으나(세션 생성 트리거 종류로 추정), "서브에이전트가 만든 세션"을 뜻하지 않는 것은 분명하다.

**대응**: `spec.md`의 User Story 3, FR-007, SC-005, Assumptions를 "빈 세션 제외"로 수정했다. 근거는 R4.

---

## R4. 실제 노이즈는 "대화가 없는 빈 세션"이다

**결정**: 대화 로그가 비어 있는 세션을 목록에서 제외한다.

**실측** — 세션별 로그 규모와 제목 유무:

| 로그 줄 수 | 바이트 | 제목 |
|---:|---:|---|
| 0 | 0 | (no title) ×5 |
| 14 | 15,402 | kiro 로그인하려면 어떻게 해야해? |
| 20 | 386,685 | 현재 프로젝트 아키텍처 및 제공 기능 조사 |
| 56 | 294,720 | windows 95 스타일의 디자인을… |
| 174 | 985,912 | ## Agentic Workbench MCP tools |
| 446 | 1,367,158 | 첫 번째 이슈 처리 사이클을 준비하세요… |
| 1255 | 5,299,381 | kiro-cli 를 trust 모드로 실행하려면…? |

빈 세션 5건은 모두 2026-08-24에 만들어진 것으로, **ACP 핸드셰이크 검증 중 생성된 세션**이다(#183 작업 중 `initialize` + `session/new`만 하고 프롬프트를 보내지 않음). 이어갈 내용이 없으므로 목록에 뜰 이유가 없다.

**판별 기준**: 로그 파일이 비었는지(0바이트 / 유효 이벤트 0건)로 판단한다. 제목 유무로 판단하지 않는 이유는, 대화가 진행 중이라 제목이 아직 없는 세션을 잘못 제외할 수 있기 때문이다(현재 데이터에서는 제목 유무와 로그 유무가 일치하지만, 제목은 대화 후 생성되므로 시점에 따라 어긋날 수 있다).

---

## R5. 대화 로그 이벤트 구조와 message_count

**결정**: `kind`가 `Prompt` 또는 `AssistantMessage`인 이벤트를 센다. `ToolResults`는 제외한다.

**실측** — 전체 세션의 `kind` 분포:

```
986 AssistantMessage
924 ToolResults
 62 Prompt
  1 Compaction
```

각 줄 구조: `{"kind": "...", "version": "v1", "data": {...}}`. `Prompt`의 `data`는 `content`, `message_id`, `meta`를 가진다.

**근거**: 기존 `parse_codex`도 `response_item` 중 `message` 타입만 세고 도구 호출은 제외한다. 같은 의미론을 유지한다.

**읽기 상한**: `read_json_lines(path, max_lines)`가 이미 상한을 받는다. Codex는 200줄을 쓴다. Kiro 최대 세션이 1255줄이므로 200줄 상한이면 message_count가 실제보다 작게 나온다.

**결정**: Codex와 동일하게 200줄 상한을 쓰고, `message_count`는 "표시용 근사치"로 취급한다. 5MB 로그를 전부 읽는 비용(SC-003: 2초 이내)이 정확한 카운트보다 중요하다. 이 트레이드오프를 코드 주석과 `data-model.md`에 남긴다.

**Alternatives considered**:
- 전체 줄 수 세기(`wc -l` 상당): 5MB × N개 세션을 매 조회마다 읽는다. SC-003 위반 위험. 기각.
- 메타의 `conversation_metadata.user_turn_metadatas` 길이 사용: 사용자 턴 수만 나오고 어시스턴트 응답이 빠진다. 다만 로그를 아예 안 읽어도 된다는 장점이 있다. **후보로 남겨두되**, 기존 provider와 의미론을 맞추기 위해 로그 기반을 택한다.

---

## R6. 제목이 없는 세션의 대체 표기

**결정**: 메타의 `title`이 없으면 로그 첫 `Prompt` 이벤트의 `data.content`에서 발췌해 제목으로 쓴다.

**근거**: 기존 `parse_codex`가 동일하게 동작한다(`extract_codex_user_text`로 첫 user 메시지에서 제목 생성). 일관성을 위해 같은 패턴을 따른다.

빈 세션은 R4에 따라 아예 제외되므로, "제목도 없고 프롬프트도 없는" 경우는 목록에 도달하지 않는다.

---

## R7. 세션 재개 경로

**결정**: 기존 재개 흐름을 그대로 사용한다. Kiro 전용 처리를 추가하지 않는다.

**근거**:
- Kiro는 `initialize` 응답에서 `loadSession: true`를 광고한다(#183에서 실측).
- AW는 이미 `session/load`로 세션을 재개하는 경로를 갖고 있다(`runner.rs`의 load 흐름).
- `ProviderSession.id`가 `session_id`와 같으므로 그대로 넘기면 된다.

**미검증 리스크**: 실제로 Kiro가 `session/load`에 응답하는지는 이번 조사에서 확인하지 않았다. `loadSession: true` 광고만 확인했다. **구현 단계에서 실제 재개를 검증해야 한다**(tasks에 검증 항목으로 포함). 실패 시 FR-009(실패를 사용자에게 알리고 새 세션으로 진행 가능)로 처리된다.

---

## R8. 통합 지점

**결정**: 변경 범위는 `apps/agentic-workbench/src-tauri` 안의 두 파일로 한정된다.

| 파일 | 변경 |
|---|---|
| `domain/provider_session.rs` | `ProviderKind::Kiro` 추가, `provider_kind_for`에 `"kiro-cli"` 매핑 |
| `infrastructure/fs_provider_session_repository.rs` | `kiro_root()`, `parse_kiro()` 추가, `list()`에 분기 추가 |

프론트엔드 변경은 불필요하다. 세션 목록 UI는 provider에 무관하게 `ProviderSession`을 렌더링한다.

**환경변수 규약**: 기존 provider가 각각 `CLAUDE_CONFIG_DIR`/`CODEX_HOME`/`PI_CODING_AGENT_SESSION_DIR`로 루트를 재정의할 수 있다. Kiro도 동일하게 테스트 주입이 가능해야 하므로 `SessionRoots`에 `kiro` 필드를 추가한다. 환경변수 이름은 Kiro 자체 규약이 확인되지 않았으므로 `KIRO_SESSION_DIR`을 쓰되, 없으면 `~/.kiro/sessions/cli`를 기본값으로 한다.

---

## 미해결 사항 요약

| 항목 | 처리 |
|---|---|
| `session/load`가 Kiro에서 실제로 동작하는지 | 구현 후 수동 검증 (tasks 포함) |
| `session_created_reason`의 정확한 의미 | 사용하지 않으므로 불필요 |
| `message_count` 200줄 상한으로 인한 과소 계산 | 의도적 트레이드오프, 문서화 |
| `.json`만 남고 `.jsonl`이 없는 세션 | 현재 데이터에 없음. 발생 시 누락되며, 이는 허용 가능한 열화 |
