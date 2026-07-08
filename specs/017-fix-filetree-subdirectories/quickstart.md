# Quickstart: AW Worktree Session Files 하위 디렉터리 조회 수정 검증

## Prerequisites

- repository root에서 실행한다.
- `apps/agentic-workbench` dependencies가 설치되어 있어야 한다.
- 테스트 fixture 또는 실제 worktree에 다음 파일 구성이 있어야 한다.

```text
README.md
src/app.ts
src/deep/inner.ts
docs/app.ts
docs/한글 파일.md
```

## Static and Unit Verification

```bash
pnpm --dir apps/agentic-workbench check-types
pnpm --dir apps/agentic-workbench test
cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml worktree_file
```

**Expected outcome**:
- frontend type check가 통과한다.
- Files tree/preview 관련 tests가 루트 파일, 1단계 하위 파일, 다단계 하위 파일, 같은 basename, 공백/한글 경로, read failure 상태를 통과한다.
- Rust worktree file tests가 `list_worktree_files`, `read_worktree_text_file`, root escape rejection, nested relative path preservation을 통과한다.

## Storybook/UI State Verification

```bash
pnpm --dir apps/agentic-workbench storybook
```

**Expected outcome**:
- Files tree 또는 Worktree Session page story에서 하위 디렉터리를 펼칠 수 있다.
- `src/app.ts`, `src/deep/inner.ts`, `docs/app.ts`, `docs/한글 파일.md` 선택 시 preview header와 content가 선택 path와 일치한다.
- 읽기 실패 상태는 이전 파일 content를 새 선택 결과처럼 보여주지 않는다.

## Release-Path Manual Verification

릴리즈 빌드 또는 릴리즈에 준하는 Tauri 실행 환경에서 다음 사용자 흐름을 확인한다.

1. Worktree Session page를 연다.
2. Files 섹션에서 `README.md`를 선택한다.
3. `src`를 펼치고 `src/app.ts`를 선택한다.
4. `src/deep`를 펼치고 `src/deep/inner.ts`를 선택한다.
5. `docs/app.ts`와 `src/app.ts`를 번갈아 선택해 같은 basename이 경로별로 구분되는지 확인한다.
6. `docs/한글 파일.md`를 선택한다.
7. 읽을 수 없는 파일 fixture 또는 삭제된 파일 상태를 만들어 error/stale 상태에서 다른 정상 파일로 회복되는지 확인한다.

**Expected outcome**:
- 모든 정상 text file 선택은 3초 안에 해당 파일 content를 표시한다.
- preview header의 path는 선택 row의 path와 일치한다.
- 루트 파일과 하위 파일을 번갈아 5회 선택해도 선택 상태와 preview content가 어긋나지 않는다.
- 오류 상태는 Files 섹션 안에 한정되고 다른 session page 영역을 중단하지 않는다.

## Regression Boundaries

- agent 실행 panel, prompt input, Git 변경 목록, Markdown preview는 Files 하위 파일 조회 수정 때문에 동작이 바뀌면 안 된다.
- 파일 생성, 삭제, 편집 기능은 이번 검증 대상이 아니다.

## Verification Results

2026-07-08 구현 검증 결과:

- `pnpm --dir apps/agentic-workbench check-types`: 통과
- `pnpm --dir apps/agentic-workbench test`: 통과, 28 files / 146 tests
- `cargo test --manifest-path apps/agentic-workbench/src-tauri/Cargo.toml worktree_file`: 통과, 11 tests
- `pnpm --dir apps/agentic-workbench build`: 통과, Vite production build 기준 release-equivalent 정적 검증 완료

패키징된 Tauri 앱에서의 수동 클릭 검증은 별도 GUI 실행 환경에서 이어서 확인한다.
