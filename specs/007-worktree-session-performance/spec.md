# Feature Specification: Worktree Session 페이지 성능 개선

**Feature Branch**: `007-worktree-session-performance`

**Created**: 2026-07-02

**Status**: Draft

**Input**: User description: "docs/worktree-session-loading-performance-review.md 문서에 조사한 개선사항 반영"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 세션 화면 즉시 진입 (Priority: P1)

사용자가 프로젝트에서 worktree를 선택해 세션 화면(agent 패널 + workspace 패널)을 열 때, worktree 수가 많거나 저장소 상태 계산이 느린 프로젝트에서도 화면 골격이 지체 없이 나타나고, 브랜치/변경 여부 같은 부가 정보는 준비되는 대로 채워진다. 화면을 여는 동안 하나의 느린 조회가 다른 조회의 응답을 가로막지 않는다.

**Why this priority**: 세션 화면은 이 앱의 핵심 작업 공간이며, 진입 지연은 모든 사용자가 매번 겪는 비용이다. 조사 문서에서 route blocking 조회와 백엔드 요청 직렬화가 최상위 병목(P0)으로 확인되었다.

**Independent Test**: worktree가 많은(10개 이상) 프로젝트에서 세션 화면을 열어, 목록 전체 조회가 끝나기 전에 화면 골격과 입력 UI가 표시되는지, 동시에 시작된 여러 데이터 조회가 서로를 기다리지 않는지 확인한다.

**Acceptance Scenarios**:

1. **Given** worktree가 10개 이상인 프로젝트, **When** 사용자가 특정 worktree의 세션 화면을 열면, **Then** 전체 worktree 목록과 각 worktree의 변경 여부 계산이 끝나기 전에 세션 화면 골격(agent 패널, workspace 패널, prompt 입력)이 표시된다.
2. **Given** 세션 화면 골격이 먼저 표시된 상태, **When** worktree 메타데이터(브랜치, 변경 여부) 조회가 완료되면, **Then** 화면의 상태 badge가 로딩 표시에서 실제 값으로 갱신된다.
3. **Given** 존재하지 않는 worktree 경로로 세션 화면을 열었을 때, **When** 검증이 완료되면, **Then** 화면 안에 명확한 오류 상태가 표시된다.
4. **Given** 큰 저장소에서 commit graph 조회가 오래 걸리는 상황, **When** 같은 시점에 agent 설정 조회가 요청되면, **Then** agent 설정은 graph 조회 완료를 기다리지 않고 응답된다.

---

### User Story 2 - Agent 실행 중에도 반응성 유지 (Priority: P2)

사용자가 agent 실행으로 파일이 계속 변경되는 동안에도 세션 화면이 버벅이지 않고, 파일/Git 정보 갱신이 화면에 보이는 범위에 한정되어 수행된다. 아무 작업도 하지 않는 idle 상태에서는 백그라운드 갱신이 스스로 연쇄 갱신을 유발하지 않는다.

**Why this priority**: 초기 진입 이후 세션 사용 시간 전체에 걸친 지속 비용이다. 조사 문서에서 watcher 이벤트에 의한 전면 갱신과 주기적 상태 조회의 되먹임 가능성이 확인되었다.

**Independent Test**: agent가 다수 파일을 수정하는 실행을 진행시키면서 UI 반응성과 갱신 빈도를 관찰하고, idle 10분 동안 불필요한 반복 갱신이 발생하지 않는지 확인한다.

**Acceptance Scenarios**:

1. **Given** agent가 파일을 연속으로 수정 중, **When** workspace의 Git 탭만 열려 있으면, **Then** 파일 목록 전체 재스캔 같은 비활성 탭용 갱신은 수행되지 않거나 탭 활성화 시점으로 미뤄진다.
2. **Given** 사용자가 아무 작업도 하지 않는 idle 상태, **When** 주기적인 상태 갱신이 실행되면, **Then** 그 갱신 자체가 저장소 변경 감지 이벤트를 재차 유발해 commit 이력/그래프 재조회가 반복되는 일이 없다.
3. **Given** 빌드 산출물 디렉터리에서 파일이 변경될 때, **When** 변경 감지가 동작하면, **Then** 화면에 표시되지 않는 산출물 변경은 갱신을 유발하지 않는다.
4. **Given** 파일이 연속으로 변경되는 상황, **When** 변경이 멈추면, **Then** 마지막 변경 내용이 누락 없이 화면에 반영된다.

