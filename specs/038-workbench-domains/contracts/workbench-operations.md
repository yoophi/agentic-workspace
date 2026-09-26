# Contract: 038 operation 29개 (`Workbench.call`)

봉투(`CallRequest`/`CallReply`/`WorkbenchFault`), `protocolVersion`, `requestId`/`idempotencyKey`, 세 경로 동일성 원칙은 037 [workbench-call.md](../../037-workbench-seam/contracts/workbench-call.md)를 따른다. 여기서는 operation별 계약과 038이 추가하는 규칙만 적는다. 입출력 형태는 [data-model.md §1](../data-model.md).

## 1. 공통 규칙 (038 추가)

1. **조회 17개**는 `idempotencyKey`·`expectedRevision`을 무시하지 않고 **거절**한다: 037과 같이 조회에 키가 오면 `invalidArgument`("idempotencyKey is only accepted for command operations.").
2. **변경 12개**는 키 필수. 없으면 `invalidArgument`("idempotencyKey is required for `<operation>`.").
3. **Git 변경 2개**는 `expectedRevision`을 받지 않는다 → `invalidArgument`("expectedRevision is not supported for git operations."). 저장 단위 변경 10개는 037 규칙(불일치 → `preconditionFailed`, outcome `notApplied`).
4. **응답 `revision`**: 저장 단위 변경은 새 aggregate revision, Git 변경과 조회는 필드 없음. Git 변경은 같은 키 재생 응답에도 `revision`이 없다(구현 확인 2026-09-27).
5. **`null` 출력**: `project.delete`·`savedPrompt.delete`·`goal.clear`·`git.createWorktree`·`git.deleteWorktree`는 `output: null`.
6. **오류 message**는 기존 데스크톱 문구 그대로(§3). Git 비정상 종료는 stderr 본문 그대로.
7. **경로 입력**은 서버가 `trim` 후 검증한다. 빈 값 → `invalidArgument`(`"<Label> is required."`, `details.fieldPath`).
8. **자원 예약은 진행 중(`pending`)에만 배타**다(R16). 같은 worktree 경로의 생성이 진행 중일 때 두 번째 생성(다른 키)은 `conflict`, outcome `unknown`, retryable=true("Another change to this worktree path is still in progress."). 앞선 실행이 끝나면(`applied`·`failed`·`unknown`) 같은 경로의 다음 변경은 정상 실행된다 — 만들고 지우고 다시 만들기, 실패 뒤 재시도 모두 새 키로 가능.
9. **upsert**(`goal.create`·`agentRunSettings.save`)는 기존 항목을 교체하며 재시작 판정은 항상 `불명`이다. 같은 키 재요청은 `conflict` outcome `unknown`(not retryable)이고, 사용자는 재조회로 실제 상태를 확인한 뒤 새 키로 다시 보낸다.

## 2. operation별 계약

표기: `scope` / kind / 주요 실패 코드. 성공 출력은 data-model 참조.

### project

| operation | scope | kind | 실패 |
|---|---|---|---|
| `project.update` | `project:write` | command | `invalidArgument`(name·workingDirectory 필수), `notFound`("Project not found."), `preconditionFailed`, `conflict` |
| `project.delete` | `project:write` | command | `notFound`, `preconditionFailed`, `conflict` |

### savedPrompt

| operation | scope | kind | 실패 |
|---|---|---|---|
| `savedPrompt.list` | `savedPrompt:read` | query | `unavailable`(저장 파일 손상 복구 실패) |
| `savedPrompt.create` | `savedPrompt:write` | command | `invalidArgument`("Button label is required." / "Prompt is required."), `conflict` |
| `savedPrompt.update` | `savedPrompt:write` | command | 위 + `notFound`("Saved prompt not found.") |
| `savedPrompt.delete` | `savedPrompt:write` | command | `notFound`, `conflict` |

### goal

| operation | scope | kind | 실패 |
|---|---|---|---|
| `goal.get` | `goal:read` | query | `invalidArgument`("Working directory is required.") — 없으면 `output: null` |
| `goal.create` | `goal:write` | command | `invalidArgument`(workingDirectory·objective 필수), `conflict`. **upsert**: 같은 worktree의 기존 목표(진행 포함)를 교체(오늘과 같음). 예약 없음, 재시작 판정 `불명` |
| `goal.update` | `goal:write` | command | `invalidArgument`(objective가 주어졌는데 빈 값), `notFound`("Goal not found.") |
| `goal.clear` | `goal:write` | command | `invalidArgument`, `notFound`("Goal not found." — 오늘 서비스와 같음. 초안의 "대상 없음은 성공"은 오기였다) |
| `goal.recordProgress` | `goal:write` | command | `invalidArgument`, `notFound` |

