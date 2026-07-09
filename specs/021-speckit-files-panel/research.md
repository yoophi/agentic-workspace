# Research: Speckit Files Panel

## Decision: 기존 worktree file 조회/감시 경계를 우선 재사용한다

**Rationale**: AW에는 이미 `list_worktree_files`, `read_worktree_text_file`, worktree watcher, markdown viewer flow가 있다. Speckit 패널은 현재 worktree의 markdown 산출물을 읽기 전용으로 탐색하는 기능이므로, 별도 저장소나 신규 backend capability보다 기존 root/path 검증이 있는 파일 경계를 재사용하는 것이 안전하고 작다.

**Alternatives considered**:
- Speckit 전용 Tauri command 신설: UI 요구에 맞는 payload를 바로 받을 수 있지만 backend surface가 커지고 기존 파일 조회 기능과 중복된다.
- frontend에서 전체 파일트리를 모두 받아 임의 필터링: 구현은 단순하지만 큰 worktree에서 비용이 커질 수 있다.
- 파일 트리 탭만 확장: Speckit 문서 탐색 목표가 흐려지고 tasks 진행 요약 같은 기능별 정보 표현이 어렵다.

## Decision: Speckit 문서 분류와 tasks 진행 계산은 app-local pure model로 둔다

**Rationale**: 문서 유형 분류, 기능 폴더 그룹핑, markdown checkbox count는 UI와 backend 양쪽에 의존하지 않는 순수 로직이다. `features/worktree-workspace/model`에 fixture-driven unit test를 두면 경로/문서 유형/진행률 회귀를 빠르게 검증할 수 있다. 아직 다른 앱 소비자가 없으므로 `packages/*`로 이동하지 않는다.

**Alternatives considered**:
- backend에서 Speckit summary를 완성해 반환: root validation과 파일 읽기를 한 번에 처리할 수 있지만 UI 전용 분류 규칙이 backend에 고정된다.
- markdown annotation core에 tasks parser 추가: 재사용 가능성은 있지만 현재 요구는 AW Speckit panel 전용이며 cross-app 소비 근거가 부족하다.

## Decision: Speckit 패널은 기존 workspace tab 구성에 추가한다

**Rationale**: issue #111은 Git/files/markdown과 별도 Speckit 전용 패널을 요구한다. 기존 `WorktreeWorkspacePanel`은 tab 방식으로 worktree 작업 surface를 구성하므로 Speckit tab을 추가하면 사용자는 기존 흐름을 유지하면서 기능 산출물만 별도 시야로 볼 수 있다.

**Alternatives considered**:
- markdown tab 안에 Speckit filter 추가: 별도 패널 요구를 만족하지 못하고 기능별 grouping이 약해진다.
- project dashboard에 Speckit 카드 추가: worktree session 맥락과 파일 watcher/markdown viewer 연결이 멀어진다.
- modal explorer 제공: 반복 탐색 업무에 부적합하고 작업 surface를 가린다.

## Decision: 문서 선택은 기존 markdown 검토 흐름과 연결한다

**Rationale**: spec 요구의 핵심은 Speckit 패널에서 문서를 선택하면 기존 markdown 패널/뷰어 흐름으로 확인하는 것이다. 별도 viewer를 만들지 않으면 annotation, Mermaid, TOC, long-content 처리를 기존 검증된 경로와 일관되게 유지할 수 있다.

**Alternatives considered**:
- Speckit panel 내부 preview: 좌우 비교에는 편하지만 markdown viewer 중복과 layout 복잡도가 증가한다.
- 외부 에디터 열기: AW session 안에서 흐름을 이어가는 요구와 맞지 않는다.

## Decision: file watcher invalidation은 기존 worktree change event에 맞춘다

**Rationale**: AW는 worktree watcher 이벤트로 file/git/markdown query invalidation을 이미 처리한다. Speckit query key를 같은 이벤트 경로에 포함하면 사용자가 기존 패널에서 기대하는 자동 갱신 정책과 일관된다.

**Alternatives considered**:
- Speckit 전용 watcher 실행: 중복 watcher와 lifecycle 복잡도를 만든다.
- 수동 refresh만 제공: SC-005의 자동 또는 명시적 갱신 기대를 약화한다.

## Decision: backend 변경은 markdown scope/list depth가 부족할 때만 수행한다

**Rationale**: 현재 provider는 markdown scope와 dir/depth 조회를 지원한다. Speckit 패널은 `specs` 아래 markdown 문서만 필요하므로 기존 scope로 충분할 가능성이 높다. 필요한 경우에도 `scope.dir = "specs"`와 `kind = "markdown"` 조합이 root validation을 통과해야 하며, 별도 Speckit command는 마지막 선택지로 둔다.

**Alternatives considered**:
- 처음부터 `list_speckit_documents` command 추가: 데이터 payload는 깔끔하지만 feature-local 규칙이 backend API로 굳어진다.
- 전체 worktree walk 후 frontend 필터: 작은 repo에서는 괜찮지만 large repo 성능 목표와 맞지 않는다.