---

### User Story 3 - Git 탭 초기 조회 최소화 (Priority: P3)

사용자가 세션 화면의 Git 탭에 진입하면, 현재 보고 있는 view(기본 graph)에 필요한 데이터만 우선 조회되고, 보지 않는 view의 데이터는 나중에 또는 전환 시점에 준비된다.

**Why this priority**: 초기 진입 비용을 추가로 줄이는 항목이지만, P1(진입 차단 해소)과 P2(지속 비용 제거)보다 개별 효과가 작다.

**Independent Test**: Git 탭 진입 시 발생하는 데이터 조회를 관찰해 선택된 view의 조회만 즉시 실행되는지, view 전환 시 반대편 데이터가 정상 로드되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** Git 탭 기본 view가 graph인 상태, **When** 세션 화면에 진입하면, **Then** list view용 commit 이력 조회는 즉시 실행되지 않는다.
2. **Given** graph view를 보고 있는 상태, **When** 사용자가 list view로 전환하면, **Then** commit 목록이 정상 표시된다(사전 준비되었거나 전환 시점에 로드).

---

### User Story 4 - 대형 저장소에서의 스크롤/탐색 확장성 (Priority: P4)

commit이 수천 개인 저장소에서 commit graph/목록을 깊게 스크롤하거나, 파일이 많은 저장소에서 Files/Markdown 탭을 열어도 로딩 시간과 화면 반응성이 유지된다.

**Why this priority**: 대형 저장소 사용자에게만 체감되는 확장성 항목으로, 조사 문서에서 P2로 분류된 개선(페이지 로드 비용 증가, 전체 파일 스캔, 전체 row 렌더링)에 해당한다.

**Independent Test**: 수천 commit 저장소에서 뒤 페이지 로드 시간이 스크롤 깊이에 비례해 늘어나지 않는지, 파일 수가 많은 저장소에서 Files/Markdown 탭 진입이 수 초 안에 완료되는지 확인한다.

**Acceptance Scenarios**:

1. **Given** commit 수천 개 저장소에서 무한 스크롤로 뒤 페이지를 로드할 때, **When** 스크롤이 깊어져도, **Then** 페이지당 로드 시간이 눈에 띄게 증가하지 않는다.
2. **Given** commit 1,000개 이상을 로드한 상태, **When** graph/목록을 스크롤하고 commit을 선택하면, **Then** 스크롤과 선택 반응성이 유지된다.
3. **Given** 파일이 매우 많은 저장소, **When** Markdown 탭에 진입하면, **Then** 화면 표시에 필요한 범위만 읽어 목록이 표시된다.

---

### Edge Cases

