# Contract: 작업대·run·교환 operation과 이벤트 (040)

`Workbench.call` operation 18개를 추가한다(32 → 50). 모든 command는 멱등성 키 필수. descriptor에 `idempotencyScope`(`durable` | `epoch`, command만)를 추가한다. 입력은 최상위 `deny_unknown_fields`.

## 공통 검사 (benchId를 받는 operation)

| 조건 | fault |
|---|---|
| 작업대 없음(또는 닫힘, 이전 세대) | `notFound` `"bench not found."` |
| 작업대를 연 주체가 아님 | `forbidden` `"bench belongs to another principal."` |

## 작업대

| operation | 종류 | scope | 멱등 | 입력 → 출력 |
|---|---|---|---|---|
| `bench.open` | command | `bench:write` | epoch | `{workingDirectory}` → `{benchId, workingDirectory}`(실제 경로). 없는 경로 `invalidArgument` `"Failed to resolve workspace path: …"`, 디렉터리 아님 `"Workspace path must be a directory."`, 상한 `rateLimited` |
| `bench.close` | command | `bench:write` | epoch | `{benchId}` → `{closed: bool, cancelledRuns: [runId]}`. 모르는 id도 성공(`closed: false`). 다른 주체의 작업대는 `forbidden` |
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

## 계약 조회

- `system.describe`: operation 50개 중 principal에게 허용된 것, `idempotencyScope`, `eventSchemas`에 교환 2개·작업대 1개 추가(구독 가능·scope 허용분만).
- OpenAPI `EventBySchema`에 3개 variant 추가, TS `EventMap` 키 6개.
