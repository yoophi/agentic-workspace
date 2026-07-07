# Contract: Commit Detail Path Display

## Scope

이 계약은 AW 커밋 상세 화면이 사용하는 Git commit detail 데이터의 파일 경로 표시 규칙을 정의한다. 외부 공개 API가 아니라 shared Git core, Tauri command, TypeScript mirror, shared Git UI 사이의 내부 계약이다.

## Producers

- `crates/git-core/src/git_cli.rs`
- `apps/agentic-workbench/src-tauri/src/infrastructure/git_cli_worktree_git_provider.rs`
- `apps/agentic-workbench/src-tauri/src/application/worktree_git_service.rs`
- `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`

## Consumers

- `apps/agentic-workbench/src/entities/worktree-git/api/worktree-git-repository.ts`
- `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`
- `packages/git-ui/src/ui/commit-detail-view.tsx`

## Data Contract

### `GitCommitDetail.files[*].path`

Required behavior:

- MUST be a user-displayable file path string.
- MUST preserve Korean characters in readable form.
- MUST NOT expose Git C-style octal byte escapes such as `\\355\\202\\244`.
- MUST remain usable as the selected path passed to the file diff request.
- MUST preserve existing English/number-only paths without meaningful display changes.

### `GitFileDiff.path`

Required behavior:

- MUST refer to the same file selected from `GitCommitDetail.files[*].path`.
- SHOULD be displayable using the same path readability rules as commit detail paths.

### Rename Paths

Required behavior:

- If the current data shape exposes only one path for renamed files, the exposed path MUST be readable.
- If a future or implementation-local shape exposes both previous and current paths, both paths MUST follow the same readability rules.

## Acceptance Contract Examples

| Source form | Expected display |
|-------------|------------------|
| `docs/한글.md` | `docs/한글.md` |
| `docs/\\355\\225\\234\\352\\270\\200.md` | `docs/한글.md` |
| `src/키오스크-test_01.ts` | `src/키오스크-test_01.ts` |
| `README.md` | `README.md` |

## Non-Goals

- Do not rename files.
- Do not modify Git history.
- Do not add a user-facing encoding preference.
- Do not change unrelated Git graph/history pagination behavior.
