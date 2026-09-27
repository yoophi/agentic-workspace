# Contract: 작업대·run·교환 operation과 이벤트 (040)

`Workbench.call` operation 18개를 추가한다(32 → 50). 모든 command는 멱등성 키 필수. descriptor에 `idempotencyScope`(`durable` | `epoch`, command만)를 추가한다. 입력은 최상위 `deny_unknown_fields`.

## 공통 검사 (benchId를 받는 operation)

| 조건 | fault |
|---|---|
| 작업대 없음(닫히는 중·닫힘·이전 세대 포함) | `notFound` `"bench not found."` |
| 작업대를 연 주체가 아님 | `forbidden` `"bench belongs to another principal."` |

## 멱등성 (command 공통)

| 경우 | 응답 |
|---|---|
| `durable`(`run.start`) | 1단계 ledger 규칙 |
| `epoch`, 같은 키·같은 payload, 결과 기록 있음 | 저장된 결과 |
| `epoch`, 같은 키·같은 payload, 결과가 요약으로 강등됨 | 재실행 없음, `conflict` `outcome: applied` `"idempotency result is no longer available; the request was already applied."` |
| 같은 키·다른 payload | `conflict`(ledger와 같은 문구) |
| 작업대의 요약 기록 65,536개 도달 뒤 새 키 | `rateLimited` `retryable: false` `"bench idempotency capacity exhausted; close and reopen the bench."` |
| 작업대가 닫힌 뒤 재시도 | `notFound` `"bench not found."` (기록은 작업대와 함께 사라짐 — 중복 실행 없음) |

## 닫기와 입장 경계

- `bench.close`는 `Open → Closing` 전이를 원자적으로 한 뒤, 이미 입장한 동작(`run.start`의 소유 기록, 교환 쓰기, 과도기 orchestration 기동)이 끝나기를 기다리고, 소유 run을 모두 취소한다. 반환 시점에 그 작업대 소유의 살아 있는 run은 0개다.
- `Closing` 이후 들어온 입장 동작은 `notFound`. 기존 run 제어·조회는 입장하지 않으므로 닫기를 막지 않는다(경합하면 run이 취소되어 "비활성 run" 오류).
- 두 닫기가 겹치면 두 번째는 첫 번째가 끝날 때까지 기다린 뒤 `closed: false`.

## 작업대

| operation | 종류 | scope | 멱등 | 입력 → 출력 |
|---|---|---|---|---|
| `bench.open` | command | `bench:write` | epoch | `{workingDirectory}` → `{benchId, workingDirectory}`(실제 경로). 없는 경로 `invalidArgument` `"Failed to resolve workspace path: …"`, 디렉터리 아님 `"Workspace path must be a directory."`, 상한 `rateLimited` |
| `bench.close` | command | `bench:write` | epoch | `{benchId}` → `{closed: bool, cancelledRuns: [runId]}`. 모르는 id·닫히는 중·닫힘도 성공(`closed: false`, 닫히는 중이면 끝날 때까지 대기). 다른 주체의 작업대는 `forbidden`. 멱등 기록 없이도 자연 멱등 |
| `bench.requestTitle` | command | `presentation:write` | epoch | `{runId, title}` → `{ok: true, appliedTitle}`. agent 전용(주체 run == runId, 아니면 `forbidden` `"The requested run does not match the authenticated capability."`). 제목 검증 문구는 오늘과 같음(80자). run의 활성 작업대 없음 → `notFound` `"Agent run is not active or is not owned by a session window."` |

## run

| operation | 종류 | scope | 멱등 | 입력 → 출력 · 오류 |
|---|---|---|---|---|
| `run.listToolCandidates` | query | `run:read` | — | `{benchId, query: AgentToolCandidateQuery}` → `AgentToolCandidateResponse`. 소유 불일치 `forbidden` `"tool command candidates were requested from a non-owner window"` |
| `run.start` | command | `run:write` | **durable** | `{benchId, request: AgentRunRequest, panelId?}` → `AgentRun`. `"duplicate run id: …"`·`"concurrent run limit (n) reached; …"`·`"agent run was cancelled before it started"`·decorator 오류(Main Coordinator 문구) → `notApplied` fault. 재시작 판정: `pending` → `unknown` |
| `run.sendPrompt` | command | `run:write` | epoch | `{benchId, runId, prompt}` → `{}`. `"prompt is empty"`, `"agent run is not active"`, 소유 불일치 `forbidden` `"run is owned by another bench."`. 전송 중 실패는 run 이벤트 |
| `run.steer` | command | `run:write` | epoch | `{benchId, runId, prompt}` → `{}`. `"steer prompt is empty"`, `"steer unsupported: …"`, `"steer dispatch failed: …"`, 소유 불일치 |
| `run.cancelAndSend` | command | `run:write` | epoch | `{benchId, runId, prompt}` → `{}`. `"prompt dispatch failed: …"` 등, 소유 불일치 |
| `run.setPermissionMode` | command | `run:write` | epoch | `{benchId, runId, mode}` → `{}`. 소유 불일치 |
| `run.cancel` | command | `run:write` | epoch | `{benchId, runId}` → `{}`(이미 끝난 run도 성공 — 오늘과 같음). 살아 있는 다른 작업대 run이면 `forbidden` |
| `run.respondPermission` | command | `run:write` | epoch | `{benchId, runId, permissionId, optionId}` → `{}`. `"unknown or finished run: …"`, `"permission response was sent from a non-owner window"`, `"unknown or already answered permission: …"`, `"permission … belongs to a different run"`, `"permission waiter is no longer active"` |

