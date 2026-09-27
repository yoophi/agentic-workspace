---
status: accepted
date: 2026-09-27
---

# run·교환 변경 중 `run.start`만 변경 기록을 쓰고, 나머지는 세대 범위 멱등성을 쓴다

1단계 규칙은 "모든 변경 operation은 SQLite 변경 기록을 intent-first로 통과한다"이다. 그러나 run과 교환·작업대 상태는 메모리에만 있어 재시작하면 사라지므로, 상태와 기록을 같은 경계에 commit한다는 정본 불변식 6이 성립하지 않고 프롬프트마다 SQLite 쓰기만 늘어난다. 040에서는 되돌릴 수 없는 외부 효과(agent 프로세스 기동, Worktree 변경 가능성)를 가진 **`run.start`만** 변경 기록을 통과시킨다. 기동 시 `pending`으로 남은 `run.start`는 프로세스가 떴는지·무엇을 바꿨는지 확인할 방법이 없으므로 `unknown`으로 판정하고 그 run은 실행 정보 유실로 본다(종료 상태 규칙의 예외). 나머지 run 제어와 교환·작업대 operation은 같은 세대 안에서만 멱등성 키로 중복을 거르는 **세대 범위 멱등성**을 쓰며, 재시작 뒤에는 대상이 없어 `notFound`가 된다.

## Considered Options

- 모든 run·교환 변경이 변경 기록 통과 — 규칙은 하나지만 메모리 상태에 대해 거짓 durability를 약속한다.
- 멱등성 없음 — 3단계 HTTP 재시도에서 프롬프트가 두 번 들어갈 수 있다.

## Consequences

- operation descriptor가 멱등 규칙의 종류(영속·세대 범위)를 드러내야 한다.
- run 상태를 영속 저장소로 옮기는 단계(daemon 이후)가 오면 이 ADR을 대체한다.