### agentRunSettings

| operation | scope | kind | 실패 |
|---|---|---|---|
| `agentRunSettings.get` | `agentRunSettings:read` | query | `invalidArgument` — 없으면 `output: null` |
| `agentRunSettings.save` | `agentRunSettings:write` | command | `invalidArgument`("Working directory is required.", "At least one built-in agent profile must stay enabled.", ralph 상한) |

### git (저장소 단위)

| operation | scope | kind | 실패 |
|---|---|---|---|
| `git.listRemotes` / `git.listBranches` | `git:read` | query | `invalidArgument`, `unavailable`(git 없음). 저장소가 아니거나 디렉터리가 없어 git이 실패하면 **빈 목록**(오늘과 같음 — 초안의 `notFound`는 동작 변경이라 채택하지 않음) |
| `git.listWorktrees` | `git:read` | query | 같음. `includeStatus` 생략 시 **true**(오늘 command 기본값과 같음 — 초안의 false는 오기) |
| `git.createWorktree` | `git:write` | command | `invalidArgument`(필수 입력, 기본 경로를 만들 수 없음 — "Failed to resolve project directory name." 등), `unavailable`, `internal`(예: 경로 이미 존재·잘못된 reference — "Failed to create git worktree: <git stderr>"), `conflict`(같은 키 다른 내용 / 같은 경로 생성이 **진행 중** → 재시도 없이 `conflict` outcome `unknown` retryable, 규칙 8) |
| `git.deleteWorktree` | `git:write` | command | `invalidArgument`("Worktree path is required."), `notFound`("Git worktree not found." — 목록에 없는 경로), `preconditionFailed`("Worktree has changes and cannot be deleted." / "Worktree status is not resolved yet and cannot be deleted.") outcome `notApplied`, `unavailable`, `internal`, `conflict`(규칙 8 — 삭제도 대상 경로를 `pending` 동안 예약) |

### worktree (체크아웃 디렉터리 단위)

| operation | scope | kind | 실패 |
|---|---|---|---|
| `worktree.listChanges` / `worktree.getChanges` / `worktree.getFileDiff` | `worktree:read` | query | Git 조회 공통. git-core reader를 거치는 조회(`getChanges`·`getFileDiff`·이력·그래프·커밋)는 실패가 전부 `internal`이다(git-core가 `String` 오류를 돌려주고, git-core는 바꾸지 않는다) |
| `worktree.listFiles` | `worktree:read` | query | `invalidArgument`, `notFound`("Working directory must be a directory."), `forbidden`("File path must stay inside the worktree." — `scope.dir` 탈출) |
| `worktree.readTextFile` | `worktree:read` | query | `invalidArgument`("File path is required."), `forbidden`(탈출), `notFound`("Only regular files can be previewed." / 없음). 512KB 초과는 성공 + `truncated: true`. UTF-8 아님은 오늘처럼 오류 "Only UTF-8 text files can be previewed." → `invalidArgument`(`/path`) |
| `worktree.listHistory` / `worktree.getGraph` | `worktree:read` | query | Git 조회 공통. `maxCount` 상한 500(초과 시 clamp, 오늘과 같음) |
| `worktree.getCommitDetail` / `worktree.getCommitFileDiff` | `worktree:read` | query | Git 조회 공통. 없는 커밋은 git stderr → `internal`(stderr 해석 안 함, grill Q5) |

### agent

| operation | scope | kind | 실패 |
|---|---|---|---|
| `agent.list` | `agent:read` | query | 없음(환경 설정 오류는 빈 목록·기본값, 오늘과 같음) |
| `agent.listProviderSessions` | `agent:read` | query | `invalidArgument`(`agentId` 누락 — 입력 형식), `internal`(목록 전체를 만들 수 없음, 예: HOME 없음). 빈 `agentId`·미지원 provider는 빈 목록(오늘과 같음 — 초안의 "빈 값 → invalidArgument"는 동작 변경이라 채택하지 않음). 손상 항목은 건너뜀 |

## 3. 보존 문구(골든)

`Working directory is required.` · `Worktree path is required.` · `File path is required.` · `Button label is required.` · `Prompt is required.` · `Project name is required.` · `Project not found.` · `Saved prompt not found.` · `Goal not found.` · `At least one built-in agent profile must stay enabled.` · `File path must stay inside the worktree.` · `Working directory must be a directory.` · `Only regular files can be previewed.` · `Worktree has changes and cannot be deleted.` · `Worktree status is not resolved yet and cannot be deleted.` · `Failed to resolve project directory name.` · `Failed to resolve project parent directory.` · 그리고 goal/settings 서비스의 `"{label} is required."` 계열.

