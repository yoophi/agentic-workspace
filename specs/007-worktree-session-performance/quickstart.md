# Quickstart: Worktree Session 페이지 성능 개선 검증

구현 완료 후(또는 단계별 PR마다) 이 절차로 spec의 Success Criteria를 검증한다. 계약 상세는 [contracts/tauri-commands.md](./contracts/tauri-commands.md), 모델은 [data-model.md](./data-model.md) 참조.

## 사전 준비

```bash
pnpm install
cargo check --workspace

# 계측 활성화 상태로 앱 실행
AW_PERF_LOG=1 pnpm --filter agentic-workbench tauri dev
```

테스트 저장소 조건:

- **다중 worktree 프로젝트**: worktree 10개 이상 (예: `git worktree add`를 반복 생성한 fixture)
- **대형 이력 저장소**: commit 5,000개 이상 (예: 본 monorepo 또는 오픈소스 clone)
- **파일 다수 저장소**: 파일 수만 개 monorepo

## 자동 검증 (회귀 게이트)

```bash
# 공유 crate와 양쪽 소비 앱 (Atomic Cross-App Verification)
cargo test -p git-core
cargo check -p agentic-workbench   # src-tauri crate 이름 기준, 필요 시 경로로 실행
cargo check -p git-explorer

# 공유 패키지와 소비 앱
pnpm --filter @yoophi/git-ui test && pnpm --filter @yoophi/git-ui check-types
pnpm --filter @yoophi/workspace-auto-refresh test
pnpm --filter agentic-workbench check-types && pnpm --filter agentic-workbench test
pnpm --filter git-explorer check-types
```

새 단위 테스트가 포함되어야 하는 영역: cursor 페이지네이션(유효/무효 cursor, 폴백), `include_status=false` 경로, watcher 이벤트 필터(`EXCLUDED_DIRS`, `.git` 세분화), trailing debounce(마지막 이벤트 발행), `list_worktree_files` scope(markdown 필터, depth=1, 경로 탈출 거부).

## 시나리오 검증

### S1. 세션 즉시 진입 (SC-001, SC-002 / User Story 1)

1. worktree 10개 이상 프로젝트에서 세션 화면 진입.
2. **기대**: worktree 목록 조회 완료 전에 화면 골격(agent 패널·prompt 입력·workspace 탭)이 표시되고, 상태 badge는 `unknown` → 실제 값으로 갱신된다.
3. dev 콘솔에서 `session:shell-rendered` measure가 **1초 이내**인지 확인.
4. `AW_PERF_LOG` 출력에서 세션 진입 직후 command들의 `wait_ms`가 크지 않은지(직렬화 해소) 확인 — 특정 command의 `run_ms`가 커도 다른 command의 `wait_ms`가 그에 비례해 커지지 않아야 한다.
5. 존재하지 않는 worktree 경로 URL로 진입 → 골격 표시 후 화면 내부에 검증 실패 상태가 표시된다.
6. `AW_PERF_LOG` 로그에서 `includeStatus=false` 경로일 때 worktree당 `git status` 실행이 없는지 확인. 프로젝트 상세 화면의 clean/dirty badge는 기존과 동일해야 한다(회귀 확인).

### S2. Idle 되먹임 없음 (SC-003 / User Story 2)

1. 세션 화면을 연 채 10분간 아무 작업도 하지 않는다.
2. **기대**: `AW_PERF_LOG`의 watcher 로그에 `kind=git` 이벤트가 0회, history/graph refetch가 interval 외 요인으로 발생하지 않는다.
3. 외부 터미널에서 해당 worktree에 `git status` 실행 → 여전히 이벤트 0회(`--no-optional-locks` 적용 확인은 앱 내부 실행 기준이므로, 외부 실행이 유발하는 index 변화는 `.git/index` 무시 규칙으로 걸러져야 한다).

### S3. Agent 실행 중 반응성 (SC-004 / User Story 2)

1. agent에게 다수 파일을 수정하는 작업을 실행시킨다.
2. **기대**: 실행 중 prompt 입력·스크롤이 프리즈 없이 동작하고, Git 탭만 열려 있을 때 파일 목록 전체 rescan이 반복 실행되지 않는다(watcher 로그에서 file 이벤트 빈도와 refetch 대상 확인).
3. 빌드 산출물 디렉터리(`dist` 등)만 변경되는 상황(빌드 실행) → watcher 이벤트 미발행.
4. 파일 변경 직후 500ms 내 추가 변경 후 정지 → 마지막 변경이 파일 목록/미리보기에 반영된다(trailing debounce 유실 없음).
5. 외부에서 commit 생성 → `kind=git` 이벤트 1회, graph가 갱신된다(세분화 후에도 정상 감지 회귀 확인).

### S4. Git 탭 선택 view 우선 (SC-005 / User Story 3)

1. 세션 진입(기본 graph view).
2. **기대**: `AW_PERF_LOG`에 `list_worktree_git_history` 실행이 없다.
3. List로 전환 → commit 목록 정상 로드. Graph로 복귀 → 캐시로 즉시 표시.

### S5. 대형 저장소 확장성 (SC-006, SC-007 / User Story 4)

1. commit 5,000개 이상 저장소에서 graph를 끝까지 스크롤하며 페이지를 이어 로드.
2. **기대**: `AW_PERF_LOG`에서 뒤 페이지의 `run_ms`가 첫 페이지의 2배를 넘지 않고, `rev-list --count`/`for-each-ref`가 첫 페이지 이후 실행되지 않는다.
3. 1,000+ commit 로드 후 스크롤/commit 선택 반응성 유지(virtualization 적용 확인: DOM row 수가 viewport 수준으로 제한).
4. 스크롤 도중 외부에서 `git rebase`로 이력 변경 → 다음 페이지 요청이 실패로 남지 않고 목록이 초기화되어 다시 로드된다(`cursorInvalidated` 폴백).
5. 파일 수만 개 저장소에서 Files/Markdown 탭 진입 → 목록 표시가 수 초 내 완료되고, Files 탭은 폴더 펼침 시 하위가 로드된다.

### S6. 기존 기능 회귀 없음 (SC-008)

- commit 선택 → 상세/파일 diff 표시 정상.
- Markdown 탭 annotation 작성 → agent prompt 전송 정상.
- 프로젝트 상세 화면 worktree 카드의 clean/dirty/prunable badge와 삭제 동작 정상.
- git-explorer 앱 기동·history/graph/working tree 화면 정상(`git-core` 계약 변경 영향 확인).

## 완료 기준 요약

| SC | 측정 방법 | 목표 |
|---|---|---|
| SC-001 | `session:shell-rendered` measure | < 1s (worktree 10+) |
| SC-002 | perf 로그 `wait_ms` 분포 | 직렬 대기 소멸 |
| SC-003 | watcher 로그 10분 관찰 | git 이벤트 0회 |
| SC-004 | S3 수동 관찰 | 프리즈 없음 |
| SC-005 | perf 로그 command 목록 | history 미실행 |
| SC-006 | perf 로그 페이지별 `run_ms` | 첫 페이지 2배 이내 |
| SC-007 | S5 수동 관찰 + DOM row 수 | 반응성 유지 |
| SC-008 | S6 + 자동 검증 전체 통과 | 회귀 없음 |
