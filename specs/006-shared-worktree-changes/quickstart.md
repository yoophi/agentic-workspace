# Quickstart: 검증 가이드 — 워킹 트리(미커밋) 변경사항 조회

**Feature**: 006-shared-worktree-changes | **Date**: 2026-07-02

## 사전 조건

- pnpm, Rust toolchain, git CLI 설치
- 저장소 루트에서 `pnpm install` 완료

## 1. 자동화 검증 (헌법 원칙 V — 원자적 교차 검증)

```bash
# 공유 코어: git-core 단위 테스트 (porcelain 파싱, 그룹핑, diff 폴백/상한)
cargo test -p git-core

# 소비 앱 Rust 테스트
cargo test -p git-explorer        # GE (worktree_status_service 포함)
cargo test -p agentic-workbench   # AW (git_worktree_changes_service 포함)

# 공유 패키지 + 앱 타입 체크
pnpm -r check-types
```

**기대 결과**: 전부 통과. (구현 시점 기준: git-core 7, GE 32, AW 104 테스트 통과)

## 2. 테스트 저장소 준비 (엣지 케이스 포함)

```bash
FIXTURE=/tmp/wt-fixture && rm -rf $FIXTURE && git init $FIXTURE && cd $FIXTURE
echo base > committed.txt && git add . && git commit -m init

echo staged > staged.txt && git add staged.txt          # staged (A)
echo modified >> committed.txt                           # unstaged (M)
echo new > untracked.txt                                 # untracked (??)
git mv committed.txt renamed.txt 2>/dev/null || true     # rename 확인용(선택)
head -c 200000 /dev/urandom | base64 > huge.txt          # 120KB 상한 초과용
printf '\x00\x01\x02' > binary.bin                       # 바이너리
```

## 3. git-explorer 수동 검증 (User Story 1)

```bash
pnpm --filter git-explorer tauri dev
```

1. 위 fixture 저장소를 등록하고 연다.
2. 상세 패널에서 **Working tree** 토글 선택.
   - ✅ Staged / Unstaged / Untracked 그룹별 파일과 카운트 배지 표시 (FR-001, FR-002, FR-011)
3. `staged.txt` 선택 → ✅ staged 변경인데도 diff가 표시됨 (`--cached` 폴백, edge case)
4. `huge.txt` 선택 → ✅ diff가 잘리고 잘림 안내 표시 (FR-007)
5. `binary.bin` 선택 → ✅ 바이너리 안내 표시 (FR-006)
6. 그래프에서 아무 커밋 클릭 → ✅ Commit 모드로 자동 복귀 (FR-008)
7. 변경을 모두 되돌린 깨끗한 저장소에서 Working tree 모드 → ✅ 빈 상태 안내

## 4. agentic-workbench 수동 검증 (User Story 2)

```bash
pnpm --filter agentic-workbench tauri dev
```

1. 미커밋 변경이 있는 워크트리 세션을 연다.
2. 워크트리 변경 리뷰 패널 → ✅ 그룹별 목록 + 파일 diff가 마이그레이션 이전과 동등하게 표시 (FR-009)
3. 워크스페이스 Git 탭 → ✅ Commit / Working tree 토글 동작 (FR-009)

## 5. 일관성·정본 검증 (User Story 3)

```bash
# 동일 fixture를 두 앱에서 열어 그룹핑·카운트·배지·rename 표기 비교 (SC-002)

# 미커밋 status/diff 파싱 구현이 git-core 한 곳뿐인지 확인 (FR-010, SC-004)
grep -rn "porcelain" apps/ crates/ packages/ --include='*.rs'
# 기대: crates/git-core/src/git_cli.rs 만 매치 (앱 자체 파서 없음)

# AW 구 구현 삭제 확인
ls apps/agentic-workbench/src-tauri/src/domain/git_worktree_changes_provider.rs 2>&1
ls apps/agentic-workbench/src-tauri/src/infrastructure/git_cli_worktree_changes_provider.rs 2>&1
# 기대: 둘 다 No such file
```

## 6. 잔여 작업 검증 (Storybook — 미완료 시)

```bash
grep -rln "WorktreeChangesView" --include='*.stories.*' packages/ apps/
# 현재: 매치 없음 → tasks.md의 Storybook 등록 태스크로 처리
```

## 오류 시나리오 확인

- 저장소가 아닌 경로/삭제된 저장소를 대상으로 조회 → ✅ 오류 메시지 표시, 앱 계속 동작 (FR-013)

## 참조

- 타입 상세: [data-model.md](./data-model.md)
- 인터페이스 계약: [contracts/tauri-commands.md](./contracts/tauri-commands.md)
