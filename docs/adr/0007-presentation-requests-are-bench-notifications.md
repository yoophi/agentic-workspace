---
status: accepted
date: 2026-09-27
---

# 창 제목 같은 표현 요청은 작업대 알림 스트림으로 데스크톱에 보낸다

MCP 제목 도구는 오늘 창 label을 찾아 네이티브 이벤트(모든 창에 방송)와 창 삽입 두 경로로 제목을 보낸다. 창 제목은 데스크톱 표현 상태라 서버가 저장하지 않는다. 040에서 이를 **표현 요청**으로 모델링한다: operation `bench.requestTitle`(scope `presentation:write`)은 서버 상태를 바꾸지 않고 알림용 스트림 `bench:<benchId>`에 `bench.titleRequested.v1`을 발행하며, 데스크톱은 작업대 → 창으로 찾아 창 삽입 경로 하나로 전달한다.

## Considered Options

- 제목 도구를 AW에 두고 창을 직접 찾기 — 가장 작지만 방송 결함이 남고 MCP가 옮겨갈 때 다시 해야 한다.

## Consequences

- 표현 상태는 계속 Workbench 밖이다. 서버는 "요청이 있었다"는 사실만 알린다.
- 같은 스트림에 앞으로 다른 표현 요청(포커스·알림 등)을 추가할 수 있다.
