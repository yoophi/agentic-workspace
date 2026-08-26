# Contract: Provider Session Repository — Kiro

**Feature**: `036-kiro-session-resume` | **Date**: 2026-08-25

이 기능은 새 인터페이스를 만들지 않는다. 기존 포트 `ProviderSessionRepository`의 **동작 계약에 Kiro 케이스를 추가**하는 것이 전부다. 아래는 그 계약을 검증 가능한 형태로 명시한다.

---

## 1. 대상 포트

```rust
// ports/provider_session_repository.rs (변경 없음)
trait ProviderSessionRepository {
    fn list(&self, agent_id: &str, scope: &SessionScope) -> Result<Vec<ProviderSession>>;
}
```

시그니처는 그대로다. 계약이 확장되는 것은 `agent_id == "kiro-cli"` 경로다.

---

## 2. 동작 계약

### C-1. agent_id 매핑

| 입력 `agent_id` | 기대 동작 |
|---|---|
| `"kiro-cli"` | Kiro 세션 루트를 스캔해 목록 반환 |
| `"opencode"` | 빈 목록 (지원하지 않는 provider — 기존 동작 유지) |
| 기존 3종 | 기존 동작 유지 (회귀 없음) |

### C-2. 루트 해석

```
kiro_root = $KIRO_SESSION_DIR                    (설정된 경우)
          | $HOME/.kiro/sessions/cli             (기본값)
```

`$HOME`이 없으면 `Err`. 이는 기존 provider와 동일한 규약이다.

### C-3. 루트 부재

루트 디렉토리가 존재하지 않으면 **`Ok(vec![])`** — 오류가 아니다 (FR-008).

기존 `scan_agent()`가 이미 이 동작을 보장한다.

### C-4. 스캔 범위

- `<root>/*.jsonl` 파일만 대상으로 한다
- 심볼릭 링크를 따라가지 않는다
- `<root>/<uuid>/tasks/*.json` 등 하위 산출물은 대상이 아니다 (확장자 필터로 자연 배제)

### C-5. 반환 항목의 형태

각 `ProviderSession`은 `data-model.md` §1의 매핑을 따른다. 특히:

- `agent_id == "kiro-cli"`
- `id == 메타의 session_id` (없으면 파일 stem)
- `cwd`가 채워져 있어야 `SessionScope::Path` 필터가 동작한다

### C-6. 범위 필터

`SessionScope::Path(p)`가 주어지면 `session.cwd == p`인 세션만 반환한다.
`SessionScope::All`이면 전부 반환한다.

기존 `matches_scope()`를 그대로 사용하므로 Kiro 전용 로직은 없다.

### C-7. 빈 세션 제외

대화 로그에 유효 이벤트가 없는 세션은 **반환하지 않는다** (FR-007).

### C-8. 부분 실패 격리

세션 하나가 아래 상태여도 나머지 목록은 정상 반환된다 (FR-006):

- 메타 파일(`<uuid>.json`)이 없음
- 메타 JSON이 깨짐
- 로그 JSONL의 일부 줄이 깨짐
- 필수 필드 누락

해당 세션만 결과에서 빠지거나(메타 없음/깨짐), 얻을 수 있는 정보만으로 채워진다.

---

## 3. 검증 시나리오

fixture 기반 테스트로 검증한다(헌장: 파서는 fixture 테스트 필수). 각 시나리오는 임시 디렉토리를 만들고 `SessionRoots.kiro`로 주입한다.

| ID | 시나리오 | 기대 |
|---|---|---|
| T-1 | 정상 세션 1건 (메타 + 로그) | 1건 반환, 모든 필드가 매핑대로 채워짐 |
| T-2 | `SessionScope::Path`가 세션의 cwd와 일치 | 1건 반환 |
| T-3 | `SessionScope::Path`가 불일치 | 0건 |
| T-4 | 로그가 빈 세션(0바이트) | 0건 (C-7) |
| T-5 | 메타에 `title` 없음 + 로그에 `Prompt` 있음 | 첫 Prompt에서 제목 발췌 |
| T-6 | 메타 JSON이 깨진 세션 + 정상 세션 | 정상 세션 1건만 반환 (C-8) |
| T-7 | 로그에 깨진 줄 섞임 | 유효 이벤트만 카운트, 세션은 반환됨 |
| T-8 | 루트 디렉토리 없음 | `Ok(vec![])` (C-3) |
| T-9 | `message_count` 산출 | `Prompt`+`AssistantMessage`만 계수, `ToolResults` 제외 |
| T-10 | `provider_kind_for("kiro-cli")` | `Some(ProviderKind::Kiro)` |
| T-11 | 기존 provider 회귀 | `codex`/`claude-code`/`pi-coding-agent` 매핑 불변 |

---

## 4. 계약 밖 (이 기능이 보장하지 않는 것)

- **재개 성공 자체**: `session/load`가 성공하는지는 Kiro 런타임에 달려 있다. 실패는 FR-009 경로로 사용자에게 전달된다.
- **`message_count`의 정확성**: 200줄 상한으로 인해 대형 세션에서 과소 계산된다 (`data-model.md` §4.1).
- **모델 승계**: 재개된 세션이 이전 모델 설정을 유지하는지는 Kiro 동작이며 이 계약의 범위가 아니다.
