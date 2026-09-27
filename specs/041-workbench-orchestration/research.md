# Research: 041 orchestration 작업대 이관

기준 코드: main `c8b41a4`. 경로는 `apps/agentic-workbench/src-tauri/src`(AW) 기준, core는 `crates/workbench-core/src`.

## 사실 요약 (조사 결과)

- orchestration command 18개(`inbound/tauri_commands.rs:135–845`) 중 16개가 `window.label()`을 서비스에 넘긴다. `list_recoverable`는 창이 사라진 label을 찾아 `release_window`하고, `replay_orchestration_runtime_events`는 서비스 없이 core `EventHub::replay_run`을 부른다.
- 집합(aggregate)은 `OrchestrationSession`(AW `domain/agent_orchestration.rs:753`, "작업 영역"): `boundWindowLabel`, 노드(Main 1 + 자식 ≤7), 세대, 과제(과제별 revision·attempt), 보고, 명령, coordinator 알림, 분배, 멱등 기록(`actor_key = window_label`), 작업 영역 revision, schema_version 2.
- 영속: `JsonOrchestrationRepository`(`<app_data>/orchestration-sessions.json`, 세션 배열 하나). `json_store`로 tmp+`.bak` 원자 저장. **in-process lock 없음** — 모든 서비스 메서드가 파일 전체를 load → 수정 → save하고, `OrchestrationCommandService::deliver`는 await 사이에 세 번 저장한다. UI command·MCP 도구·spawn된 dispatcher가 동시에 돌므로 서로 다른 작업 영역의 변경도 덮어쓴다(사용자 점검 사항).
- 모든 서비스 메서드가 `session_for_window_mut(label)`로 작업 영역을 찾는다. `release_window`는 label을 `None`으로(복구 가능) 바꾸고 노드를 주의 필요로 표시하며 이벤트를 내지 않는다.
- scheduler(`application/orchestration_scheduler.rs`)는 프로세스 전역 메모리(`McpServerState` 안), 용량 `ACP_WORKBENCH_MAX_RUNS`(기본 4) − 1, 최소 1. 재시작에 사라지고 `recover`가 `reconcile`로 재구성한다.
- MCP 도구 16개(`infrastructure/mcp/orchestration_tool.rs`: coordinator 10, 자식 6). 토큰 `awcap_<uuid>` → `CapabilityPrincipal{actorKind, workspaceId, windowLabel, nodeId, runId, taskId, generationId}`(메모리 registry). `handle_tool`이 `principal.window_label`을 요구하고 모든 서비스 호출에 넘긴다. 교대 시 이전 세대 토큰을 폐기한다. `aw_wait_child_tasks`는 최대 30초 poll.
- 이벤트: `OrchestrationEvent{workspaceId, revision, reason, taskId?, nodeId?}`. Tauri sink가 `window.emit`(실제로는 전체 창 방송) + `window.eval` fallback으로 **두 번** 전달하고, `reason`에 "command"/"notification"이 들어 있으면 상세 이벤트도 낸다. 화면은 `listenWithFallback`으로 둘 다 듣고 `workspaceId`+revision으로 거른다. 상세 이벤트 listener는 production 코드에서 쓰지 않는다.
- 자식 run: `acp_agent_worker_adapter.rs`의 `TauriAcpWorkerRuntime::start`가 `desktop_benches::ensure(label)` → `runtime.admit` → run 시작(owner = 작업대)·`run_sink(bench)`. worktree 감시 guard는 전역 `WORKTREE_GUARDS`(창 label 포함). run 종료 시 `RunTerminalHook`(AW `tauri_desktop_bridge.rs:112`)가 guard를 보고 `fail_task_for_runtime(label, …)`.
- 창 닫힘(`lib.rs:144–165`): `desktop_benches::close` → `OrchestrationService::release_window(label)`.
- 040 과도기 접근자: `WorkbenchRuntime::run_sink`·`admit`·`run_engine().acp_registry()`·`acp_session_store()`(core `application/workbench_runtime.rs:254–265`, `ports/run_engine.rs:110`).
- protocol: `StreamKind::Orchestration`은 구독 불가(`events/mod.rs:55`), scope 자리표시 `RunRead`, 스키마 `orchestration.workspaceUpdated.v1` → `OrchestrationEventDto`만 있다. orchestration operation·DTO 없음.
- 화면: `entities/agent-orchestration/api/orchestration-repository.ts`가 18개 command와 `listenWithFallback`을 감싼다. `boundWindowLabel`은 `model/types.ts:240`(타입)과 storybook 표본에만 있고 로직은 읽지 않는다.
- 오류: command는 `OrchestrationError{code, message, retryable}`를 JSON 문자열로 돌려준다(`tauri_commands.rs:128`). 창을 언급하는 문구 3개("The window is already bound to another worktree.", "The workspace cannot be bound to this window.", "The orchestration session belongs to another window.").
- 테스트: AW Rust orchestration 테스트 60개(서비스 15·도메인 6·명령 4·dispatcher 4·scheduler 3·projector 3·저장소 4·worker 4·MCP 5·parity 1·command 3·통합 3). projector는 production에서 쓰이지 않는다(테스트만).

