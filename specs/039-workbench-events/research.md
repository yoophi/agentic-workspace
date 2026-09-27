# Research: 039 이벤트 모델 통합 (2a)

결정은 spec Clarifications(Q1–Q9)와 ADR 4건을 전제로 한다. 아래는 plan 수준 결정이다. 코드 사실은 2026-09-27 조사 기준(main `4cb6be5`).

## 사실 요약(조사)

| 항목 | 오늘 |
|---|---|
| run journal | AW `infrastructure/in_memory_runtime_event_journal.rs`. run id별 `last_sequence += 1`, run당 512개, run journal은 삭제되지 않음(`remove` 미사용). `lib.rs`에서 `.manage` |
| run 발행 | AW `TauriRunEventSink::emit`이 journal에 append(반환값 버림) → `window.emit`(Tauri, 전체 방송) + `window.eval`(삽입). sink 생성처 8곳(`tauri_commands.rs` 7, `acp_agent_worker_adapter.rs` 1), 전부 `with_target(label)` |
| run 수신 | 화면은 삽입 경로(`agent-run-event-fallback`)만 들음. `agent-run-runtime-host.tsx`가 `sequence = lastSequence + 1` 추정, `agent-run-panel.tsx`는 `{runId, event}`만 사용. 타입 `RunEventEnvelope`는 공유 패키지 `@yoophi/agent-client`(hushline도 사용) |
| run replay | `replay_orchestration_runtime_events` command(2b 이연 대상)가 journal `replay` 결과 `RuntimeEventSnapshot`을 그대로 반환 |
| worktree 감시 | AW `fs_worktree_watcher.rs`(notify recursive, 500ms trailing debounce, File/Git 분류, `git rev-parse`로 git 경로). `WorktreeWatcherState.handles: HashMap<창 label, handle>`, 같은 창 재시작은 교체, 창 파괴 시 `stop_for_window`. payload `workingDirectory`는 **호출자가 준 문자열 그대로**이고 화면이 이 값으로 필터한다 |
| protocol | `StreamCursor`·`Subscription`·`EventEnvelope` 타입만 있고 `EventStream`은 빈 struct, `WorkbenchRuntime::events`는 항상 `unsupportedSchema` |
| Tauri emit | 2.11.6 `WebviewWindow`는 `Emitter` 기본 구현 → `manager().emit`(전체 방송). 창 지정은 `emit_to`뿐 |

## R1. `EventStream`의 형태

**Decision**: protocol의 `EventStream`을 `futures_core::Stream<Item = EventItem>`을 감싼 타입으로 바꾼다. `EventItem = Event(EventEnvelope) | Gap(GapNotice)`. 스트림이 끝나는 경우(구독자 대기열 초과, 서버 종료)는 마지막에 `Gap`(reason 포함)을 내고 종료한다. `Workbench::events`는 동기 함수로 유지한다 — 구독 등록·기준점·replay 복사가 한 lock 안에서 끝나므로 await가 필요 없다. `EventStream`을 drop하면 구독이 해제된다(`Drop`에서 hub unsubscribe).

**Rationale**: protocol crate에 tokio를 들이지 않고(`futures-core`만) HTTP·Tauri·in-memory Adapter가 같은 타입을 소비한다. drop 기반 해제는 창 닫힘·WS 종료·테스트 종료를 한 규칙으로 처리한다.

**Alternatives**: async `events` + `tokio::mpsc::Receiver` 노출 — protocol에 tokio 의존, Adapter가 수신 타입에 묶임.

## R2. 구독 조정자 — "한 lock" 구현

**Decision**: core `infrastructure/event_hub`(가칭 `EventHub`)가 스트림마다 `Mutex<StreamState>`를 둔다. `publish`와 `subscribe`는 **같은 스트림 lock**을 잡는다.

- `publish(stream, schema, body)`: lock → `sequence += 1` → (상태 복원용이면) journal push·한도 적용 → 등록된 구독자 대기열에 `try_send` → unlock. 가득 찬 구독자는 overflow로 표시하고 목록에서 뺀다(발행자는 기다리지 않음).
- `subscribe(cursors)`: 권한·입력 검증 → 스트림마다 lock → 구독자 대기열 등록(수신자 먼저) → high-water = 현재 `sequence` capture → cursor 판정(아래) → `after < seq <= high-water` 구간을 대기열 **앞쪽**에 넣을 replay 목록으로 복사 → unlock. 반환 스트림은 replay 목록을 먼저 내고, 이후 대기열에서 `sequence > high-water`인 것만 낸다(중복 제거).

