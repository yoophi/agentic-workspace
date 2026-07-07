# Quickstart: AW Git Commit 상세 한글 파일명 표시 수정 검증

## Prerequisites

- macOS 또는 로컬 desktop 개발 환경
- Git CLI 사용 가능
- pnpm workspace 의존성 설치 완료
- Rust toolchain 설치 완료

## Setup

```bash
pnpm install
```

## Validation Scenario 1: Rust Git core path parsing

목표: `git-core`가 한글 파일명을 표시 가능한 경로로 반환하는지 확인한다.

```bash
cargo test -p git-core korean
cargo test -p git-core commit_detail
```

Expected outcome:

- 한글 파일명 fixture가 `\\355\\202\\244` 같은 octal byte 표기를 반환하지 않는다.
- 기존 ASCII 파일명 fixture가 변경 없이 통과한다.
- rename 또는 mixed Korean/ASCII path fixture가 통과한다.

## Validation Scenario 2: AW Tauri backend contract

목표: AW backend command-service-provider 경계가 기존 구조를 유지하면서 shared Git core 결과를 전달하는지 확인한다.

```bash
cargo check -p agentic-workbench
```

Expected outcome:

- `get_worktree_commit_detail`와 `get_worktree_commit_file_diff` 경계가 컴파일된다.
- Tauri command가 parsing/business logic을 직접 보유하지 않는다.

## Validation Scenario 3: Shared Git UI display

목표: 공통 commit detail UI가 한글 경로와 긴 경로를 깨지거나 겹치지 않게 표시하는지 확인한다.

```bash
pnpm --filter @yoophi/git-ui test
pnpm --filter @yoophi/git-ui check-types
```

Expected outcome:

- `CommitDetailView` 또는 관련 file tree fixture가 한글 경로를 그대로 표시한다.
- list/tree 선택 상태가 display path와 같은 파일을 가리킨다.

## Validation Scenario 4: AW frontend integration

목표: AW worktree workspace 화면에서 commit detail 데이터 흐름이 깨지지 않는지 확인한다.

```bash
pnpm --filter @yoophi/agentic-workbench test
pnpm --filter @yoophi/agentic-workbench check-types
```

Expected outcome:

- worktree Git entity API와 workspace panel 타입 검사가 통과한다.
- 기존 commit selection, file selection, diff loading flow가 회귀하지 않는다.

## Manual Smoke Scenario

1. 한글 파일명 또는 한글 디렉터리가 포함된 repository commit을 준비한다.
2. AW에서 해당 worktree를 연다.
3. Git graph 또는 commit list에서 해당 commit을 선택한다.
4. Changed files 목록과 diff 영역을 확인한다.

Expected outcome:

- `\\355\\202\\244` 같은 octal byte 표기가 화면에 보이지 않는다.
- 한글 파일명과 경로가 사람이 읽을 수 있는 한글로 표시된다.
- 파일을 선택하면 같은 파일의 diff가 열린다.
- 영문 파일명 commit detail은 기존과 동일하게 동작한다.

## Validation Results

2026-07-07 implementation run:

- PASS: `cargo test -p git-core korean`
- PASS: `cargo test -p git-core commit_detail`
- PASS: `cargo test -p git-core`
- PASS: `cargo check -p agentic-workbench` (existing dead-code warnings only)
- PASS: `cargo check -p git-explorer`
- PASS: `pnpm --filter @yoophi/git-ui test`
- PASS: `pnpm --filter @yoophi/git-ui check-types`
- PASS: `pnpm --filter @yoophi/agentic-workbench test`
- PASS: `pnpm --filter @yoophi/agentic-workbench check-types`
- PASS: `pnpm --filter @yoophi/git-explorer check-types`
- PASS: app-to-app import/Tauri boundary check for touched AW Tauri files
- PASS: real temporary repository smoke via `cargo test -p git-core commit_detail_returns_readable_korean_file_paths_from_real_repo`
