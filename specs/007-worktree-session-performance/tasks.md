# Tasks: Worktree Session 페이지 성능 개선

**Input**: Design documents from `/specs/007-worktree-session-performance/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/tauri-commands.md, quickstart.md

**Tests**: 사용자 여정 테스트는 quickstart.md 수동 시나리오로 대체한다. Constitution 필수 테스트(순수 로직·공유 crate/패키지·안전 경계)는 생략 불가 — 해당 구현 task 전에 테스트 task를 배치했고, 테스트는 구현 전 FAIL을 확인한다.

**Organization**: spec.md의 user story(P1~P4) 단위로 phase를 구성한다. 각 story는 독립적으로 구현·검증 가능하다.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 병렬 실행 가능(다른 파일, 미완료 task에 대한 의존 없음)
- **[Story]**: US1(세션 즉시 진입), US2(실행 중 반응성), US3(Git 탭 조회 최소화), US4(대형 저장소 확장성)

## Path Conventions

- **AW frontend**: `apps/agentic-workbench/src/{app,pages,features,entities,shared,components/ui,stories}`
- **AW Tauri backend**: `apps/agentic-workbench/src-tauri/src/{domain,application,inbound,infrastructure}`
- **공유 Rust**: `crates/git-core/src`
- **공유 TypeScript**: `packages/git-ui/src`, `packages/git-graph/src`
- **문서**: `docs/worktree-session-loading-performance-review.md`

---

## Phase 1: Setup (계측 기반)

**Purpose**: 모든 개선의 전후 비교에 필요한 계측을 먼저 넣는다 (research R12, FR-013).

- [x] T001 [P] `AW_PERF_LOG=1` 조건부 perf 로그 헬퍼(`perf kind=<command|git|watcher> name=<..> wait_ms=<n> run_ms=<n>` stderr 출력)를 `apps/agentic-workbench/src-tauri/src/infrastructure/perf_log.rs`로 추가하고 `infrastructure/mod.rs`에 등록
- [x] T002 [P] frontend 계측: `apps/agentic-workbench/src/app/App.tsx`(route 진입 mark)와 `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`(graph 첫 row 표시)에 `performance.mark`/`measure`(`session:shell-rendered`, `session:graph-first-row`) 추가, dev 모드에서 콘솔 출력
- [ ] T003 개선 전 baseline 수치 수집: quickstart.md S1/S4/S5 절차로 현재 수치를 측정해 `specs/007-worktree-session-performance/baseline.md`에 기록 (T001, T002 완료 후)

**Checkpoint**: `AW_PERF_LOG=1`로 앱을 띄우면 command/watcher 로그와 session measure가 출력된다.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 동기 command의 main thread 직렬화 해소(R1)와 status 되먹임 차단(R2). 모든 user story의 효과를 배가시키는 선행 작업.

**⚠️ CRITICAL**: 이 phase 완료 전에는 user story 작업을 시작하지 않는다.

- [x] T004 `apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs`의 Git/파일 계열 command(contracts §1 목록: `list_git_worktrees`, `list_git_branches`, `list_git_remotes`, `get_worktree_changes`, `get_worktree_file_diff`, `list_worktree_changes`, `list_worktree_git_history`, `get_worktree_git_graph`, `get_worktree_commit_detail`, `get_worktree_commit_file_diff`, `list_worktree_files`, `read_worktree_text_file`, `start_worktree_watcher`, `create_git_worktree`, `delete_git_worktree`)를 `async fn` + `tauri::async_runtime::spawn_blocking`으로 전환. provider/service 시그니처는 동기 유지. T001의 perf 로그(wait_ms/run_ms)를 command 경계에 연결
- [x] T005 [P] `apps/agentic-workbench/src-tauri/src/infrastructure/git_cli_worktree_provider.rs`의 `has_changes`에 `--no-optional-locks` 적용
- [x] T006 [P] `crates/git-core/src/git_cli.rs`의 `GitCliWorktreeStatusReader::status`에 `--no-optional-locks` 적용, 기존 테스트 통과 확인
- [x] T007 Foundational 검증: `cargo test -p git-core` + agentic-workbench/git-explorer 양쪽 `cargo check`(Atomic Cross-App Verification), 앱 기동 후 세션 화면에서 graph 로딩 중 agent settings 응답이 차단되지 않는지 perf 로그 `wait_ms`로 확인

**Checkpoint**: 서로 다른 command의 동시 invoke가 상호 차단되지 않는다(SC-002 기반 확보).

---

## Phase 3: User Story 1 - 세션 화면 즉시 진입 (Priority: P1) 🎯 MVP

**Goal**: worktree 목록/상태 계산 완료를 기다리지 않고 세션 화면 골격을 즉시 표시하고, 메타데이터는 도착 시점에 보강한다 (research R4, R5 / FR-001~003).

**Independent Test**: worktree 10개 이상 프로젝트에서 세션 진입 시 목록 조회 완료 전 골격 표시(quickstart S1), `session:shell-rendered` < 1초, 잘못된 경로에서 오류 상태 표시.

### Tests for User Story 1 (constitution-required) ⚠️

- [x] T008 [P] [US1] `apps/agentic-workbench/src-tauri/src/infrastructure/git_cli_worktree_provider.rs` 테스트 모듈에 `include_status=false`면 status가 `Unknown`/`can_delete=false`이고 prunable 판정은 유지되는 단위 테스트 추가(구현 전 FAIL 확인)
- [x] T009 [P] [US1] `apps/agentic-workbench/src/app/model/session-route.test.ts`에 placeholder worktree 구성·목록 도착 시 교체·path 불일치 시 invalid 판정 로직 테스트 추가(구현 전 FAIL 확인)

### Implementation for User Story 1

- [x] T010 [US1] domain/port 확장: `apps/agentic-workbench/src-tauri/src/domain/git_worktree.rs`의 `GitWorktreeStatus`에 `Unknown` 추가, `domain/git_worktree_provider.rs`의 `list_worktrees`에 `include_status: bool` 파라미터 추가
- [x] T011 [US1] 구현 연결: `infrastructure/git_cli_worktree_provider.rs`(include_status=false 시 `has_changes` 생략), `application/git_worktree_service.rs`, `inbound/tauri_commands.rs`의 `list_git_worktrees`에 `include_status: Option<bool>`(기본 true) 전달 — T008 테스트 통과
- [x] T012 [P] [US1] frontend 타입/repository: `apps/agentic-workbench/src/entities/project/model/git-worktree.ts`의 status에 `"unknown"` 추가, `entities/project/api/git-worktree-repository.ts`의 `listGitWorktrees`에 `{ includeStatus?: boolean }` 옵션 추가
- [x] T013 [US1] route shell 우선 렌더링: `apps/agentic-workbench/src/app/model/session-route.ts`에 placeholder 구성/교체/invalid 판정 헬퍼 구현(T009 테스트 통과), `app/App.tsx`의 `ProjectWorktreeSessionRoute`가 placeholder로 즉시 페이지를 렌더링하고 `includeStatus: false`로 목록을 조회하도록 변경
- [x] T014 [US1] 페이지 상태 UI: `apps/agentic-workbench/src/pages/project-worktree-session/ui/project-worktree-session-page.tsx`에 `unknown` status badge(확인 중 표시)와 worktree 검증 실패 상태 UI 추가
- [x] T015 [P] [US1] Storybook: 세션 페이지의 메타데이터 로딩(unknown)·검증 실패 상태 스토리를 `apps/agentic-workbench/src/stories/pages.stories.tsx`에 추가
- [x] T016 [US1] US1 검증: quickstart S1 수행(골격 1초 이내, status badge 갱신, 잘못된 경로 오류, worktree당 `git status` 미실행, 프로젝트 상세 badge 회귀 없음) + `pnpm --filter agentic-workbench check-types && pnpm --filter agentic-workbench test`

**Checkpoint**: US1 단독으로 MVP — 세션 진입 체감 속도 개선이 독립 검증된다.

---

## Phase 4: User Story 2 - Agent 실행 중에도 반응성 유지 (Priority: P2)

**Goal**: watcher 이벤트 필터·trailing debounce·선별 invalidation으로 실행 중/idle 상태의 불필요한 갱신을 제거한다 (research R3 / FR-005~007).

**Independent Test**: idle 10분간 watcher `kind=git` 이벤트 0회(quickstart S2), agent 실행 중 프리즈 없음·빌드 산출물 변경 무시·마지막 변경 반영(quickstart S3).

### Tests for User Story 2 (constitution-required) ⚠️

- [x] T017 [P] [US2] `apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_watcher.rs` 테스트 모듈에 추가(구현 전 FAIL 확인): (a) `EXCLUDED_DIRS` 전체가 무시되는지, (b) `.git/index`·`*.lock`·`FETCH_HEAD` 단독 변화 미발행, `HEAD`/`refs/`/`packed-refs` 변화는 `kind=git` 발행, (c) trailing debounce가 창 내 마지막 이벤트를 발행하는지

### Implementation for User Story 2

- [x] T018 [US2] 제외 목록 단일화: `EXCLUDED_DIRS`를 `apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_file_provider.rs`에서 `infrastructure/mod.rs`(또는 공용 모듈)로 옮겨 watcher와 공유
- [x] T019 [US2] watcher 이벤트 필터: `infrastructure/fs_worktree_watcher.rs`의 `should_ignore_event`가 공유 제외 목록을 사용하고, `.git` 내부 이벤트를 `HEAD`/`refs/`/`MERGE_HEAD`/`packed-refs`로 세분화(`index`/`*.lock`/`FETCH_HEAD` 단독 변화 미발행) — T017(a)(b) 통과
- [x] T020 [US2] trailing debounce: `infrastructure/fs_worktree_watcher.rs`의 leading-edge rate-limit을 마지막 원시 이벤트 후 500ms 발행 방식으로 교체(std thread + channel, 신규 의존성 없음), T001 perf 로그에 watcher 이벤트 카운트 연결 — T017(c) 통과
- [x] T021 [US2] frontend 선별 invalidation: `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`의 watcher listener가 활성 탭 기준으로 invalidate 범위를 한정(비활성 query는 inactive invalidation으로 즉시 refetch하지 않음, contracts §5 규칙)
- [x] T022 [US2] US2 검증: quickstart S2(idle 10분 git 이벤트 0회)·S3(agent 실행 반응성, dist 변경 무시, 마지막 변경 반영, 외부 commit 정상 감지) 수행 + `cargo test`(src-tauri) + `pnpm --filter agentic-workbench test`

**Checkpoint**: US1+US2 — 진입 속도와 세션 지속 비용이 모두 개선된 상태.

---

## Phase 5: User Story 3 - Git 탭 초기 조회 최소화 (Priority: P3)

**Goal**: 선택된 view의 데이터만 즉시 조회하고 agent query에 신선도 기준을 부여한다 (research R6, R7 / FR-008, FR-012).

**Independent Test**: 기본 graph view 진입 시 history 조회 미실행(quickstart S4), view 전환 시 정상 로드, 세션 재진입 시 agents/settings 불필요 refetch 없음.

### Implementation for User Story 3

- [x] T023 [P] [US3] `apps/agentic-workbench/src/features/worktree-workspace/ui/worktree-workspace-panel.tsx`의 `GitWorkspaceTab`에서 `historyQuery.enabled = historyView === "list"`, `graphQuery.enabled = historyView === "graph"` 적용, 새로고침 버튼·Refreshing badge가 enabled query 기준으로 동작하도록 정리
- [x] T024 [P] [US3] `apps/agentic-workbench/src/features/agent-run/ui/agent-run-panel.tsx`의 query 캐시 정책: `agentsQuery` staleTime 5분, `settingsQuery`/`appCommandSettingsQuery` staleTime 30초, `goalQuery` staleTime 10초 (contracts §6)
- [x] T025 [US3] US3 검증: quickstart S4 수행(perf 로그에 `list_worktree_git_history` 미실행, List 전환 정상, Graph 복귀 즉시 표시) + 세션 재진입 시 agent query refetch 없음 확인 + `pnpm --filter agentic-workbench check-types`

**Checkpoint**: US1~US3 — 초기 조회 세트가 최소화된 상태.

---

## Phase 6: User Story 4 - 대형 저장소 확장성 (Priority: P4)

**Goal**: cursor 페이지네이션·count/refs 첫 페이지 한정·backend 병렬화(git-core), 파일 목록 scope 조회, graph/list virtualization으로 대형 저장소에서도 성능을 유지한다 (research R8~R11 / FR-009~011).

**Independent Test**: commit 5,000+ 저장소에서 뒤 페이지 로드가 첫 페이지의 2배 이내·count/refs 반복 실행 없음, 1,000+ row 스크롤 반응성, rebase 후 cursor 폴백, 대형 저장소 Files/Markdown 탭 수 초 내 표시 (quickstart S5).

### Tests for User Story 4 (constitution-required) ⚠️

- [x] T026 [P] [US4] `crates/git-core/src/git_cli.rs` 테스트 모듈에 fixture 테스트 추가(구현 전 FAIL 확인): cursor 유효 시 이어받기, cursor 무효 시 `cursor_invalidated=true` + offset 폴백, offset>0/cursor 요청에서 count·refs 생략(`total_count=None`, refs 빈 배열)
- [x] T027 [P] [US4] `apps/agentic-workbench/src-tauri/src/infrastructure/fs_worktree_file_provider.rs` 테스트 모듈에 scope 테스트 추가(구현 전 FAIL 확인): `kind=markdown` 필터(조상 디렉터리 포함), `dir`+`depth=1` 직계 조회, `dir` 경로 탈출 거부

### Implementation for User Story 4 — git-core 페이지네이션 (R8, R9)

- [x] T028 [US4] `crates/git-core/src/domain.rs`: `GitCommitPage`/`GitGraphPage`의 `total_count`를 `Option<usize>`로, `cursor_invalidated: Option<bool>` 추가(serde 하위 호환 유지)
- [x] T029 [US4] `crates/git-core/src/ports.rs`+`git_cli.rs`: `GitHistoryReader::list_history`/`get_commit_graph`에 `cursor: Option<&str>` 추가, cursor 기반 이어받기·무효 시 offset 폴백·count/refs 첫 페이지 한정 구현 — T026 통과
- [x] T030 [US4] `crates/git-core/src/git_cli.rs`: 첫 페이지의 `head_hash`/`log`/`refs`/`count` 조회를 `std::thread::scope`로 병렬화, 기존 테스트 전체 통과 확인
- [x] T031 [US4] 소비 앱 어댑터 갱신: `apps/agentic-workbench/src-tauri/src/infrastructure/git_cli_worktree_git_provider.rs`·`application/worktree_git_service.rs`·`inbound/tauri_commands.rs`에 cursor 파라미터 전달, `apps/git-explorer/src-tauri/src`의 `GitHistoryReader` 호출 지점을 새 시그니처(기본 `None`)로 갱신 후 `cargo check -p git-explorer`(crate 경로 기준) 통과

### Implementation for User Story 4 — frontend 페이지네이션 소비

- [x] T032 [US4] TS 타입/모델: `packages/git-graph/src`의 page 타입에 `totalCount?`/`cursorInvalidated?` 반영, `packages/git-ui/src/model/commit-graph.ts`의 `combineGitCommitGraphPages`가 첫 페이지 `totalCount`/`refs`를 유지하도록 수정 + 패키지 단위 테스트 갱신
- [x] T033 [US4] `apps/agentic-workbench/src/entities/worktree-git/api/worktree-git-repository.ts`에 cursor 파라미터 추가, `features/worktree-workspace/ui/worktree-workspace-panel.tsx`의 infinite query가 `getNextPageParam`으로 마지막 commit hash를 cursor로 전달하고 `cursorInvalidated` 응답 시 목록을 초기화하도록 수정

### Implementation for User Story 4 — 파일 목록 lazy loading (R10)

- [x] T034 [US4] backend scope 구현: `apps/agentic-workbench/src-tauri/src/domain/worktree_file_provider.rs` port에 scope 파라미터 추가, `infrastructure/fs_worktree_file_provider.rs`(markdown 필터·depth·dir, 기존 `resolve_worktree_path` 검증 재사용), `application/worktree_file_service.rs`, `inbound/tauri_commands.rs`의 `list_worktree_files`에 옵션 연결 — T027 통과
- [x] T035 [US4] frontend lazy 파일 트리: `apps/agentic-workbench/src/entities/worktree-file/api/worktree-file-repository.ts`에 scope 옵션 추가, `features/worktree-workspace/ui/worktree-workspace-panel.tsx`의 Markdown 탭은 `kind: "markdown"`, Files 탭은 `dir`+`depth: 1` 폴더 펼침 조회로 전환(query key에 scope 포함)

### Implementation for User Story 4 — virtualization (R11)

- [x] T036 [P] [US4] `packages/git-ui/src`에 고정 높이 row + overscan 방식의 virtualization hook을 추가하고 `HistoryGraphView`/`CommitListView`에 적용(외부 의존성 없이, `VirtualizedRunTimeline` 패턴 참고) + 단위 테스트
- [x] T037 [P] [US4] Storybook: 1,000+ row graph/list 스토리를 git-ui 스토리(organisms)에 추가해 virtualization 동작을 시각 확인
- [x] T038 [US4] US4 검증: quickstart S5 수행(뒤 페이지 2배 이내, count/refs 미반복, DOM row 제한, rebase 폴백, 대형 저장소 탭 진입) + `cargo test -p git-core` + `pnpm --filter @yoophi/git-ui test` + 양쪽 앱 check-types

**Checkpoint**: 모든 user story 완료 — 대형 저장소 시나리오까지 독립 검증.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: 문서·회귀·경계 검증 마무리.

- [x] T039 [P] `docs/worktree-session-loading-performance-review.md`의 권장 실행 계획(PR0~PR7)에 적용 결과·실측 수치(baseline 대비)를 갱신
- [x] T040 [P] 전체 자동 검증 일괄 실행: `cargo test -p git-core`, src-tauri `cargo test`, `cargo check`(git-explorer 포함), `pnpm --filter @yoophi/git-ui test`, `pnpm --filter @yoophi/workspace-auto-refresh test`, `pnpm --filter agentic-workbench check-types && test`, `pnpm --filter git-explorer check-types` (quickstart 자동 검증 절)
- [ ] T041 quickstart S6 회귀 시나리오 수행: commit 상세/diff, Markdown annotation→prompt 전송, 프로젝트 상세 badge/삭제, git-explorer 화면 정상
- [ ] T042 경계·정리 점검: 앱 간 직접 import 미도입 확인(`grep`으로 `apps/git-explorer` ↔ `apps/agentic-workbench` 상호 참조 검사), 임시 코드·불필요 로그 제거, SC 달성 수치를 `specs/007-worktree-session-performance/baseline.md`에 최종 기록

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup/계측)**: 의존 없음 — 즉시 시작
- **Phase 2 (Foundational)**: Phase 1 완료 후 — **모든 user story를 차단**
- **Phase 3~6 (US1~US4)**: Phase 2 완료 후. 서로 파일 겹침이 있는 곳(`worktree-workspace-panel.tsx`: T021/T023/T033/T035)만 조율하면 story 간 병렬 가능
- **Phase 7 (Polish)**: 원하는 story 완료 후

### User Story Dependencies

- **US1 (P1)**: Foundational만 의존 — 다른 story 의존 없음
- **US2 (P2)**: Foundational만 의존 — US1과 독립(단, T021과 T013/T014는 다른 파일이라 충돌 없음)
- **US3 (P3)**: Foundational만 의존 — T023과 US2의 T021이 같은 파일(`worktree-workspace-panel.tsx`)이므로 동시 작업 금지, 순차 권장
- **US4 (P4)**: Foundational만 의존 — git-core 변경(T028~T031)은 다른 story와 완전 독립. frontend task(T033/T035)는 US2/US3와 같은 파일 접점 있음

### Within Each User Story

- Constitution 필수 테스트(T008/T009, T017, T026/T027)는 구현 전 작성하고 FAIL 확인
- domain/port → infrastructure/application → inbound command → frontend entities → features/pages 순
- 공유 crate/패키지 변경 시 소비 앱 검증까지 완료해야 task 완료(T031, T032)

### Parallel Opportunities

```text
Phase 1: T001 ∥ T002 → T003
Phase 2: T004 후 T005 ∥ T006 → T007
US1:     T008 ∥ T009 → T010 → T011, T012 ∥ (T010~T011) → T013 → T014, T015 ∥ T014 → T016
US2:     T017 → T018 → T019 → T020 → T021 → T022
US4:     T026 ∥ T027, T036 ∥ T037은 git-core/파일 트랙과 병렬,
         git-core 트랙(T028→T029→T030→T031)과 파일 트랙(T034→T035) 상호 병렬