한 lock 안에서 등록과 capture가 일어나므로 capture와 등록 사이에 발행이 끼어들 수 없다. 정본 Ordering 3의 "수신자 먼저 → high-water → replay → drain"을 그대로 따르되, 중복 제거 필터는 방어적으로 유지한다.

**lock 순서(교착 방지)**: hub의 lock은 `streams`(스트림 map) → 개별 스트림 → `terminal_order`·`evicted` 순으로만 잡는다. 스트림 lock을 쥔 채 `streams`를 잡지 않는다. 따라서
- `publish_run`은 스트림 lock 안에서 순번·journal·구독자·`deliver`만 처리하고, **run 정리(보관 run 수 초과 시 스트림 제거)는 스트림 lock을 푼 뒤** `streams` → 대상 스트림 순으로 잡아 수행한다.
- `subscribe`는 cursor마다 `streams`에서 스트림을 찾거나 만든 뒤 `streams`를 풀고 그 스트림 lock을 잡는다. 두 스트림 lock을 동시에 쥐지 않는다(cursor 여러 개도 하나씩).
- `EventStream` drop은 자기가 등록된 스트림 lock만 잡아 구독자를 뺀다.

**Cursor 판정**(상태 복원용 스트림):

| 조건 | 결과 |
|---|---|
| 제거된 run(tombstone 있음), `after` 무관 | `Gap(reason=evicted)` — cursor 0이어도(R5) |
| 스트림 없음 + `after == 0` | 빈 replay, live 대기(시작 전 run 미리 구독) |
| 스트림 없음 + `after > 0` | `Gap(reason=unknownStream)` |
| `epoch ≠ 현재` | `Gap(reason=epochChanged)` |
| `after > last` (같은 세대) | `invalidArgument`("cursor is ahead of the stream.") |
| `after + 1 < first_retained` | `Gap(reason=retentionExceeded)` |
| 그 외 | replay `after+1..=last` |

알림용 스트림은 cursor를 보지 않고 항상 live부터(ADR core 0003). cursor에 `afterSequence`가 있어도 무시한다.

**Rationale**: `tokio::broadcast`는 lag 시 cursor를 앞당기므로(정본 경고) 구독자별 bounded mpsc + `try_send`로 overflow를 명시적 gap으로 바꾼다.

**Alternatives**: receiver-first를 lock 없이(atomic high-water) — 구현이 복잡하고 race test가 더 어렵다.

## R3. 세대와 계약 조회

**Decision**: `WorkbenchRuntime::bootstrap`이 `uuid v4`로 세대를 만든다(`epoch`). `system.describe` 출력에 `epoch`와 `eventSchemas`(principal에게 허용된 것만: `schema`, `streamKind`, `class`, `requiredScopes`)를 추가한다(additive, `PROTOCOL_VERSION` 1 유지). 모든 `EventEnvelope.epoch`와 `GapNotice.epoch`는 이 값이다.

## R4. 스트림 식별자와 권한

**Decision**: 스트림 식별자는 `<kind>:<key>` 문자열. 039가 구독을 여는 kind는 `run`(key = run id)과 `worktree`(key = 호출자가 준 경로 → hub가 `canonicalize`한 실제 경로로 정규화). `orchestration`·`exchange`는 이름만 예약한다(구독 시 `invalidArgument` "stream kind is not available yet."). 권한: `run` → 신설 `run:read`, `worktree` → `worktree:read`. Scope가 14개가 되고 데스크톱·조회 전용 호출자 모두 두 scope를 갖는다. 허용되지 않은 kind는 1단계와 같은 `forbidden`.

## R5. run journal 이동과 제거 표식(tombstone) (Codex 리뷰 2차 반영)

**Decision**: AW `InMemoryRuntimeEventJournal`·`ports/runtime_event_journal.rs`를 삭제하고 hub의 상태 복원용 스트림이 journal을 겸한다. 한도: run당 512(오늘 값), 보관 run 수 상한 256(ADR core 0002, Q8). 상한 초과 시 **terminal로 표시된 run 중 terminal이 가장 먼저 된 것**부터 스트림째 제거한다. 진행 중 run은 제거하지 않으므로 상한을 일시적으로 넘을 수 있다. terminal 판정은 오늘과 같다(`Lifecycle Completed | Cancelled`).

