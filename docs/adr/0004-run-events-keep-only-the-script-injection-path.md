---
status: accepted
date: 2026-09-27
---

# run 이벤트는 창 스크립트 삽입 경로 하나만 유지한다

오늘 run 이벤트는 Tauri 이벤트(`agent-run-event`)와 창 스크립트 삽입(`window.eval`로 `agent-run-event-fallback` CustomEvent 발생) 두 경로로 나가지만, 화면은 삽입 경로만 듣는다. Tauri 2(2.11.6에서 확인)의 `WebviewWindow`는 `Emitter`의 기본 `emit`을 그대로 쓰며, 이는 `manager().emit` — 창 하나가 아니라 **모든 대상에 방송**한다(창 지정은 `emit_to`뿐). 따라서 쓰이지 않는 Tauri 경로는 다른 창으로 run 이벤트가 새는 통로이기도 하다. 039는 서버 순번을 실은 봉투를 삽입 경로로만 보내고 Tauri `agent-run-event` 발행을 제거한다.

## Considered Options

- Tauri `emit_to(label)` + `listen`으로 전환하고 삽입 경로 제거 — 정석이지만 삽입 경로를 도입한 이유가 기록에 없고, 4단계(Desktop HTTP/WebSocket 전환)에서 전달 방식이 다시 바뀐다.
- 두 경로 유지 — 변경은 가장 적지만 이중 전달과 누출 통로가 남는다.

## Consequences

- orchestration·exchange·창 제목 이벤트도 같은 `window.emit` 방송을 쓴다(창 격리가 되지 않음). 2b에서 같은 기준으로 정리한다.
- 4단계에서 데스크톱이 WebSocket 구독자가 되면 삽입 경로도 제거한다.
