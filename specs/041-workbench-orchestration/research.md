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

**Decision**: orchestration 저장은 `StorageCoordinator`의 aggregate `orchestration-sessions` 하나로 직렬화한다. 저장소 포트를 `load`/`save` 두 메서드에서 **`read(|sessions| …)`와 `update(|sessions| -> R)`** 로 바꾸고, `update`는 aggregate lock 안에서 파일 전체 load → 클로저 → save를 한 번에 한다(손상 시 `.bak` 복구 후 1회 재시도, 038 `with_aggregate` 규칙). 서비스의 모든 변경은 이 `update` 하나 안에서 끝난다. await가 필요한 다단계 흐름(명령 전달: 기록 → 엔진 호출 → 결과 기록, 자식 run 기동)은 **각 단계가 독립된 `update`**이며, 단계 사이에서 aggregate lock을 쥐지 않는다. 같은 작업 영역의 다단계 흐름 순서는 작업 영역별 async lock(아래 R2)으로 맞춘다.

- lock 순서: 작업 영역 async lock → 저장소 aggregate lock(동기, await 없음). 역순 획득 없음.
- revision: 작업 영역 revision은 `update` 안에서 변경마다 +1(오늘 규칙). aggregate 수준 revision은 추적하지 않는다(`STORE_AGGREGATES`에 넣지 않음, ledger 미사용).
- 검증: cross-workspace 동시성 테스트 — 작업 영역 2개 이상(각각 다른 작업대)에 thread 여러 개로 변경 100회 이상을 동시에 넣고 끝난 뒤 모든 변경(과제 수·보고 수·revision 합)이 남았는지 확인한다. 같은 작업 영역에 화면 명령 + agent 보고를 동시에 넣는 테스트도 둔다. lock을 빼면 실패하는지(회귀 검출) 확인한다.

**Rationale**: 파일이 하나인 한 작업 영역별 lock은 다른 작업 영역의 저장이 오래된 사본으로 전체를 덮어쓰는 것을 막지 못한다. 038이 이미 같은 문제를 aggregate lock으로 풀었으므로 같은 장치를 쓴다. 작업 영역마다 파일을 나누는 방식은 저장 형식 변경(이전 빌드 호환 깨짐)이라 이 단계에서 피한다.

**Alternatives**: 작업 영역별 파일 분할(형식 변경, 롤백 불가), SQLite 이전(범위 초과, 별도 단계), 파일 lock(fs2)만(프로세스 안 경합에는 in-process lock이 필요하고, 단일 writer인 서버에는 과함).

## R2. 작업 영역별 흐름 직렬화

**Decision**: core `OrchestrationRuntime`이 작업 영역 id별 `tokio::sync::Mutex`를 가진다(작업 영역 수만큼, 작업 영역이 사라지면 정리). 한 작업 영역을 바꾸는 operation handler는 이 lock을 잡고 서비스 단계들을 부른다. 서로 다른 작업 영역은 병렬로 진행하되 저장은 R1 경계로 직렬화된다.

**Rationale**: 명령 전달·자식 기동처럼 await가 낀 흐름이 같은 작업 영역에서 교차하면(예: 취소와 재시도) 오늘도 revision 충돌·중복 기동이 가능하다. 작업 영역 단위로 흐름을 직렬화하면 서비스 규칙을 바꾸지 않고 막는다.

**Alternatives**: 흐름 전체를 aggregate lock 안에서(await 불가·서버 전체 정지), revision CAS 재시도만(흐름 중간 외부 효과 중복 위험).

## R3. 작업 영역 ↔ 작업대 묶임

**Decision**: 묶임은 core 메모리 표 `OrchestrationBindings{workspace_id → Binding{bench_id, binding_id}}`와 역방향 `bench_id → workspace_id`. 작업대 하나에 작업 영역 최대 1개(오늘 창 하나에 1개). 영속 `boundWindowLabel`은 **읽을 때 무시하고 쓸 때 생략**(serde `default`·`skip_serializing`) — 이전 빌드는 필드가 없으면 `None`(복구 가능)으로 읽으므로 호환된다. 서버 재시작 뒤 모든 작업 영역은 복구 가능.

- 묶기: `orchestration.bootstrap`(새로 만들기/재개), `orchestration.recover`. 이미 다른 작업대에 묶인 작업 영역은 오늘 문구 "The workspace cannot be bound to this window."로 거절, 작업대가 이미 다른 worktree에 묶였으면 "The window is already bound to another worktree.".
- 풀기: 작업대 닫기 hook(core `BenchServices::add_close_hook`)이 오늘 `release_window` 규칙(노드 주의 필요·run 재조정)을 `update`로 적용하고, 묶임 스트림을 제거한다. 040 닫기 순서상 소유 run 취소 뒤에 실행된다.
- 창 문구 3개는 바이트 동일을 위해 유지한다(4단계 화면 전환·8단계 정리 때 재검토, 이 결정을 contracts에 명시).

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