**제거 표식**: 스트림을 제거할 때 run id를 `evicted` 집합(FIFO, 상한 4,096개)에 남긴다. 그래야 "제거된 run"과 "아직 시작하지 않은 run"을 cursor 0에서도 구별할 수 있다.

| 조회 | 제거된 run(표식 있음) | 표식도 스트림도 없음 |
|---|---|---|
| `Workbench.events` | cursor와 무관하게 `Gap(reason=evicted)` | cursor 0 → live 대기, >0 → `Gap(unknownStream)` |
| 호환 replay command | `{events: [], lastSequence: 0, terminal: true, gapDetected: true}` → 화면이 `gap` 표시 | 오늘과 같음(cursor 0 → 빈 snapshot, >0 → `gapDetected`) |

**제거되는 스트림의 구독자**: 스트림을 제거할 때 그 스트림에 등록된 구독자에게는 `Gap(reason=evicted)`를 보내고 그 스트림에서 해제한다(구독의 다른 스트림은 유지). 조용히 끊기지 않는다.

**제거된 run에 다시 발행**: terminal 뒤 오래 지나 같은 run id로 발행이 오면(예: 끝난 run에 대한 늦은 cancel 이벤트) 스트림을 새로 만들지 않고 버린다(hub·구독자·데스크톱 모두 전달 안 함, 진단 로그만). 새로 만들면 순번이 1부터 다시 시작해 이전 cursor가 "미래"가 되기 때문이다.

표식 4,096개를 넘으면 가장 오래된 표식부터 버린다. 그보다 오래된 run은 다시 "알 수 없는 run"이 된다(보관 run 256 + 표식 4,096 = 한 세대 안에서 4,352개 run까지 구별). 표식은 run id 문자열만 담으므로 수백 KB 이내다.

`replay_orchestration_runtime_events`(2b 이연 command)는 hub의 run replay를 읽어 **오늘과 같은 `RuntimeEventSnapshot` 형태**로 돌려준다(화면 hydrate 코드 불변). 제거된 run의 응답만 위 표처럼 새로 정해진다(오늘은 run journal을 지우지 않았으므로 이 경우가 없었다).

**이전 세대의 run(한계)**: 서버 재시작 뒤 새 세대의 hub는 이전 세대 run을 모른다. cursor를 가진 구독자는 `Gap(epochChanged)`를 받지만, 화면처럼 **새 컨트롤러가 cursor 0으로** 재수화하면 오늘과 같이 빈 `ready`가 된다(2026-09-27 확인: 오늘도 재시작 뒤 `gapDetected`는 cursor > 0일 때만 켜진다). 이전 세대 run을 "실행 정보 유실"로 확정하려면 run 목록의 정본이 필요하며, run registry가 core로 오는 2b에서 다룬다. spec US2-3·FR-005·Edge Cases를 이 사실에 맞게 정정했다.

## R6. 데스크톱 run 전달 — 순번 부여와 전달을 한 lock에서 (Codex 리뷰 2차 반영)

**문제**: 한 run에 여러 발행자가 동시에 emit한다(ACP 본 흐름, stderr 읽기 task `runner.rs:184`의 `Diagnostic`, steer·cancel 경로). `publish_run`이 lock을 풀고 봉투를 돌려준 뒤 sink가 전달하면, 발행자 A가 11을 받고 멈춘 사이 B가 12를 창에 먼저 보낼 수 있다. 화면은 12를 적용하고 늦게 온 11을 중복으로 버린다(R7의 `sequence <= lastSequence` 규칙). `gap` 표시로는 잃은 메시지·권한 요청이 돌아오지 않는다.

**Decision**: hub가 **순번 부여와 데스크톱 전달을 같은 스트림 lock 안에서** 수행한다. `publish_run(run_id, event, terminal, deliver)`는 lock 안에서 sequence 부여 → journal·구독자 전달 → `deliver(&envelope)` 호출 → unlock 순으로 실행한다. AW sink는 `deliver`에서 대상 창에 삽입 스크립트를 넣는다. 서로 다른 sink 인스턴스가 같은 run에 발행해도 hub의 스트림 lock이 하나이므로 창에 들어가는 순서 = 순번 순서다. webview 스크립트 실행은 FIFO이므로 화면 도착 순서도 같다.

