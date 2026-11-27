# td GitHub 저장소 기능 조사

조사일: 2026-10-13. 대상은 `yoophi/td`의 `main` 커밋 `c64d7873d8f555dd49875e34e6b39ae5257e2e0c`다. GitHub에서 임시 checkout으로 내려받아 README, 저장소 계약 문서, 실제 CLI/HTTP 구현을 대조했다. 아래는 해당 소스의 구현 조사이며 AW 연동의 실행 검증이나 전체 td 테스트 통과를 의미하지 않는다.

## 결론

td는 GitHub Issues를 직접 사용하는 작업 저장소, 다섯 단계 상태, 리뷰 규칙, TDQ 필터 보드, 공유 카드 순서를 이미 제공한다. AW는 이 규칙을 재구현하기보다 Workbench 서버의 td CLI 어댑터를 통해 자체 칸반 화면을 제공하는 것이 적합하다. 여기서 GitHub 기반 보드는 **GitHub Projects v2 보드가 아니라 Issues와 td 보드 메타데이터를 이용한 보드**다. td의 `sync-project` 역시 GitHub 저장소 선택/관리 기능이며 Projects v2 컬럼 API가 아니다. [저장소 선택 계약](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/docs/storage/github-projects.md), [보드 스키마](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/docs/storage/github-metadata.md#board-carrier-schema-read-foundation)

## 실제 제공 기능

| 영역 | 확인된 기능 | AW 활용 |
|---|---|---|
| 작업 | 생성/목록/상세/수정/종료/재개/논리 삭제/복원; native Issues 읽기; PR 제외 | 카드와 상세 화면 |
| 상태 | `open`, `in_progress`, `blocked`, `in_review`, `closed`; start/unstart/block/unblock/review/approve/reject/close/reopen | 열 간 이동 시 전용 workflow 실행 |
| 보드 | 기본 `bd-all-issues`; 사용자 보드 `bd-gh-N`; TDQ 필터; 이름/필터/view-mode 수정; 순서 저장/해제 | 프로젝트별 보드 선택/필터/정렬 |
| 협업 | 세션 귀속 claim, 구조화 handoff, 로그, 댓글, 리뷰 기록 | 에이전트 배정/진행/인계 표시 |
| 관계 | 부모/epic/의존성, 순환 검사, 일부 상태 cascade | 부모와 blocker 표시 |
| 조회 | JSON CLI, TDQ, HTTP REST, monitor 통합 큐, stats/labels, SSE | 백엔드 정규화 DTO와 갱신 |
| 복구 | 관측 revision 검사, operation ID, command journal, 제한된 guarded undo | 불확실한 쓰기 결과 조회/복구 안내 |

기능별 플래그와 전체 GUI 계약의 감사는 아직 부분적이다. 명령 존재를 모든 옵션 조합의 지원으로 확대해서 해석하지 않는다. [README](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/README.md), [지원 감사](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/docs/storage/github-support-matrix.md)

## 설정과 데이터 기준

- 프로젝트 `.todos/config.json`에서 `store=gh-issue`와 선택 remote/repository를 저장한다. `git`, `gh`, github.com Git remote, 인증, Issues 활성화가 필요하다. GitHub Enterprise는 현재 지원하지 않는다. remote가 선택된 repository와 달라지면 오류이며 자동 fallback은 없다.
- GitHub가 번호/제목/본문/라벨/native open·closed/timestamp를 소유한다. td 세부 상태·우선순위·리뷰·관계 등은 본문의 versioned HTML comment(`td:issue:v1`)에 저장한다. 보드와 journal도 별도의 보조 Issue carrier를 쓴다. 일반 task 목록에서 carrier는 제외된다.
- `td:open` 등 예약 라벨은 **상태 미러**다. 사용자가 붙인 `진행중` 등의 라벨은 td 상태 전환이 아니며, 라벨 수정으로 claim/승인을 만들 수 없다. AW 열은 td가 반환한 유효 상태를 기준으로 한다. 미러 불일치는 진단으로 표시하고 `config sync-state-labels`는 별도 명시 작업으로 취급한다.
- native Issue에는 기본 type task/priority P2가 적용될 수 있다. native close/reopen은 유효 상태와 승인 효력에 영향을 준다. 메타데이터를 알 수 없거나 손상된 경우 오류이며 임의로 덮어쓰지 않는다.
- 저장소 모드 전환은 마이그레이션이 아니다. 기존 SQLite task를 자동 업로드/삭제/동기화하지 않는다. GitHub에서는 직접 원격 쓰기를 하며 offline pending queue도 없다.

근거: [메타데이터](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/docs/storage/github-metadata.md), [sync 의미](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/docs/storage/github-sync.md), [README 설정 계약](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/README.md#project-storage-configuration-fork)

## CLI 보드 계약과 주의점

실제 `cmd/gh_boards.go`는 `list/create/show/edit/delete/move/unposition`을 GitHub 저장소로 라우팅한다. README의 일부 “CLI/TUI pending” 문구는 현재 소스와 다르므로 소스를 우선한다. [실제 CLI 구현](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/cmd/gh_boards.go), [최신 보드 계약](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/docs/storage/github-metadata.md#board-cli)

```sh
td --work-dir /approved/project/path board list --json
td --work-dir /approved/project/path board show bd-all-issues --status open,in_progress,blocked,in_review,closed --json
td --work-dir /approved/project/path board move bd-all-issues gh-123 1 --json
td --work-dir /approved/project/path board unposition bd-all-issues gh-123 --json
```

`list --json`는 `Board[]`, `show --json`는 `{board_id,position,has_position,issue,category}[]`, 보드 쓰기는 저장된 Board 객체다. `show` 기본값은 closed를 제외하므로 완료 열을 제공하려면 다섯 status를 명시해야 한다. 배열은 빈 경우에도 배열이며 일반 task 목록의 기본 50개 limit와 달리 보드 조회는 자체 전체 candidate 경로를 쓴다. 카드의 `position`은 `has_position=true`일 때만 의미가 있다. [CLI 구현](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/cmd/gh_boards.go#L91-L168), [모델](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/internal/models/models.go#L256-L263)

`board move`는 **양의 1-based saved-position slot**을 지정하며 상태를 바꾸지 않는다. pinned task가 먼저 나오고 unpositioned tail은 query 순서를 유지한다. query 밖의 task를 pin했다고 필터를 우회하는 수동 membership이 되지 않는다. 같은 열 순서 변경과 열 간 상태 전환은 서로 다른 작업이다. CLI 보드 카드 모델에는 REST 상세의 `available_transitions`, dependency summary, revision, status-label 진단이 직접 포함되지 않으므로 필요한 정보는 선택 카드 상세/별도 context 조회에서 보강해야 한다. 모든 카드마다 상세를 조회하는 N+1은 피한다. [보드 query 실행](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/internal/issuestore/github_boards.go), [CLI slot 의미](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/docs/storage/github-metadata.md#board-cli)

CLI `--json`은 지원 명령에서 사용하되 command별 출력 모양을 어댑터에서 검증한다. board/list/show/read/write를 단일 envelope로 가정하지 않는다. bulk workflow는 NDJSON과 부분 성공을 낼 수 있다. 예상 JSON이 없거나 사람이 읽는 출력이 나온 경우 이를 성공 DTO로 파싱하지 말고 exit code/stdout/stderr를 보존한 뒤 원격 상태를 다시 읽는다. 테스트 없는 fallback mutation 재실행은 금지한다. [JSON 계약](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/README.md#json-output), [batch 회귀 테스트](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/cmd/gh_workflow_batch_test.go)

## 세션·리뷰·에이전트 연결

device-local context key는 host/repository/worktree/branch/agent fingerprint/`TD_CONTEXT_ID`/`TERM_SESSION_ID`/`TMUX_PANE`에서 계산한다. `TD_SESSION_ID`는 프로세스 감지보다 우선하는 explicit **fingerprint**이며 실제 저장 세션 ID는 별도로 생성된 `ses_...`다. AW run ID를 td 세션 ID로 곧바로 쓰지 말고 `whoami --json`에서 확인된 실제 ID를 매핑한다. [context 소스](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/internal/ghcontext/context.go#L33-L64), [fingerprint 소스](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/internal/session/agent_fingerprint.go#L81-L88)

AW 서버가 수행한 start와 해당 agent가 실행한 후속 td 명령을 같은 actor로 연결하려면 두 실행 경로에서 동일한 context/fingerprint/terminal 범위를 명시해야 한다. agent run을 다시 연결할 때 identity 환경을 유지하고, 독립 child/reviewer에는 다른 값을 준다. child가 부모의 identity 환경을 무조건 상속하면 동일 리뷰 actor로 합쳐질 수 있다. branch 변경도 context를 바꾸므로 기존 mapping을 재검증해야 한다. `TD_CONTEXT_ID` 변경만으로 모든 identity 조건을 통제할 수 없다. 이 수명 정책은 AW가 정의하고 fixture로 입증할 사항이다. [context 생성/새 세션](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/internal/ghcontext/context.go#L132-L145)

기본 trusted 정책은 실제 리뷰 수행자를 기록하는 자기 리뷰/attributed review를 허용하지만 이를 독립 리뷰로 증명하지 않는다. delegated/strict는 involved actor 승인에 더 엄격하다. 공유 metadata의 세션명도 검증된 GitHub 계정 신원이 아니다. AW에서 완료 열로 이동한 것만으로 approve나 독립 리뷰를 만들어서는 안 된다. 구현 claim 충돌 검사는 best-effort이며 분산 lock이 아니다. [리뷰 설정](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/internal/features/features.go#L73-L86), [writer audit](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/docs/storage/github-writer-audit.md)

## HTTP 대안과 갱신 비용

`td serve`는 GitHub mode에서 SQLite를 열지 않고 issues/workflow/comments/dependencies/boards/monitor/stats/labels/events를 실제 등록한다. `/`의 501은 API root 미지원이며 서버 자체의 GitHub 미지원이 아니다. `/v1/project`의 capability/endpoints와 응답 DTO를 검사한다. 그러나 HTTP mutation은 persistent web actor로 귀속되며 CLI actor와 분리된다. HTTP board `@me`도 CLI의 실제 actor query와 다르게 empty actor 범위를 사용한다. 따라서 agent별 identity가 필요한 MVP는 CLI adapter가 단순하다. [serve wiring](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/cmd/serve.go#L76-L108), [HTTP 보드](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/internal/serve/github_boards.go), [query actor](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/internal/issuestore/github_boards.go#L19-L50)

aggregate cache TTL은 30초, incremental baseline의 full reconciliation deadline은 5분이다. 캐시는 일부 aggregate 소비자만 대상으로 하며 모든 보드/개별 issue 호출에 적용된다고 가정하면 안 된다. native 삭제는 delta에서 즉시 제거되지 않을 수 있다. `--refresh`는 full 관측을 강제하며 쓰기 retry가 아니다. GitHub SSE의 **현재 실제 최소 간격은 5분**이다. README의 일부 30초 SSE 설명보다 실제 이벤트 구현을 우선한다. AW는 active board의 제한된 polling과 쓰기 후 즉시 재조회, last-successful 관측 시각, 오류 상태를 제공한다. [캐시](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/docs/storage/github-snapshot-cache.md), [SSE 최소 간격](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/internal/serve/github_events.go#L167-L173)

remote read는 원자 snapshot이 아니며 write PATCH도 원자 CAS를 제공하지 않는다. 충돌·timeout·cancellation 뒤 이미 반영됐을 수 있고 multi-issue cascade는 일부만 저장될 수 있다. operation/journal ID와 applied IDs를 보존하고 새 조회로 결과를 확인해야 한다. 실패를 빈 보드로 표시하거나 자동 재전송하면 안 된다. rate limit은 typed `rate_limited`와 reset guidance를 유지하고 backoff한다. 직접 CLI subprocess를 매 호출 새로 실행하면 process-local cooldown을 공유하지 못하므로 AW에도 repository별 대기/동시성 제한을 둔다. [캐시/cooldown 한계](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/docs/storage/github-snapshot-cache.md), [GUI에서 실제 timeout 후 원격 반영 사례](https://github.com/yoophi/td/blob/c64d7873d8f555dd49875e34e6b39ae5257e2e0c/docs/storage/td-gui-github-compatibility.md#verified-failure-review-response-exceeds-gui-deadline)

## AW MVP 전제와 권고

1. 해당 fork/commit 수준의 CLI 계약을 별도 검증하고 최소 기능 gate를 둔다. SemVer만으로 fork의 GitHub 보드 기능 유무를 판정하지 않는다.
2. WebView는 AW API만 호출한다. 서버의 허용된 project/worktree path에서 argv 기반 subprocess를 실행하고 td/gh 진단을 제공한다.
3. read-only 기본 보드 → CRUD/명시적 workflow → 순서/드래그 → agent binding 순으로 연동한다. 상태 이동 전에 적용 command/필수 reason/리뷰 귀속을 설명한다.
4. canonical task key를 `(repository, issue ID)`로 만든다. 저장소가 다른 `gh-123`과 `bd-gh-123`을 혼동하지 않는다.
5. UI actor와 agent actor를 분리하고 agent claim은 실제 확인된 세션 ID와 연결한다. child/reviewer 독립성은 run lifecycle 테스트로 확인한다.
6. MVP acceptance는 native Issue, metadata Issue, 완료 열, 예약/사용자 label 불일치, 충돌, rate limit, timeout 후 저장됨, 부모/의존성 cascade, 외부 수정, 변경 remote, 읽기 실패 상태를 포함한다.

이 권고는 위 소스 계약으로부터 도출한 AW 설계 제안이며 td가 AW를 지원한다는 배포 보장은 아니다.
