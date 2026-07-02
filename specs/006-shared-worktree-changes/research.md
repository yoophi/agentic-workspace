# Research: 워킹 트리(미커밋) 변경사항 조회 공유화

**Feature**: 006-shared-worktree-changes | **Date**: 2026-07-02

본 문서는 구현 과정에서 실제로 내려진 기술 결정을 회고적으로 정리한다. Technical Context에 NEEDS CLARIFICATION 항목은 없으며, 모든 결정은 커밋된 코드로 검증 가능하다.

## R1. 정본 위치: git-core로 승격 (AW 구현을 이관)

- **Decision**: AW의 자체 `git status`/`git diff` 파싱 로직을 `crates/git-core`로 이관하고, AW 자체 구현(domain provider + infrastructure provider 2개 파일)은 삭제한다.
- **Rationale**: GE가 동일 기능을 필요로 하게 되어 소비 앱이 2개가 됨 → 헌법 원칙 I의 공유 승격 요건 충족. git-core는 이미 history/graph/detail/diff의 공유 정본이므로 worktree status/diff도 같은 crate에 두는 것이 응집도가 높다.
- **Alternatives considered**:
  - *AW 구현 유지 + GE에 복제*: 파싱 로직 중복, 그룹핑 규칙 드리프트 위험 → 기각.
  - *별도 신규 crate(git-worktree-core)*: 소비자·주제(git CLI 파싱)가 동일한데 crate만 분리하면 워크스페이스 관리 비용만 증가 → 기각.

## R2. 포트 분리: GitWorktreeStatusReader를 GitHistoryReader와 별도 trait으로

- **Decision**: `GitWorktreeStatusReader { status(path), diff(path, file_path) }`를 기존 `GitHistoryReader`에 합치지 않고 별도 포트로 정의한다.
- **Rationale**: 관심사가 다르다(커밋 이력 조회 vs 현재 작업 트리 상태). AW는 worktree status만 필요하고 history 포트 구현을 강제받으면 안 된다(인터페이스 분리 원칙). 테스트 시 FakeReader 구현 부담도 최소화된다.
- **Alternatives considered**: *GitHistoryReader에 메서드 추가* — 기존 구현체·기존 소비자 전부에 불필요한 메서드가 전파됨 → 기각.

## R3. 식별자 규약: path 기반 포트 + 앱 facade에서 변환

- **Decision**: 포트는 `working_directory`(경로) 문자열만 받는다. GE의 `repositoryId` 같은 앱별 식별자는 앱의 application 계층 facade(`WorktreeStatusService`)가 경로로 변환해 전달한다.
- **Rationale**: git-core가 앱별 저장소 레지스트리 개념에 오염되지 않는다(도메인 순수성). 기존 `GitHistoryReader`와 동일한 규약이라 일관성 유지.
- **Alternatives considered**: *포트가 repositoryId를 받고 resolver를 주입* — git-core에 앱 개념 유입, 두 앱의 식별자 체계가 달라 일반화 곤란 → 기각.

## R4. status 파싱: `git status --porcelain=v1 -uall`

- **Decision**: porcelain v1 형식을 파싱하고 `-uall`로 untracked 파일을 디렉터리 축약 없이 개별 나열한다. XY 2글자 코드에서 staged/unstaged 상태를 분리 보관하고, 그룹은 conflicted(U 계열/DD/AA) > staged > unstaged > untracked 규칙으로 판정한다.
- **Rationale**: porcelain v1은 안정된 스크립팅 계약이고 AW 기존 구현이 이미 검증한 방식. `-uall`이 없으면 새 디렉터리가 `dir/` 한 줄로 축약되어 파일 단위 UI를 만들 수 없다.
- **Alternatives considered**:
  - *porcelain v2*: 정보량은 많지만 필요 필드(XY·rename·경로) 기준 이득이 없고 파서 복잡도만 증가 → 기각.
  - *libgit2(git2 crate) 사용*: 시스템 git 의존 제거 이점은 있으나 git-core 전체가 이미 git CLI 기반(GitCliHistoryReader)이라 혼합 시 동작 불일치 위험 → 기각.

## R5. diff 조회: `git diff` + `--cached` 폴백

- **Decision**: 파일 diff는 먼저 `git diff -- <path>`(작업 트리 vs index)를 시도하고, 출력이 비면 `git diff --cached -- <path>`(index vs HEAD)로 폴백한다. 두 조회 모두 비면(untracked 파일 또는 무변경) "No textual git diff is available…" 안내 문구를 content로 반환한다 — untracked 파일의 내용 diff(`/dev/null` 대비)는 생성하지 않으며, 이는 구 AW 구현과 동일한 동작이다(무회귀).
- **Rationale**: 스테이징만 된 파일은 작업 트리 diff가 비므로, 폴백 없이는 "변경 있음으로 목록에 뜨는데 diff는 빈 화면"이 된다(spec edge case). 사용자 기대는 "이 파일에서 무엇이 바뀌었나"이므로 두 단계 조회가 맞다. untracked 내용 diff 생성은 기존 AW에도 없던 기능이라 마이그레이션 범위에서 제외했다(후속 개선 여지).
- **Alternatives considered**: *그룹에 따라 diff 명령 분기(호출자가 staged 여부 전달)* — API 표면이 넓어지고 호출자가 git 지식을 가져야 함 → 폴백 캡슐화로 기각.