## R1. 저장소 전체 읽기-수정-쓰기 경계 (사용자 점검 사항)

**Decision**: orchestration 저장은 `StorageCoordinator`의 aggregate `orchestration-sessions` 하나로 직렬화한다. 저장소 포트를 `load`/`save`에서 **`read(|sessions| …)`와 `update(|sessions| -> R)`** 로 바꾸고, `update`는 aggregate lock 안에서 파일 전체 load → 클로저 → save를 한 번에 한다. 서비스의 모든 변경은 `update` 하나 안에서 **상태를 검사하고 바꾼다**(예전의 "load → 작업 → save" 사이에 다른 await가 끼는 경로를 모두 없앤다). await가 필요한 다단계 흐름(명령 전달, 알림 전달, 자식 기동)은 단계마다 독립된 `update`이며 단계 사이에 어떤 lock도 쥐지 않는다(R2).

- 실행 문맥: `update`·`read`는 동기 파일 I/O·fsync를 하므로 async 코드에서는 항상 `spawn_blocking`으로 부른다(038 handler 규칙). 저장소 lock 안에서 엔진·전달·다른 lock을 부르지 않는다.
- 손상 복구: 오늘 `json_store::load_json`은 읽기 경로 안에서 `.bak`을 복구한다(`json_store.rs:34-57`). 이 복구가 aggregate lock 안에서 일어나므로 그대로 쓴다 — `with_aggregate`의 "손상 시 recover 후 1회 재시도"는 쓰지 않고 lock만 잡는다(`run_locked`에 해당하는 공개 함수 `with_aggregate_lock`). 백업도 없거나 `validate()`가 실패하면 오늘처럼 오류(`WorkerUnavailable`)를 돌려준다.
- revision: 작업 영역 revision은 `update` 안에서 그 작업 영역을 바꿀 때마다 +1(오늘 규칙). aggregate revision은 추적하지 않는다(`STORE_AGGREGATES` 밖, ledger 미사용).
- 검증: cross-workspace 동시성 테스트 — 작업 영역 2개 이상(각각 다른 작업대)에 thread 여러 개로 변경 100회 이상을 동시에 넣고 끝난 뒤 모든 변경(과제 수·보고 수·revision 합)이 남았는지 확인한다. 같은 작업 영역에 화면 명령 + agent 보고를 동시에 넣는 테스트도 둔다. lock을 빼면 실패하는지(회귀 검출) 확인한다.