fault 코드 매핑: 빈 입력·형식 → `invalidArgument`, 비활성 run·모르는 권한 → `notFound`, 소유·주체 불일치 → `forbidden`, 동시 실행 상한 → `rateLimited`, 그 외 엔진 오류 → `internal`. `message`는 오늘 문자열 그대로.

## 교환

도메인 오류는 fault `message` = 도메인 message, `details.exchangeCode` = 도메인 code(`invalidPanels` 등 21종). fault 코드: 검증 계열 → `invalidArgument`, `unknown*` → `notFound`, `stale*`·`targetClosing` → `preconditionFailed`, `duplicateConflict`·`invalidTransition` → `conflict`, `windowUnavailable`·`deliveryFailed` → `unavailable`.

| operation | 종류 | scope | 멱등 | 입력 → 출력 |
|---|---|---|---|---|
| `exchange.syncWorkspace` | command | `exchange:write` | epoch | `{benchId, request: AgentWorkspaceSyncRequest}` → `{revision, acceptedPanels}` |
| `exchange.send` | command | `exchange:write` | epoch | `{benchId, request: SendAgentExchangeRequest}` → `AgentExchange` |
| `exchange.acknowledge` | command | `exchange:write` | epoch | `{benchId, request: AgentExchangeAckRequest}` → `AgentExchange`. 같은 결과의 두 번째 확인: 같은 값 반환, 이벤트 없음 |
| `exchange.list` | query | `exchange:read` | — | `{benchId}` → `[AgentExchange]` |
| `exchange.listPeers` | query | `exchange:read` | — | `{runId}` → `{peers: [AgentPanelEndpoint]}`. agent 전용 |
| `exchange.sendFromRun` | command | `exchange:write` | epoch | `{runId, request}` → `AgentExchange`. agent 전용 |
| `exchange.getForRun` | query | `exchange:read` | — | `{runId, requestId}` → `AgentExchange`. agent 전용 |

"agent 전용"은 principal 주체의 run이 입력 `runId`와 같아야 한다는 뜻이다(데스크톱이 부르면 `forbidden`).

## 이벤트 (039 계약 확장)

| 스키마 | 스트림 | 본문 | 비고 |
|---|---|---|---|
| `exchange.requested.v1` | `exchange:<benchId>` | `ExchangeRequestedDto{requestId, source, target, message, delivery, createdAt}` | 상태 복원용, 작업대당 512 |
| `exchange.status.v1` | `exchange:<benchId>` | `AgentExchangeDto`(위 `AgentExchange`) | 같은 스트림, 같은 순번 체계 |
| `bench.titleRequested.v1` | `bench:<benchId>` | `{title}` | 알림용, scope `bench:read` |

- `StreamKind::Exchange` 구독 가능, `StreamKind::Bench` 신설. `orchestration:*`은 계속 `"stream kind is not available yet."`.
- 작업대가 닫히면 `exchange:<id>`·`bench:<id>` 구독자는 `Gap(evicted)`, 이후 cursor 0 구독도 `Gap(evicted)`(제거 표식 4,096).
- 알림용 발행도 데스크톱 전달(`deliver`)을 부른다(구독자가 없어도).
- **구독 권한**(구현 리뷰 반영): `exchange:<id>`·`bench:<id>`는 scope에 더해 작업대를 연 주체만 구독한다. 다른 주체 `forbidden` `"bench belongs to another principal."`, agent principal도 `forbidden`(MCP 도구는 요청·응답만 쓴다), 한 번도 없던 id `notFound` `"bench not found."`(나중에 열릴 스트림에 미리 붙지 않게), 닫힌 작업대(제거 표식)는 위 규칙대로 `Gap(evicted)`. cursor 하나라도 거절되면 구독 전체를 거절한다.
- **agent 교환 전송과 닫기**: `exchange.sendFromRun`은 출발 run의 작업대에 입장해야 한다. 닫는 중이면 `notFound` `"bench not found."`(run이 아직 살아 있어도). 작업 영역이 지워진 작업대에는 교환을 저장하지 않는다(`unknownWorkspace`).

## 계약 조회

- `system.describe`: operation 50개 중 principal에게 허용된 것, `idempotencyScope`, `eventSchemas`에 교환 2개·작업대 1개 추가(구독 가능·scope 허용분만).
- OpenAPI `EventBySchema`에 3개 variant 추가, TS `EventMap` 키 6개.
