# Feature Specification: Worktree Auto Refresh

**Feature Branch**: `001-worktree-auto-refresh`

**Created**: 2026-07-01

**Status**: Draft

**Input**: User description: "worktree session page 에서, 파일이 변경된 것 및 커밋, 브랜치 등이 변경된 것을 감지하여 file tree pane, git explorer pane 목록이 자동 갱신되도록 하려고 합니다. 추가사항: auto reload 기능은 agentic-workbench 뿐 아니라, git-explorer, markdown-annotator 에도 동시에 적용되어야 합니다."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - File tree reflects worktree file changes (Priority: P1)

사용자가 `agentic-workbench` worktree session page, `git-explorer`, 또는
`markdown-annotator`를 열어 둔 상태에서 agent, 사용자의 editor, 또는 Git operation이
파일을 추가, 삭제, 이름 변경, 수정하면 각 앱의 file tree, markdown document, 또는
파일 기반 preview가 자동으로 최신 상태를 표시한다.

**Why this priority**: file tree는 현재 작업 내용을 탐색하는 기본 entry point다. 파일
목록이 낡으면 사용자는 수동 새로고침을 반복하거나 실제 worktree 상태를 오해하게 된다.

**Independent Test**: 세 앱 각각에서 대상 repository/worktree/markdown file을 연 뒤
외부에서 파일을 추가/삭제/수정하고, 사용자가 새로고침 버튼이나 페이지 reload 없이
목록, preview, 선택 상태 변화를 확인한다.

**Acceptance Scenarios**:

1. **Given** file tree pane이 열린 worktree session 또는 git-explorer repository,
   **When** 새 파일이 worktree/repository에 생성됨, **Then** file tree pane은 해당
   파일을 자동으로 표시한다.
2. **Given** file tree pane에서 파일이 선택되어 있음, **When** 선택된 파일이 삭제됨,
   **Then** file tree pane은 삭제된 파일을 제거하고 선택 불가 상태를 명확히 표시한다.
3. **Given** file tree pane 또는 markdown-annotator에서 파일 내용 preview가 표시 중임,
   **When** 해당 파일 내용이 변경됨, **Then** preview는 최신 내용을 자동으로 표시하되
   사용자의 탐색 맥락을 불필요하게 초기화하지 않는다.

---

### User Story 2 - Git explorer reflects commit and branch changes (Priority: P1)

사용자가 `agentic-workbench` 또는 `git-explorer`를 열어 둔 상태에서 commit, branch,
ref, worktree dirty 상태가 바뀌면 두 앱의 git explorer pane, commit list, graph,
status summary가 자동으로 최신 상태를 표시한다.

**Why this priority**: agent 작업과 사용자의 Git 조작은 session 중 자주 발생한다. commit
list나 branch 정보가 낡으면 사용자는 잘못된 commit detail을 기준으로 판단할 수 있다.

**Independent Test**: `agentic-workbench`와 `git-explorer`에서 Git pane을 연 뒤 외부에서
commit 생성, branch 전환, branch 생성, 변경 파일 생성 중 하나를 수행하고 UI가 수동
새로고침 없이 최신 정보를 보여주는지 확인한다.

**Acceptance Scenarios**:

1. **Given** git explorer pane이 열린 worktree session 또는 git-explorer repository,
   **When** 새 commit이 생성됨, **Then** commit list와 graph에 새 commit이 자동으로
   나타난다.
2. **Given** git explorer pane이 열린 worktree session, **When** 현재 branch가 변경됨,
   **Then** branch/ref 표시와 commit list 기준이 새 branch 상태와 일치한다.
3. **Given** commit detail이 표시 중임, **When** 선택한 commit이 여전히 존재함,
   **Then** 자동 갱신 후에도 해당 commit detail 선택이 유지된다.

---

### User Story 3 - Automatic refresh does not interrupt active review (Priority: P2)

사용자가 file tree, markdown preview, commit list, commit detail을 검토 중일 때 자동
갱신은 최신 상태를 반영하지만 현재 읽고 있는 맥락을 가능한 한 유지한다.

**Why this priority**: 자동 갱신이 너무 공격적이면 사용자가 리뷰 중인 파일/commit이
갑자기 사라지거나 scroll/selection이 초기화되어 작업 흐름을 방해한다.