## 4. fixture (`crates/workbench-protocol/fixtures/`)

파일 이름 규칙 `<domain>-<verb>-<case>.json`. 037 형식 + R8 확장:

```json
{
  "name": "worktree-list-history-seeded",
  "principal": "desktop",
  "seed": {
    "gitRepo": {
      "commits": [
        { "message": "init", "files": { "README.md": "hello\n" } },
        { "message": "feat", "files": { "src/a.rs": "fn a() {}\n" }, "branch": "feature/a" }
      ],
      "workingChanges": { "README.md": "hello world\n" }
    }
  },
  "request": { "operation": "worktree.listHistory", "input": { "workingDirectory": "{{repo}}" } },
  "expect": { "reply": { "kind": "complete", "output": { "commits": [ { "subject": "feat" }, { "subject": "init" } ] } } },
  "ignoreFields": ["output.commits[].hash", "output.commits[].authoredAt"]
}
```

- `seed.savedPrompts` / `seed.goals` / `seed.agentRunSettings`: 배열을 해당 파일에 기록.
- `{{repo}}`·`{{repoName}}` 치환은 request와 expect 양쪽 문자열에 적용.
- 필수 fixture 집합(FR-003·SC-002): operation마다 성공 1 + 실패 1 이상; 변경 12개는 추가로 `idempotent-replay`·`conflict-different-payload`·`missing-idempotency-key`; 저장 단위 변경 10개는 `stale-revision`; `git.createWorktree`는 `expected-revision-rejected`; `worktree.readTextFile`은 `outside-worktree`·`truncated`·`non-utf8`; `system-describe-desktop/readonly` 갱신(32/19).

## 5. 세 경로 검증

`tests/contract_suite.rs`가 전 fixture를 in-memory와 HTTP로 실행해 서로 비교(037 그대로). Tauri 경로는 `workbench_compat.rs` 유닛 테스트가 같은 fixture의 request를 `*Input`→`CallRequest` 변환 결과와 대조하고, `reply`→`Result<_, String>` 변환에서 message만 남는지 확인한다.

## 6. 재시작 판정 시나리오 (037 `ledger_crash_points.rs`는 수정하지 않고 `us1_crash_points.rs`·`git_reconcile.rs`·`reservation_lifecycle.rs`에 추가)

| 시나리오 | 중단 지점 | 기대 |
|---|---|---|
| `savedPrompt.create` | AfterPending / AfterJsonSave / BeforeApplied | unknown / applied / applied, 항목 수 정확 |
| `goal.recordProgress` | AfterJsonSave | `unknown`(수정은 판정 불가), 파일에는 반영됨, 같은 키 재요청 → `conflict` outcome `unknown` |
| `savedPrompt.delete` | AfterJsonSave | `applied`(대상 없음) |
| `git.createWorktree` | AfterPending / AfterJsonSave(=git 명령 뒤) | unknown / applied(경로가 목록에 있음) |
| `git.deleteWorktree` | AfterJsonSave | applied(경로 없음) |
| `agentRunSettings.save` + `FailPoint::LedgerComplete` | 저장 뒤 확정 실패 | `unavailable` outcome `unknown`, pending 유지, 재시작 뒤 `unknown` 확정 |
| `goal.create` — worktree D에 **기존 목표 G1**이 있고 다른 objective로 교체 요청 | AfterPending | `unknown`(G1이 있어도 적용 증거가 아님), 파일에는 G1 그대로, 같은 키 재요청 → `conflict` outcome `unknown`, 새 키로 재요청 → 교체 성공 |
| `git.createWorktree` P → `git.deleteWorktree` P → `git.createWorktree` P (전부 새 키) | 중단 없음 | 세 번 모두 성공(예약이 종료 상태에서 해제됨) |
| `git.createWorktree` P가 잘못된 reference로 `failed` → 같은 P 새 키 재시도 | 중단 없음 | 성공 |
| 같은 P에 `git.createWorktree` 두 건 동시(다른 키) | 중단 없음 | 하나 `applied`, 하나 `conflict` outcome `unknown` retryable |
| 037이 만든 v1 ledger 파일(`applied` 행에 예약 남음)로 기동 | — | schema v2로 승격, 같은 예약 값으로 `begin` 성공 |
