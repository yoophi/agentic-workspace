# agent-session-manager 로컬 파일 기반 세션 조회 분석

- 조사일: 2026-08-28
- 대상 저장소: [yoophi/agent-session-manager](https://github.com/yoophi/agent-session-manager)
- 검토 커밋: `1e4819eaa778dca0eb35f61ac280871198e4e21f`
- 관련 조사: [활성 코딩 에이전트 세션 조회 조사](active-coding-agent-session-discovery.md)

## 결론

현재 `agent-session-manager`는 로컬 파일을 이용하는 **저장 세션 조회기**다. Claude Code, Codex, Pi Coding Agent의 JSONL transcript를 찾아 공통 형식으로 출력하지만, 세션이 현재 실행 중인지 또는 모델이 실제로 작업 중인지 판정하지 않는다.

현재 구현은 다음 용도에 적합하다.

- 과거에 저장된 세션 목록 조회
- 세션 ID, 작업 디렉터리, 모델, 제목 등 metadata 표시
- 작업 디렉터리별 세션 필터링
- 세션 transcript 휴지통 이동

다음 용도에는 별도 구현이 필요하다.

- 실행 중인 세션만 조회
- `working`, `waiting_user`, `idle` 구분
- PID 및 프로세스 시작 시각 연결
- 상태 판정의 신뢰도 표시
- OpenCode 지원

따라서 기존 `SessionRepository`는 history 조회용으로 유지하고, 별도의 `SessionActivityProbe` 계층을 추가하는 방식이 적절하다.

## 현재 구현의 파일 탐색 방식

핵심 구현은 [`src/outbound/filesystem.rs`](https://github.com/yoophi/agent-session-manager/blob/main/src/outbound/filesystem.rs)에 있다.

| 에이전트 | 기본 조회 경로 | override | 현재 결과가 의미하는 것 |
| --- | --- | --- | --- |
| Claude Code | `~/.claude/projects` | `CLAUDE_CONFIG_DIR` | 저장된 Claude transcript |
| Codex | `~/.codex/sessions` | `CODEX_HOME` | 저장된 Codex rollout/thread |
| Pi | `~/.pi/agent/sessions` | `PI_CODING_AGENT_SESSION_DIR` | 저장된 Pi session |

각 root 아래를 `WalkDir`로 재귀 순회하며, 확장자가 `.jsonl`인 모든 일반 파일을 파싱한다. symlink는 따라가지 않는다.

```rust
for entry in WalkDir::new(root)
    .follow_links(false)
    .into_iter()
    .filter_map(Result::ok)
    .filter(|entry| entry.file_type().is_file())
    .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
{
    // agent별 parser 실행
}
```

### Claude Code

`~/.claude/projects/**/*.jsonl`을 읽는다. 경로 중 `subagents`가 포함된 파일은 기본 목록에서 제외한다.

추출하는 필드는 다음과 같다.

- `sessionId`
- `cwd`
- `timestamp`
- `message.model`
- `gitBranch`
- `entrypoint` 또는 `promptSource`
- 첫 user message를 이용한 title
- user와 assistant message 수

### Codex

`~/.codex/sessions/**/*.jsonl`을 읽는다. `session_meta`와 `response_item` 레코드를 중심으로 다음 정보를 추출한다.

- `payload.id`
- `payload.cwd`
- `payload.source`
- `payload.git.branch`
- 첫 user message를 이용한 title
- message 수와 model

`~/.codex/archived_sessions`는 현재 탐색하지 않는다.

### Pi Coding Agent

`~/.pi/agent/sessions/**/*.jsonl`을 읽는다. 다음 entry를 사용한다.

- `session`: ID, cwd, 생성 시각
- `model_change`: model
- `message`: message 수와 첫 user message
- `session_info`: session name

구현은 `PI_CODING_AGENT_SESSION_DIR`만 확인한다. `~/.pi/agent/settings.json`의 `sessionDir` 또는 사용자가 개별 실행에서 지정한 `--session-dir`은 자동 발견하지 못한다.

## 현재 공통 출력 형식

[`src/domain/mod.rs`](https://github.com/yoophi/agent-session-manager/blob/main/src/domain/mod.rs)의 `AgentSession`은 다음 정보를 보관한다.

```text
agent
id
cwd
title
file
message_count
created_at
updated_at
model
branch
source
is_subsession
parent_session_id
```

다음 필드는 없다.

```text
pid
runtime
activity
status
waiting_for
last_heartbeat_at
confidence
```

따라서 현재 JSON이나 CSV 출력만으로 active 여부를 알 수 없다. 자세한 출력 형식은 저장소의 [`docs/output-schema.md`](https://github.com/yoophi/agent-session-manager/blob/main/docs/output-schema.md)에 정의되어 있다.

## 로컬 데이터 검증 결과

검증 환경에서 다음 명령을 실행했다.

```bash
cargo run --quiet -- list --all-paths --output json
```

저장된 세션 수는 다음과 같았다.

| 에이전트 | 저장된 세션 수 |
| --- | ---: |
| Claude | 112 |
| Codex | 838 |
| Pi | 24 |
| 합계 | 974 |

같은 시점에 공식 런타임 명령으로 확인된 Claude 세션은 13개였다.

```bash
claude agents --json
```

이는 다음 두 조건이 서로 다르다는 것을 보여준다.

```text
agent-sessions list에 나타남 = 과거 대화 파일이 존재함
active session = 현재 런타임이 관리 중이거나 작업 중임
```

전체 Rust 테스트 17개는 통과했다.

## 발견한 정확성 문제

### 처음 200줄만 파싱함

모든 parser는 다음 함수를 호출한다.

```rust
read_json_lines(path, 200)
```

`read_json_lines`는 파일의 앞부분 200줄까지만 읽는다.

```rust
for line in reader.lines().take(max_lines) {
    // JSON parse
}
```

557줄짜리 실제 Codex session으로 비교한 결과는 다음과 같았다.

| 항목 | 처음 200줄 | 전체 파일 |
| --- | ---: | ---: |
| 감지된 message 수 | 9 | 25 |
| 마지막 timestamp | 12:07 | 12:21 |

따라서 긴 세션에서는 다음 값이 실제보다 오래되거나 작게 나온다.

- `message_count`
- `updated_at`
- 마지막으로 관측된 model
- 갱신 시각 기준 정렬

JSON에서 timestamp를 하나라도 발견하면 filesystem `mtime`으로 보정하지 않는다. 그러므로 최근에 계속 갱신 중인 긴 파일도 오래된 세션처럼 정렬될 수 있다.

### 전체 탐색 성능

974개 저장 세션을 JSON으로 조회하는 데 약 7.2초가 걸렸다.

현재 방식은 실행할 때마다 다음 작업을 반복한다.

1. 세 에이전트의 디렉터리 전체 재귀 순회
2. 모든 JSONL 파일 열기
3. 파일마다 최대 200개 JSON 레코드 파싱
4. 전체 결과 재정렬

따라서 수 초 간격의 실시간 polling에 직접 사용하기 어렵다.

### 사용자 prompt 노출 가능성

명시적 세션 이름이 없으면 첫 user message를 80자로 잘라 title로 사용한다. JSON 출력을 로그, telemetry 또는 외부 서버로 전송하면 사용자의 prompt 일부가 노출될 수 있다.

기본 UI에서는 title을 opt-in으로 표시하거나 별도의 redaction 설정을 두는 것이 안전하다.

### path 필터의 strict equality

CLI가 받은 path는 가능한 경우 canonicalize하지만, transcript에서 읽은 cwd에는 같은 처리를 하지 않는다.

```rust
session.cwd.as_ref().is_some_and(|cwd| cwd == path)
```

symlink, 대소문자 표현 또는 이미 사라진 경로가 포함되면 실제로 같은 작업 디렉터리여도 필터에서 누락될 수 있다.

## 로컬 파일 기반 active 판정 가능 범위

### Claude Code

사용 가능한 추가 로컬 파일은 다음과 같다.

```text
~/.claude/daemon/roster.json
~/.claude/jobs/<job-id>/state.json
```

로컬 검증에서 `state.json`에는 `working`, `blocked`, `failed` 같은 background job 상태와 session ID, cwd, 갱신 시각 등이 존재했다. `roster.json`에는 supervisor PID와 worker PID·session 연결 정보가 있었다.

이 파일들을 이용하면 background session은 부분적으로 판정할 수 있다.

```text
state=working  -> working
state=blocked  -> waiting_user
state=failed   -> error
roster worker  -> PID 연결 후보
```

하지만 이 파일들만으로 모든 interactive session을 포괄하지 못한다. 완전한 조회에는 `claude agents --json`을 함께 사용하는 것이 안전하다.

### Codex

추가로 사용할 수 있는 로컬 파일은 다음과 같다.

```text
~/.codex/thread-writer-locks/<thread-id>.lock
```

중요한 것은 lock 파일의 **존재**가 아니라 OS advisory lock이 실제로 점유되어 있는지다. 로컬 검증에서는 15개 held lock이 8개의 Codex PID에 연결되어 있었고, 한 Codex 프로세스가 여러 thread lock을 잡은 경우도 있었다.

따라서 가능한 판정은 다음 정도다.

```text
lock이 실제 점유됨 -> 해당 thread에 살아 있는 writer가 있음
lock 파일만 존재함 -> active 증거가 아님
```

writer lock은 `working`, `waiting_user`, `idle`을 구분하지 못한다. 결과는 `runtime=alive`, `activity=unknown`, `confidence=partial`로 표시하는 것이 적절하다.

`~/.codex/state_5.sqlite`의 `threads` 테이블에는 저장 thread metadata가 있지만 runtime status 필드는 없다.

### OpenCode

현재 저장소의 `AgentKind`는 Claude, Codex, Pi만 포함하므로 OpenCode를 지원하지 않는다.

OpenCode metadata는 다음 SQLite DB에 저장된다.

```text
~/.local/share/opencode/opencode.db
```

`session` 테이블에는 ID, directory, title, 생성·갱신 시각, model, token과 cost 정보 등이 있다. 그러나 `busy`, `idle`, `waiting_user` 같은 런타임 상태는 저장되지 않는다. 런타임 상태는 실행 중인 OpenCode server 메모리에 있으므로 로컬 DB만으로 정확한 active 판정은 불가능하다.

파일 기반만 허용한다면 OpenCode plugin 또는 wrapper가 별도의 runtime registry 파일을 작성하도록 해야 한다.

### Pi Coding Agent

Pi의 session JSONL은 history 저장소다. 현재 로컬 환경에서는 Pi session 파일을 열고 있는 프로세스나 별도의 lock 파일이 발견되지 않았다.

따라서 transcript만으로 알 수 있는 것은 다음과 같다.

- 저장 세션 존재 여부
- 마지막으로 저장된 메시지와 metadata
- filesystem `mtime`

파일이 최근 수정되었다는 사실만으로 현재 작업 중이라고 판정하면 안 된다. 임의로 실행된 Pi TUI의 정확한 상태를 파일만으로 얻으려면 extension이 별도의 registry를 작성하도록 해야 한다.

## 권장 설계

### history와 runtime 분리

기존 `SessionRepository`는 history 조회에만 사용한다.

```rust
pub trait SessionRepository {
    fn list(
        &self,
        agent: AgentKind,
        scope: &SessionScope,
    ) -> Result<Vec<AgentSession>>;
}
```

별도의 runtime port를 추가한다.

```rust
pub trait SessionActivityProbe {
    fn probe(&self, session: &AgentSession) -> Result<SessionActivity>;
}

pub struct SessionActivity {
    pub runtime: RuntimeState,
    pub activity: ActivityState,
    pub pid: Option<u32>,
    pub last_heartbeat_at: Option<SystemTime>,
    pub source: ActivitySource,
    pub confidence: Confidence,
}
```

상태 enum 예시는 다음과 같다.

```rust
pub enum RuntimeState {
    Alive,
    Managed,
    Stopped,
    Unknown,
}

pub enum ActivityState {
    Working,
    WaitingUser,
    Idle,
    Retrying,
    Error,
    Unknown,
}
```

### provider별 activity probe

| adapter | 입력 | 출력 신뢰도 |
| --- | --- | --- |
| `ClaudeLocalStateProbe` | `jobs/*/state.json`, `daemon/roster.json` | background는 높음, interactive는 부분적 |
| `CodexWriterLockProbe` | 실제 점유된 `thread-writer-locks` | runtime 존재만 부분적 |
| `OpenCodeRuntimeRegistryProbe` | wrapper/plugin이 작성한 registry | registry를 직접 관리하면 높음 |
| `PiRuntimeRegistryProbe` | extension이 작성한 registry | registry를 직접 관리하면 높음 |

### 자체 runtime registry

파일 기반 통합이 필수라면 모니터가 다음과 같은 registry 형식을 정의할 수 있다.

```json
{
  "tool": "pi",
  "sessionId": "session-id",
  "pid": 12345,
  "processStartedAt": "2026-08-28T12:00:00Z",
  "cwd": "/path/to/project",
  "activity": "working",
  "waitingFor": null,
  "updatedAt": "2026-08-28T12:00:03Z"
}
```

registry writer는 다음 규칙을 따라야 한다.

- 임시 파일에 쓴 뒤 atomic rename
- 2~5초 간격 heartbeat
- PID와 process 시작 시각을 함께 기록해 PID 재사용 방지
- `updatedAt`이 TTL을 넘으면 stale 처리
- 정상 종료 시 파일 제거
- 종료 파일 제거에 실패해도 TTL로 복구
- prompt와 transcript 본문은 registry에 기록하지 않음

### 증분 history index

974개 세션 전체 재파싱 문제를 해결하려면 SQLite index가 필요하다.

파일별로 다음 정보를 저장한다.

```text
path
agent
file_size
mtime
last_read_offset
session_id
metadata
message_count
last_timestamp
```

다음 조회에서는 파일 크기와 mtime이 바뀐 파일만 마지막 offset 이후부터 추가 파싱한다. 삭제된 파일은 정상적인 missing 상태로 처리한다.

## 구현 우선순위

1. 처음 200줄 제한으로 생기는 `updated_at`과 message count 오류 수정
2. history와 runtime 모델 분리
3. Claude `jobs/state.json`·`roster.json` adapter 추가
4. Codex writer lock 점유 검사 추가
5. 결과에 `activity`, `pid`, `confidence` 추가
6. Pi runtime registry extension 추가
7. OpenCode metadata adapter와 runtime registry 또는 server adapter 추가
8. SQLite 기반 증분 index 추가

## 최종 판단

`agent-session-manager`의 현재 로컬 파일 탐색 코드는 세 에이전트의 저장 세션을 하나로 정규화하는 기반으로는 유용하다. 그러나 이 결과를 active session 목록으로 사용하면 과거 세션이 대량으로 섞이고, 긴 세션의 갱신 시각도 부정확해진다.

로컬 파일 기반으로 구현하려면 다음 원칙을 지켜야 한다.

- transcript는 history metadata에만 사용한다.
- Claude background 상태와 Codex writer lock처럼 별도의 runtime artifact가 있을 때만 부분적인 active 판정을 한다.
- OpenCode와 Pi는 plugin 또는 wrapper가 작성하는 자체 heartbeat registry를 사용한다.
- 정확히 알 수 없는 상태는 `working`으로 추정하지 않고 `unknown`으로 표시한다.
- 모든 판정에는 `confidence`를 함께 제공한다.
