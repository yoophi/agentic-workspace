# Research: Worktree Auto Refresh

## Decision: React Query 기반 polling과 targeted invalidation을 1차 자동 갱신 메커니즘으로 사용한다

**Rationale**: 현재 `WorktreeWorkspacePanel`과 `git-explorer`의 `ChangesPanel`은 file/Git 데이터를 React Query로 조회한다. 기존 구조에 `refetchInterval`, `refetchOnWindowFocus`, active tab 기반 `enabled`, query key 기반 invalidation을 추가하면 3초 갱신 목표를 만족할 수 있다. `markdown-annotator`는 React Query가 없으므로 같은 refresh interval/stale policy를 local document reload adapter로 소비한다. Tauri backend의 기존 command 경계도 유지된다.

**Alternatives considered**:

- Tauri filesystem watcher: 더 즉각적이지만 platform별 watcher event 차이, Git `.git` 내부 변경 감지 범위, burst coalescing, window lifecycle 정리가 추가된다. 1차 구현에서는 요구사항 대비 비용이 크다.
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

## Decision: burst 변경은 짧은 interval과 background fetch 상태로 흡수하고, UI는 마지막 성공 데이터를 유지한다

**Rationale**: React Query는 refetch 중에도 기본적으로 이전 데이터를 유지해 사용자가 보고 있는 목록을 즉시 비우지 않는다. `isFetching`과 `isError`를 구분해 비차단 refresh indicator를 표시하면 최신화 중임을 알리면서 review context를 보존할 수 있다.

**Alternatives considered**:

- 매 filesystem event마다 즉시 refetch: burst 상황에서 flicker와 중복 command 호출이 커진다.
- explicit debounce queue를 먼저 구현: watcher 없이 polling만 쓰는 1차 구현에서는 복잡도 대비 이득이 작다. 추후 watcher 도입 시 필요하다.

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

## Decision: backend watcher command는 이번 plan의 필수 범위에서 제외한다

**Rationale**: 기존 Tauri commands가 이미 path 검증, file read, Git history/detail/diff 조회를 제공한다. 3초 이내 갱신은 shared frontend policy와 app-local adapter로 달성 가능하다. watcher는 배터리/CPU/권한/platform event coalescing 정책이 필요하므로 별도 성능 최적화 feature로 다루는 편이 경계가 명확하다.

**Alternatives considered**:

- `watch_worktree_changes` command 신설: 실시간성은 높지만 command lifecycle, unlisten, channel/event contract, watcher provider 테스트가 필요하다.
- Rust에서 Git 상태 fingerprint만 polling: frontend polling과 중복되며 command surface가 늘어난다.