## R6. diff 응답 상한: MAX_WORKTREE_DIFF_BYTES = 120,000

- **Decision**: diff 본문을 120,000바이트에서 자르고 `is_truncated: true`를 함께 반환한다. 바이너리는 diff 생성 대신 `is_binary: true`로 반환한다.
- **Rationale**: Tauri IPC로 수 MB 문자열을 넘기면 직렬화·렌더링 모두에서 UI가 멈춘다. AW 기존 구현의 검증된 상한값을 그대로 승계해 마이그레이션 무회귀를 보장한다.
- **Alternatives considered**: *페이지네이션/스트리밍 diff* — 조회 전용 1차 범위에 과설계. 상한+잘림 안내로 충분 → 기각(후속 과제 여지).

## R7. GitWorktreeFileDiff: 커밋 GitFileDiff와 필드 정렬, commit_hash 제거

- **Decision**: 미커밋 diff 타입을 신설하되 필드(`path`, `content`, `is_binary`, `is_truncated`)를 커밋 `GitFileDiff`와 정렬하고 `commit_hash`만 뺀다. AW의 기존 자체 형태 `{diff, truncated, binary}`는 이 타입으로 교체한다.
- **Rationale**: 공유 `DiffViewer`가 커밋 diff와 미커밋 diff를 동일 계약으로 소비할 수 있어 뷰어 분기가 사라진다. `commit_hash`를 Option으로 남기는 것보다 타입 분리가 의미상 명확하다.
- **Alternatives considered**: *GitFileDiff 재사용(commit_hash를 Option화)* — 기존 커밋 diff 소비자 전부에 Option 처리 전파 → 기각.

## R8. TS 타입 전달: git-graph에 수동 미러링

- **Decision**: Rust 도메인 타입을 `packages/git-graph/src/types.ts`에 수동으로 미러링한다(serde `rename_all = "camelCase"`와 필드명 정합). 앱 프론트 타입은 `@yoophi/git-graph`에서 re-export.
- **Rationale**: 기존 git-graph 타입들(commit/graph)과 동일한 관례. 타입 수가 4개로 작아 수동 동기화 비용이 낮다.
- **Alternatives considered**: *ts-rs/specta 등 코드 생성* — 도구 도입 비용 대비 타입 4개는 이득이 없음. 모노레포 전체 코드생성 전략은 별도 과제 → 기각.

## R9. 공유 뷰: 표현 전용 WorktreeChangesView (CommitDetailView 패턴)

- **Decision**: `packages/git-ui`에 데이터·콜백을 전부 props로 주입받는 `WorktreeChangesView`를 추가한다(changes, selectedFilePath, onSelectFile, diff, diffLoading, diffError, diffClassName). 그룹 순서 상수 `conflicted → staged → unstaged → untracked`, 2글자 배지, rename 화살표, 내부에서 공유 `DiffViewer` 사용.
- **Rationale**: 헌법 원칙 IV — 두 앱의 표시 요구가 이미 수렴했고(그룹 목록+diff), 기존 `CommitDetailView`가 같은 주입 패턴으로 검증됨. 데이터 페칭을 앱에 남겨 Tauri/react-query 비의존 유지.
- **Alternatives considered**: *훅 포함 컨테이너 공유(useWorktreeChanges)* — 두 앱의 커맨드 이름·쿼리 키 체계가 달라 컨테이너 일반화가 어긋남. headless 수준 공유는 현재 뷰 계약으로 충분 → 기각.

## R10. 앱 통합 UX: 상세 패널 내 Commit / Working tree 토글

- **Decision**: 별도 화면이 아니라 기존 상세 패널(GE `ChangesPanel`, AW Git 탭)에 모드 토글을 추가한다. 커밋을 선택하면 자동으로 Commit 모드로 복귀한다.
- **Rationale**: "지금 상태 확인 ↔ 이력 확인"은 같은 맥락의 작업이라 화면 전환 없이 오가는 것이 작업 밀도(헌법 Engineering Standards의 operational density) 요구에 부합. 커밋 클릭 시 자동 복귀는 사용자의 명시적 의도(그 커밋을 보겠다)를 우선.
- **Alternatives considered**: *워킹 트리를 그래프 최상단 가상 노드로 표시* — 그래프 레이아웃/모델 변경 파급이 커서 조회 전용 1차 범위 초과 → 기각(후속 아이디어로 유지).

## 미해결 항목

없음. 단, Storybook 등록(헌법 의무)은 결정이 아닌 미완료 작업으로 plan.md Constitution Check와 tasks.md에서 추적한다.
