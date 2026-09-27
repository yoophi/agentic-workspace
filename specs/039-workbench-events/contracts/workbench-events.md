# Contract: `Workbench.events` (039)

봉투·principal·fault 체계는 037 [workbench-call.md](../../037-workbench-seam/contracts/workbench-call.md)를 따른다.

## 1. 구독

`events(principal, Subscription{cursors})` → `EventStream` 또는 `WorkbenchFault`.

| 검사 | 실패 |
|---|---|
| cursor 수 1–64 | `invalidArgument` |
| 같은 스트림 중복(worktree는 실제 경로 기준, 별칭 포함) | `invalidArgument`("duplicate stream in subscription: <streamId>") |
| `streamId` 형식 `<kind>:<key>`, kind ∈ {run, worktree} | `invalidArgument`("stream kind is not available yet." — orchestration·exchange) |
| kind의 scope(`run:read`/`worktree:read`) | `forbidden` |
| 같은 세대에서 `afterSequence > last` | `invalidArgument`("cursor is ahead of the stream.") |
| worktree 경로 없음 | `notFound`("Cannot watch missing worktree path: <path>") |
| 동시 구독 256 초과 | `rateLimited` |

fault가 없으면 스트림은 cursor마다 research R2 표대로 replay/gap을 먼저 내고 live로 전환한다.

## 2. 순서 보장

1. 한 스트림 안에서 `sequence`는 빈틈없이 증가하고 한 구독에 같은 `sequence`는 한 번만 온다.
2. 구독 시작과 동시에 발행된 이벤트는 replay 또는 live 중 정확히 한 곳에서 한 번 온다.
3. 스트림 사이의 순서는 약속하지 않는다.
4. gap을 받은 스트림에는 그 구독에서 더 이상 이벤트가 오지 않는다(새로 구독해야 한다). `subscriberLagged`는 구독 전체를 닫는다.
5. 구독 중인 run 스트림이 보관 한도로 제거되면 그 스트림에 `Gap(evicted)`가 오고, 구독의 다른 스트림은 계속된다. 제거된 run id로 늦게 들어온 발행은 버려진다(순번이 1부터 다시 시작하지 않는다).
6. **알 수 없는 run과 제거된 run은 구별된다**: 제거 표식이 있으면 `Gap(evicted)`, 없고 cursor 0이면 시작 전 run으로 보고 live를 기다린다. 표식 상한(4,096)을 넘어 오래된 run은 다시 "알 수 없음"이 된다. 이전 세대 run은 cursor가 있으면 `Gap(epochChanged)`, cursor 0이면 알 수 없는 run과 같다(run 목록의 정본은 2b). 시작 전 run을 기다리던 구독이 모두 떠나면 그 빈 스트림은 지워지고, 보관 run 수에는 한 번이라도 발행된 run만 센다(리뷰 반영).

## 3. 스트림 종류

| kind | key | class | replay | 비고 |
|---|---|---|---|---|
| `run` | run id | state | 512개 보관 | terminal run은 보관 run 수 256 초과 시 먼저 끝난 순으로 제거하고 **제거 표식**(최대 4,096)을 남긴다. 제거된 run 구독은 cursor 0이어도 `Gap(evicted)` |
| `worktree` | 경로(실제 경로로 정규화) | notification | 없음 | 첫 구독에 감시 시작, 마지막 해지에 중지. 500ms 묶음, file/git 분류 |

## 4. 세대

- `system.describe.output.epoch` = 현재 세대. 모든 봉투·gap에 같은 값.
- 이전 세대 cursor → `Gap(epochChanged)`. 클라이언트는 재조회하고 진행 중이던 run을 "실행 정보 유실"로 표시한다.
- `afterSequence == 0`인 cursor는 세대를 보지 않는다: "처음부터"는 어느 세대에서나 같은 뜻이다(구현 중 확정).

## 5. 계약 조회 확장

`system.describe.output.eventSchemas[]`: principal에게 허용되고 **구독 가능한** 스키마만. 039에서는 데스크톱·조회 전용 모두 `run.event.v1`, `worktree.changed.v1` 두 개다. 2b 예약 스키마(orchestration·exchange)는 registry에 있지만 구독을 열 때까지 목록에 나오지 않는다.

## 6. 테스트 WebSocket (`tests/support/http_harness.rs`)

`GET /v1/events` + `Authorization: Bearer <token>` → upgrade. 서버 `hello` → 클라이언트 `subscribe` 한 번 → 서버 `event`/`gap`… 또는 `fault` 후 close. 연결 종료 = 구독 해제. 테스트 전용이며 운영 노출은 3단계.

## 7. fixture (`crates/workbench-protocol/fixtures/events/*.json`)

```json
{
  "name": "run-replay-from-middle",
  "principal": "desktop",
  "publish": [{ "stream": "run:r1", "events": 10 }],
  "subscribe": [{ "streamId": "run:r1", "epoch": "{{epoch}}", "afterSequence": 4 }],
  "publishAfter": [{ "stream": "run:r1", "events": 2 }],
  "expect": { "items": [{ "event": { "sequence": 5 } }, "…", { "event": { "sequence": 12 } }] }
}
```

필수 fixture: 처음부터·중간·끝·unknown(0)·unknown(>0)·**제거된 run(cursor 0과 >0, 새 컨트롤러 재수화 포함, 구독 중 제거, 제거 뒤 늦은 발행)**·retention 초과·epoch 불일치·ahead·forbidden(readonly가 아닌 거부 시나리오는 scope 없는 principal로)·kind 미지원·cursor 수 초과·lag(test-hooks로 대기열 4)·worktree 알림 묶음·worktree 두 구독자·같은 스트림 중복(run, worktree 별칭). 두 경로(in-memory, WS)에서 같은 결과.
