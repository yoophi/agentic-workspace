# Data Model: 044

모두 서버 메모리 상태다. 데이터 디렉터리의 도메인 파일·ledger 형식은 바꾸지 않는다. 새로 생기는 파일은 `workbench/server/`의 잠금·안내 파일뿐이다(contracts/server-lifecycle.md §2).

## ServerInstance

| 필드 | 뜻 |
|---|---|
| `instanceId` | 기동마다 새 uuid. 안내 파일과 handshake가 같은 값이어야 붙는다 |
| `serverEpoch` | 런타임 세대(오늘과 같음) |
| `state` | `starting` → `serving` ↔ `draining{idle}` / `draining{wait}` → `stopping` |
| `ownerToken` | 소유자 자격 증명(안내 파일에만 저장. 서버 메모리에는 digest만) |
| `idleSince` | 유휴 조건이 시작된 시각 |

전이는 contracts/server-lifecycle.md §5. `draining{wait}`에서 `serving`으로는 돌아가지 않는다.

## Lease

| 필드 | 뜻 |
|---|---|
| `leaseId` | uuid |
| `clientKind` | `desktop`·`cli`·`test` |
| `clientId` | 클라이언트가 준 식별자(데스크톱: 앱 인스턴스 uuid) |
| `expiresAt` | 갱신마다 now+TTL. 지나면 제거 |

## ActiveWork (파생 값)

`busyRuns`(진행 중 turn·엔진 대기열 prompt·권한 대기가 있는 run. 세션 수가 아님), `orchestrationTasks`(배정된 진행 중), `queuedTasks`(비우기 시작 전에 만든 대기 task 중 **배정할 쪽이 있는 것** — coordinator run이 살아 있고, 바쁘거나 그 coordinator에게 미전달 알림이 있을 때. 배정할 쪽이 없는 준비 task와 비우기가 시작된 뒤 만든 준비 task는 `deferredTasks`로 보고만 한다. 구현 중 정책 변경, research R7 참조), `pendingNotifications`(활성 coordinator 세대의 살아 있는 coordinator에게 갈 알림 중 `pending`·`dispatching`인 것은 시도 수와 무관하게, 재시도를 기다리는 재시도 가능 실패는 `attemptCount < MAX_NOTIFICATION_ATTEMPTS_FOR_STOP`(3)일 때만. 넘은 것은 `stalledNotifications`로 보고만 하고 저장 상태는 바꾸지 않는다, research R7), `pendingExchanges`(전달 prompt 미소비 `send`/`queue` 교환, 데스크톱 임대가 있을 때만), `pendingOperations`(이 프로세스가 적용 중인 ledger `pending`), `acceptedCalls`(HTTP·MCP 분리 호출).

- 모두 0이면 wait 비우기가 `stopping`으로 간다.
- 모두 0이고 임대도 0이면 유휴 판정이 시작된다.
- ledger `unknown`은 포함하지 않는다(`unresolvedOperations`로 따로 보고, research R7).

## DrainClass

`Q | C | K(조건) | N`. `drain_class(OperationId, input)`(contracts/drain-classification.md).

## Principal 추가

| kind | 주체 | scope | 작업대 소유 판정 |
|---|---|---|---|
| `Owner`(신규) | `local:owner` | 전체 + `server:admin` | 우회 |
| `Desktop` 창(043) | `desktop:window:<label>:<incarnation>` | 오늘과 같음 | 자기 것만 |
| `Agent` | `agent:<runId>` | 오늘과 같음 | 자기 run의 작업대 범위 |

## EpochIdempotency 추가 (#207)

| 필드 | 뜻 |
|---|---|
| `closed_benches: HashSet<BenchId>` | 이 세대에 닫힌 작업대. `drop_bench`가 넣는다. `record`·실행 전 조회가 이 작업대 scope를 비어 있는 것으로 본다 |

## 데스크톱(메모리)

| 상태 | 뜻 |
|---|---|
| `closeIntent: Set<label:incarnation>` | 사용자가 닫으려 한 창(R8, spike 뒤 확정) |
| `quitting: bool` | 앱 종료 의도가 선 뒤 true |
| `lease` | 앱 인스턴스의 임대 id와 갱신 타이머 |

## 교환 전달 소비 (K)

| 필드 | 뜻 |
|---|---|
| `deliveryConsumed: Map<requestId, RunId>` | `continuation`으로 받은 전달 prompt. 교환마다 1회. 작업대가 닫히면 사라진다 |

## 창 폐기 tombstone

| 필드 | 뜻 |
|---|---|
| `retiredWindows: HashSet<label:incarnation>` | 세대 동안 폐기한 창 주체. 발급기가 같은 잠금 아래에서 발급을 거절한다 |

## RunActivity (운영용)

run별 `{turnInProgress, queuedPrompts, permissionWaits}`. run 이벤트로 유지한다. 모두 0이면 쉬는 세션이다(활성 작업 아님).