**Rationale**: 파일이 하나인 한 작업 영역별 lock은 다른 작업 영역의 저장이 오래된 사본으로 전체를 덮어쓰는 것을 막지 못한다. 038이 같은 문제를 aggregate lock으로 풀었다. 작업 영역마다 파일을 나누는 방식은 저장 형식 변경(이전 빌드 호환 깨짐)이라 피한다.

**Alternatives**: 작업 영역별 파일 분할(형식 변경), SQLite 이전(범위 초과, 별도 단계), 파일 lock만(프로세스 안 경합에는 in-process lock이 필요).

## R2. lock 규칙 — operation 범위 작업 영역 lock을 두지 않는다 (설계 리뷰 반영)

**Decision**: operation 전체를 감싸는 작업 영역 async lock은 **두지 않는다**. 대신:

1. **상태 검사는 `update` 안에서**: 같은 작업 영역의 교차(취소·재시도, 중복 보고, 중복 기동)는 `update` 안의 상태 조건(과제 상태·`attempt`·`currentRunId`·요청 id 멱등·revision)으로 판정한다 — 오늘 서비스 규칙 그대로이며, 달라지는 것은 판정과 저장이 한 원자 단계가 된다는 점뿐이다.
2. **await 경계에서 lock 없음**: `notify_coordinator`(Main 턴 대기), `waitChildTasks`, 엔진 `start`·`send`·`cancel`, 작업대 닫기 대기, agent 진행 대기 중에는 저장소 lock·binding mutex를 포함한 **어떤 lock도 쥐지 않는다**. 다단계 흐름은 "`update`로 의도 기록(Dispatching·Launching) → lock 없이 외부 호출 → `update`로 결과 기록"이다.
3. **run 종료 처리·작업대 닫기 hook은 lock을 기다리지 않는다**: 이 둘은 다른 흐름의 await 안에서 호출될 수 있다(엔진 취소가 종료 이벤트를 호출자 task 안에서 바로 내보냄 — `cancel_agent_run.rs:30-31`, `workbench_run_sink.rs:79-83`). 그래서 `update` 한 번(`spawn_blocking`)만 쓰고, 조건 `(taskId, attempt, runId)`가 맞을 때만 적용한다.
4. **binding mutex**: 묶기·풀기(R3)만 전역 동기 mutex 하나로 직렬화한다. 이 mutex 안에서는 저장소 `update`(동기)와 묶임 표 변경만 하고 await하지 않는다.
5. **lock 순서**: 작업대 입장권(040) → binding mutex → 저장소 aggregate lock. 역순 획득 없음. 입장권을 쥔 흐름은 작업대 닫기 hook이 기다리는 어떤 것도 기다리지 않는다.

**Rationale**: 설계 리뷰가 확인한 네 교착 — (a) 알림 전달이 Main 턴을 기다리는 동안 Main의 도구 호출이 같은 lock을 기다림(`coordinator_notification_dispatcher.rs:76`, `acp_agent_worker_adapter.rs:271`), (b) `waitChildTasks`가 기다리는 자식 보고가 같은 lock을 기다림(`orchestration_tool.rs:459-481`), (c) 엔진 취소 안의 종료 처리가 이미 쥔 lock을 다시 잡음, (d) 동기 닫기 hook이 async lock을 잡을 수 없음 — 이 모두 "operation 범위 lock"에서 나온다. 오늘 코드도 흐름 단위 lock 없이 상태 조건으로 동작하므로, 저장만 원자로 만들면 규칙이 바뀌지 않는다.

**Alternatives**: 작업 영역 lock + 예외 목록(규칙이 복잡하고 새 경로가 추가될 때마다 교착 위험), 재진입 lock(tokio에 없고 원인을 숨김).

## R3. 작업 영역 ↔ 작업대 묶임

