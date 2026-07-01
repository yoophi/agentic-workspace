# Research: 프로젝트 대시보드 시작화면

## Decision: 루트 화면을 dashboard page로 교체하고 기존 project list는 전체 목록 역할로 유지한다

**Rationale**: spec은 앱 진입 시 최근 프로젝트, 주요 상태, 빠른 action을 중심에 두도록 요구한다. 기존 `ProjectListPage`는 전체 프로젝트 관리와 CRUD에 적합하므로 시작화면을 그대로 확장하면 관리 목록과 작업 재개 대시보드의 역할이 섞인다. 새 `ProjectDashboardPage`를 루트에 배치하고 기존 project list/detail/worktree/session routes를 하위 흐름으로 유지하면 역할이 분명하다.

**Alternatives considered**:

- 기존 `ProjectListPage` 안에 요약 패널을 추가: 빠르지만 목록 관리 화면과 시작 대시보드의 정보 구조가 충돌한다.
- 프로젝트 상세를 시작화면으로 사용: 최근 프로젝트가 여러 개인 사용자의 재개 흐름을 해결하지 못한다.

## Decision: v1은 기존 project/worktree/session 조회를 조합하고, 부족한 요약은 비동기 degraded state로 표시한다

**Rationale**: 현재 AW에는 `list_projects`, `list_git_worktrees`, worktree change, provider session 관련 command와 repository 경계가 있다. 대시보드는 "시작을 막지 않는" 화면이어야 하므로 프로젝트 목록은 즉시 표시하고, worktree/session/change 요약은 가능한 범위에서 비동기로 채운다. 조회 실패는 전체 화면 오류가 아니라 항목별 "확인 불가" 상태로 표시한다.

**Alternatives considered**:

- 새 통합 dashboard summary command를 먼저 만든다: 성능과 호출 단순성은 좋지만 요구사항을 과도하게 backend 중심으로 만든다.
- 모든 상태가 준비될 때까지 대시보드 표시를 지연한다: 최근 작업 재개 목적과 맞지 않는다.

## Decision: 최근 프로젝트 정렬은 최근 활동 신호를 우선하고, 없으면 안정적인 기존 정렬로 fallback한다

**Rationale**: spec은 최근 사용한 프로젝트를 우선 보여주라고 요구하지만 현재 project model은 명시적 `lastUsedAt`을 갖고 있지 않다. 구현은 사용 가능한 최근 세션/worktree 활동 신호를 우선 사용하고, 신호가 없을 때는 기존 이름 정렬 또는 저장 순서를 일관되게 사용해야 한다. 이 규칙은 순수 helper로 테스트한다.

**Alternatives considered**:

- project model에 즉시 `lastOpenedAt` persistence를 추가: 장기적으로 유용하지만 v1의 필수 조건은 아니며 migration/safety 범위가 커진다.
- 항상 이름순 표시: "최근" 대시보드 기대와 맞지 않는다.

## Decision: UI는 dense operational layout으로 구성하고 card 남용을 피한다

**Rationale**: issue와 constitution 모두 실제 작업 진입을 첫 화면 중심에 두고 marketing/landing page처럼 보이지 않아야 한다고 명시한다. 대시보드는 상단 quick actions, 최근 프로젝트 리스트, 상태/요약 column 또는 compact sections로 구성하고, 과장된 hero나 장식적 배경을 쓰지 않는다.

**Alternatives considered**:

- 큰 hero와 설명 중심 onboarding: 신규 사용자에게는 친절하지만 반복 사용성과 operational UI 원칙에 맞지 않는다.
- 기존 table만 유지: 상태 요약과 빠른 재개 action이 충분히 드러나지 않는다.

## Decision: Storybook page states를 주요 검증 수단으로 삼고, 순수 summary helper는 Vitest로 검증한다

**Rationale**: spec은 빈 상태, 최근 프로젝트 없음, 로딩/오류, 긴 콘텐츠 상태를 포함하고 Storybook 또는 UI 상태 검증을 요구한다. 기존 AW는 `src/stories/pages.stories.tsx`와 sample data를 사용하므로 여기에 dashboard states를 추가하는 것이 가장 일관적이다. 정렬/요약/상태 매핑은 화면에서 분리해 unit test로 검증한다.

**Alternatives considered**:

- 수동 QA만 수행: 상태별 회귀를 잡기 어렵다.
- E2E 테스트를 필수로 둔다: routing과 Tauri mock 준비 비용이 크므로 v1 필수 검증으로는 과하다.