```

---

## Parallel Example: User Story 1

```bash
# 테스트 먼저 (병렬):
Task: "T008 git_cli_worktree_provider.rs include_status 단위 테스트"
Task: "T009 session-route.test.ts placeholder/invalid 판정 테스트"

# 구현 (백엔드 체인과 프론트 타입은 병렬):
Task: "T010→T011 backend include_status 체인"
Task: "T012 frontend 타입/repository 옵션"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1(계측) → Phase 2(Foundational: async화 + no-optional-locks) 완료
2. Phase 3(US1) 완료 → quickstart S1로 독립 검증
3. **STOP and VALIDATE**: baseline 대비 `session:shell-rendered` 수치 확인 후 배포/데모 가능

### Incremental Delivery

1. Setup + Foundational → 직렬화 해소 자체가 체감 개선(SC-002)
2. US1 → 진입 속도 (MVP!)
3. US2 → 세션 지속 비용 제거 (idle/agent 실행)
4. US3 → 초기 조회 최소화
5. US4 → 대형 저장소 확장성
6. 각 단계는 이전 단계를 깨지 않고 독립 검증된다(각 phase 마지막 검증 task)

### 주의: 파일 접점

`features/worktree-workspace/ui/worktree-workspace-panel.tsx`는 T002/T021/T023/T033/T035가 순차적으로 수정한다. 병렬 작업 시 이 파일을 만지는 task는 한 번에 하나만 진행한다.