**Decision**: 묶임은 core 메모리 표 `OrchestrationBindings{workspace_id → Binding{bench_id, binding_id}}`와 역방향 `bench_id → workspace_id`. 작업대 하나에 작업 영역 최대 1개. 영속 `boundWindowLabel`은 **읽을 때 무시하고 쓸 때 생략**(serde `default`·`skip_serializing`) — 이전 빌드는 필드가 없으면 `None`(복구 가능)으로 읽는다. 서버 재시작 뒤 모든 작업 영역은 복구 가능.

- **묶기**(`orchestration.bootstrap` 새로 만들기·재개, `orchestration.recover`): ① 작업대 입장권을 얻는다(닫히는 중이면 `notFound`), ② binding mutex 안에서 "작업 영역 찾기/만들기(저장소 `update`) + 이미 묶임 검사 + 묶임 표 삽입"을 한 번에 한다, ③ 입장권을 놓는다. 작업 영역 id를 아직 모르는 새로 만들기·worktree로 찾는 복구도 같은 mutex로 직렬화되어 두 작업대가 둘 다 "없음"을 보고 중복 생성·중복 묶기를 할 수 없다. 입장권 덕분에 닫기 hook이 먼저 돌아 죽은 작업대에 묶이는 일이 없다.
- 거절 문구: 이미 다른 작업대에 묶인 작업 영역 → "The workspace cannot be bound to this window.", 작업대가 이미 다른 worktree 작업 영역에 묶임 → "The window is already bound to another worktree."(바이트 동일 유지, contracts에 명시).
- **풀기**: 작업대 닫기 hook을 **async**로 바꾼다(`BenchCloseHook`이 future를 돌려주고 `finish_close`가 await — 닫기 정리는 이미 spawn된 task 안). hook은 소유 run 취소 뒤에 돌며 ① binding mutex 안에서 묶임 표 제거 + 오늘 `release_window` 규칙(노드 주의 필요·run 재조정)을 `update`(`spawn_blocking`)로 적용, ② 마지막 갱신 이벤트 발행, ③ 묶임 스트림 제거(040 `remove_stream`, 제거 표식) 순서다.
- agent operation은 작업대 입장권을 쥐지 않는다. 닫기 뒤 늦게 도착한 agent 호출은 묶임이 없으므로 조회는 오늘 결과, 자식 기동은 입장 실패로 과제가 대기/실패로 남는다(R8).

**Rationale**: 작업대가 메모리 전용이므로 묶임도 메모리여야 일관된다. 오늘 앱 재시작 뒤 모든 창 label이 무효가 되어 사실상 복구 가능이 되는 것과 사용자 결과가 같다.

## R4. 멱등 기록의 행위자

**Decision**: 작업 영역 내부 멱등 기록(`idempotencyRecords`)의 `actor_key`를 창 label 대신 **호출 주체**(데스크톱 `desktop`, agent `agent:<runId>`)로 바꾼다. 기록은 작업 영역에 속하므로 작업 영역 간 충돌은 없다. 이전 빌드가 남긴 기록(행위자 = `session-…`)은 새 행위자와 겹치지 않아 재사용되지 않는다(요청 id는 화면이 새로 만든 uuid). seam 수준 멱등은 040과 같은 세대 범위(`idempotencyScope: epoch`) — 영속 멱등은 작업 영역 내부 기록이 이미 맡는다(ADR core 0005의 "ledger는 run.start만" 유지).

## R5. operation 목록과 이름

**Decision**: 데스크톱 operation 18개(모두 `benchId` 입력, 작업대 주체 + 묶임 검사):

