# Kiro CLI 세션 재개

`agentic-workbench`는 Kiro CLI가 로컬에 남긴 대화 세션을 "기존 세션 재사용"
목록에 노출하고, 선택한 세션을 이어서 진행할 수 있게 한다. Codex·Claude
Code·Pi Coding Agent와 동일한 흐름이며 Kiro 전용 UI는 없다.

관련 스펙: `specs/036-kiro-session-resume/`

## 세션 저장소 구조

Kiro는 세션 하나를 파일 두 개로 남긴다.

```text
~/.kiro/sessions/cli/
├── <uuid>.json      # 메타 — 목록 표시에 필요한 정보가 전부 여기 있다
├── <uuid>.jsonl     # 대화 로그 — 이벤트 한 줄에 하나
├── <uuid>.history   # 슬래시 커맨드 입력 이력 (사용하지 않음)
└── <uuid>/tasks/    # 태스크 산출물 (사용하지 않음)
```

루트는 `KIRO_SESSION_DIR` 환경변수로 재정의할 수 있고, 없으면
`$HOME/.kiro/sessions/cli`를 쓴다. 다른 provider와 같은 규약이다.

스캔은 `.jsonl` 파일만 순회한다. 메타는 같은 stem의 `.json`에서 읽는다.

### 메타에서 가져오는 값

| ProviderSession | 출처 |
|---|---|
| `id` | `session_id` (없으면 파일 stem) |
| `cwd` | `cwd` |
| `title` | `title`, 없으면 첫 프롬프트에서 발췌 |
| `created_at` / `updated_at` | 동명 필드 (RFC3339) |
| `model` | `session_state.rts_model_state.model_info.model_id` |
| `branch` | 항상 `None` — Kiro 메타에 git 정보가 없다 |
| `source` | 항상 `None` — 아래 참조 |

## 빈 세션을 제외하는 이유

대화 로그에 유효 이벤트가 하나도 없는 세션은 목록에서 뺀다. 연결만 맺고
프롬프트를 주고받지 않은 경우로, 이어갈 내용이 없다.

판별은 **로그 기준**이며 제목 유무로 하지 않는다. 제목은 대화가 어느 정도
진행된 뒤에 붙기 때문에, 제목으로 거르면 진행 중인 세션을 잘못 제외한다.

### `session_created_reason`을 쓰지 않는 이유

메타에는 `session_created_reason` 필드가 있고 이름만 보면 "사람이 시작한
세션"을 가려낼 수 있을 것 같지만, **쓸 수 없다**.

로컬 세션 11건을 전수 조사한 결과 전부 값이 `subagent`였다. 사용자가 터미널
에서 직접 타이핑한 대화도 마찬가지였다.

```text
2026-08-21T04:52:28  subagent  ~/payhereinc  "kiro 로그인하려면 어떻게 해야해?"
```

이 값으로 필터링하면 목록이 100% 비게 된다. 같은 이유로 `source` 필드에도
싣지 않는다 — 모든 세션이 같은 값이라 정보량이 없다.

## 알려진 제약

### `message_count`는 근사치다

목록 표시를 위해 대화 로그를 **최대 200줄까지만** 읽는다(다른 provider와 동일
상한). 실측 최대 세션이 1255줄 5.3MB였고, 이런 세션을 매 조회마다 전부 읽으면
목록이 느려진다.

따라서 200줄을 넘는 세션의 메시지 수는 실제보다 작게 표시된다. 목록의 보조
표시값이므로 정확도보다 응답 속도를 택했다.

계수 대상은 `Prompt`와 `AssistantMessage` 이벤트다. `ToolResults`와
`Compaction`은 메시지가 아니므로 세지 않는다 — Codex 파서가 도구 호출을 빼고
메시지만 세는 것과 같은 기준이다.

### `.jsonl`이 없는 세션은 목록에 뜨지 않는다

스캔이 `.jsonl` 기준이라, 메타(`.json`)만 남고 로그가 없는 세션은 누락된다.
조사 시점 로컬 데이터에서는 `.json`과 `.jsonl`이 11쌍으로 정확히 짝을 이뤄
이런 세션이 없었다. Kiro 구현이 바뀌면 발생할 수 있는 열화다.

### 재개 성공 여부는 Kiro에 달려 있다

목록에서 세션을 고르면 ACP `session/load`로 재개를 시도한다. Kiro는
`initialize` 응답에서 `loadSession: true`를 광고한다.

재개가 실패하면 실패 사유가 사용자에게 전달되고 새 세션으로 진행할 수 있는
상태가 유지된다.

### 재개 시 모델·effort

재개된 세션에 적용되는 모델·effort는 기존 실행 설정 흐름을 따른다. 이전 세션의
모델 설정이 그대로 승계되는지는 Kiro 동작에 달려 있으며 이 기능이 보장하지
않는다. Kiro의 모델·effort 전달 방식은 `docs/acp-agent-command-override.md`의
"모델·effort 전달 경로"를 참고한다.

## 구현 위치

| 파일 | 역할 |
|---|---|
| `apps/agentic-workbench/src-tauri/src/domain/provider_session.rs` | `ProviderKind::Kiro`, `provider_kind_for` 매핑 |
| `apps/agentic-workbench/src-tauri/src/infrastructure/fs_provider_session_repository.rs` | `kiro_root()`, `parse_kiro()`, 이벤트 판별 |

목록 정렬(최근 활동 우선)과 개수 제한은
`application/list_provider_sessions.rs`가 provider 무관하게 처리하므로 Kiro
전용 코드가 없다. 프론트엔드도 변경이 없다.