- 잘못된(삭제되었거나 오타인) worktree 경로로 세션을 열면 골격 우선 표시 후 오류 상태가 화면 내부에 표시되어야 한다.
- worktree 메타데이터가 늦게 도착하는 동안 상태 badge는 "확인 중" 같은 중간 상태를 허용해야 한다.
- 무한 스크롤 중 rebase/reset으로 이력이 바뀌어 이어받을 기준 commit이 사라지면, 조회가 실패 상태로 남지 않고 처음부터 다시 로드하는 폴백이 동작해야 한다.
- 여러 세션 창을 동시에 열었을 때 각 창의 변경 감지와 갱신이 서로 간섭하지 않아야 한다.
- 변경 감지 대상 디렉터리가 세션 사용 중 삭제되면 감지가 중단되고 오류가 사용자 흐름을 막지 않아야 한다.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 세션 화면은 URL에 worktree 경로가 있으면 전체 worktree 목록 조회 완료를 기다리지 않고 화면 골격을 먼저 표시해야 한다. 브랜치/변경 여부 등 메타데이터는 도착 시점에 보강한다.
- **FR-002**: worktree 경로가 유효하지 않은 경우, 세션 화면 내부에 명확한 오류 상태를 표시해야 한다.
- **FR-003**: 세션 진입 경로에서 worktree 목록 조회는 모든 worktree의 변경 여부(clean/dirty) 계산을 강제하지 않아야 한다. 변경 여부가 필요한 화면(프로젝트 상세)은 기존 정보를 그대로 제공받는다.
- **FR-004**: 백엔드 데이터 조회는 서로를 차단하지 않아야 한다. 하나의 느린 조회(예: commit graph)가 진행 중이어도 다른 조회(예: agent 설정)는 독립적으로 응답되어야 한다.
- **FR-005**: 저장소 상태를 주기적으로 확인하는 동작은 그 자체로 저장소 변경 감지 이벤트를 유발하지 않아야 한다.
- **FR-006**: 파일 변경 감지는 화면에 표시되지 않는 디렉터리(파일 목록 화면에서 제외되는 빌드 산출물·의존성 디렉터리와 동일한 목록)의 변경을 무시해야 한다.
- **FR-007**: 파일 변경 감지로 인한 데이터 갱신은 현재 활성 화면에 필요한 범위로 한정되어야 하며, 연속 변경이 멈춘 뒤 마지막 변경이 누락 없이 반영되어야 한다.
- **FR-008**: Git 탭은 현재 선택된 view(graph 또는 list)에 필요한 데이터만 즉시 조회하고, 반대편 view 데이터는 지연 조회 또는 유휴 시점 사전 로드로 처리해야 한다.
- **FR-009**: commit 이력/그래프의 추가 페이지 로드 비용은 이미 로드한 양(스크롤 깊이)에 비례해 증가하지 않아야 하며, 페이지 사이에 변하지 않는 정보(전체 개수, ref 목록)를 매 페이지 다시 계산하지 않아야 한다.
- **FR-010**: commit graph/목록은 로드된 row 수가 많아져도(1,000개 이상) 스크롤·선택 반응성을 유지해야 한다.
- **FR-011**: Files/Markdown 탭의 파일 목록은 저장소 전체 파일 수와 무관하게 표시에 필요한 범위만 읽는 방식으로 동작해야 한다.
- **FR-012**: agent 패널의 초기 데이터(agent 목록, 실행 설정, goal)는 재진입 시 불필요하게 다시 조회되지 않도록 적절한 신선도 기준을 가져야 한다.
- **FR-013**: 개선 전후 비교와 회귀 감지를 위해 주요 구간(화면 진입→골격 표시, 진입→graph 첫 row, 조회별 실행/대기 시간, 변경 감지 이벤트 빈도)을 측정할 수 있는 계측 수단이 있어야 한다.

### Key Entities

- **Worktree 메타데이터**: 경로, 브랜치, 변경 여부(clean/dirty/prunable), 삭제 가능 여부. 변경 여부는 지연 로드될 수 있으며 "미확인" 상태를 가질 수 있다.
- **변경 감지 이벤트**: 변경된 경로와 종류(일반 파일/저장소 이력)를 담는다. 종류에 따라 갱신 대상 범위가 달라진다.
- **Commit 페이지**: commit 목록의 조회 단위. 이어받기 기준(마지막 commit)과 전체 개수 정보를 가지며, 전체 개수는 첫 페이지에서만 계산될 수 있다.

## Constitution Alignment *(mandatory)*

