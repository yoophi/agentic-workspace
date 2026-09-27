---
status: accepted
date: 2026-09-28
---

# 데스크톱 창은 창마다 별도의 principal(창 주체)이다

042까지 데스크톱의 모든 호출은 principal `desktop` 하나였다. 창 사이 격리는 호환 command가 창 label로 작업대를 찾아 넣는 것으로만 보장했다. 043에서 화면이 네트워크 경로로 직접 `benchId`를 싣고 호출하면, 한 창의 자격 증명으로 다른 창의 작업대를 조작할 수 있게 된다. 그래서 창마다 주체 `desktop:window:<label>:<incarnation>`을 둔다. 작업대 소유(workbench-core ADR 0004)가 주체로 갈리므로, 격리는 서버의 기존 소유 판정이 강제한다. incarnation은 창을 만들 때 발급하고 창이 닫히면 거둬들인다. 이때 그 주체의 토큰과 쓰지 않은 이벤트 표도 모두 폐기한다. 호환 경로 command도 같은 창 주체로 부른다.

## Considered Options

- 주체는 `desktop` 하나로 두고, 서버가 요청의 창 label을 따로 검사 — 창 label이 서버 계약에 새어 들어가고(ADR 0004가 막은 것), 네트워크 요청의 label은 호출자가 지어낼 수 있다.
- 토큰만 창별로 나누고 주체는 공유 — 토큰을 폐기해도 작업대 소유가 갈리지 않아, 살아 있는 다른 창의 토큰이 그 작업대를 쓸 수 있다.
- incarnation 없이 label만 주체로 — 같은 label로 다시 연 창(설정 창 등)이 옛 창의 미만료 토큰과 작업대를 물려받는다.

## Consequences

- 세대 범위 멱등 기록의 범위가 주체별이다. 같은 창의 새로고침은 같은 주체라 멱등 재생이 유지되지만(교환 전달 키 1회 전달의 전제), 다른 창에서 같은 키를 쓰면 별개 요청이다.
- 창이 닫힌 뒤 늦게 도는 command는 주체가 없어 거절된다. 새 주체를 만들어 내지 않는다.
- Tauri async command는 인자를 future 안에서 추출하고 `Window`에 label 말고 식별자가 없다. 그래서 label을 다시 쓰는 창(`settings`, `main`)의 늦은 command는 새 창 주체로 풀릴 수 있다. 이 창들의 호출은 작업대와 무관하고, 작업대에 묶인 session 창 label은 재사용되지 않는다.
