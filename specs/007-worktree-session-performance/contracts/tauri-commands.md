# Contracts: Tauri Commands / 공유 crate 계약 변경

프론트(entities API)와 백엔드(inbound command) 사이, 그리고 `crates/git-core` port의 계약 변경을 정의한다. 모든 변경은 **하위 호환**(신규 파라미터는 옵션, 기존 호출은 동작 불변)이다.

## 1. 실행 방식 변경 (계약 시그니처 불변)

아래 command는 payload/응답 구조 변경 없이 `async fn` + `spawn_blocking`으로 전환된다. 호출자는 영향 없음.

- `list_git_worktrees`, `list_git_branches`, `list_git_remotes`
- `get_worktree_changes`, `get_worktree_file_diff`, `list_worktree_changes`
- `list_worktree_git_history`, `get_worktree_git_graph`, `get_worktree_commit_detail`, `get_worktree_commit_file_diff`
- `list_worktree_files`, `read_worktree_text_file`
- `start_worktree_watcher`(내부 `git rev-parse` 2회 포함), `create_git_worktree`, `delete_git_worktree`

**동작 보장**: 서로 다른 command의 동시 invoke가 상호 차단되지 않는다. 같은 worktree에 대한 read-only git 동시 실행은 허용된다(status 계열은 `--no-optional-locks`로 index 쓰기 없음).

## 2. `list_git_worktrees` (파라미터 추가)

```ts
// entities/project/api/git-worktree-repository.ts
listGitWorktrees(workingDirectory: string, options?: { includeStatus?: boolean }): Promise<GitWorktree[]>
```

```rust
#[tauri::command]
async fn list_git_worktrees(working_directory: String, include_status: Option<bool>) -> Result<Vec<GitWorktree>, String>
```

- `includeStatus` 생략/`true`: 현행과 동일(clean/dirty 계산).
- `includeStatus: false`: worktree당 `git status` 생략. `status: "unknown"`, `canDelete: false`.
- 호출처: session route → `false`, 프로젝트 상세/dashboard → 생략(기존 동작).
- port 변경: `GitWorktreeProvider::list_worktrees(&self, working_directory: &str, include_status: bool)`.

## 3. `list_worktree_git_history` / `get_worktree_git_graph` (파라미터·응답 확장)

```ts
listWorktreeGitHistory(path, { maxCount, offset, cursor? }): Promise<GitCommitHistory>
getWorktreeGitGraph(path, { maxCount, offset, cursor? }): Promise<GitCommitGraph>
```

```rust
#[tauri::command]
async fn get_worktree_git_graph(
    working_directory: String,
    max_count: Option<usize>,
    offset: Option<usize>,
    cursor: Option<String>,   // 신규: 마지막으로 받은 commit hash
) -> Result<GitCommitGraph, String>
```

응답 확장 (`GitCommitPage`/`GitGraphPage`):

| 필드 | 규칙 |
|---|---|
| `totalCount?` | 첫 페이지(cursor 없음, offset 0)에서만 채움. 이후 페이지 `null` |
| `cursorInvalidated?` | cursor가 현재 이력에 없으면 `true` + offset 방식 결과 반환. 프론트는 누적 페이지를 초기화 |
| `refs`(graph만) | 첫 페이지에서만 채움. 이후 페이지 빈 배열 |

- cursor와 offset이 함께 오면 cursor 우선.
- git-core port 변경: `GitHistoryReader::list_history/get_commit_graph`에 `cursor: Option<&str>` 파라미터 추가. **git-explorer도 이 port를 소비하므로 시그니처 변경 시 양쪽 앱 어댑터를 함께 갱신하고 각각 검증한다(하위 호환 기본값 `None`).**

## 4. `list_worktree_files` (scope 파라미터 추가)

```ts
listWorktreeFiles(workingDirectory: string, scope?: {
  kind?: "all" | "markdown";
  dir?: string;      // 상대 경로, 경로 탈출 방지 검증 적용
  depth?: number;    // 1 = 직계만
}): Promise<WorktreeFileEntry[]>
```

- 생략 시 현행(전체 트리)과 동일.
- `kind: "markdown"`: markdown 파일 + 조상 디렉터리만 반환.
- `dir` + `depth: 1`: 해당 디렉터리 직계 항목만(Files 탭 폴더 펼침).
- 오류: `dir`이 worktree 밖이면 기존과 동일한 검증 오류 문자열.

## 5. Watcher 이벤트 (`workspace://worktree-changed`) 발행 규칙 변경

이벤트 payload 구조는 불변. 발행 규칙만 변경:

| 규칙 | Before | After |
|---|---|---|
| debounce | leading-edge 500ms(창 내 이후 이벤트 폐기) | trailing 500ms(마지막 변경 후 1회 발행, 유실 없음) |
| 무시 디렉터리 | `node_modules`, `target` | `EXCLUDED_DIRS` 전체(`.git` 제외 목록과 동일 상수) |
| `kind: "git"` 판정 | `.git` 하위 전체 | `HEAD`, `refs/`, `MERGE_HEAD`, `packed-refs`만. `index`/`*.lock`/`FETCH_HEAD` 단독 변화는 미발행 |

프론트 invalidation 계약(worktree-workspace-panel): `kind: "file"` → 파일 목록·미리보기·working tree 상태만, `kind: "git"` → 위 항목 + history/graph/commit-detail/diff. 활성 탭이 아닌 query는 TanStack Query의 inactive invalidation(다음 mount 시 refetch)으로 처리되며 즉시 refetch를 강제하지 않는다.

## 6. 프론트 query 정책 (계약이 아닌 관측 가능 규칙)

| Query | 규칙 |
|---|---|
| `historyQuery` | `enabled: historyView === "list"` |
| `graphQuery` | `enabled: historyView === "graph"` |
| `agentRunQueryKeys.agents` | `staleTime: 5분` |
| `agentRunQueryKeys.settings(*)` | `staleTime: 30초` |
| `agentRunQueryKeys.goal(*)` | `staleTime: 10초` |

## 7. 계측 로그 (opt-in, 안정 계약 아님)

`AW_PERF_LOG=1`에서 stderr로 `perf kind=<command|git|watcher> name=<...> wait_ms=<n> run_ms=<n>` 형식 라인을 발행한다. 프론트 `performance.measure` 이름: `session:shell-rendered`, `session:graph-first-row`. 로그 포맷은 디버깅 용도로, 외부 소비 계약이 아니다.