---

## Notes

- [P] = 다른 파일·의존 없음. [Story] = spec.md user story 추적용
- 각 task 또는 논리 그룹 완료 시 커밋
- 각 checkpoint에서 멈추고 해당 story를 독립 검증할 수 있다
- 공유 crate(git-core)/패키지(git-ui, git-graph) 변경 task는 소비 앱 검증까지가 완료 조건(Constitution V)

---

## 구현 후 잔여 항목 (수동 검증 필요)

자동 검증(단위/fixture 테스트, check-types, cross-app cargo check/test)은 모두 통과했다.
아래 항목은 GUI 앱 실행이 필요해 사람이 quickstart.md 절차로 수행한다.

- [ ] T003 후속: baseline 수치 측정 — PR0 이전 커밋(`d305d46^`)에서 `AW_PERF_LOG=1`로 개선 전 수치를 수집해 `baseline.md`에 기입
- [ ] T016/T022/T025/T038의 수동 시나리오: quickstart S1~S5 (골격 1초, idle 10분 git 이벤트 0회, agent 실행 반응성, history 미실행, 뒤 페이지 2배 이내)
- [ ] T041: quickstart S6 회귀 시나리오 (commit 상세/diff, Markdown annotation, 프로젝트 상세 badge/삭제, git-explorer 화면)
- [ ] T042 잔여: SC 달성 수치를 `baseline.md`에 최종 기록 (경계 검사·코드 정리·자동 검증은 완료)

참고: 기존 dead_code 경고 2건(`record_goal_usage`, `apply_run_usage`)은 본 기능 이전부터 존재하던 것으로 범위 밖이다.
