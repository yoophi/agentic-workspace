---
status: accepted
date: 2026-09-27
---

# 데스크톱은 run 이벤트를 구독하지 않고 발행 결과를 그대로 전달한다 (2a 한정)

039(2a)에서 run 이벤트 발행은 Workbench(core)로 옮기지만, 데스크톱 창은 `Workbench.events` 구독자가 되지 않는다. AW의 run sink가 core에 발행하고, 돌려받은 봉투(스트림·세대·순번 포함)를 지금처럼 대상 창에 보낸다. 창이 어떤 run을 받을지는 오늘처럼 sink를 만들 때의 창 label이 정한다.

## Considered Options

- 창마다 `Workbench.events`로 run 스트림을 구독 — 세 경로가 완전히 같은 경로를 쓰지만, "창이 어떤 run을 구독하는가"를 정하려면 run 소유를 창 label에서 분리하는 2b의 작업을 먼저 해야 한다.

## Consequences

- 데스크톱이 받는 이벤트는 구독자가 받는 것과 정체(스트림·세대·순번)가 같으므로 화면은 순번 추정을 하지 않는다.
- 2b에서 창 정체를 client instance로 분해하면 데스크톱도 구독자로 바꾸고 이 ADR을 대체한다.
- 040(2b) 갱신: 소유는 작업대로 분리되었지만([core ADR 0004](../../crates/workbench-core/docs/adr/0004-benches-own-runs-and-exchanges.md)) 데스크톱 전달 방식은 유지하기로 했다(grill Q5). 대상 창은 "작업대 → 창" 대응표가 정하고, 같은 방식을 교환 이벤트와 표현 요청에도 적용한다. 데스크톱을 구독자로 바꾸는 일은 4단계(HTTP 전환)에서 한다.