**Independent Test**: 사용자가 특정 파일 또는 commit을 선택한 상태에서 관련 없는 파일
변경이나 새 commit 생성을 발생시키고, 가능한 selection/scroll/context가 유지되는지
확인한다.

**Acceptance Scenarios**:

1. **Given** 사용자가 file tree에서 유지되는 파일을 선택 중임, **When** 다른 파일이
   변경됨, **Then** 선택된 파일과 preview는 유지되고 목록만 최신화된다.
2. **Given** 사용자가 commit detail을 보고 있음, **When** 새 commit이 추가됨, **Then**
   기존 선택 commit이 유효하면 detail pane은 유지되고 목록/graph만 갱신된다.
3. **Given** 선택한 파일이나 commit이 더 이상 존재하지 않음, **When** 자동 갱신이
   완료됨, **Then** UI는 stale selection을 해제하고 사용자가 다음 선택을 할 수 있게
   명확한 상태를 보여준다.

---

### User Story 4 - Refresh state is visible and recoverable (Priority: P3)

사용자는 자동 갱신이 진행 중인지, 마지막 갱신이 실패했는지, 실패 후 다시 시도할 수
있는지를 worktree session page에서 확인할 수 있다.

**Why this priority**: filesystem이나 Git 상태 조회가 실패할 수 있으므로 사용자는 현재
보는 정보가 최신인지 판단할 수 있어야 한다.

**Independent Test**: worktree 접근 실패 또는 Git 상태 조회 실패 상황을 만들고, UI가
기존 정보를 무리하게 지우지 않으면서 실패 상태와 재시도 경로를 표시하는지 확인한다.

**Acceptance Scenarios**:

1. **Given** 자동 갱신 중인 pane, **When** 사용자가 pane을 보고 있음, **Then** UI는
   현재 작업을 가리지 않는 방식으로 갱신 진행 상태를 표시한다.
2. **Given** 자동 갱신이 실패함, **When** 사용자가 pane을 확인함, **Then** UI는 마지막
   성공 상태를 유지하고 실패 원인과 재시도 가능성을 표시한다.

### Edge Cases

- 짧은 시간 안에 많은 파일 변경이나 여러 Git 변경이 연속으로 발생한다.
- 선택 중인 파일이 삭제되거나 이름이 변경된다.
- 선택 중인 commit이 rebase, reset, branch 변경으로 더 이상 현재 history에 없다.
- worktree가 삭제되었거나 접근 권한이 사라진다.
- Git repository가 일시적으로 lock 상태이거나 command가 실패한다.
- 파일 tree와 Git explorer가 서로 다른 시점의 상태를 받는다.
- 사용자가 자동 갱신 도중 수동 새로고침이나 pane 전환을 수행한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST detect relevant file changes in the active worktree,
  active repository, or active markdown document while `agentic-workbench`,
  `git-explorer`, or `markdown-annotator` is open.
- **FR-002**: System MUST automatically refresh file tree data after detected
  file additions, deletions, renames, and content changes.
- **FR-003**: System MUST automatically refresh file preview data when the
  currently selected file changes and remains readable.
- **FR-004**: System MUST detect relevant Git state changes in the active
  worktree, including commit history, branch/ref, and dirty status changes.
- **FR-005**: System MUST automatically refresh git explorer data after detected
  commit, branch, ref, or worktree status changes.
- **FR-006**: System MUST preserve the selected file, selected commit, and visible
  review context when the selected item still exists after refresh.
- **FR-007**: System MUST clear or mark stale selections when a selected file or
  commit no longer exists after refresh.
- **FR-008**: System MUST avoid duplicate or excessive visible refreshes when many
  changes occur in a short period.
- **FR-009**: System MUST expose a visible non-blocking refresh state for panes
  that are currently updating.
- **FR-010**: System MUST show recoverable error feedback when automatic refresh
  fails and MUST keep the last successful data visible when possible.
- **FR-011**: System MUST keep existing manual refresh behavior available as a
  fallback.
- **FR-012**: System MUST scope refresh behavior to the active worktree session
  and MUST NOT update panes for unrelated worktrees.
- **FR-013**: System MUST apply the auto reload behavior consistently to
  `apps/agentic-workbench`, `apps/git-explorer`, and `apps/markdown-annotator`.
- **FR-014**: System MUST share pure refresh policy/state logic through
  workspace packages when the same behavior is used by more than one app, while
  keeping app-specific Tauri commands and UI adapters local to each app.

