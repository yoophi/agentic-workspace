# Data Model: AW Worktree Session Files 하위 디렉터리 조회 수정

## Worktree Session

사용자가 특정 worktree에서 agent 작업 결과와 파일을 검토하는 화면 단위다.

**Fields**:
- `worktree.path`: 파일 목록과 파일 preview 조회의 root path
- `run/session identity`: 화면이 어느 작업 세션을 표시하는지 구분하는 식별 정보
- `workspace tabs`: Files, Markdown, Git 등 session page 안의 탐색 영역

**Validation rules**:
- Files 조회는 현재 `worktree.path` 안으로 제한되어야 한다.
- session 외부 경로나 다른 worktree의 파일을 조회하면 안 된다.

## Files Tree Node

Files 섹션 파일트리에 표시되는 파일 또는 디렉터리 항목이다.

**Fields**:
- `name`: 파일트리에 표시할 마지막 path segment
- `path`: filesystem absolute path 또는 provider가 반환한 실제 path
- `relativePath`: worktree root 기준 상대 경로. 파일 선택과 조회의 주 식별자
- `isDir`: 디렉터리 여부
- `size`: 파일 크기. 디렉터리는 0
- `modifiedMs`: 수정 시각
- `depth`: UI row indent 계산용 depth
- `isExpanded`: 디렉터리 펼침 상태

**Relationships**:
- `relativePath` segment로 parent/child 관계를 구성한다.
- 디렉터리 node는 lazy loading query의 `dir` 값으로 사용될 수 있다.
- 파일 node는 `Selected File`이 될 수 있다.

**Validation rules**:
- `relativePath`는 root 기준 전체 경로를 유지해야 하며 basename만 남기면 안 된다.
- 같은 `relativePath`는 중복 표시하지 않는다.
- 같은 `name`을 가진 파일도 `relativePath`가 다르면 서로 다른 파일이다.

## Selected File

사용자가 마지막으로 선택한 파일트리 파일 항목이다.

**Fields**:
- `selectedFilePath`: 선택한 파일의 root 기준 `relativePath`
- `selectedFile`: loaded entries 중 `selectedFilePath`와 일치하는 파일 node
- `staleFileSelection`: parent directory가 로드된 상태에서 선택 path가 더 이상 entries에 없을 때의 stale 상태

**State transitions**:
- `none` → `loading`: 사용자가 파일 node를 선택한다.
- `loading` → `loaded`: 선택한 path의 text file preview가 성공한다.
- `loading` → `error`: 파일을 읽을 수 없거나 지원하지 않는다.
- `loaded` → `loading`: 사용자가 다른 파일을 선택한다.
- `loaded/error` → `stale`: 선택한 파일의 parent directory가 로드되어 있고 파일이 목록에서 사라졌다.

**Validation rules**:
- 디렉터리 node를 선택해 파일 preview를 요청하면 안 된다.
- 파일 선택 시 `selectedFilePath`는 row의 `relativePath` 전체 값을 사용해야 한다.
- preview 결과의 `relativePath`는 `selectedFilePath`와 일치해야 한다.

## File Viewer State

선택 파일 preview 영역의 사용자-visible 상태다.

**States**:
- `empty`: 선택된 파일 없음
- `loading`: 선택 파일을 읽는 중
- `loaded`: 선택 파일 content, size, truncated status 표시
- `error`: 선택 파일을 preview할 수 없음
- `stale`: 선택 파일이 현재 로드된 파일 목록에 없음

**Validation rules**:
- 새 파일 선택 후 이전 파일 내용이 새 선택 결과처럼 유지되면 안 된다.
- error/stale 상태 이후 정상 파일을 선택하면 loaded 상태로 회복되어야 한다.
- error message는 Files 섹션 안에 한정되고 session page 전체를 중단시키면 안 된다.

## Worktree Text File

Tauri backend가 반환하는 preview 가능한 text file 결과다.

**Fields**:
- `path`: 실제 파일 path
- `relativePath`: worktree root 기준 상대 경로
- `content`: UTF-8 text preview content
- `size`: 전체 파일 크기
- `truncated`: preview가 크기 제한으로 잘렸는지 여부

**Validation rules**:
- `relativePath`는 `Selected File.selectedFilePath`와 일치해야 한다.
- binary 또는 non-UTF-8 파일은 기존 정책에 따라 error 상태로 표시한다.
- root escape, absolute path, parent-dir path는 거부되어야 한다.
