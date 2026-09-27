---
status: accepted
date: 2026-09-27
---

# run과 교환 작업 영역의 주인은 작업대(Bench)이며, 작업대는 연 principal에 묶인다

오늘은 창 label 하나가 run 소유자·교환 작업 영역 키·orchestration 바인딩·이벤트 전달 대상을 모두 맡는다. 창이 없는 호출자(CLI·TUI·HTTP)는 run을 시작하거나 제어할 수 없고, 창 label은 열 때마다 새로 만들어져 저장된 바인딩은 재시작 뒤 일치하지 않는다. 040(2b)에서 소유를 서버가 발급하는 **작업대**로 옮긴다. 작업대는 Worktree 하나를 대상으로 열리고(`bench.open`), 명시적으로 끝낼 때까지(`bench.close`) run과 교환 작업 영역을 소유한다. run·교환의 모든 제어 operation은 `benchId`를 받고 "대상 run이 이 작업대 소유인가"를 검사한다. 작업대는 연 principal을 기록해, `benchId`를 알아낸 다른 principal이 쓰지 못하게 한다. 창 label은 데스크톱 어댑터 안의 "창 → 작업대" 대응표에만 남는다.

## Considered Options

- 오늘처럼 창 label을 소유자로 두고 계약에만 싣기 — 창이 없는 호출자가 여전히 소유자가 될 수 없고, 창 label이 서버 계약에 새어 나간다.
- Client Instance(구독 연결)를 소유자로 쓰기 — 연결이 끊기면 소유도 끝나 정본 결정 8(연결 끊김 ≠ run 취소)을 어긴다.
- 작업대를 principal에 묶지 않기(`benchId`만 알면 사용) — 지금은 principal이 데스크톱 하나라 차이가 없지만, 3단계에서 principal이 여럿이 되면 규칙을 다시 바꿔야 한다.

## Consequences

- 오늘 소유 검사가 없던 run 제어 4종(프롬프트 전송·조향·현재 프롬프트 취소 후 전송·권한 모드 변경)과 run 취소에도 검사가 생긴다. 화면은 자기 창의 run만 제어하므로 보이는 변화는 없다.
- `acp-agent-core`의 run 소유자는 원래 불투명한 문자열이라 `benchId`를 그대로 넣는다(변경 없음, hushline·ask-code 영향 없음).
- 041(orchestration)은 workspace 바인딩을 창 label 대신 작업대로 옮긴다.