`deliver` 제약(문서·주석으로 고정):
- 막히지 않아야 한다. `window.eval`은 스크립트를 webview 대기열에 넣고 바로 돌아오므로 충족한다.
- hub를 다시 호출하면 안 된다(같은 스트림 lock 재진입 → 교착).
- 실패해도 hub 상태에 영향이 없다(반환값 없음).

이 lock 보유 시간 증가는 `eval` 호출 비용뿐이며 SC-006 측정(R13)에 포함한다. 삽입 payload는 공유 타입 `RunEventEnvelope {runId, event}`의 **상위집합** `{runId, event, sequence, epoch, streamId, eventId}`이다(`@yoophi/agent-client`의 `RunEventEnvelope`(hushline 공유)는 바꾸지 않고 AW에서 확장 타입을 둔다. 패널은 추가 필드를 무시한다). Tauri `agent-run-event` 발행과 `target_label = None` 분기(도달 불가)는 제거한다. 창이 없으면 전달하지 않는다(오늘도 그 창의 listener가 없다). worktree guard 검증 등 sink의 부수 로직은 lock 밖(`publish_run` 뒤)에서 그대로 한다.

**Concurrency test**(`tests/run_delivery_order.rs`): 두 thread가 같은 run에 발행하고, `deliver`가 순번 11에서 50ms 멈추는 동안 다른 thread가 발행한다 → 기록된 전달 순서가 순번 오름차순이고 빠진 번호가 없다. 1,000회 반복(무작위 지연).

**Alternatives**: 봉투를 돌려받은 뒤 AW에 run별 전달 큐를 두기 — 모든 sink 인스턴스가 같은 큐를 공유해야 하고, 큐 적재도 순번 부여와 원자적이어야 하므로 결국 같은 lock이 필요하다. 화면에서 순서가 뒤바뀐 이벤트를 재정렬 버퍼로 기다리기 — 언제까지 기다릴지 정할 수 없다.

## R7. 프론트 변경 범위 — 재수화 중 live 버퍼링 (Codex 리뷰 반영, 2026-09-27)

**문제**(오늘 reducer 그대로 두면): 데스크톱은 hub 구독을 쓰지 않으므로(ADR 0003) hub의 replay/live 동기화가 화면 재수화를 보호하지 않는다. cursor 0으로 replay를 요청해 서버가 1–10을 capture한 뒤 응답 전에 live 11이 오면, `applyLiveRuntimeEvent`가 `lastSequence`를 11로 올리고 이어 도착한 snapshot(`lastSequence` 10)은 `applyRuntimeSnapshot`의 `snapshot.lastSequence < state.lastSequence` 규칙에 걸려 **통째로 버려진다**. 1–10이 gap 표시 없이 사라진다. 서버 번호를 실어도 이 경합은 남는다.

**Decision**: 컨트롤러가 재수화가 끝날 때까지 live 이벤트를 **버퍼링**한다.

| 컨트롤러 상태 | live 이벤트 처리 |
|---|---|
| `idle`·`loading`(재수화 전·중) | 적용하지 않고 `pendingLive`에 보관(같은 `sequence`는 한 번만) |
| snapshot 도착(`applySnapshot`) | ① snapshot 이벤트 중 `sequence > lastSequence`를 적용 ② `pendingLive`를 `sequence` 오름차순으로 정렬해 `sequence <= lastSequence`는 버리고, `sequence == lastSequence + 1`이면 적용 ③ 그보다 크면(빈틈) 나머지는 적용하되 상태를 `gap`으로 표시 ④ 버퍼 비움 |
| `ready`·`gap` | 바로 적용. `sequence <= lastSequence`는 무시(중복), `sequence > lastSequence + 1`이면 적용하고 `gap`으로 표시 |
| 재수화 실패(`markRuntimeLost`) | 버퍼의 이벤트를 위 ②③ 규칙으로 적용하되 상태는 `runtimeLost` 유지(유실 표시가 우선) |

