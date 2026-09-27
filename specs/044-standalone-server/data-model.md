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

`runs`(진행·예약), `permissionWaits`, `orchestrationTasks`(진행 중), `pendingExchanges`(확인 전 + 대상 run 살아 있음), `pendingOperations`(이 프로세스가 적용 중인 ledger `pending`), `acceptedCalls`(HTTP·MCP 분리 호출).

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