### Key Entities

- **Worktree Session**: 현재 사용자가 열어 둔 worktree 작업 화면과 해당 worktree path,
  선택된 pane, file selection, commit selection을 포함하는 사용자 맥락.
- **File Tree State**: worktree 내 표시 가능한 파일/디렉터리 목록, 선택된 파일,
  preview 상태, 마지막 성공 갱신 시점, 오류 상태.
- **Git Explorer State**: worktree의 dirty summary, branch/ref 상태, commit list,
  commit graph, 선택된 commit/detail 상태, 마지막 성공 갱신 시점, 오류 상태.
- **Refresh Event**: file 또는 Git 상태가 바뀌었음을 나타내며 특정 worktree session에
  귀속되는 갱신 필요 신호.
- **Application Refresh Consumer**: `agentic-workbench`, `git-explorer`,
  `markdown-annotator` 중 하나의 앱에서 refresh policy를 소비하는 UI와 adapter 묶음.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 대상은 `apps/agentic-workbench`, `apps/git-explorer`,
  `apps/markdown-annotator` 세 앱이다. 세 앱이 공유하는 refresh policy/state helper는
  `packages/*`에 두고, app shell/Tauri command/UI adapter는 각 `apps/*`에 둔다.
- **Frontend layering**: `agentic-workbench` 화면 조립은 `pages/project-worktree-session`,
  사용자 상호작용과 pane state는 `features/worktree-workspace`, worktree file/Git 데이터
  모델과 adapters는 `entities/worktree-file` 및 `entities/worktree-git` 범위로 둔다.
  `git-explorer`와 `markdown-annotator`는 각 앱의 기존 FSD/feature 경계를 유지한다.
- **Backend boundary**: worktree file/Git 상태 관찰이나 조회 확장이 필요하면 Tauri
  command는 `inbound`, use case는 `application`, 순수 모델/port는 `domain` 또는 `ports`,
  filesystem/Git adapter는 `infrastructure`에 둔다.
- **Shared core vs UI**: 이 기능은 자동 갱신 orchestration이 핵심이므로 pure refresh
  policy/state helper를 먼저 공유한다. 공유 UI는 세 앱의 표시 요구가 수렴한 경우에만
  별도 검토한다. Git/Markdown 공유 패키지는 기존 소비 경계를 유지한다.
- **Persistence and safety**: 파일/Git 상태는 active worktree path에만 scope된다. 파일
  접근은 기존 root/path 검증과 size/UTF-8 안전 규칙을 유지해야 한다.
- **Documentation and Storybook**: worktree session workspace 설계 문서에 자동 갱신
  동작을 추가하고, 가능한 경우 workspace pane의 loading/error/stale-selection 상태를
  Storybook에서 확인 가능하게 한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 파일 생성, 삭제, 수정 후 사용자는 페이지 reload나 수동 refresh 없이
  3초 이내에 `agentic-workbench`, `git-explorer`, `markdown-annotator`의 관련 파일 목록
  또는 preview에서 변경을 확인할 수 있다.
- **SC-002**: 새 commit 생성 또는 branch 변경 후 사용자는 페이지 reload나 수동 refresh
  없이 3초 이내에 `agentic-workbench`와 `git-explorer`의 git explorer pane에서 변경을
  확인할 수 있다.
- **SC-003**: 세 앱은 같은 refresh interval, stale selection, last-successful-data 유지
  정책을 공유 helper 기준으로 일관되게 적용한다.
- **SC-005**: 갱신 실패 시 사용자는 실패 상태와 재시도 가능성을 확인할 수 있고, 마지막
  성공 데이터가 유지되는지 여부를 명확히 알 수 있다.

## Assumptions

- 자동 갱신은 각 앱에서 관련 repository/worktree/markdown file이 열려 있는 동안에만
  필요하다.
- 변경 감지는 active worktree path, selected repository, selected markdown file에
  한정되며 다른 worktree/repository/document를 갱신하지 않는다.
- markdown file tree는 file tree 변경 감지의 영향을 함께 받는다.
- 자동 갱신은 사용자의 현재 선택을 최대한 유지하되, 더 이상 존재하지 않는 대상은
  stale 상태로 처리한다.
- 실시간성의 목표는 협업 편집 수준이 아니라 local worktree review에 충분한 짧은 지연이다.
