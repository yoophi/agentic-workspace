# Contract: Worktree Session Files 파일트리 조회

## Scope

이 계약은 `apps/agentic-workbench` Worktree Session 화면의 Files 섹션에서 파일트리 항목을 표시하고 선택 파일 preview를 조회하는 사용자-visible 동작을 정의한다.

## UI Contract

### File tree loading

- Files 섹션은 worktree root의 직계 항목을 먼저 표시한다.
- 사용자가 디렉터리를 펼치면 해당 디렉터리의 직계 항목을 추가로 표시한다.
- 하위 항목의 `relativePath`는 항상 worktree root 기준 전체 상대 경로여야 한다.
- 이미 로드된 항목은 중복 표시하지 않는다.

### Row selection

- 디렉터리 row 클릭은 펼침/접힘 상태만 변경한다.
- 파일 row 클릭은 `selectedFilePath`를 row의 전체 `relativePath`로 설정한다.
- 선택 표시와 preview header는 같은 `selectedFilePath`를 기준으로 갱신되어야 한다.
- 같은 `name`을 가진 파일이 여러 개 있어도 `relativePath`로 구분한다.

### Preview state

- 선택 파일이 없으면 empty 상태를 표시한다.
- 파일 선택 직후 loading 상태를 표시한다.
- 파일 조회 성공 시 content, `relativePath`, size, truncated 상태를 표시한다.
- 파일 조회 실패 시 error 상태를 표시하고 이전 파일 content를 새 결과처럼 보여주지 않는다.
- 선택 파일이 현재 로드된 목록에서 사라졌고 parent directory가 로드되어 있으면 stale 상태를 표시한다.

## Tauri Command Contract

### `list_worktree_files`

**Input**:
- `workingDirectory`: 현재 worktree root
- `scope.kind`: `all` 또는 `markdown`
- `scope.dir`: 선택적으로 조회 시작 디렉터리의 root 기준 상대 경로
- `scope.depth`: 선택적으로 조회 depth. Files lazy loading에서는 `1`을 사용한다.

**Output**:
- `WorktreeFileEntry[]`
- 각 entry는 `name`, `path`, `relativePath`, `isDir`, `size`, `modifiedMs`를 가진다.

**Rules**:
- `scope.dir`가 있으면 해당 디렉터리 하위만 조회한다.
- 반환 entry의 `relativePath`는 root 기준 전체 경로다.
- absolute path, parent-dir path, worktree 밖 path는 거부한다.

### `read_worktree_text_file`

**Input**:
- `workingDirectory`: 현재 worktree root
- `path`: 읽을 파일의 root 기준 상대 경로

**Output**:
- `WorktreeTextFile`
- `path`, `relativePath`, `content`, `size`, `truncated`를 가진다.

**Rules**:
- `path`는 파일 row의 전체 `relativePath`와 같아야 한다.
- 디렉터리는 preview 대상이 아니다.
- non-UTF-8/binary 파일은 기존 preview 불가 오류로 처리한다.
- absolute path, parent-dir path, worktree 밖 path는 거부한다.

## Required Fixtures

- `README.md`: root-level file
- `src/app.ts`: one-level nested file
- `src/deep/inner.ts`: two-level nested file
- `docs/app.ts`: same basename as another path
- `docs/한글 파일.md`: Korean and space path
- `missing/deleted.ts`: stale or read-failure scenario

## Acceptance Mapping

- FR-001, FR-002, FR-005: file tree loading and row selection
- FR-003, FR-006: `relativePath` identity and special path fixtures
- FR-004: root-level fixture regression
- FR-007, FR-009: preview state contract
- FR-008, FR-011: quickstart release-path verification
- FR-010: scope remains Files section only
