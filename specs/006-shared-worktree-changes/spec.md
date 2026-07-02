# Feature Specification: 워킹 트리(미커밋) 변경사항 조회 기능의 공유 패키지화 및 양 앱 통합

**Feature Branch**: `006-shared-worktree-changes`

**Created**: 2026-07-02

**Status**: Draft

**Input**: User description: "워킹 트리(미커밋) 변경사항 조회 기능의 공유 패키지화 및 양 앱 통합 — agentic-workbench(AW)의 자체 git status/diff 파싱 로직을 공유 계층(git-core)으로 승격해 단일 정본으로 만들고, 공유 UI(WorktreeChangesView)를 추가하며, git-explorer(GE)에 미커밋 변경 조회 기능을 새로 탑재하고, AW의 자체 구현을 삭제해 공유 구현으로 대체한다."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - git-explorer에서 미커밋 변경사항 확인 (Priority: P1)

git-explorer 사용자가 로컬 저장소를 열었을 때, 커밋 히스토리뿐 아니라 아직 커밋하지 않은 워킹 트리 변경사항(스테이징됨/스테이징 안 됨/추적 안 됨/충돌)을 그룹별 목록으로 확인하고, 파일을 선택해 해당 파일의 미커밋 diff를 볼 수 있다.

**Why this priority**: 이번 작업에서 사용자에게 새로 제공되는 핵심 가치다. 기존 git-explorer는 커밋된 이력만 볼 수 있어, 사용자가 "지금 저장소가 어떤 상태인가"를 확인하려면 별도 도구(터미널 등)로 이탈해야 했다.

**Independent Test**: 미커밋 변경이 있는 저장소를 git-explorer에서 열고, 상세 패널을 Working tree 모드로 전환해 그룹별 파일 목록과 선택 파일의 diff가 표시되는지 확인하는 것만으로 독립적으로 검증 가능하다.

**Acceptance Scenarios**:

1. **Given** 스테이징된 파일 1개·수정된 파일 1개·새 파일 1개가 있는 저장소, **When** 사용자가 상세 패널을 Working tree 모드로 전환, **Then** Staged/Unstaged/Untracked 그룹에 각 파일이 분류되어 표시되고 총 변경 수 배지가 3으로 표시된다.
2. **Given** Working tree 모드에서 파일 목록이 표시된 상태, **When** 사용자가 파일 하나를 선택, **Then** 해당 파일의 미커밋 diff가 커밋 diff와 동일한 뷰어 형식으로 표시된다.
3. **Given** Working tree 모드가 활성화된 상태, **When** 사용자가 그래프에서 커밋을 선택, **Then** 상세 패널이 자동으로 Commit 모드로 전환되어 커밋 상세를 표시한다.
4. **Given** 미커밋 변경이 전혀 없는 깨끗한 저장소, **When** Working tree 모드로 전환, **Then** 변경 없음 상태가 명확히 안내된다.

---

### User Story 2 - agentic-workbench 기능 동등성 유지 (Priority: P2)

agentic-workbench 사용자가 워크트리 변경 리뷰 패널과 워크스페이스 Git 탭에서 기존과 동일하게 미커밋 변경사항을 확인할 수 있다. 내부 구현이 공유 계층으로 교체되어도 사용자 관점의 동작·정보량은 후퇴하지 않는다.

**Why this priority**: 마이그레이션의 무회귀 보장. AW는 이미 이 기능을 제공하고 있으므로, 공유 구현 채택이 기존 사용자 경험을 깨뜨리면 안 된다.

**Independent Test**: 마이그레이션 전후 동일한 저장소 상태에 대해 AW 리뷰 패널·Git 탭이 동일한 그룹핑·카운트·diff를 표시하는지 비교 검증할 수 있다.

**Acceptance Scenarios**:

1. **Given** 미커밋 변경이 있는 워크트리 세션, **When** 사용자가 워크트리 변경 리뷰 패널을 연다, **Then** 마이그레이션 이전과 동일한 그룹별 파일 목록과 파일 diff가 표시된다.
2. **Given** 워크스페이스 Git 탭이 열린 상태, **When** 사용자가 Commit/Working tree 토글을 전환, **Then** 커밋 상세와 미커밋 변경 뷰 사이를 오갈 수 있다.
3. **Given** 마이그레이션 완료 상태, **When** 기존 자동화 테스트(백엔드 테스트·타입 체크)를 실행, **Then** 전부 통과한다.

---

### User Story 3 - 두 앱에서 일관된 표시와 단일 정본 유지보수 (Priority: P3)

두 앱 사용자는 동일한 저장소 상태에 대해 동일한 그룹 순서(Conflicted → Staged → Unstaged → Untracked), 동일한 상태 배지, 동일한 rename 표기, 동일한 diff 형식을 본다. 유지보수자는 미커밋 조회 로직을 저장소 내 한 곳에서만 수정하면 두 앱에 동시에 반영된다.