| command | operation | 종류 |
|---|---|---|
| `bootstrap_orchestration_workspace` | `orchestration.bootstrap` | command |
| `get_orchestration_workspace` | `orchestration.get` | query |
| `list_recoverable_orchestration_workspaces` | `orchestration.listRecoverable` | query |
| `bind_main_coordinator_run` | `orchestration.bindCoordinator` | command |
| `delegate_orchestration_goal` | `orchestration.delegateGoal` | command |
| `adopt_manual_orchestration_child` | `orchestration.adoptManualChild` | command |
| `list_orchestration_tasks` | `orchestration.listTasks` | query |
| `collect_orchestration_reports` | `orchestration.collectReports` | query |
| `set_orchestration_presentation` | `orchestration.setPresentation` | command |
| `send_orchestration_child_command` | `orchestration.sendChildCommand` | command |
| `respond_orchestration_input` | `orchestration.respondInput` | command |
| `cancel_orchestration_task` | `orchestration.cancelTask` | command |
| `retry_orchestration_task` | `orchestration.retryTask` | command |
| `reassign_orchestration_task` | `orchestration.reassignTask` | command |
| `handoff_orchestration_coordinator` | `orchestration.handoffCoordinator` | command |
| `replay_orchestration_runtime_events` | `run.replay` | query (run 도메인, 작업대 소유 run만) |
| `dispatch_orchestration_prompt` | `orchestration.dispatchPrompt` | command |
| `recover_orchestration_workspace` | `orchestration.recover` | command |

agent operation 16개 + 역할 조회 1개(`orchestration.getAgentRole`, MCP `tools/list`용)(입력 `runId`, agent 전용, 역할은 서버 상태로 판정 — R7): `orchestration.createChildTask`·`assignChildTask`·`listChildTasks`·`sendChildMessage`·`waitChildTasks`·`collectChildResults`·`interruptChildTask`·`cancelChildTask`·`retryChildTask`·`reassignChildTask`(coordinator), `orchestration.getOwnTask`·`reportProgress`·`reportResult`·`requestParentInput`·`reportBlocked`·`sendParentMessage`(자식). operation은 50 → 85개(데스크톱 18 + agent 17).

`listRecoverable`의 "사라진 창 정리"는 없어진다(묶임은 작업대 닫기에서 풀린다).

## R6. scope

**Decision**: `orchestration:read`·`orchestration:write` 추가(22개). 데스크톱은 전부, readonly는 `:read`, agent는 `orchestration:read`·`orchestration:write`를 더한다(agent 전용 operation과 데스크톱 operation은 역할·작업대 주체 검사로 서로 막힌다 — agent는 작업대를 열지 않으므로 데스크톱 operation은 `forbidden`). 스트림 scope는 `orchestration:read`.

## R7. agent 역할 판정 (설계 리뷰 반영)

**Decision**: agent principal(`agent:<runId>`)의 역할을 서버 상태로 찾는다.

- **coordinator**: 어떤 작업 영역의 `activeCoordinatorGenerationId`가 가리키는 세대가 Active이고 그 `runId`가 주체 run. (마지막 세대가 아니다 — 연결 해제 뒤 active는 None이 되지만 마지막 세대는 남는다, `orchestration_service.rs:404-412`.) 교대로 이전 세대가 된 run은 자동으로 coordinator가 아니다.
- **자식**: 어떤 작업 영역의 노드 `currentRunId`가 주체 run이고, 그 노드가 **orchestration이 기동한 자식**(배정 과제의 현재 시도)이다. 자식 기동은 `start` 전에 기동 예정 run id를 노드 `currentRunId`·과제 시도에 기록(Launching)하므로, 자식의 첫 턴에서 부르는 `getOwnTask`·보고도 허용된다(오늘은 토큰 주장으로 허용됨 — `orchestration_service.rs:831`의 사후 `bind_child_run`을 사전 기록으로 바꾼다).
- **수동 채택 자식**: 오늘 그 run의 토큰은 `LegacyRun`이라 orchestration 도구가 없다(`orchestration_tool.rs:81`). 이를 유지한다 — 자식 역할은 orchestration이 기동한 run에만 준다.
- 그 외 → 거절. 문구는 오늘 도구 오류 그대로: 역할 불일치 `forbiddenActor` "The authenticated agent role cannot call this tool.", 묶인 작업 영역 없음 `scopeMismatch` "This run is not bound to an orchestration workspace."(`orchestration_tool.rs:283-296`).
- MCP `tools/list`: 오늘은 토큰의 actor kind로 목록을 고른다(`mcp/mod.rs:273-281`). 041은 요청 시점의 서버 역할로 고른다(core에 `orchestration.role`에 해당하는 내부 조회를 두고 AW MCP가 부른다 — agent principal로 부르는 조회 operation `orchestration.getAgentRole`을 추가, agent 전용). 역할이 없으면 오늘 `LegacyRun`과 같은 목록.
- MCP 토큰 registry는 AW에 남아 **token → run id**만 한다. 역할 주장은 토큰에서 제거한다. 오늘의 폐기 시점(재시도·재배정 시 `revoke_run`, 교대 시 `revoke_generation` — `orchestration_tool.rs:600`, `tauri_commands.rs:597`, `:704`)은 토큰 수명 관리로 유지하되 권한 근거는 아니다.

