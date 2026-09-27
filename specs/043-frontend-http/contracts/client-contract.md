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
| `get_workbench_connection` | — | `{baseUrl, token, expiresAt, incarnation}` | 042. 토큰은 창 주체(현재 incarnation)에 묶인다. 등록이 없는 창(닫힘)은 거절 |
| `ensure_window_bench` | `{open: boolean, hint?: string \| null}` | `benchId \| null` | `open`이면 없을 때 창 주체로 연다(작업 디렉터리 = 창 경로 또는 hint, 호환 command의 `ensure`). `open`이 아니면 있을 때만(`lookup`, 없으면 `null`) |
| `declare_network_delivery` | `{incarnation}` | — | 창의 **현재** incarnation일 때만 받아들인다. 그 창에 앱 내부 창 삽입 전달을 끈다(창 제목 적용은 계속). 창이 닫히면 그 incarnation의 선언만 해제 |

## 3. 경로 선택

부팅: `get_workbench_connection` → handshake(`supportedProtocolVersions: [1]`) 성공 → `declare_network_delivery` → 네트워크 경로. 어느 단계든 실패 → 호환 경로, 진단 기록 `[workbench-client] using compat path: <reason>`. 이후 전환 없음.

## 4. 호출 규칙

- 판정 원칙: `notApplied`는 클라이언트가 스스로 보내지 않은 경우(연결 상태가 이미 끊김)만. 보내기를 시도한 뒤의 모든 실패(`fetch` 거절 포함)는 `unknown`.
- 조회: 연결 없으면 보내지 않고 `notApplied`. 응답 유실이면 재연결 뒤 새 요청으로 다시(조회는 안전).
- 변경: 멱등성 키는 사용자 조작 하나당 하나. 연결 없으면 보내지 않고 `notApplied`. 응답 유실이면 재연결 handshake의 `serverEpoch`가 보낼 때와 같으면 같은 키로 **한 번** 재시도(retryable conflict는 짧게 반복, 상한 있음), 다르면 재전송 없이 `unknown` + 재조회 신호.
- `401`: 자격 증명 한 번 갱신 뒤 같은 요청 재시도(미적용이므로 안전).
- 오류 문자열: `faultToString(fault, flavor)` — compat Rust와 같다. 기본은 메시지 그대로. 교환은 `details.exchangeCode`가 **있을 때만** `{"code","message"}` JSON(없으면 메시지). orchestration은 `details.orchestrationError`가 있으면 그 JSON.

## 5. 구독 규칙

- 스트림당 WebSocket 하나. 표 발급 cursor = 수신자 `deliveredSequence` 최솟값(처음이면 호출자가 준 기준점, 없으면 0).
- 수신자: `(event) => void | Promise<void>`. Promise는 settle까지 기다린다. 수신자마다 순서대로 하나씩. 이행 = 반영 완료(`deliveredSequence` 전진), 거절·동기 예외 = 그 수신자만 실패 → 그 수신자 스냅샷 재동기 후 기준점으로 전진, 다른 수신자는 계속. 이미 넘긴 순번은 그 수신자에게 다시 넘기지 않는다.
- 수신자 0명 동안은 스트림 대기열(상한 1,024 — 넘으면 닫고 cursor에서 재구독). 해제는 마지막 수신자가 떠나고 유예 뒤.
- `hello`는 복구 성공 신호가 아니다. gap은 언제 와도 절차를 다시 시작한다.
- 보관 범위 gap(`retentionExceeded`·`evicted`·`unknownStream`): ① gap의 `lastSequence`로 새 표(live 확보) ② 도착 이벤트 버퍼 ③ 스트림별 스냅샷(run `run.replay`, orchestration `orchestration.get`, 교환 `exchange.list`) ④ 스냅샷을 초기화로 넘기고 버퍼를 기준으로 걸러 이어 넘김(run 순번, orchestration `revision`, 교환 `requestId`+`updatedAt`). 재시도 최대 3회.
- 교환 재조정: 창 원장 `requestId → routed/acked`. 스냅샷의 `accepted` 교환(이 창 패널 대상): 라우팅 전이면 라우팅+ack, 라우팅 뒤 ack 미확인이면 ack만(서버 ack는 `requestId` 멱등). 트리거는 구독 시작(첫 hello, `resyncOnStart`), 같은 세대 재연결(`resyncOnReconnect`), 보관 gap, 수신자 재동기다. 교환 prompt의 run 전송(대기열 전송·즉시 전달의 run 시작)은 멱등성 키 `exchange-delivery:<requestId>`.
- 교환 병합: 스냅샷에 같은 requestId가 있으면 그보다 늦은(`updatedAt`) 상태 이벤트만 넘긴다. 요청 이벤트는 스냅샷에서 아직 `accepted`일 때만 넘긴다.
- 재설정은 수신자마다 스냅샷을 읽는다(수신자 둘이면 조회 둘). 재연결 hello의 재조정과 뒤이은 gap 복구가 같은 스냅샷을 두 번 적용할 수 있다 — 수신자는 멱등하게 반영한다.
- 알림 스트림(worktree): 보관이 없어 끊긴 사이 알림이 사라진다 — 재연결마다 전체 다시 읽기 신호(`{kind: "git", reason: "resync"}`)를 보내 화면이 파일 목록·변경·Git 이력을 무효화한다. `epochChanged`: 창 전체 재동기(기본: 창 다시 불러오기). `subscriberLagged`: 같은 cursor로 재연결. 연결 종료: 재연결 루프(backoff 250ms→10s, jitter).

## 6. 연결 상태 표시

`reconnecting`·`disconnected`일 때만 보이는 표시(새 문구). `connected`면 숨김.