- snapshot을 버리는 규칙(`snapshot.lastSequence < state.lastSequence`)은 버퍼링으로 재수화 중에는 도달하지 않는다. 재수화가 아닌 경로에서 오래된 snapshot이 오는 경우를 위해 남겨 둔다.
- 버퍼 크기는 run당 journal 보관 한도(512)를 상한으로 둔다. 넘치면 가장 오래된 것부터 버리고 drain 때 빈틈으로 `gap` 표시된다.
- `terminal` 계산(오류 포함)은 오늘 화면 동작이므로 유지한다.
- 변경 파일: `features/agent-run/model/agent-run-controller.ts`(버퍼·drain·gap 규칙, 순수 함수로 분리), `agent-run-runtime-host.tsx`(`sequence: envelope.sequence`), `entities/agent-run/model/types.ts`(`DeliveredRunEvent`), `entities/agent-run/api/agent-run-repository.ts`(콜백 타입).

**Regression tests**(vitest, `agent-run-controller.test.ts`):
1. `loading` 중 live 11 도착 → snapshot(1–10, last 10) → 최종 `events` = 1–11 순서, `lastSequence` 11, 상태 `ready`.
2. `loading` 중 live 12·11(역순) 도착 → snapshot(1–10) → 1–12 순서.
3. `loading` 중 live 10·11 도착(10은 snapshot에도 있음) → 10은 한 번만.
4. `loading` 중 live 13 도착 → snapshot(1–10) → 1–10, 13 적용, 상태 `gap`.
5. `ready`에서 live 12(`lastSequence` 10) → 적용, 상태 `gap`.
6. 재수화 실패 → 버퍼의 이벤트 적용, 상태 `runtimeLost`.

**Alternatives**: 데스크톱을 hub 구독자로 바꿔 서버 쪽 동기화를 쓰기 — 창↔run 연결을 정해야 해 2b 범위(ADR 0003). replay 응답에 high-water를 싣고 그 이하 live를 버리는 방식 — 버퍼링과 같은 효과지만 응답 전 도착한 live를 어딘가 보관해야 하는 점은 같다.

## R8. run 이벤트 본문 계약

**Decision**: protocol `events/run.rs`에 `RunEventDto`(acp-agent-core `RunEvent` 14 variant, `#[serde(tag = "type", rename_all = "camelCase")]` 등 원본 serde 속성 그대로)와 보조 DTO(`LifecycleStatus` 등)를 둔다. hub는 본문을 원본 `RunEvent`의 `serde_json::to_value`로 저장·전달하고, DTO는 스키마 생성과 wire 동일성 테스트(core, 모든 variant)에만 쓴다. 스키마 이름 `run.event.v1`, 분류 상태 복원용.

## R9. worktree 스트림

**Decision**: AW `fs_worktree_watcher.rs`를 core `infrastructure/fs/worktree_watcher.rs`로 옮긴다(`notify` 의존 core로 이동, perf 로그는 core `infrastructure::perf`). hub는 실제 경로별 **참조 수**를 두어 첫 구독에서 `watch_worktree`를 시작하고 마지막 구독 drop에서 handle을 버린다(감시 thread 종료). 감시 콜백이 `publish(worktree:<canonical>, "worktree.changed.v1", body)`를 부른다. 본문은 `{workingDirectory: <canonical>, changedPath, kind}`. 분류 알림용(보관 없음).

**AW 호환**: `start_worktree_watcher(window, working_directory)`는 `runtime.events(desktop, Subscription{cursors:[worktree:<wd>]})`를 blocking pool에서 호출하고, 반환 스트림을 소비하는 task를 띄워 이벤트마다 본문 `workingDirectory`를 **그 창이 준 원래 문자열로 바꿔** `emit_to(label, "workspace://worktree-changed")`한다(화면 필터 호환). task handle을 `WorktreeWatcherState`에 창 label로 저장(교체 시 기존 task abort → 스트림 drop). `stop_worktree_watcher`·창 파괴는 abort. 감시 시작 실패 문구(`Cannot watch missing worktree path: …`)는 fault message로 그대로 전달한다.

## R10. 한도 기본값