**Rationale**: 토큰 주장은 발급 시점의 사본이라 교대·재배정·복구 뒤 어긋난다. 서버 상태를 근거로 하면 폐기 누락이 권한 누수가 되지 않는다.

## R8. 자식 run 기동과 worktree 감시를 core로

**Decision**: `AgentWorker` 포트의 운영 구현을 core `EngineAgentWorker`로 옮긴다. 자식 기동 = ① `update`: 과제 시도에 기동 예정 run id 기록(Launching, R7) ② 작업대 입장권(기동 끝까지) ③ `RunLaunchDecorator`(MCP env) ④ `RunEngine::start(owner = 작업대)`(spawn 후 반환) ⑤ `update`: 결과 기록(Running 또는 실패 → 과제 대기/실패). 입장 실패(닫히는 중)면 ⑤에서 과제를 오늘 실패 규칙으로 되돌린다. 보내기·중단·취소는 `RunEngine` 메서드이고 lock 없이 부른다.

worktree 감시(기동 시 지문, 종료 시 비교 → 과제 실패)는 core가 run 종료 이벤트를 받는 자리(`WorkbenchRunSink` 종료 처리)에서 하되, 종료 처리가 호출자 task 안에서 바로 불릴 수 있으므로 **lock을 기다리지 않고** 조건부 `update`(`(taskId, attempt, runId)` 일치 시만)를 `spawn_blocking`으로 띄운다(R2-3). AW `RunTerminalHook`의 orchestration·worktree 부분은 삭제하고, 포트가 비면 포트 자체를 없앤다. `WORKTREE_GUARDS` 전역은 core 런타임 소유 표가 된다(창 label 제거).

## R9. scheduler·알림 전달·복구를 core로

**Decision**: `OrchestrationScheduler`(용량 규칙·FIFO 동일), `CoordinatorNotificationDispatcher`, `OrchestrationCommandService`, 복구 흐름(`reconcile_runtime`·`scheduler.reconcile`·`reconcile_pending`·`recover_interrupted`·`dispatch_pending`)을 core로 옮긴다. scheduler는 런타임당 하나(자체 동기 mutex, 짧게만). 용량·자식 agent 프로필은 `RuntimeAdapters.orchestration`(환경 변수 `ACP_WORKBENCH_MAX_RUNS`·`AW_ORCHESTRATION_AGENT_PROFILE`을 AW가 읽어 주입)으로 받는다.

- **알림 전달**: 알림마다 `update`로 Dispatching 표시 → lock 없이 `notify_coordinator`(Main 턴 끝까지 대기) → `update`로 결과 기록. Main이 그 턴에서 부르는 coordinator 도구(수집·대기)는 lock 경합 없이 진행된다(설계 리뷰 H1, 오늘 테스트 `processed_collection_wins_over_late_delivery_completion`이 이 교차를 고정).
- **명령 전달**: 같은 3단계(Pending→Dispatching `update`, lock 없이 엔진 호출, Accepted/Failed `update`).
- 백그라운드 전달은 core가 tokio task로 띄운다.

