# Contract: orchestration operation과 이벤트 (041)

`Workbench.call` operation 34개를 추가한다(50 → 84). 모든 command는 멱등성 키 필수, `idempotencyScope: epoch`(영속 멱등은 작업 영역 내부 기록). 입력은 최상위 `deny_unknown_fields`. scope `orchestration:read`·`orchestration:write` 추가.

## 공통 검사

| operation 종류 | 검사 | fault |
|---|---|---|
| 데스크톱(`benchId` 입력) | 작업대 없음 | `notFound` `"bench not found."` |
| 〃 | 작업대 연 주체 아님 | `forbidden` `"bench belongs to another principal."` |
| 〃 | 작업대에 묶인 작업 영역 없음(`bootstrap`·`listRecoverable`·`get` 제외) | 오늘 서비스 오류(`OrchestrationError` `notFound`) |
| agent(`runId` 입력) | 주체 run ≠ 입력 run | `forbidden` `"The requested run does not match the authenticated capability."` |
| 〃 | 역할 불일치(coordinator 아님·다른 과제·이전 세대) | 오늘 도구 권한 오류(`OrchestrationError` `unauthorized`) → `forbidden` |

도메인 오류는 fault `message` = 도메인 message, `details.orchestrationError` = `{code, message, retryable}` 원본. fault 코드: `invalidInput`·`invalidTopology` → `invalidArgument`, `notFound` → `notFound`, `scopeMismatch`·`unauthorized`·`readOnlyViolation` → `forbidden`, `revisionConflict`·`duplicateConflict`·`invalidTransition` → `conflict`, `capacityExceeded` → `rateLimited`, `coordinatorInactive`·`coordinatorBusy`·`workerUnavailable`·`runtimeLost` → `unavailable`(retryable은 원본 값).

## 데스크톱 operation (18)

| operation | 종류 | scope | 입력 → 출력 |
|---|---|---|---|
| `orchestration.bootstrap` | command | `orchestration:write` | `{benchId, worktreePath, resumeWorkspaceId?}` → `OrchestrationSessionDto`(작업대에 묶음) |
| `orchestration.get` | query | `orchestration:read` | `{benchId}` → `OrchestrationSessionDto \| null` |
| `orchestration.listRecoverable` | query | `orchestration:read` | `{benchId, worktreePath}` → `[OrchestrationSessionDto]`(묶이지 않은 것) |
| `orchestration.bindCoordinator` | command | `orchestration:write` | `{benchId, request: BindMainRunRequest}` → Session |
| `orchestration.delegateGoal` | command | `orchestration:write` | `{benchId, request: DelegateGoalRequest}` → `DelegateGoalOutcome`(coordinator run에 프롬프트 전송 포함) |
| `orchestration.adoptManualChild` | command | `orchestration:write` | `{benchId, panelId, title}` → Session |
| `orchestration.listTasks` | query | `orchestration:read` | `{benchId, generationId}` → `[OrchestrationTaskDto]` |
| `orchestration.collectReports` | query | `orchestration:read` | `{benchId}` → `[TaskReportDto]` |
| `orchestration.setPresentation` | command | `orchestration:write` | `{benchId, request}` → Session |
| `orchestration.sendChildCommand` | command | `orchestration:write` | `{benchId, input: DeliverTaskCommandInput}` → `TaskCommandDto` |
| `orchestration.respondInput` | command | `orchestration:write` | `{benchId, request: TaskActionRequest}` → TaskCommand |
| `orchestration.cancelTask` · `retryTask` · `reassignTask` | command | `orchestration:write` | `{benchId, request: TaskActionRequest}` → Session |
| `orchestration.handoffCoordinator` | command | `orchestration:write` | `{benchId, request: CoordinatorHandoffRequest}` → Session |
| `orchestration.dispatchPrompt` | command | `orchestration:write` | `{benchId, request: DispatchPromptRequest}` → `PromptDispatchDto` |
| `orchestration.recover` | command | `orchestration:write` | `{benchId}` → Session(작업대의 worktree에서 복구 가능한 작업 영역을 묶고 재조정·대기 전달) |
| `run.replay` | query | `run:read` | `{benchId, runId, afterSequence}` → `RunReplayDto`. 판단 근거는 run 스트림의 **소유 작업대 기록**(첫 발행 때 hub가 기록, run이 끝나도 journal과 함께 남음). 다른 작업대 소유면 `forbidden` `"run is owned by another bench."`, 스트림이 없으면 모르는 run과 같은 빈 replay |

## agent operation (16, agent 전용)

coordinator(현재 세대): `orchestration.createChildTask`·`assignChildTask`·`listChildTasks`·`sendChildMessage`·`waitChildTasks`(최대 30초 대기, 오늘 결과 형태)·`collectChildResults`·`interruptChildTask`·`cancelChildTask`·`retryChildTask`·`reassignChildTask`.

자식(자기 과제): `orchestration.getOwnTask`·`reportProgress`·`reportResult`·`requestParentInput`·`reportBlocked`·`sendParentMessage`.

입력 = `{runId, ...오늘 도구 인자}`, 출력 = 오늘 도구 결과 JSON(DTO). 보고류는 요청 id 멱등(작업 영역 내부 기록).

## 이벤트

| 스키마 | 스트림 | 본문 | 비고 |
|---|---|---|---|
| `orchestration.workspaceUpdated.v1` | `orchestration:<bindingId>` | `OrchestrationEventDto{workspaceId, revision, reason, taskId?, nodeId?}` | 상태 복원용, 묶임당 256 |

- `StreamKind::Orchestration` 구독 가능. 스트림 key는 묶임 id(작업 영역 DTO `eventStreamId`).
- 구독 권한: 묶임의 작업대를 연 주체만. 다른 주체·agent `forbidden`, 모르는 id `notFound`, 끝난 묶임 `Gap(evicted)`. cursor 하나라도 거절되면 구독 전체 거절(040 규칙).
- **run 스트림 구독 권한(보강)**: `run:<runId>` 구독도 소유 작업대 기록의 연 주체만 허용한다(040은 scope만 검사 — exchange·bench와 같은 누수). 아직 발행 전인 run(기록 없음)은 그 run의 현재 소유 작업대(엔진)로 판단하고, 둘 다 없으면 오늘처럼 빈 스트림 대기를 허용하지 않고 `notFound`.
- 발행: 작업 영역이 바뀌는 모든 자리(서비스 변경, 명령 전달 단계, 알림 전달, 작업대 닫기로 복구 가능 전환 — 마지막은 스트림 제거 직전 발행).

## 계약 조회

- `system.describe`: 데스크톱 84, readonly(`:read`만), agent(교환·제목·orchestration agent 16·orchestration 조회 중 agent가 부를 수 있는 것 — scope 기준). `eventSchemas`에 orchestration 추가(구독 가능).
- OpenAPI·TS `OperationMap`에 34개, `EventMap["orchestration.workspaceUpdated.v1"]`.
