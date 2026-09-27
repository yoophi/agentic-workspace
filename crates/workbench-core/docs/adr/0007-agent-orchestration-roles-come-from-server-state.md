---
status: accepted
date: 2026-09-27
---

# agent orchestration 역할은 토큰 주장이 아니라 서버 상태에서 나온다

040까지 MCP 토큰은 발급 시점의 역할 주장(coordinator/child, 작업 영역, 창 label, 세대)을 담았고 도구 권한은 그 주장을 믿었다. 041부터 토큰은 **run 하나만** 가리키고, agent operation마다 서버가 자기 상태로 역할을 정한다: coordinator = 작업대에 묶인 작업 영역의 활성 세대 run, 자식 = coordinator가 만든 자식 노드의 현재(또는 기동 중) run. 수동 채택 자식은 역할이 없다. 작업 영역이 어느 작업대에도 묶이지 않았으면 `scopeMismatch`, 역할이 도구와 맞지 않으면 `forbiddenActor`다(오늘 문구). `tools/list`도 요청 시점 역할(`orchestration.getAgentRole`)로 고른다.

## Considered Options

- 토큰 주장 유지 + 교대·재시도 때 폐기 — 폐기 시점을 놓치면 이전 세대 coordinator나 교체된 자식이 계속 권한을 가진다. 창 label이 계약에 남는다.
- 매 호출 서버 판정(채택) — 조회 한 번이 더 들지만 교대·재시도·작업대 닫기가 즉시 권한에 반영된다.

## Consequences

- 토큰 폐기(재시도·재배정·교대로 물러난 run)는 수명 관리일 뿐 권한 근거가 아니다.
- 자식의 첫 턴은 노드에 run이 기록되기 전에 올 수 있으므로 서버가 기동 중 run(launching 표)을 자식으로 인정한다.
- 검증: `tests/orchestration_agent.rs`(역할·거절·교대 뒤 이전 coordinator·첫 턴), 계약 fixture `orchestration-agent-*`.
