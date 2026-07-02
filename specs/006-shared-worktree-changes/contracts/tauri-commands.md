# Contracts: 포트 · Tauri 커맨드 · 공유 뷰

**Feature**: 006-shared-worktree-changes | **Date**: 2026-07-02

타입 정의는 [data-model.md](../data-model.md) 참조. 이 문서는 계층 간 인터페이스 계약만 기술한다.

## 1. Rust 포트 (crates/git-core/src/ports.rs)

```rust
/// working_directory(path) 기반 미커밋(working-tree) status/diff 조회 포트.
pub trait GitWorktreeStatusReader {
    fn status(&self, repository_path: &str) -> Result<GitWorktreeChanges, String>;
    fn diff(&self, repository_path: &str, file_path: &str) -> Result<GitWorktreeFileDiff, String>;
}
```

- 기존 `GitHistoryReader`와 **별도 trait** (인터페이스 분리).
- 경로 문자열만 받는다. 앱별 식별자(repositoryId 등)는 소비 앱 facade가 경로로 변환.
- 오류는 `Err(String)` — 사용자 표시 가능한 메시지(`git_error_message` 규약).

**기본 구현**: `GitCliWorktreeStatusReader` (git_cli.rs)
- `status`: `git status --porcelain=v1 -uall` 파싱
- `diff`: `git diff -- <path>` → 비면 `git diff --cached -- <path>` 폴백. 둘 다 비면(untracked 등) "No textual git diff is available…" 안내 문구를 content로 반환(untracked 내용 diff 미생성 — 구 AW 동작 승계)
- diff 본문 상한 `MAX_WORKTREE_DIFF_BYTES = 120_000` bytes → 초과 시 잘라내고 `is_truncated: true`

## 2. Tauri 커맨드 — git-explorer

| 커맨드 | 요청 | 응답 | 위임 대상 |
|---|---|---|---|
| `get_worktree_status` | `{ repositoryId: string }` | `GitWorktreeChanges` | `WorktreeStatusService::get_worktree_status` (repositoryId→path 해석 후 reader.status) |
| `get_worktree_file_diff` | `{ repositoryId: string, filePath: string }` | `GitWorktreeFileDiff` | `WorktreeStatusService::get_worktree_file_diff` |

- 등록 위치: `apps/git-explorer/src-tauri/src/lib.rs`, 구현: `adapters/inbound/tauri_commands.rs:207-220`
- 존재하지 않는 repositoryId → `Err(String)` 오류 메시지.

**프론트 API 래퍼** (`apps/git-explorer/src/entities/repository/api.ts`):

```ts
getWorktreeStatus(repositoryId: string): Promise<GitWorktreeChanges>
getWorktreeFileDiff(repositoryId: string, filePath: string): Promise<GitWorktreeFileDiff>
// queryKeys: repositoryKeys.worktreeStatus(id), repositoryKeys.worktreeFileDiff(id, path)
```

## 3. Tauri 커맨드 — agentic-workbench

| 커맨드 | 요청 | 응답 | 위임 대상 |
|---|---|---|---|
| `get_worktree_changes` | `{ workingDirectory: string }` | `GitWorktreeChanges` | `git_worktree_changes_service::get_worktree_changes` (trim + 빈 값 거부 후 reader.status) |
| `get_worktree_file_diff` | `{ workingDirectory: string, path: string }` | `GitWorktreeFileDiff` | `git_worktree_changes_service::get_worktree_file_diff` |

- 등록 위치: `apps/agentic-workbench/src-tauri/src/lib.rs:91-92`, 서비스: `application/git_worktree_changes_service.rs`
- 입력 검증: `normalize_required` — trim 후 빈 문자열이면 `"{field} is required."` 오류.
- reader는 `git_core::GitCliWorktreeStatusReader`를 주입(기존 자체 provider 2종은 삭제됨).

## 4. 공유 뷰 계약 (packages/git-ui)

```ts
export type WorktreeChangesViewProps = {
  changes?: GitWorktreeChanges;        // undefined = 로딩/미조회
  selectedFilePath?: string;
  onSelectFile: (path: string) => void;
  diff?: GitWorktreeFileDiff;
  diffLoading?: boolean;
  diffError?: string | null;
  diffClassName?: string;              // diff 뷰어 컨테이너 클래스 오버라이드
};
export function WorktreeChangesView(props: WorktreeChangesViewProps): JSX.Element;
```

**렌더 계약**:
- 그룹 순서: `conflicted → staged → unstaged → untracked` (빈 그룹은 숨김)
- 헤더에 전체 변경 수 배지, 그룹별 파일 수 표시
- 각 파일: 2글자 상태 배지(X/Y), rename 시 `oldPath → path` 화살표
- 선택 파일 diff는 내부적으로 공유 `DiffViewer` 사용; `isBinary`/`isTruncated`/`diffError`/`diffLoading` 상태별 안내 렌더
- **비의존 제약**: Tauri API·react-query·앱 셸·라우팅에 의존하지 않는다. 데이터 페칭·상태 관리는 소비 앱 책임.

## 5. 앱 UI 통합 계약

| 앱 | 위치 | 동작 |
|---|---|---|
| git-explorer | `widgets/changes-panel/ui/ChangesPanel.tsx` | 상세 패널에 Commit / Working tree 토글. Working tree 모드에서 `WorktreeChangesView` 렌더. 그래프에서 커밋 선택 시 Commit 모드로 자동 복귀 |
| agentic-workbench | `ui/worktree-changes-panel.tsx` (리뷰 패널), `ui/worktree-workspace-panel.tsx` (Git 탭) | 두 위치 모두 `WorktreeChangesView` 렌더. Git 탭 상세 영역에 Commit / Working tree 토글 |

## 6. 호환성 규칙

- 이 계약(직렬화 필드명 포함)은 두 앱이 동시에 소비하므로, 필드 추가는 하위 호환(optional)으로만 하고 필드 제거·의미 변경은 양 앱 + git-graph 미러 + git-ui를 원자적으로 함께 갱신해야 한다(헌법 원칙 V).
