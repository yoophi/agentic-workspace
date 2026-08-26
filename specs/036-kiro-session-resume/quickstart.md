# Quickstart: Kiro CLI Session Resume 검증

**Feature**: `036-kiro-session-resume` | **Date**: 2026-08-25

구현이 끝난 뒤 이 기능이 실제로 동작하는지 확인하는 절차다. 자동 검증(§2)과 앱에서의 수동 검증(§3)으로 나뉜다.

---

## 1. 사전 조건

- `kiro-cli`가 설치되어 있고 로그인되어 있다 (`kiro-cli whoami`로 확인)
- 로컬에 Kiro 세션이 최소 1건 있다 (`ls ~/.kiro/sessions/cli/*.jsonl`)
- 세션이 없다면 아무 디렉토리에서 `kiro-cli chat`으로 한 번 대화하고 종료한다

세션 저장소 확인:

```bash
ls -la ~/.kiro/sessions/cli/ | head
```

`<uuid>.json`과 `<uuid>.jsonl` 쌍이 보이면 준비된 것이다.

---

## 2. 자동 검증

### 2.1 단위/fixture 테스트

```bash
cd apps/agentic-workbench/src-tauri
cargo test --lib provider_session
```

`contracts/provider-session-repository.md` §3의 T-1 ~ T-11이 모두 통과해야 한다.

### 2.2 전체 백엔드 회귀

```bash
cd apps/agentic-workbench/src-tauri
cargo test --lib
```

기존 provider(Codex/Claude/Pi) 테스트가 깨지지 않아야 한다 (계약 C-1의 회귀 조항).

### 2.3 포맷·린트 게이트

```bash
cargo fmt --all --check
cargo clippy --lib
```

---

## 3. 앱에서의 수동 검증

자동 테스트는 fixture만 검증한다. 실제 Kiro 세션을 읽고 재개하는 것은 앱에서 확인해야 한다.

### 3.1 앱 실행

```bash
cd apps/agentic-workbench
pnpm tauri dev
```

> 포트 1420이 사용 중이면:
> `VITE_DEV_SERVER_PORT=1421 pnpm tauri dev --config '{"build":{"devUrl":"http://localhost:1421"}}'`

### 3.2 목록 노출 확인 (US1, US2)

1. 기존 Kiro 세션이 있는 디렉토리를 작업 디렉토리로 연다
   - 로컬 세션의 `cwd` 확인: `jq -r .cwd ~/.kiro/sessions/cli/*.json`
2. agent로 **Kiro CLI**를 선택한다
3. **기존 세션 재사용**으로 전환한다

**기대**:
- 해당 디렉토리에서 진행했던 대화가 목록에 나타난다
- 각 항목에 제목과 시각이 보인다
- 최근 활동한 세션이 위에 있다

**빈 목록이 뜬다면**: 작업 디렉토리와 세션의 `cwd`가 다른 것이다. `jq`로 확인한 경로를 그대로 사용한다.

### 3.3 재개 확인 (US1) — 가장 중요

1. 목록에서 세션 하나를 선택하고 실행을 시작한다
2. 이전 대화 내용을 참조하는 질문을 던진다
   - 예: "방금 전에 우리가 뭘 논의하고 있었지?"

**기대**: 맥락을 다시 설명하지 않아도 이전 대화를 참조한 답이 온다.

**실행 중인 프로세스로 교차 확인**:

```bash
ps -eo pid,args | grep "kiro-cli acp" | grep -v grep
```

**실패 시**: `session/load`가 거부됐을 수 있다. 이 경우 FR-009대로 실패 사유가 사용자에게 전달되고 새 세션으로 진행 가능한 상태여야 한다. 오류 메시지가 나오지 않고 조용히 새 세션이 시작된다면 그것이 결함이다.

### 3.4 빈 세션 제외 확인 (US3)

대화 없는 세션을 하나 만든다 — ACP를 연결만 하고 프롬프트 없이 종료:

```bash
{ printf '%s\n' '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{"fs":{"readTextFile":true,"writeTextFile":true},"terminal":true},"clientInfo":{"name":"probe","version":"0.1.0"}}}'
  sleep 3
  printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"session/new\",\"params\":{\"cwd\":\"$PWD\",\"mcpServers\":[]}}"
  sleep 8
} | timeout 20 kiro-cli acp > /dev/null 2>&1
```

새로 생긴 세션의 로그가 비었는지 확인:

```bash
ls -lS ~/.kiro/sessions/cli/*.jsonl | tail -3   # 0바이트 파일이 보이면 성공
```

**기대**: 앱의 세션 목록을 새로고침해도 이 빈 세션은 나타나지 않는다.

### 3.5 손상 파일 격리 확인 (FR-006)

> 실제 세션을 건드리지 말고 복사본으로 시험한다.

```bash
cp ~/.kiro/sessions/cli/<uuid>.json /tmp/kiro-meta-backup.json
echo 'not json' > ~/.kiro/sessions/cli/<uuid>.json
```

**기대**: 해당 세션만 목록에서 빠지고 나머지는 정상 표시된다. 목록 전체가 실패하거나 앱이 오류를 띄우면 결함이다.

복구:

```bash
cp /tmp/kiro-meta-backup.json ~/.kiro/sessions/cli/<uuid>.json
```

### 3.6 성능 확인 (SC-003)

세션이 30건 이상인 환경에서 "기존 세션 재사용" 전환 시 목록이 2초 안에 뜨는지 체감으로 확인한다.

현재 로컬은 11건이므로, 부하를 보려면 fixture를 복제해 임시 루트를 만들고 `KIRO_SESSION_DIR`로 가리킨 뒤 앱을 실행한다.

---

## 4. 검증 완료 기준

| 항목 | 근거 |
|---|---|
| fixture 테스트 T-1 ~ T-11 통과 | contracts §3 |
| 기존 provider 회귀 없음 | C-1 |
| 실제 Kiro 세션이 목록에 표시됨 | US1, US2 |
| 선택한 세션이 맥락을 유지한 채 재개됨 | US1, FR-005 |
| 빈 세션이 목록에 없음 | US3, FR-007 |
| 손상 세션이 나머지를 막지 않음 | FR-006 |
| `cargo fmt --check` / `clippy` 통과 | 헌장 |

§3.3(실제 재개)은 이 기능의 핵심이므로 **반드시 수동으로 확인**해야 한다. research R7에서 `session/load` 실동작을 검증하지 않았기 때문이다.
