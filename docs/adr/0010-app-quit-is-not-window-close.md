---
status: accepted
date: 2026-09-28
---

# 앱 종료는 창 닫기가 아니다: 닫기 의도는 종료 의도보다 먼저 온 그 창의 `CloseRequested`다

ADR 0005는 창 닫힘을 작업대 닫기로 정했다. 043까지는 창의 `Destroyed`마다 작업대를 닫았고, 앱 종료 경로도 `close_all_benches`를 불렀다. 그래서 앱을 끄면 모든 run이 끝났다. 044에서 서버가 run을 계속 소유하려면 "사용자가 이 창을 닫으려 했다"와 "앱이 끝나면서 창을 걷어 냈다"를 구별해야 한다.

실제 앱(macOS)에서 종료 경로마다 이벤트 순서를 관측했다(044 research R8-spike).

- 빨간 버튼, `Window > Close Window` 메뉴, 마지막 창 닫기: 그 창의 `CloseRequested`가 `Destroyed`보다 먼저 온다.
- 앱 메뉴 Quit(Cmd+Q), Dock Quit, AppleScript `quit`: `RunEvent::Exit`만 오고, 그 전에 창 이벤트가 없다.
- `SIGTERM`: 이벤트가 없다.

그래서 닫기 의도는 종료 의도 표시(`ExitRequested`·`Exit`에서 섬)가 서기 **전에** 온 그 창의 `CloseRequested`다. `Destroyed`는 닫기 의도가 있으면 `desktop.retireWindow{closeBench:true}`, 없으면 `{closeBench:false}`(토큰·표만 폐기)를 부른다. 외부 서버 모드의 종료 경로는 `close_all_benches`를 부르지 않는다. 대기 중인 폐기를 흘려보내고 임대만 푼다(상한 2초). `SIGTERM`은 앱 처리가 없고 임대 TTL로 거둔다.

## Considered Options

- 순서와 무관하게 `Destroyed`마다 닫기 — 오늘의 동작이다. 앱 종료가 run을 끝내 5단계 (a)가 성립하지 않는다.
- 앱 종료 때 모든 창을 먼저 표시하고 이후 창 이벤트를 무시 — 관측 전에는 종료 신호가 창 이벤트보다 먼저 온다는 보장이 없다. 이 결정은 관측한 순서에 기대고, 관측을 판정 함수(`window_close_intent`)의 시험으로 고정한다.
- 종료 메뉴 항목을 직접 처리해 더 이른 신호를 세움 — 관측한 (c)(d)(e)에서 `CloseRequested`가 종료 신호보다 먼저 오지 않아 필요 없었다. 그런 경로가 새로 관측되면 이 수단이 필요하다.

## Consequences

- 실제 앱 스모크로 확인했다(044 `reviews/app-smoke.md`). (c)(d)(e)(g) 뒤 앱 PID가 사라져도 소유자 클라이언트로 같은 run을 이어 보고 취소했다. (a)(b1)(f)에서는 그 작업대의 run이 제거되고 같은 창 토큰이 닫기 뒤 401이었다.
- 위험: 자동화한 Cmd+W 키 입력(Settings가 앞)은 `CloseRequested`를 두 창에 내고 두 창을 모두 닫았다. 원인을 확인하지 못했고, 사람이 누른 Cmd+W는 확인하지 않았다.
- 관측하지 못한 종료 경로(로그아웃·재시동)와 Windows·Linux의 이벤트 순서는 이 결정의 근거에 없다. 그 플랫폼에서 순서가 다르면 판정 함수와 시험을 다시 봐야 한다.
- `Exit` 뒤에 창 이벤트가 오더라도 종료 의도가 이미 서 있어 작업대를 닫지 않는다.