- **Monorepo boundary**: 주 대상은 `apps/agentic-workbench`(프론트+Tauri 백엔드). commit 이력/그래프·worktree 상태 조회 개선은 공유 crate `crates/git-core`에 반영되어 git-explorer도 함께 혜택을 받는다. 공유 UI 개선(virtualization)은 `packages/git-ui` 범위.
- **Frontend layering**: route/페이지 골격 우선 렌더링은 `app`/`pages` 레이어, Git 탭 query 정책과 watcher 기반 갱신은 `features/worktree-workspace`, query 옵션·repository는 `entities` 레이어에서 처리한다.
- **Backend boundary**: 조회 명령의 비차단화는 `inbound`(Tauri command) 계층의 실행 방식 변경이며 domain/application 계약은 유지한다. 변경 감지 필터와 Git CLI 호출 옵션은 `infrastructure` 계층 책임이다.
- **Shared core vs UI**: Git 조회 로직 개선은 순수 로직(git-core)을 먼저 바꾸고, 앱은 어댑터를 통해 소비한다. row virtualization은 이미 공유 UI(git-ui)에 있는 view 컴포넌트에 적용한다.
- **Persistence and safety**: 파일 목록/변경 감지는 기존 worktree 경계 검증(경로 탈출 방지)을 그대로 유지해야 한다. 새로운 영속 데이터는 없다.
- **Documentation and Storybook**: `docs/worktree-session-loading-performance-review.md`의 실행 계획 상태를 갱신한다. 신규 UI 상태(메타데이터 로딩 badge, worktree 오류 상태)는 Storybook 스토리 추가를 검토한다.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: worktree 10개 이상 프로젝트에서 세션 화면 진입 시 화면 골격이 1초 이내에 표시된다(개선 전 대비 측정).
- **SC-002**: 세션 진입 직후 동시에 시작된 데이터 조회들의 총 완료 시간이 각 조회 시간의 합보다 짧다(직렬화 해소가 계측으로 확인된다).
- **SC-003**: 사용자가 아무 작업도 하지 않는 10분 동안, 저장소 변경 감지 이벤트로 인한 commit 이력/그래프 재조회가 0회다.
- **SC-004**: agent가 파일을 연속 수정하는 동안 세션 화면의 입력·스크롤 반응이 눈에 띄는 멈춤(수백 ms 이상 프리즈) 없이 유지된다.
- **SC-005**: 기본 graph view로 Git 탭 진입 시 list view용 이력 조회가 즉시 실행되지 않는다(초기 조회 수 감소가 계측으로 확인된다).
- **SC-006**: commit 5,000개 이상 저장소에서 뒤 페이지 로드 시간이 첫 페이지 로드 시간의 2배를 넘지 않는다.
- **SC-007**: commit 1,000개 이상 로드 후에도 스크롤과 commit 선택 반응이 지연 없이 동작한다.
- **SC-008**: 기존 기능 동작(프로젝트 상세의 worktree 상태 표시, commit 상세/diff, Markdown annotation 흐름)이 회귀 없이 유지된다.

## Assumptions

- 적용 범위는 조사 문서의 실행 계획 전체(PR0~PR7)를 우선순위 순으로 반영하는 것으로 본다. 사용자 스토리 P1~P4가 문서의 P0~P2 병목 그룹에 대응하며, P1/P2 스토리만으로도 독립적인 가치가 있는 MVP가 된다.
- 성능 수치 기준(SC-001, SC-006 등)은 조사 문서에 명시된 목표가 없어 일반적인 데스크톱 앱 체감 기준으로 설정했다. 계측 도입 후 실측 기반으로 조정될 수 있다.
- "주기적 상태 확인이 변경 감지를 되먹임한다"는 가설은 조사 문서에서 실측 검증 필요로 표시되어 있다. 본 기능에는 이를 확인할 계측(FR-013)이 포함되며, 실측 결과 되먹임이 없더라도 예방 조치(FR-005)는 저비용이므로 적용한다.
- 세션 화면 외 다른 화면(프로젝트 목록/상세)의 성능 개선은 공유 로직 개선의 부수 효과로만 다루고, 별도 목표로 삼지 않는다.
- git-explorer 앱의 UI 변경은 범위 밖이다. 공유 crate 개선의 혜택만 받는다.
