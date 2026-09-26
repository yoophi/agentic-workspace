---
status: accepted
date: 2026-09-26
---

# Git·파일·provider 세션 어댑터는 workbench-core에 두고 git-core는 확장하지 않는다

`crates/git-core`는 AW와 git-explorer 두 앱이 쓰는 공유 crate로 커밋 이력·working-tree status·diff 리더를 제공한다. AW에는 그 밖의 Git CLI provider(원격·브랜치·worktree 목록/생성/삭제·inline diff 포함 변경 목록)와 파일시스템·provider 세션 어댑터가 있고, 이들의 모델(worktree 삭제 가능 여부·prune 이유·inline diff)은 AW 화면 형태에 맞춰져 있다. 038에서 이 어댑터들은 전부 `crates/workbench-core`의 infrastructure로 옮기고 `git-core`는 지금처럼 의존만 한다. 소비자가 하나뿐인 모델을 공유 crate에 넣지 않으며(헌장 I), git-explorer 재검증 범위를 PR에 끌어들이지 않기 위해서다(헌장 V).

## Considered Options

- Git provider만 `git-core`로 합치기 — Git 코드가 한곳에 모이지만 AW 전용 모델이 공유 crate에 들어가고 git-explorer가 검증 대상이 된다.
- 새 crate `workbench-git` — 소비자가 workbench-core 하나라 헌장 I의 공유 기준에 미달하고 경계만 늘어난다.

## Consequences

- Git 관련 Rust 코드가 `git-core`(두 앱 공용 리더)와 `workbench-core`(Workbench 어댑터) 두 곳에 있는 것은 의도된 배치다. "git 코드를 git-core로 합치자"는 제안은 이 ADR을 먼저 뒤집어야 한다.
- 두 번째 소비자(예: CLI가 다른 Git 모델을 요구할 때)가 생기면 그때 공용 부분만 `git-core`로 올린다.