## R10. 이벤트 스트림

**Decision**: 스트림 = `orchestration:<bindingId>`(묶일 때마다 새 uuid, FR-009). 분류는 상태 복원용(묶임당 journal 256). 스키마 하나 `orchestration.workspaceUpdated.v1`(본문 `OrchestrationEventDto` 그대로 — 명령·알림 변경도 `reason`으로 구분). 발행은 core 서비스의 `persist_mutation`·`emit_workspace_changed` 자리와 오늘 호출자가 손으로 내던 자리(명령 전달·알림 전달·release)를 모두 core 발행으로 모은다. 구독 권한: 묶임의 작업대를 연 주체만(040 `authorize_bench_streams` 확장), 묶임이 끝난 스트림은 제거 표식 → `Gap(evicted)`. 작업 영역 DTO에 `eventStreamId`(현재 묶임의 스트림 id, 없으면 null)를 싣는다.

데스크톱 전달: `DesktopDelivery::Orchestration{bench_id, event}` → 그 작업대 창에 `orchestration-workspace-updated-fallback` CustomEvent 한 번, `reason`이 command/notification이면 오늘처럼 상세 fallback 이벤트도 한 번. 네이티브 `emit`(전체 방송)은 제거한다.

## R11. 호환 command와 `boundWindowLabel`

**Decision**: AW command는 `DesktopBenches`로 창의 작업대를 찾아(`bootstrap`·`recover`는 `ensure`, 나머지는 조회만) `Workbench.call`을 부르고, 결과 작업 영역에 `boundWindowLabel`을 다시 채운다(묶인 작업대가 이 창의 작업대면 이 창 label, 아니면 null) — 화면 타입(`string | null`, 필수 필드) 유지, core에는 창 label 없음. 창에 작업대가 없으면 core를 부르지 않고 오늘 서비스가 내던 결과(`get` → null, 나머지 → 오늘 "작업 영역 없음" 오류 JSON)를 그대로 돌려준다. 오류는 `OrchestrationError` JSON 문자열(fault `details.orchestrationError`에 원본을 싣고 compat가 재구성, 040 교환과 같은 방식).

## R12. MCP 도구

**Decision**: 도구 16개는 `principal.run_id`로 agent principal을 만들어 해당 operation을 `Workbench.call`하고, 결과·오류를 오늘 도구 결과 형태로 되돌린다. 도구 쪽 `runId` 일치 검사 유지. `aw_wait_child_tasks`의 대기는 core operation 안으로 옮기되 **lock을 쥐지 않는다**: 반복마다 `read` 한 번, 깨우기는 작업 영역 revision `watch` 채널(모든 `update`가 그 작업 영역을 바꾸면 알림)로 하고 100ms poll을 대체한다. 최대 30초·결과 형태는 오늘과 같다. `tools/list`는 R7대로 서버 역할에서 고른다.

## R13. DTO

**Decision**: protocol에 작업 영역·노드·세대·과제·보고·명령·알림·분배 DTO를 미러로 둔다(040 run DTO와 같은 방식: core 도메인과 serde 왕복 `convert` + wire parity 테스트). DTO에는 `boundWindowLabel`이 없고 `eventStreamId`가 있다.

## R14. 과도기 통로 제거

**Decision**: 제거 목록 — core `WorkbenchRuntime::run_sink`·`admit`(외부 공개)·`run_engine().acp_registry()`·`acp_session_store()` 공개 접근, AW `desktop_benches::label_for`의 서버 방향 사용(데스크톱 전달 경로만 남김), `resolve_agent_run_launch_principal`, `RunTerminalHook`의 orchestration 부분, 창 `Destroyed`의 `release_window`, `McpServerState`의 scheduler, `TauriOrchestrationEventSink`, AW orchestration 도메인·서비스·저장소(모두 core로 이동). 검증: core·protocol grep에서 창 label 0건, AW에서 `acp_registry`·`run_sink` 호출 0건.

