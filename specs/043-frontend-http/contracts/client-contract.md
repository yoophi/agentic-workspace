# Contracts: 043 화면 클라이언트

서버 네트워크 계약은 042(`specs/042-workbench-http/contracts/workbench-http.md`) 그대로다. 이 문서는 화면이 지키는 클라이언트 규칙과 새 Tauri command·주체 규칙을 정한다.

## 1. 창별 데스크톱 주체

- subject `desktop:window:<label>:<incarnation>`, kind `desktop`, scope = 오늘 데스크톱 scope. incarnation은 창 생성 때 만든 uuid로 창 수명 동안 고정이다. 창 `Destroyed`에서 incarnation과 그 주체로 발급한 토큰을 모두 폐기한다 — 같은 label로 다시 연 창은 새 incarnation이므로 옛 토큰(폐기 전이라도)이 새 작업대를 조작할 수 없다.
- `get_workbench_connection`이 발급하는 토큰은 호출 창의 출처와 이 주체에 묶인다.
- 세션 창의 작업대는 그 창의 주체가 연다(`desktop_benches::ensure`). 호환 경로 command도 모두 호출 창의 주체로 부른다(작업대 범위 command가 소유 판정을 통과하려면 필수).
- 결과: 다른 창의 자격 증명으로 작업대 조작(`bench.*`, `run.*`, `exchange.*`, `orchestration.*`)·구독(`run:`·`exchange:`·`bench:`·`orchestration:`)은 서버의 소유 판정으로 거절된다(오늘 문구). 전역 데이터(프로젝트·prompt·목표·설정·Git·파일·agent 목록)는 창과 무관하다.

## 2. 새 Tauri command

| command | 입력 | 출력 | 규칙 |
|---|---|---|---|
| `get_workbench_connection` | — | `{baseUrl, token, expiresAt}` | 042. 토큰 주체가 창별로 바뀐다 |
| `ensure_window_bench` | `{hint?: string}` | `{benchId}` | 세션 창만. 작업 디렉터리 = 창 경로 또는 hint. 닫힌 창은 오늘 문구로 거절 |
| `declare_network_delivery` | — | — | 이 창(incarnation)에 앱 내부 이벤트 전달(fallback·Tauri emit)을 끈다. 창이 닫히면 해제 |

## 3. 경로 선택

부팅: `get_workbench_connection` → handshake(`supportedProtocolVersions: [1]`) 성공 → `declare_network_delivery` → 네트워크 경로. 어느 단계든 실패 → 호환 경로, 진단 기록 `[workbench-client] using compat path: <reason>`. 이후 전환 없음.

## 4. 호출 규칙

- 판정 원칙: `notApplied`는 클라이언트가 스스로 보내지 않은 경우(연결 상태가 이미 끊김)만. 보내기를 시도한 뒤의 모든 실패(`fetch` 거절 포함)는 `unknown`.
- 조회: 연결 없으면 보내지 않고 `notApplied`. 응답 유실이면 재연결 뒤 새 요청으로 다시(조회는 안전).
- 변경: 멱등성 키는 사용자 조작 하나당 하나. 연결 없으면 보내지 않고 `notApplied`. 응답 유실이면 재연결 handshake의 `serverEpoch`가 보낼 때와 같으면 같은 키로 **한 번** 재시도(retryable conflict는 짧게 반복, 상한 있음), 다르면 재전송 없이 `unknown` + 재조회 신호.
- `401`: 자격 증명 한 번 갱신 뒤 같은 요청 재시도(미적용이므로 안전).
- 오류 문자열: `faultToString(fault)` — 메시지 그대로, 교환 경로는 `{"code": details.exchangeCode ?? code, "message"}` JSON(호환 층과 같음).

## 5. 구독 규칙

- 스트림당 WebSocket 하나. 스트림당 큐 상한 1,024 — 넘으면 닫고 `appliedSequence`에서 재구독. 표 발급 cursor = `appliedSequence`(처음이면 호출자가 준 기준점, 없으면 0).
- `hello` 뒤 `live`. 순번 ≤ `appliedSequence` 프레임은 버린다.
- 수신자 콜백은 동기(Promise를 기다리지 않음). 수신자마다 `deliveredSequence`, 스트림 cursor = 붙은 수신자 최솟값. 다시 받은 프레임은 아직 받지 않은 수신자에게만 넘긴다. 콜백 예외는 기록하고 그 수신자에게 넘긴 것으로 친다(무한 재시도·다른 수신자 중복 없음). 수신자 0명이면 큐에 보관(유예 뒤 해제).
- gap 사유·스트림별 복구(research R8 대응표): run은 `run.replay` 스냅샷(기준점) → 기준점 뒤 구독, orchestration은 구독(hello) → `orchestration.get` → 스냅샷 `revision` 이하 이벤트 버림·이후는 재조회 트리거, 교환은 구독(hello) → `exchange.list` → `requestId` 멱등 upsert(늦은 `updatedAt`만), 알림은 구독 → 재조회, 세대 변경은 창 전체 재동기, 지연은 같은 cursor로 재연결.
- 연결 종료(서버 닫음·오류)는 재연결 루프(backoff 250ms→10s, jitter).

## 6. 연결 상태 표시

`reconnecting`·`disconnected`일 때만 보이는 표시(새 문구). `connected`면 숨김.
