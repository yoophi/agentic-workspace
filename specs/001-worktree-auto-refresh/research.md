# Research: Worktree Auto Refresh

## Decision: Tauri filesystem watcher event를 1차 자동 갱신 메커니즘으로 사용하고 React Query polling은 fallback으로 유지한다

**Rationale**: 사용자가 열린 worktree/repository/document를 외부에서 변경하면 UI는 즉시 반응해야 한다. `agentic-workbench`와 `markdown-annotator`는 Tauri/Rust `notify` watcher가 filesystem event를 debounce한 뒤 window-scoped event를 emit하고, frontend가 active scope query만 invalidate/reload한다. `git-explorer`는 이미 존재하는 `repository-changed` watcher invalidation을 유지한다. React Query `refetchInterval`은 watcher event 누락, app lifecycle, focus 복귀를 보정하는 30초 fallback으로만 사용한다.

**Alternatives considered**:

- 짧은 React Query polling만 사용: 구현은 단순하지만 세 앱의 file/Git/markdown query가 계속 실행되어 불필요한 Tauri/Git 호출이 늘어난다.
- Git command hook/post-commit hook: 사용자의 repository에 hook을 설치해야 하므로 앱 기능으로 부적절하다.
- 전체 workspace reload: 구현은 쉽지만 selection/scroll 보존 요구와 충돌한다.

## Decision: refresh policy와 stale selection 판정은 `packages/workspace-auto-refresh`에서 공유한다

**Rationale**: auto reload가 세 앱에 동시에 적용되어야 하므로 interval, focus refresh, stale selection, last-successful-data 유지 정책은 pure helper로 공유해야 한다. Constitution의 Shared Core Before Shared UI 원칙에 따라 UI는 공유하지 않고 정책/상태 판정만 공유한다.

**Alternatives considered**:

- 세 앱에 helper를 복사: 초기 구현은 빠르지만 정책 drift가 생긴다.
- 공유 UI까지 추출: 세 앱의 화면 구조와 디자인 요구가 다르므로 현재 단계에서는 과하다.

## Decision: 파일 목록과 Git explorer는 active worktree/repository/document key로만 갱신한다

**Rationale**: 요구사항은 unrelated worktree/repository/document 갱신 금지를 명시한다. workbench query key는 `worktreeFileQueryKeys.list(worktree.path)`, `worktreeGitQueryKeys.history(worktree.path)`, `worktreeGitQueryKeys.graph(worktree.path)`, `projectQueryKeys.worktreeChanges(worktree.path)`처럼 worktree path를 포함한다. git-explorer query key는 repository id와 query option을 포함한다. markdown-annotator는 active markdown file path를 reload scope로 사용한다. 이 구조를 유지하면 다른 session/window cache invalidation을 피할 수 있다.

**Alternatives considered**:

- 모든 worktree/repository/document query invalidation: 구현은 단순하지만 unrelated 대상까지 불필요하게 갱신한다.
- project root 단위 invalidation: branch/worktree 혼동이 생기고 scope 요구와 맞지 않는다.

## Decision: burst 변경은 Rust watcher debounce와 foreground fetch 상태로 흡수하고, UI는 마지막 성공 데이터를 유지한다

**Rationale**: filesystem watcher는 저장/rename/checkout 과정에서 짧은 시간에 여러 event를 보낼 수 있다. Rust watcher가 300~500ms 단위로 debounce한 뒤 frontend에 scope event를 보내고, React Query는 refetch 중에도 이전 데이터를 유지한다. `isFetching`과 `isError`를 구분해 비차단 refresh indicator를 표시하면 최신화 중임을 알리면서 review context를 보존할 수 있다.

**Alternatives considered**:

- 매 filesystem event마다 즉시 refetch: burst 상황에서 flicker와 중복 command 호출이 커진다.
- frontend debounce만 사용: window가 background 상태일 때 event 처리와 query fetch lifecycle이 섞이므로 watcher source에서 1차 coalescing하는 편이 단순하다.

## Decision: selected file/commit은 refresh 후 존재 여부를 검증하고 stale 상태로 전환한다

**Rationale**: 자동 갱신의 핵심 위험은 사용자가 보던 대상이 삭제, rename, rebase, reset으로 사라지는 경우다. 파일은 refreshed `WorktreeFileEntry[]`에서 `relativePath` 존재 여부로 판단한다. commit은 refreshed history/graph의 loaded commit hash 또는 detail query error를 기준으로 stale 처리한다. 존재하면 선택을 유지하고, 사라지면 detail pane에 stale 상태와 재선택 안내를 보여준다.

**Alternatives considered**:

- 선택을 항상 초기화: 사용자의 review context를 불필요하게 잃는다.
- 존재하지 않는 대상의 detail을 계속 유지: stale data를 최신 정보로 오해하게 만든다.

## Decision: infinite commit list/graph는 최신 head를 우선 갱신하고, 필요 시 loaded pages를 reset한다

**Rationale**: 새 commit이나 branch 변경은 첫 페이지에 가장 큰 영향을 준다. 자동 refresh는 첫 페이지를 최신화하는 것이 우선이며, branch/ref가 바뀌거나 commit hash continuity가 깨지면 loaded pages 전체를 reset/refetch해야 stale pagination을 피할 수 있다.

**Alternatives considered**:

- 모든 interval마다 모든 loaded page refetch: 긴 history에서 불필요한 command 호출이 커진다.
- 기존 pages 유지 후 append만 수행: branch 전환/rebase/reset에서 잘못된 commit list가 섞일 수 있다.

## Decision: git-explorer의 기존 repository watcher는 유지하되 shared refresh policy와 충돌하지 않게 보강한다

**Rationale**: `git-explorer`에는 repository event 기반 invalidation이 이미 있다. 이를 제거하지 않고 shared refresh policy를 query fallback으로 적용하면 watcher event 누락이나 focus 복귀 상황에서도 최신성이 유지된다.

**Alternatives considered**:

- watcher만 사용: watcher event가 누락되거나 앱 lifecycle에서 끊기면 stale data가 남을 수 있다.
- polling만 사용: 이미 존재하는 watcher invalidation 이점을 버리게 된다.

## Decision: backend watcher command는 app-local lifecycle로 제한한다

**Rationale**: watcher는 filesystem adapter이므로 각 Tauri 앱의 infrastructure에 둔다. inbound command는 `start_*_watcher`/`stop_*_watcher` lifecycle과 Tauri event emit만 담당하고, file/Git 조회 로직은 기존 application service/query command에 남긴다. watcher handle은 window label별로 보관해 session/document window unmount 시 정리한다.

**Alternatives considered**:

- shared Rust watcher crate 선행 추출: 세 앱의 path scope와 event 종류가 아직 다르므로 app-local 구현 후 중복이 안정되면 추출한다.
- Rust에서 Git 상태 fingerprint polling: frontend fallback polling과 중복되며 command surface가 늘어난다.
