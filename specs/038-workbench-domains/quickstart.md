# Quickstart: 038 나머지 도메인 이관 검증

037 [quickstart.md](../037-workbench-seam/quickstart.md)의 전제(Rust stable, pnpm, Node 22, `pnpm install --frozen-lockfile`)를 따른다. 워크트리: `/Users/yoophi/project/worktrees/038-workbench-domains`.

## 1. Rust: 계약·core·AW 검증

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p workbench-core --lib -- -D warnings            # test-hooks 없는 프로덕션 빌드
cargo test --workspace --all-targets
```

기대: 실패 0. 새 테스트 묶음이 보여야 한다 —
- `workbench-protocol`: `openapi::committed_openapi_matches_registry`(32 variant), DTO 스키마 유닛.
- `workbench-core` 유닛: `dto::wire_parity::*`(도메인↔DTO JSON 동일), `intent_first::*`, `json_collection_store::*`, 도메인 서비스 이동 테스트(기존 AW 테스트 그대로).
- `workbench-core` 통합: `contract_suite`(fixture 전부 × in-memory·HTTP), `ledger_crash_points`(037 3지점 + 038 시나리오 7건, `goal.create` 교체 upsert 포함), `concurrency`(같은 저장 단위 20건), `revision_retention`(aggregate 4개), `recovery_under_lock`(저장 단위 4개), `git_reconcile`(종료 상태 규칙), `reservation_lifecycle`(만들고 지우고 다시 만들기·실패 뒤 재시도·같은 경로 동시 생성·v1 ledger 승격).
- `agentic-workbench`: `workbench_compat` 변환 테스트 29+건, 기존 command 테스트.

## 2. 계약 생성물과 drift 검사

```bash
pnpm run generate:contracts
git status --short crates/workbench-protocol/openapi packages/workbench-client/src/generated   # 변경 없음
```

drift 검출 확인: `crates/workbench-protocol/src/operations/goal.rs`에서 필드 이름 하나를 바꾸고 다시 생성 → `git diff --exit-code -- crates/workbench-protocol/openapi packages/workbench-client/src/generated`가 실패해야 한다. 되돌린다.

## 3. TypeScript

```bash
pnpm --filter @yoophi/workbench-client check-types
pnpm --filter @yoophi/workbench-client test
pnpm check-types
```

기대: `operation-map.test-d.ts`에 32키 union·도메인별 상관 타입·`goal.get` nullable·`@ts-expect-error` 테스트가 통과.

## 4. 앱 수동 확인(무회귀)

`pnpm --filter agentic-workbench tauri dev`로 실행하고 다음을 각각 이전 버전과 비교한다.

| # | 조작 | 기대 |
|---|---|---|
| 1 | 프로젝트 이름 수정 → 삭제 | 목록 갱신, 빈 이름은 `Project name is required.` |
| 2 | saved prompt 추가·수정·삭제 | 목록 순서·내용 동일, 빈 label은 `Button label is required.` |
| 3 | 세션에서 목표 생성 → run 종료 시 진행 기록 → 목표 지우기 | 진행 수치 누적, 지운 뒤 `goal.get`이 없음 |
| 4 | 설정에서 agent·모델·권한 모드 저장 후 재기동 | 그대로 복원. built-in 프로필 전부 비활성 시 문구 표시 |
| 5 | 프로젝트 화면: 원격·브랜치·worktree 목록, worktree 생성·삭제 | 이전과 같은 목록. 변경 있는 worktree 삭제 시 `Worktree has changes and cannot be deleted.` |
| 6 | worktree 창: 변경 파일·diff·파일 트리·텍스트 미리보기(512KB 초과 파일, 바이너리 파일 포함) | 동일 표시. `../` 경로는 UI에서 만들 수 없으므로 §1 fixture로 확인 |
| 7 | Git 이력·그래프·커밋 상세·파일 diff | 동일 표시, 페이지 커서 동작 |
| 8 | 새 run 시작 화면의 agent 목록, provider 세션 이어 붙이기 | 동일 목록·필터 |
| 9 | `sqlite3 "<app data dir>/workbench/ledger.sqlite" 'select max(version) from schema_version; select operation, state, count(*) from operation_ledger group by 1,2'` | 037이 만든 파일이 버전 **2**로 승격되어 있고, 위 조작의 변경이 `applied`로 기록됨 |
| 11 | 같은 worktree 경로를 만들고 → 지우고 → 다시 만들기 | 세 번 모두 성공(예약 해제 확인) |
| 10 | `shasum <app data dir>/{saved-prompts,goals,agent-run-settings}.json` 형식 | 이전 버전이 만든 파일을 그대로 읽고, 새 저장도 같은 키 집합 |

## 5. 종료 상태 규칙 수동 확인(선택)

`cargo test -p workbench-core --test git_reconcile -- --nocapture`가 다음을 출력하는지 본다: 생성 pending + 경로 존재 → applied, 생성 pending + 경로 없음 → unknown, 삭제 pending + 경로 없음 → applied.

## 6. 지연 측정(선택)

```bash
cargo test -p workbench-core --test list_latency -- --ignored --nocapture
```

`savedPrompt.list`·`goal.recordProgress`·`worktree.listFiles` p95가 각각 한 자릿수 ms(저장 단위 변경은 ledger 2 commit 포함)인지 기록한다(SC-001).

## 7. 문서

- `docs/workbench-seam.md`: 상태 줄, 범위·비범위, "command 인벤토리(71)" 표(data-model §6), 038에서 실제 적용된 절차, 종료 상태 규칙 링크(`crates/workbench-core/docs/adr/0001`).
- `docs/client-server-architecture-research.md`: 진행 상태 각주에 038 완료 추가.
- `CONTEXT-MAP.md`·`crates/workbench-core/CONTEXT.md`·ADR 3건은 이미 작성됨 — 구현 중 용어가 바뀌면 함께 갱신.

## 8. PR 전 체크

```bash
git diff --stat origin/main -- apps/agentic-workbench/src              # 0 파일
ls apps/agentic-workbench/src-tauri/src/domain | grep -E "saved_prompt|goal|agent_run_settings|git_|worktree_(change|file|git)|provider_session"   # 없음(재노출만 mod.rs)
ls apps/agentic-workbench/src-tauri/src/infrastructure | grep -E "json_(saved_prompt|goal|agent_run_settings)|git_cli_|fs_worktree_file|fs_provider_session"   # 없음
grep -c "Json.*Repository::from_app\|GitCli.*Provider\|FsWorktreeFileProvider\|FsProviderSessionRepository" apps/agentic-workbench/src-tauri/src/inbound/tauri_commands.rs   # 0
```

CI `validate`(quality.yml)가 drift 단계 포함 통과해야 한다.
