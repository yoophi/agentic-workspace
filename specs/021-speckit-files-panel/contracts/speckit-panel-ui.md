# Contract: Speckit Files Panel

## Scope

이 계약은 `apps/agentic-workbench` Worktree Session 화면의 Speckit 패널에서 기능 목록, 문서 선택, tasks 진행 요약, 빈/오류 상태, 파일 변경 갱신을 사용자에게 어떻게 제공해야 하는지 정의한다.

## UI Contract

### Panel entry

- Worktree Session의 workspace panel tab 목록에는 `Speckit` 항목이 표시된다.
- Speckit tab은 Git, Files, Markdown tab과 같은 수준의 주요 패널이다.
- tab 전환은 기존 worktree session state를 깨뜨리지 않아야 한다.

### Feature list

- 패널은 현재 worktree root 기준 `specs/*` 아래 기능 폴더를 표시한다.
- 표시 대상 feature는 하나 이상의 Speckit markdown 문서를 가진다.
- 각 feature row는 기능 이름과 root 기준 상대 경로를 구분 가능하게 보여준다.
- 긴 기능 이름 또는 긴 경로는 다른 row나 action을 밀어내지 않아야 한다.

### Document grouping

- core 문서: `spec.md`, `plan.md`, `tasks.md`, `research.md`, `data-model.md`, `quickstart.md`
- contract 문서: `contracts/*.md`, `contracts/**/*.md`
- checklist 문서: `checklists/*.md`, `checklists/**/*.md`
- 문서 row는 type label, file name, relative path를 구분 가능하게 표시한다.
- 존재하지 않는 문서는 비활성 row로 만들지 않고 목록에서 생략한다.

### Document selection

- 사용자가 문서 row를 선택하면 해당 root 기준 `relativePath`가 기존 markdown viewer 흐름의 선택 문서가 된다.
- 같은 basename을 가진 문서도 feature path와 relative path로 구분한다.
- 선택한 문서가 삭제되었거나 읽을 수 없으면 사용자에게 오류 또는 stale 상태를 보여주고 이전 문서를 새 결과처럼 표시하지 않는다.

### Task progress

- feature에 `tasks.md`가 있으면 checkbox task count를 기반으로 완료/전체/미완료 요약을 표시한다.
- checkbox task가 없으면 숫자 진행률 대신 작업 항목 없음 상태를 표시한다.
- `tasks.md`가 없으면 tasks 요약 없음 상태를 표시한다.
- 요약 계산 실패는 해당 feature의 tasks summary 오류로 제한한다.

### Empty and error states

- `specs` 디렉터리가 없으면 Speckit 문서 없음 empty state를 표시한다.
- `specs` 디렉터리는 있지만 표시 가능한 문서가 없으면 같은 empty state를 표시한다.
- 전체 목록 조회 실패는 panel-level error state로 표시한다.
- 개별 문서 읽기 실패는 document-level error state로 표시한다.

### Refresh behavior

- worktree file change event가 들어오면 Speckit 목록과 task summary query는 기존 Files/Markdown 갱신 정책과 일관되게 stale 또는 refetch 처리된다.
- 사용자는 명시적 refresh action을 통해 목록을 다시 불러올 수 있다.
- active tab이 Speckit일 때는 file change 후 최신 목록이 빠르게 보이도록 한다.

## Data Contract

### Input from file listing

- `workingDirectory`: 현재 worktree root
- `scope.dir`: `specs`
- `scope.kind`: markdown 문서 중심 조회
- returned entry identity: root 기준 `relativePath`

### Output to UI model

- `SpeckitFeature[]`
- 각 feature는 `documents`, optional `taskProgress`, `status`를 가진다.
- 모든 경로는 worktree root 기준 상대 경로다.

## Acceptance Mapping

- FR-001: Panel entry
- FR-002, FR-003, FR-004, FR-006: Feature list and document grouping
- FR-005: Document selection
- FR-007, FR-008: Task progress
- FR-009, FR-011: Empty and error states
- FR-010: Refresh behavior
- FR-012: Long list and long path layout
- FR-013: Storybook and test state coverage