**Why this priority**: 중복 구현 제거와 표시 일관성은 이번 작업의 구조적 목표이지만, 개별 앱의 기능 제공(P1)과 무회귀(P2)가 선행되어야 의미가 있다.

**Independent Test**: 동일 저장소를 두 앱으로 열어 그룹핑·카운트·배지·diff 표시가 일치하는지 비교하고, 조회 로직 구현 파일이 저장소 내에 정확히 한 곳(공유 계층)만 존재하는지 확인할 수 있다.

**Acceptance Scenarios**:

1. **Given** rename·수정·신규·충돌이 섞인 저장소 상태, **When** 두 앱에서 각각 미커밋 변경 뷰를 연다, **Then** 그룹 순서·파일 분류·카운트·상태 배지·rename 화살표 표기가 두 앱에서 동일하다.
2. **Given** 마이그레이션 완료 상태, **When** 저장소에서 미커밋 status/diff 파싱 구현을 검색, **Then** 공유 계층 한 곳에만 존재하고 앱별 중복 구현은 삭제되어 없다.

---

### Edge Cases

- 미커밋 변경이 전혀 없는 깨끗한 저장소 → 빈 상태 안내가 표시되어야 한다.
- 바이너리 파일이 변경된 경우 → 텍스트 diff 대신 바이너리 파일임을 안내해야 한다.
- diff가 설정된 크기 상한(120,000바이트)을 초과하는 경우 → 상한까지만 표시하고 잘렸음을 사용자에게 알린다.
- 파일이 rename된 경우 → 이전 경로 → 새 경로가 함께 표기되어야 한다.
- 병합 충돌 중인 파일 → Conflicted 그룹으로 최우선 노출되어야 한다.
- 스테이징된 변경만 있는 파일 → 스테이징 영역 기준 diff가 표시되어야 한다(작업 트리 diff가 비어 있어도 빈 화면이 아님).
- 유효하지 않은 저장소 경로 또는 git 명령 실패 → 오류 메시지가 사용자에게 표시되고 앱이 중단되지 않는다.
- diff 로딩 중 → 로딩 상태가 표시된다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 지정된 저장소 경로의 미커밋 변경 파일 목록을 조회하고, 각 파일을 Staged / Unstaged / Untracked / Conflicted 4개 그룹 중 하나로 분류해야 한다.
- **FR-002**: 미커밋 변경 뷰는 전체 변경 파일 수와 그룹별 파일 수를 배지로 표시해야 한다.
- **FR-003**: 사용자는 목록에서 파일을 선택해 해당 파일의 미커밋 diff를 볼 수 있어야 하며, diff는 기존 커밋 diff와 동일한 뷰어 형식으로 표시되어야 한다.
- **FR-004**: rename된 파일은 이전 경로와 새 경로를 화살표로 함께 표기해야 한다.
- **FR-005**: 각 파일에는 git 상태를 나타내는 2글자 상태 배지가 표시되어야 한다.
- **FR-006**: 바이너리 파일 diff 요청 시 텍스트 diff 대신 바이너리임을 안내해야 한다.
- **FR-007**: diff 크기가 상한(120,000바이트)을 초과하면 상한까지만 반환하고 잘림 여부를 함께 전달해, UI가 잘렸음을 표시할 수 있어야 한다.
- **FR-008**: git-explorer의 변경 상세 패널은 Commit 모드와 Working tree 모드 간 전환 토글을 제공해야 하며, 커밋을 선택하면 자동으로 Commit 모드로 복귀해야 한다.
- **FR-009**: agentic-workbench의 워크트리 변경 리뷰 패널과 워크스페이스 Git 탭은 공유 미커밋 변경 뷰를 사용해야 하며, 마이그레이션 이전 대비 기능 후퇴가 없어야 한다. Git 탭에도 Commit/Working tree 전환 토글을 제공해야 한다.
- **FR-010**: 미커밋 status/diff 조회 로직은 저장소 내 단일 공유 구현(정본)만 존재해야 하며, 앱별 중복 구현은 삭제되어야 한다.
- **FR-011**: 미커밋 변경 그룹의 표시 순서는 Conflicted → Staged → Unstaged → Untracked로 두 앱에서 동일해야 한다.
- **FR-012**: 본 기능은 읽기 전용이다. stage/unstage/discard/commit 등 저장소 상태를 변경하는 동작은 제공하지 않는다.
- **FR-013**: 저장소 경로가 유효하지 않거나 조회가 실패하면 오류를 사용자에게 표시하고 앱 동작은 계속되어야 한다.

### Key Entities