| 한도 | 값 | 초과 시 |
|---|---|---|
| run당 보관 | 512 | 오래된 것부터 버림(오늘과 같음) |
| 보관 run 수 | 256 | 가장 먼저 terminal이 된 run 제거 + 제거 표식 |
| 제거 표식 | 4,096 run id | 가장 오래된 표식부터 버림(그 run은 다시 "알 수 없음") |
| 구독자 대기열 | 1,024 항목 | 그 구독을 `Gap(reason=subscriberLagged)`로 닫음 |
| 동시 구독 수 | 256 | `events`가 `rateLimited` |
| 구독 하나의 cursor 수 | 64 | `invalidArgument` |

run 이벤트는 초당 수십 건(스트리밍 토큰 단위 메시지)이므로 1,024는 화면이 수 초 멈춰도 견딘다. 값은 상수로 두고 test-hooks로 낮춰 overflow를 시험한다.

## R11. 테스트 HTTP WebSocket

**Decision**: 037 `http_harness`에 `GET /v1/events`(WebSocket upgrade, bearer 헤더 인증)를 추가한다. dev-dependency: `axum` `ws` feature, `tokio-tungstenite`. 프레임(JSON text):

| 방향 | 프레임 |
|---|---|
| server → client | `{"type":"hello","protocolVersion":1,"epoch":…}` |
| client → server | `{"type":"subscribe","cursors":[StreamCursor…]}`(첫 프레임, 한 번) |
| server → client | `{"type":"event","event":EventEnvelope}` · `{"type":"gap",…GapNotice}` · `{"type":"fault","fault":WorkbenchFault}`(뒤이어 close) |

프레임 타입은 protocol `EventFrame`(oneOf)로 정의해 OpenAPI component로 내보낸다. 3단계 운영 Adapter가 같은 프레임을 쓰고 ticket 인증만 추가한다.

## R12. 계약 생성

**Decision**: OpenAPI components에 `EventEnvelope`(기존), `GapNotice`, `EventItem`, `EventFrame`, `EventSchemaDescriptor`, 그리고 `EventBySchema` oneOf(variant마다 `schema` 단일값 enum + typed `body`)를 registry(`events::EVENT_SCHEMAS`)에서 조립한다. 039 variant: `run.event.v1`(RunEventDto), `worktree.changed.v1`(WorktreeChangedDto), 2b 예약 `orchestration.workspaceUpdated.v1`(OrchestrationEventDto: workspaceId·revision·reason·taskId?·nodeId? — AW 타입이 작아 지금 정의). exchange 본문(`AgentExchange` 등)은 AW 도메인 타입이 크고 2b에서 core로 옮겨질 때 미러를 만든다 — 039는 이름·분류만 registry에 둔다(`body` 스키마 없이 describe에만). TS: `EventMap = { [K in EventSchemaId]: Extract<EventBySchema, {schema: K}>["body"] }`와 test-d.

## R13. 지연 측정(SC-006)

**Decision**: core `tests/event_latency.rs`(#[ignore])에서 `publish_run` p95와 구독자 수신까지 p95를 측정한다(1,000회). 기준선은 AW 오늘 경로의 journal append(동일 크기 `Value`). 삽입(`window.eval`) 비용은 변하지 않으므로 측정 대상에서 뺀다.

## R14. race test(SC-001)

**Decision**: `tests/event_subscription_race.rs`: 발행 thread가 한 run 스트림에 연속 발행하는 동안 1,000회 `subscribe(cursor=임의 k)` → 각 구독에서 처음 N개를 받아 `k+1, k+2, …`로 연속인지 확인. 대기열 overflow·epoch·retention fixture는 in-memory와 WS 경로 공통 fixture(`crates/workbench-protocol/fixtures/events/*.json`)로 실행한다.

## R15. 파일 경로 정규화

worktree kind는 hub에서 `std::fs::canonicalize`. 실패(없음)면 fault `notFound`("Cannot watch missing worktree path: …" — 오늘 문구). 같은 실제 경로의 서로 다른 표기(끝 `/`, 심볼릭 링크)는 같은 스트림이다.

## 정리: 의존성 변화

| crate/package | 추가 | 제거 |
|---|---|---|
| `workbench-protocol` | `futures-core` | — |
| `workbench-core` | `notify`(AW에서 이동) | — |
| `workbench-core` dev | `axum` `ws` feature, `tokio-tungstenite`, `futures-util` | — |
| `apps/agentic-workbench/src-tauri` | — | `notify`(사용처가 `fs_worktree_watcher.rs` 하나뿐임을 확인) |
