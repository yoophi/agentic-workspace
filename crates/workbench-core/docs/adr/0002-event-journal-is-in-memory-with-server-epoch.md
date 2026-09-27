---
status: accepted
date: 2026-09-27
---

# 이벤트 journal은 메모리에 두고 서버 세대로 구분한다

Workbench 이벤트 스트림의 journal(상태 복원용 이벤트의 보관 구간)은 프로세스 메모리에만 둔다. 서버는 기동마다 새 세대(Server Epoch)를 만들고 모든 이벤트·cursor에 싣는다. 세대가 다른 cursor로 구독하면 gap을 돌려주며, 재시작 전에 진행 중이던 run은 "실행 정보 유실(runtime lost)"로 보인다. durable journal을 두지 않으므로 서버가 재시작되면 snapshot에 없는 agent 메시지·생각·diff 일부가 사라질 수 있고, 화면과 이후 CLI는 이를 유실 표시로 알린다.

## Considered Options

- SQLite durable journal(이미 있는 ledger 파일 옆) — 재시작 뒤 완전 replay가 가능하지만 run process 자체는 재시작을 넘어 살아남지 않으므로(process proxy 없음) 이어 붙일 실행이 없다. 보존 기간·용량·민감 정보 redaction 정책이 먼저 필요하다.
- 세대 없이 run별 순번만 유지(오늘) — 재시작 뒤 같은 순번이 다른 이벤트를 가리켜 클라이언트가 누락·중복을 판별할 수 없다.

## Consequences

- 한도: run당 512개(오늘 값), 보관 run 수가 상한을 넘으면 가장 오래전에 끝난 run부터 지운다. 진행 중 run은 지우지 않는다. 지워진 run을 구독하면 gap이다.
- 완전 replay가 필요해지면(서버 재시작을 넘는 run 재연결) process proxy와 함께 별도 설계로 durable journal을 추가한다. 정본 결정 7(2026-09-26).