## R15. 테스트

**Decision**: AW orchestration 테스트 60개를 core로 옮긴다(위치만 이동, 기대값 유지 — 창 label을 쓰던 테스트는 작업대 id로 입력만 바꾼다). projector(production 미사용)는 테스트와 함께 삭제한다. 새 테스트: fixture(데스크톱 18·agent 16 operation, 교차 작업대·교차 주체·이전 세대·다른 과제 거절), 묶임 스트림 fixture(순번·구독 거절·닫기 gap·재묶임 새 스트림), cross-workspace·same-workspace 동시성 테스트(R1), liveness 테스트 5종(R2: 알림 전달 중 Main 도구, 대기 중 자식 보고, 취소 안 종료 처리, 기동 중 닫기, 동시 bootstrap·recover), 작업대 닫기 → 복구 가능 → 다른 작업대로 복구, worktree 변경 시 과제 실패(가짜 엔진), scheduler 대기·진행. describe fixture 재생성(데스크톱 85, readonly, agent).

## R17. run 재생·구독 권한 (설계 리뷰 반영)

**Decision**: hub가 run 스트림을 처음 만들 때 소유 작업대 id를 스트림 메타로 기록한다(journal과 같은 수명 — 보관 한도로 스트림이 제거되면 함께 사라진다). `run.replay` 허용 조건은 둘 중 하나다:

1. 스트림 메타의 소유 작업대를 호출자(작업대 연 주체)가 소유, 또는
2. 그 run이 호출자 작업대에 **지금 묶인 작업 영역의 노드 run**(노드 `currentRunId`, 세대 `runId`, 과제 시도 run id 중 하나)이다 — 작업대 A가 닫힌 뒤 B로 복구한 작업 영역에서 화면이 이전 자식 기록을 재생하는 흐름(`agent-run-runtime-host.tsx:65-75`)을 유지한다(설계 리뷰 H8).

- 보관 한도로 이미 제거된 run(제거 표식)은 **소유 검사 없이** 오늘의 Evicted 형태(`terminal: true, gapDetected: true`, 이벤트 없음)를 돌려준다 — 내용이 없으므로 누수가 없다. 모르는 run은 오늘의 Missing 형태(`gapDetected: afterSequence > 0`).
- `run:<id>` 구독도 같은 소유 조건을 쓴다. 발행 전 run을 기다리는 구독은 엔진에 소유가 등록된 run만 허용한다. 이 검사는 `EventHub`가 아니라 seam `events` 진입점(040 `authorize_bench_streams` 자리)에서 한다(엔진 조회가 async이고 hub 단위 테스트가 임의 run id를 구독하므로). 데스크톱은 hub 구독이 아니라 전달 + `run.replay`를 쓰므로 영향이 없다.

**Rationale**: run 소유(엔진 `run_owners`)는 run이 끝나면 지워져 끝난 run의 재생 권한을 판단할 수 없다. journal이 남아 있는 동안은 재생이 가능해야 하므로(오늘 동작) 판단 근거도 journal과 수명을 같이한다.

## R16. ADR

**Decision**: 되돌리기 어렵고 놀랍고 실제 trade-off가 있는 결정 두 가지를 core ADR로 남긴다 — `0006-orchestration-store-is-one-serialized-aggregate`(R1·R2: 파일 분할·SQLite 대신 aggregate lock, operation 범위 lock 없이 `update` 안 상태 조건 + 묶기 전용 binding mutex), `0007-agent-orchestration-roles-come-from-server-state`(R7: 토큰 주장 대신 서버 상태). 묶임별 스트림 id(R10)는 `0006`이 아니라 `docs/workbench-seam.md`에 규칙으로 적는다.