agent operation 16개(입력 `runId`, agent 전용, 역할은 서버 상태로 판정 — R7): `orchestration.createChildTask`·`assignChildTask`·`listChildTasks`·`sendChildMessage`·`waitChildTasks`·`collectChildResults`·`interruptChildTask`·`cancelChildTask`·`retryChildTask`·`reassignChildTask`(coordinator), `orchestration.getOwnTask`·`reportProgress`·`reportResult`·`requestParentInput`·`reportBlocked`·`sendParentMessage`(자식). operation은 50 → 84개(데스크톱 18 + agent 16).

`listRecoverable`의 "사라진 창 정리"는 없어진다(묶임은 작업대 닫기에서 풀린다).

## R6. scope

**Decision**: `orchestration:read`·`orchestration:write` 추가(22개). 데스크톱은 전부, readonly는 `:read`, agent는 `orchestration:read`·`orchestration:write`를 더한다(agent 전용 operation과 데스크톱 operation은 역할·작업대 주체 검사로 서로 막힌다 — agent는 작업대를 열지 않으므로 데스크톱 operation은 `forbidden`). 스트림 scope는 `orchestration:read`.

## R7. agent 역할 판정

**Decision**: agent principal(`agent:<runId>`)의 run을 서버 상태로 찾는다 — 작업 영역 중 현재 세대의 coordinator `run_id`가 그 run이면 coordinator, 노드의 `current_run_id`가 그 run이고 배정 과제가 있으면 그 과제의 자식. 둘 다 아니면 `forbidden`(오늘 권한 거절 문구 유지). 교대로 이전 세대가 된 run은 coordinator가 아니므로 자동으로 거절된다 — 토큰 폐기가 권한의 근거가 아니다. MCP 토큰 registry는 AW에 남아 **token → run id**만 한다(040 agent principal과 같음). 역할 주장(workspace·node·task·generation)은 토큰에서 제거한다.

**Rationale**: 토큰 주장은 발급 시점의 사본이라 교대·재배정·복구 뒤 어긋난다. 서버 상태를 근거로 하면 폐기 누락이 권한 누수가 되지 않는다.

## R8. 자식 run 기동과 worktree 감시를 core로

**Decision**: `AgentWorker` 포트의 운영 구현을 core `EngineAgentWorker`로 옮긴다. 자식 기동 = 작업대 입장(040 `admit`, 기동 끝까지 쥠) → `RunLaunchDecorator`(MCP env) → `RunEngine::start(owner = 작업대)`. 보내기·중단·취소는 `RunEngine`의 해당 메서드. worktree 감시(기동 시 지문, 종료 시 비교 → 과제 실패)는 core가 run 종료 이벤트를 받는 자리(`WorkbenchRunSink` 종료 처리)에서 한다. AW `RunTerminalHook`의 orchestration·worktree 부분은 삭제하고, 포트가 비면 포트 자체를 없앤다. `WORKTREE_GUARDS` 전역은 core 런타임 소유 표가 된다(창 label 제거).

## R9. scheduler·알림 전달·복구를 core로

**Decision**: `OrchestrationScheduler`(용량 규칙·FIFO 동일), `CoordinatorNotificationDispatcher`, `OrchestrationCommandService`, 복구 흐름(`reconcile_runtime`·`scheduler.reconcile`·`reconcile_pending`·`recover_interrupted`·`dispatch_pending`)을 core로 옮긴다. scheduler는 런타임당 하나(오늘 프로세스 전역과 같은 의미). 용량·자식 agent 프로필은 `RuntimeAdapters.orchestration`(환경 변수 `ACP_WORKBENCH_MAX_RUNS`·`AW_ORCHESTRATION_AGENT_PROFILE`을 AW가 읽어 주입)으로 받는다. spawn되는 백그라운드 전달은 core가 tokio task로 띄운다.

## R10. 이벤트 스트림

**Decision**: 스트림 = `orchestration:<bindingId>`(묶일 때마다 새 uuid, FR-009). 분류는 상태 복원용(묶임당 journal 256). 스키마 하나 `orchestration.workspaceUpdated.v1`(본문 `OrchestrationEventDto` 그대로 — 명령·알림 변경도 `reason`으로 구분). 발행은 core 서비스의 `persist_mutation`·`emit_workspace_changed` 자리와 오늘 호출자가 손으로 내던 자리(명령 전달·알림 전달·release)를 모두 core 발행으로 모은다. 구독 권한: 묶임의 작업대를 연 주체만(040 `authorize_bench_streams` 확장), 묶임이 끝난 스트림은 제거 표식 → `Gap(evicted)`. 작업 영역 DTO에 `eventStreamId`(현재 묶임의 스트림 id, 없으면 null)를 싣는다.

데스크톱 전달: `DesktopDelivery::Orchestration{bench_id, event}` → 그 작업대 창에 `orchestration-workspace-updated-fallback` CustomEvent 한 번, `reason`이 command/notification이면 오늘처럼 상세 fallback 이벤트도 한 번. 네이티브 `emit`(전체 방송)은 제거한다.

