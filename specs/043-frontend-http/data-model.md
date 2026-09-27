# Data Model: 043

## 연결 정보 (Connection)

| 필드 | 설명 |
|---|---|
| `baseUrl` | 루프백 끝점 |
| `token` | 짧은 데스크톱 자격 증명 — 창 출처와 **창 주체**(`desktop:window:<label>:<incarnation>`)에 묶임. 창이 닫히면 폐기 |
| `expiresAt` | 만료 시각. 80% 지점에 갱신 |

## 창 경로 (WindowTransport)

`http | compat` — 창 부팅 때 한 번 정해지고 바뀌지 않는다. `http`면 `declare_network_delivery`를 마쳤다.

## 연결 상태 (ConnectionState)

```mermaid
stateDiagram-v2
    [*] --> connecting
    connecting --> connected: handshake 성공
    connected --> reconnecting: 호출·구독 연결 실패
    reconnecting --> connected: handshake 성공(같은 세대)
    reconnecting --> resyncing: handshake 성공(새 세대)
    resyncing --> connected: 작업대 재수령·재조회·재구독 완료
    reconnecting --> disconnected: 시도 중 사용자 조작 발생 시 표시상 끊김(재시도는 계속)
    disconnected --> connected: handshake 성공
```

`disconnected`·`reconnecting` 동안의 변경 호출은 보내지 않는다(notApplied).

## 호출 결과 (CallOutcome)

| 종류 | 조건 | 화면 |
|---|---|---|
| `ok` | 응답 수신 | 결과 |
| `fault` | 서버 fault | 오늘 문자열(`faultToString`) |
| `notApplied` | 보내기 전 연결 없음 | "적용되지 않음" 문구 |
| `unknown` | 보낸 뒤 응답 유실 + (새 세대 또는 재시도 실패) | "적용 여부 불명" 문구 + 재조회 |

변경 호출은 `{operation, input, idempotencyKey, sentEpoch}`를 응답까지 보존한다. 같은 세대 재연결이면 한 번 재시도.

## 구독 상태 (StreamSubscription)

| 필드 | 설명 |
|---|---|
| `streamId` | `run:<id>` · `exchange:<bench>` · `bench:<bench>` · `orchestration:<binding>` · `worktree:<path>` |
| `epoch` | 받은 세대 |
| `cursor` | 붙은 수신자 `deliveredSequence` 최솟값 — 재연결 표 cursor |
| `backlog` | 수신자 0명 동안 받은 프레임(상한 1,024) |
| `recovery` | 보관 gap 복구 중: `{ baseline L, buffer[], attempt }` |
| `state` | `connecting`(표·hello 전) · `live` · `recovering` |

## 수신자 (StreamListener)

| 필드 | 설명 |
|---|---|
| `callback` | `(event) => void \| Promise<void>` — settle까지 기다림 |
| `deliveredSequence` | 이 수신자가 반영을 마친 순번 |
| `queue` | 이 수신자에게 넘길 프레임(순서대로 하나씩) |
| `state` | `ready` · `busy` · `failed`(스냅샷 재동기 중) |

```mermaid
stateDiagram-v2
    [*] --> ready
    ready --> busy: 프레임 넘김
    busy --> ready: 이행(deliveredSequence 전진)
    busy --> failed: 거절·동기 예외
    failed --> ready: 스냅샷 재동기 성공(deliveredSequence = 기준점)
    failed --> failed: 재동기 실패(backoff, 상한 뒤 화면 오류)
```

## 교환 원장 (ExchangeLedger, 창 메모리)

| 필드 | 설명 |
|---|---|
| `requestId` | 교환 요청 id |
| `routedAt` | 대상 패널에 라우팅한 시각(없으면 미라우팅) |
| `ackedOutcome` | 서버가 확인한 ack 결과(`delivered`·`rejected`, 없으면 미확인) |

교환 prompt의 run 전송 멱등성 키: `exchange-delivery:<requestId>`.

## 창 작업대 (WindowBench)

세션 창 하나 ↔ 작업대 id 하나. `ensure_window_bench`로 받는다. 새 세대면 다시 받는다. 닫기는 Rust가 창 Destroyed에서.
