# Quickstart: Workbench Seam (037) 검증 가이드

**Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md) · **Contracts**: [contracts/](contracts/)

## 0. 전제

- 워크트리 `/Users/yoophi/project/worktrees/037-workbench-seam`, 브랜치 `037-workbench-seam`
- Rust stable(≥1.88; 로컬 1.98.1), Node 22, pnpm(corepack), clang(macOS 기본) — `rusqlite` bundled 컴파일용
- `pnpm install --frozen-lockfile` (새 package `@yoophi/workbench-client` 추가 후 lockfile 갱신 커밋 필요)

## 1. Rust: 계약·core·AW 검증

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p workbench-protocol -p workbench-core -p agentic-workbench
cargo test --workspace --all-targets          # CI와 동일
```

기대:
- `workbench-core` contract suite: fixture 전부가 in-memory·HTTP 두 경로에서 같은 결과(SC-002)
- `ledger_crash_points`: `pending` 직후 / JSON 저장 직후 / `applied` 직전 세 지점 중단 후 재시작 → reconciler 판정이 `applied`/`unknown` 계약과 일치(SC-003)
- `concurrency`: 서로 다른 키 20건 → 프로젝트 20개·revision 20; 같은 키 20건 → 프로젝트 1개(SC-004)
- `revision_retention`: 생성 3건 → 모든 ledger row 만료 처리 → GC → runtime 재생성 → 다음 생성의 revision이 4, `expectedRevision: 3`은 `preconditionFailed`(FR-010)
- `recovery_under_lock`: 손상된 `projects.json` + 정상 `.bak` 상태에서 read 1 + create N 동시 실행 → 최종 파일에 N개와 복구된 이전 프로젝트가 모두 있고, `applied`인데 사라진 프로젝트 0건
- `agentic-workbench`: `workbench_compat` 변환 테스트 통과, 기존 project 관련 테스트 무수정 통과(SC-001)

수동 벤치(SC-001 보조, CI 제외):

```sh
cargo test -p workbench-core --test list_latency -- --ignored --nocapture
```

## 2. 계약 생성물과 drift 검사

```sh
pnpm run generate:contracts
git diff --exit-code -- crates/workbench-protocol/openapi packages/workbench-client/src/generated
```

기대: diff 없음. 의도적으로 `crates/workbench-protocol/src/operations/project.rs`의 필드 이름을 바꾸고 다시 실행하면 두 번째 명령이 실패해야 한다(SC-005). 확인 후 되돌린다.

## 3. TypeScript: 상관 타입 컴파일 테스트

```sh
pnpm --filter @yoophi/workbench-client check-types
pnpm --filter @yoophi/workbench-client test
```

기대: `operation-map.test-d.ts`에서 `OperationMap["project.list"]["output"]`가 `Project[]`와 같고, `project.create`의 output을 `Project[]`로 다루는 줄이 `@ts-expect-error`로 잡힌다(SC-005). 프론트 앱은 이 패키지를 import하지 않으므로 `pnpm run check-types` 전체도 그대로 통과해야 한다.

## 4. 앱 수동 확인(무회귀)

```sh
cd apps/agentic-workbench && pnpm tauri dev
```

1. 프로젝트 대시보드에서 기존 프로젝트 목록이 이전과 같은 순서·내용으로 보인다.
2. 새 프로젝트 등록 → 목록에 나타나고, `~/Library/Application Support/<bundle id>/projects.json`이 이전과 같은 배열 형식이다(revision 필드 없음).
3. 같은 디렉터리에 `workbench/ledger.sqlite`가 생기고 `sqlite3 workbench/ledger.sqlite 'select operation,state,revision from operation_ledger'`에 `project.create | applied | N` 한 줄이 있다.
4. 이름을 비우고 저장 → 화면 오류가 정확히 `Project name is required.`(contracts/tauri-compat-commands.md 골든).
5. 프로젝트 수정·삭제가 이전과 같이 동작한다(037은 배선만 바꿈).
6. 앱 기동 시간이 체감상 늘지 않는다(reconciler는 `pending` 0건이면 즉시 끝난다).

## 5. 중단 복구 수동 확인(선택)

테스트 suite가 같은 시나리오를 자동으로 다루므로 필수는 아니다. 직접 보려면 `ledger_crash_points` 테스트의 fixture 디렉터리를 `--nocapture`로 출력해 `pending` row와 `projects.json`을 대조한다.

## 6. 문서

- `docs/workbench-seam.md`가 있고 Mermaid로 세 경로·Seam·상태 전이를 그린다.
- `docs/client-server-architecture-research.md` 상단에 "1a(037) 완료" 각주가 있다.

## 7. PR 전 체크

- [ ] §1~§3 명령이 모두 통과
- [ ] `git status`에 생성물(`openapi/workbench.openapi.json`, `src/generated/workbench.ts`)과 `pnpm-lock.yaml` 변경이 포함
- [ ] `.github/workflows/quality.yml`에 drift 단계 추가됨
- [ ] 프론트엔드 `apps/agentic-workbench/src/**` 변경 0건(`git diff --stat -- apps/agentic-workbench/src`)