## R11. 호환 command와 `boundWindowLabel`

**Decision**: AW command는 `DesktopBenches`로 창의 작업대를 찾아(`bootstrap`·`recover`는 `ensure`, 나머지는 조회만) `Workbench.call`을 부르고, 결과 작업 영역에 `boundWindowLabel`을 다시 채운다(묶인 작업대가 이 창의 작업대면 이 창 label, 아니면 null) — 화면 타입(`string | null`, 필수 필드) 유지, core에는 창 label 없음. 창에 작업대가 없으면 core를 부르지 않고 오늘 서비스가 내던 결과(`get` → null, 나머지 → 오늘 "작업 영역 없음" 오류 JSON)를 그대로 돌려준다. 오류는 `OrchestrationError` JSON 문자열(fault `details.orchestrationError`에 원본을 싣고 compat가 재구성, 040 교환과 같은 방식).

## R12. MCP 도구

**Decision**: 도구 16개는 `principal.run_id`로 agent principal을 만들어 해당 operation을 `Workbench.call`하고, 결과·오류를 오늘 도구 결과 형태로 되돌린다. 도구 쪽 `runId` 일치 검사 유지. `aw_wait_child_tasks`의 30초 poll은 core operation 안으로 옮긴다(도구는 한 번 호출).

## R13. DTO

**Decision**: protocol에 작업 영역·노드·세대·과제·보고·명령·알림·분배 DTO를 미러로 둔다(040 run DTO와 같은 방식: core 도메인과 serde 왕복 `convert` + wire parity 테스트). DTO에는 `boundWindowLabel`이 없고 `eventStreamId`가 있다.

## R14. 과도기 통로 제거

**Decision**: 제거 목록 — core `WorkbenchRuntime::run_sink`·`admit`(외부 공개)·`run_engine().acp_registry()`·`acp_session_store()` 공개 접근, AW `desktop_benches::label_for`의 서버 방향 사용(데스크톱 전달 경로만 남김), `resolve_agent_run_launch_principal`, `RunTerminalHook`의 orchestration 부분, 창 `Destroyed`의 `release_window`, `McpServerState`의 scheduler, `TauriOrchestrationEventSink`, AW orchestration 도메인·서비스·저장소(모두 core로 이동). 검증: core·protocol grep에서 창 label 0건, AW에서 `acp_registry`·`run_sink` 호출 0건.

## R15. 테스트

**Decision**: AW orchestration 테스트 60개를 core로 옮긴다(위치만 이동, 기대값 유지 — 창 label을 쓰던 테스트는 작업대 id로 입력만 바꾼다). projector(production 미사용)는 테스트와 함께 삭제한다. 새 테스트: fixture(데스크톱 18·agent 16 operation, 교차 작업대·교차 주체·이전 세대·다른 과제 거절), 묶임 스트림 fixture(순번·구독 거절·닫기 gap·재묶임 새 스트림), cross-workspace·same-workspace 동시성 테스트(R1), 작업대 닫기 → 복구 가능 → 다른 작업대로 복구, worktree 변경 시 과제 실패(가짜 엔진), scheduler 대기·진행. describe fixture 재생성(데스크톱 84, readonly, agent).

## R17. run 스트림 소유 기록 (run.replay·run 구독 권한)

**Decision**: hub가 run 스트림을 처음 만들 때(첫 발행 또는 발행 전 구독 대기) 소유 작업대 id를 스트림 메타로 기록한다. 기록은 journal과 같은 수명(보관 한도로 제거될 때 함께 사라짐)이다. `run.replay`와 `run:<id>` 구독은 이 기록(없으면 엔진의 현재 소유)으로 작업대 주체를 검사한다. 040 코드 리뷰가 exchange·bench 스트림에서 막은 "scope만 검사" 누수가 run 스트림에도 남아 있어 함께 닫는다. 발행 전 run을 기다리는 구독은 엔진에 소유가 등록된 run만 허용한다(모르는 id로 미리 붙기 방지).

**Rationale**: run 소유(엔진 `run_owners`)는 run이 끝나면 지워져 끝난 run의 재생 권한을 판단할 수 없다. journal이 남아 있는 동안은 재생이 가능해야 하므로(오늘 동작) 판단 근거도 journal과 수명을 같이한다.

## R16. ADR

**Decision**: 되돌리기 어렵고 놀랍고 실제 trade-off가 있는 결정 두 가지를 core ADR로 남긴다 — `0006-orchestration-store-is-one-serialized-aggregate`(R1·R2: 파일 분할·SQLite 대신 aggregate lock, 다단계 흐름은 작업 영역 lock), `0007-agent-orchestration-roles-come-from-server-state`(R7: 토큰 주장 대신 서버 상태). 묶임별 스트림 id(R10)는 `0006`이 아니라 `docs/workbench-seam.md`에 규칙으로 적는다.