- **미커밋 변경 집합(GitWorktreeChanges)**: 한 저장소(작업 디렉터리)의 현재 미커밋 변경 전체. 변경 파일 목록을 가진다.
- **변경 파일(GitChangedFile)**: 개별 변경 파일. 경로, rename 시 이전 경로, 2글자 상태 코드, 소속 그룹을 가진다.
- **변경 그룹(GitChangedFileGroup)**: Staged / Unstaged / Untracked / Conflicted 4종의 분류 값.
- **미커밋 파일 diff(GitWorktreeFileDiff)**: 선택 파일의 미커밋 diff 내용. 커밋 diff와 필드 구조가 호환되어 동일 뷰어로 렌더링 가능하며, 잘림 여부·바이너리 여부를 포함한다. 커밋 해시는 갖지 않는다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 두 앱(agentic-workbench, git-explorer)이 소비하는 기능이므로 공유 계층 승격 요건(2개 앱 이상 소비)을 충족한다. 조회 로직은 `crates/git-core`, 타입 미러는 `packages/git-graph`, 공유 뷰는 `packages/git-ui`에 두고, 앱은 이를 소비만 한다. 앱 간 직접 import는 없다.
- **Frontend layering**: GE는 `entities/repository`(API 어댑터·쿼리 키)와 `widgets/changes-panel`(화면 조립)에서 소비한다. AW는 `entities/project`(API·모델)와 해당 패널 UI에서 소비한다. 공유 뷰는 데이터·콜백을 props로 주입받는 표현 전용 컴포넌트로 유지한다.
- **Backend boundary**: 도메인 모델(변경 집합·파일·그룹·diff)과 포트(`GitWorktreeStatusReader`)는 git-core domain/ports에, git CLI 어댑터는 git-core infrastructure 성격의 `git_cli`에 둔다. 포트는 기존 `GitHistoryReader`와 분리한다. 앱의 Tauri 커맨드는 입력 검증 후 application 서비스(facade: repositoryId→경로 변환)에 위임하며 비즈니스 로직을 갖지 않는다.
- **Shared core vs UI**: 순수 코어(도메인 타입 + porcelain 파싱 + 테스트)를 먼저 공유하고, UI는 두 앱의 요구(그룹 목록 + diff 뷰)가 이미 수렴했고 기존 공유 `DiffViewer`가 존재하므로 `WorktreeChangesView`로 공유한다. 공유 뷰는 앱 셸·Tauri 커맨드·라우팅 상태에 의존하지 않는다.
- **Persistence and safety**: 영속화 없음(조회 전용). 저장소 경로는 각 앱의 facade/서비스 경계에서 검증하고, diff 응답에 크기 상한(120,000바이트)을 적용해 과대 응답을 차단한다.
- **Documentation and Storybook**: 공유 컴포넌트 `WorktreeChangesView`는 Storybook 등록 대상이다(로딩·빈 상태·오류·대용량 잘림 상태 포함). 필요 시 `docs/`에 공유 Git 계층 구조 문서를 갱신한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: git-explorer 사용자가 저장소를 연 뒤 두 번 이하의 조작(모드 전환 → 파일 선택)으로 미커밋 파일의 diff를 확인할 수 있다.
- **SC-002**: 동일한 저장소 상태에 대해 두 앱이 표시하는 그룹 분류·파일 수·상태 배지·rename 표기가 100% 일치한다.
- **SC-003**: agentic-workbench의 기존 미커밋 변경 관련 사용자 시나리오가 마이그레이션 후에도 전부 동작하며, 두 앱의 자동화 검증(백엔드 테스트, 타입 체크)이 모두 통과한다.
- **SC-004**: 미커밋 status/diff 파싱 구현이 저장소 내 정확히 1곳에만 존재한다(앱별 중복 구현 0개).
- **SC-005**: 120,000바이트를 초과하는 대용량 diff에서도 뷰가 응답성을 잃지 않고, 잘림 안내가 표시된다.

## Assumptions

- 대상 저장소는 로컬 git 저장소이며, 시스템에 git CLI가 설치되어 있다.
- 본 기능은 조회 전용이다. 저장소 상태를 바꾸는 쓰기 동작(stage/unstage/discard 등)은 후속 과제로 명시적으로 범위 밖이다.
- diff 렌더링 방식 자체는 변경하지 않으며, 기존 공유 diff 뷰어를 재사용한다.
- 미커밋 변경의 실시간 자동 갱신(파일 시스템 감시)은 본 범위에 포함하지 않는다. 기존 앱별 갱신 정책(수동/기존 자동 갱신)을 따른다.
- submodule 내부 변경의 세분화 표시는 고려하지 않는다(git 기본 출력 수준으로 표시).
- markdown-annotator 앱은 이 기능의 소비자가 아니다.
- diff 크기 상한 120,000바이트는 UI 응답성을 위한 합리적 기본값으로 채택한다.
