---
status: accepted
date: 2026-09-27
---

# MCP 도구는 agent principal로 `Workbench.call`을 거친다

MCP 교환·제목 도구는 오늘 AW 안에서 run → 창 label을 찾아 서비스를 직접 부른다. 040에서 교환 서비스가 core로 옮겨지면 데스크톱은 `Workbench.call`을 거치는데, MCP만 서비스를 직접 부르면 권한·멱등 규칙을 가진 두 번째 변경 경로가 생긴다. 그래서 run 실행 토큰을 **agent principal**(`PrincipalKind::Agent`, run id가 묶임, 필요한 scope만)로 바꾸고 MCP 도구도 `Workbench.call`을 부른다. 작업대는 호출한 run의 소유로 서버가 찾으며 agent는 `benchId`를 몰라도 된다. 도구 권한 규칙(run 일치 검사)과 오류 문구는 오늘과 같다.

## Considered Options

- MCP 도구가 core 서비스를 프로세스 안에서 직접 호출하고 조회 키만 바꾸기 — 변경은 가장 적지만 Seam을 우회하는 경로가 남는다.

## Consequences

- principal 종류가 하나 늘어난다. agent principal의 scope는 교환 쓰기와 표현 요청으로 제한된다.
- MCP 서버가 나중에 서버 쪽으로 옮겨가거나(7단계) 별도 프로세스가 되어도 호출 계약은 같다.
