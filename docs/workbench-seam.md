# Workbench Seam (서버-클라이언트 전환 1단계)

> 상태: 037·038·039·040 구현 완료(`specs/037-workbench-seam` 2026-09-26, `specs/038-workbench-domains` 2026-09-27, `specs/039-workbench-events` 2026-09-27, `specs/040-workbench-owners` 2026-09-27). 정본 설계는 [서버-클라이언트 전환 조사](client-server-architecture-research.md)이며, 이 문서는 1단계(1a Seam, 1b 도메인 이관)와 2단계(2a 이벤트 Seam, 2b-1 작업대)가 실제 코드에서 어떻게 성립했는지와 이후 단계가 따를 규칙을 기록한다.

## 범위

- `crates/workbench-protocol`: wire 계약 — `CallRequest`/`CallReply`/`WorkbenchFault`, principal·scope, operation descriptor, OpenAPI 3.1 생성. operation **50개**.
- `crates/workbench-core`: `Workbench` 구현 — operation registry, authorization, 멱등성, intent-first runner, 재시작 판정(reconciler), `StorageCoordinator`, SQLite operation ledger, 그리고 AW에서 옮겨 온 도메인·서비스·어댑터.
- `apps/agentic-workbench/src-tauri`: Tauri command **43개**가 `Workbench.call`을 쓰는 호환 어댑터다(`inbound/workbench_compat.rs`). 옮긴 도메인의 코드는 AW에 남아 있지 않다.
- `packages/workbench-client`: 생성 타입(`src/generated/workbench.ts`)과 조건부 타입 `OperationMap`·`EventMap`. 아직 어떤 앱도 import하지 않는다.
- 039: `Workbench.events` — core `EventHub`가 run(상태 복원용)·worktree(알림용) 스트림을 발행·구독한다. AW run 이벤트와 worktree watcher command 2개가 hub를 거친다. 아래 [이벤트 스트림](#이벤트-스트림-039).
- 040: **작업대(Bench)** — 창 label 대신 서버가 발급한 작업대가 run·교환을 소유한다. run 8·교환 4 command와 MCP 교환·제목 도구가 `Workbench.call`을 통과하고(operation 18개 추가), 교환(`exchange:<작업대>`)·작업대(`bench:<작업대>`) 스트림이 구독 가능해졌다. 아래 [작업대](#작업대-040).

038이 옮긴 도메인과 operation(29개):

| 도메인 | operation | 저장·부작용 |
|---|---|---|
| 프로젝트 | `project.update` · `project.delete` | `projects.json` |
| saved prompt | `savedPrompt.list` · `create` · `update` · `delete` | `saved-prompts.json` |
| goal | `goal.get` · `create` · `update` · `clear` · `recordProgress` | `goals.json` |
| agent 실행 설정 | `agentRunSettings.get` · `save` | `agent-run-settings.json` |
| Git(저장소 단위) | `git.listRemotes` · `listBranches` · `listWorktrees` · `createWorktree` · `deleteWorktree` | 사용자 저장소(생성·삭제는 외부 부작용) |
| worktree(체크아웃 디렉터리 단위) | `worktree.listChanges` · `getChanges` · `getFileDiff` · `listFiles` · `readTextFile` · `listHistory` · `getGraph` · `getCommitDetail` · `getCommitFileDiff` | 사용자 저장소·파일시스템(읽기) |
| agent | `agent.list` · `agent.listProviderSessions` | 실행 환경(환경 변수 catalog, provider 로컬 파일) |

## 비범위

프론트엔드 통신 방식(여전히 Tauri `invoke`), **orchestration command 18개**(2b-2(041), [ADR 0001](adr/0001-defer-event-bound-commands-to-stage-2.md); watcher 2개는 039, run 8·교환 4는 040에서 이관), **데스크톱 표현 상태 command 8개**(글꼴·layout·창 열기·외부 URL — 데스크톱에 유지), 운영 HTTP/WS 노출과 토큰(3단계), Desktop 전환(4단계), daemon(5단계 이후). 전체 목록은 아래 [command 인벤토리](#command-인벤토리-71).

## 세 호출 경로와 Seam

```mermaid
flowchart LR
    subgraph AW["apps/agentic-workbench/src-tauri"]
        TC["tauri_commands (31개 호환 command)"]
        Compat["inbound/workbench_compat.rs<br/>인자 → CallRequest.input<br/>CallReply/Fault → Result&lt;_, String&gt;"]
        Setup["lib.rs setup: DataPaths → WorkbenchRuntime::bootstrap → app.manage"]
    end
    subgraph Core["crates/workbench-core"]
        RT["WorkbenchRuntime<br/>protocolVersion → authorize → dispatch"]
        Reg["Registry (handlers/*): 32 operation"]
        IF["intent_first::IntentFirst<br/>멱등성 → pending → lock 안 apply → applied"]
        Rec["reconcilers/*<br/>기동 시 pending 판정"]
        Coord["StorageCoordinator<br/>aggregate lock · revision 캐시"]
        Ledger["SqliteOperationLedger (schema v2)<br/>operation_ledger · aggregate_revision"]
        JSON["JsonCollectionStore&lt;T&gt;<br/>projects · saved-prompts · goals · agent-run-settings"]
        Git["infrastructure/git/*<br/>git CLI 어댑터"]
        FS["infrastructure/fs/*<br/>파일 목록·미리보기·provider 세션"]
        Ad["RuntimeAdapters<br/>agent catalog · provider 세션"]
    end
    subgraph Tests["crates/workbench-core/tests"]
        Mem["in-memory: runtime.call 직접 호출"]
        HTTP["http_harness: Axum POST /v1/calls (dev-dependency)"]
        Fx["crates/workbench-protocol/fixtures/*.json (128개)"]
    end
    TC --> Compat --> RT
    Setup --> RT
    Mem --> RT
    HTTP --> RT
    RT --> Reg
    Reg --> IF --> Coord --> JSON
    IF --> Ledger
    IF --> Git
    Reg --> Git
    Reg --> FS
    Reg --> Ad
    Rec --> Ledger
    Rec --> JSON
    Rec --> Git
    Fx -.같은 fixture.-> Mem
    Fx -.같은 fixture.-> HTTP
    Fx -.변환 유닛 테스트.-> Compat
```

세 경로는 같은 `AuthenticatedPrincipal`·`CallRequest`를 `WorkbenchRuntime::call`에 넘기고 같은 `CallReply`/`WorkbenchFault`를 받는다. contract suite(`tests/contract_suite.rs`)가 fixture마다 in-memory와 HTTP 결과를 서로 비교한다. Git fixture는 경로마다 결정적인 임시 저장소를 만들고(`seed.gitRepo`, 고정 author·날짜 → 같은 커밋 해시), 비교 전 저장소 경로를 자리표시자로 되돌린다.

## 이벤트 스트림 (039)

계약 정본: `specs/039-workbench-events/contracts/workbench-events.md`. 용어: `crates/workbench-core/CONTEXT.md` "이벤트".

- **봉투**: `EventEnvelope {eventId, streamId, epoch, sequence, schema, occurredAt, correlationId?, body}`. `streamId`는 `<kind>:<key>`, `sequence`는 스트림 안에서 1부터 1씩 증가한다.
- **분류**: run은 상태 복원용(run당 512개 보관, replay), worktree는 알림용(보관 없음, 구독 이후만) — [core ADR 0003](../crates/workbench-core/docs/adr/0003-notification-events-are-not-replayed.md). 040부터 교환(`exchange:<작업대>`, 상태 복원용, 작업대당 512)·작업대(`bench:<작업대>`, 알림용) 스트림도 연다. orchestration 스키마는 예약(구독 거절 `stream kind is not available yet.`, 041).
- **세대**: 기동마다 새 `epoch`(uuid). journal은 메모리에만 있다 — [core ADR 0002](../crates/workbench-core/docs/adr/0002-event-journal-is-in-memory-with-server-epoch.md). `afterSequence == 0`은 세대를 보지 않는다.
- **구독 순서**: 한 스트림의 발행(순번 부여 → journal → 구독자 fan-out → 데스크톱 전달)과 구독(등록 → 기준점 → replay 복사)이 **같은 스트림 lock** 안에서 끝난다. 그래서 replay와 live 사이에 빈틈·중복이 없다(race test 1,000회). lock 순서는 `streams` → 스트림 → `retention`이고, 제거는 스트림 lock을 놓은 뒤 한다.
- **gap**: 이어 붙일 수 없으면 `GapNotice{reason}`을 보내고 그 스트림 전달을 멈춘다 — `unknownStream`(없는 스트림, cursor > 0), `evicted`(보관 한도로 지운 run, 제거 표식 4,096개), `epochChanged`, `retentionExceeded`, `subscriberLagged`(대기열 1,024 초과 시 구독 전체 종료), `shutdown`. cursor가 스트림 끝보다 앞서면 `invalidArgument`.
- **한도**(`EventHubLimits`, `RuntimeAdapters.event_limits`로 주입): run당 512 · 보관 run 256(가장 먼저 끝난 run부터 제거) · 제거 표식 4,096 · 구독자 대기열 1,024 · 동시 구독 256(`rateLimited`) · 구독당 cursor 64(같은 스트림 중복 거절). 한도는 한 번이라도 발행된 run만 센다 — 시작 전 run을 기다리던 빈 스트림은 구독이 모두 떠나면 지운다.
- **권한**: run은 `run:read`(신규), worktree는 `worktree:read`, 교환은 `exchange:read`, 작업대는 `bench:read`(040).
- **계약 생성**: `EVENT_SCHEMAS` registry → `system.describe.eventSchemas`, OpenAPI `EventBySchema`(스키마 id ↔ typed 본문), TS `EventMap`. 본문은 원본 타입을 그대로 직렬화하고 protocol DTO는 미러다(wire parity 테스트).
- **테스트 경로**: fixture 27개(`crates/workbench-protocol/fixtures/events/`)를 in-memory와 테스트 WebSocket(`GET /v1/events`)에서 실행해 결과를 비교한다.

### 데스크톱 전달

데스크톱은 구독자가 아니다. run sink(040부터 core `WorkbenchRunSink`, 전달은 `DesktopBridge` 포트의 AW 구현 `TauriDesktopBridge`)가 `publish_run`으로 발행하고, hub가 스트림 lock 안에서 넘겨주는 봉투를 그대로 창에 삽입한다 — [ADR 0003](adr/0003-desktop-forwards-published-run-events.md). 전달 경로는 `window.eval` CustomEvent(`agent-run-event-fallback`) 하나다([ADR 0004](adr/0004-run-events-keep-only-the-script-injection-path.md)). 창이 받는 payload는 공유 봉투의 상위 집합 `{runId, event, sequence, epoch, streamId, eventId}`다.

```mermaid
sequenceDiagram
    participant Runner as run 작업(여러 task)
    participant Sink as WorkbenchRunSink
    participant Hub as EventHub (run:<id> lock)
    participant Win as 창 (agent-run-runtime-host)
    participant Ctl as AgentRunController
    Runner->>Sink: emit(RunEvent)
    Sink->>Hub: publish_run(run, event, terminal, deliver)
    Hub->>Hub: sequence += 1 · journal · fan-out
    Hub->>Win: deliver(envelope) → DesktopBridge → window.eval CustomEvent
    Win->>Ctl: applyLive(sequence = envelope.sequence)
    Note over Ctl: idle·loading이면 pendingLive에 모았다가<br/>snapshot 적용 뒤 순번 순으로 비움. 빈틈이면 gap
    Win->>Hub: replay_orchestration_runtime_events → replay_run
    Hub-->>Ctl: RunReplay(같은 순번) → applySnapshot
```

순번을 lock 안에서 붙이고 같은 lock 안에서 창에 넣으므로, 여러 task가 동시에 발행해도 창은 순번 순서로 받는다(`tests/run_delivery_order.rs`). 화면은 재수화 전·중에 온 live 이벤트를 버퍼(최대 512)에 두었다가 snapshot 뒤 비운다 — replay 응답보다 먼저 온 live가 snapshot 전체를 버리게 하던 경합을 없앤다.

### worktree 구독 호환

`start_worktree_watcher`는 blocking pool에서 `Workbench.events(desktop, [worktree:<경로>])`를 구독하고, async task가 스트림을 소비해 오늘과 같은 `workspace://worktree-changed` 이벤트를 그 창에만 보낸다. 감시는 실제 경로(`canonicalize`)별 참조 수로 공유되어 첫 구독에서 시작하고 마지막 해지에서 멈춘다. 스트림은 실제 경로로 공유되지만 화면은 자신이 넘긴 경로 문자열로 이벤트를 거르므로, 본문 `workingDirectory`는 호출자 문자열로 되돌려 보낸다. `stop_worktree_watcher`와 창 닫힘은 task를 abort해 구독을 놓는다. 없는 경로는 오늘 문구(`Cannot watch missing worktree path: …`) 그대로다.

## 작업대 (040)

계약 정본: `specs/040-workbench-owners/contracts/{workbench-benches,tauri-compat}.md`. 용어: `crates/workbench-core/CONTEXT.md` "작업대".

- **소유**: run과 교환 작업 영역은 창 label이 아니라 **작업대**(`benchId`, 서버 발급 uuid)가 소유한다 — [core ADR 0004](../crates/workbench-core/docs/adr/0004-benches-own-runs-and-exchanges.md). core·protocol에는 창 label이 없다. 작업대는 연 principal의 주체(`PrincipalSubject`)에 묶이고, `benchId`를 받는 모든 호출이 주체를 대조한다(`forbidden` `"bench belongs to another principal."`).
- **수명**: `bench.open`(작업 디렉터리 → `benchId`, 상한 256) · `bench.close`(멱등). 상태는 `Open`·`Closing` 둘뿐이고 메모리 전용이다(재시작하면 사라진다). 연결이 끊겨도 닫히지 않는다 — 닫기는 명시 호출뿐이다([ADR 0005](adr/0005-window-close-explicitly-closes-the-bench.md)).
- **입장 경계**: 새 자원을 등록하는 동작(`run.start`의 소유 기록, 교환 쓰기, 과도기 orchestration 기동)은 작업대의 read guard를 얻어야 한다. registry lock 안에서 `try_read_owned`로 얻고 lock 밖에서 await한다. 닫기는 `Open → Closing`을 원자적으로 바꾼 뒤 write guard를 기다리고, 소유 run 취소 → 교환 작업 영역 삭제 → 스트림 제거(구독자 `Gap(evicted)`) → registry 삭제 순으로 끝낸다. 닫기가 `closed: true`로 돌아온 시점에 그 작업대 소유의 살아 있는 run은 0개다.
- **멱등성**: `run.start`만 SQLite ledger(`durable`)를 쓰고, 나머지 command는 작업대 수명의 세대 범위 기록(`epoch`, 결과 1,024 + 요약 65,536)을 쓴다 — [core ADR 0005](../crates/workbench-core/docs/adr/0005-only-run-start-uses-the-operation-ledger.md). descriptor의 `idempotencyScope`로 드러난다.
- **run 엔진**: `RunEngine` 포트. 운영 구현 `AcpRunEngine`(acp-agent-core `AppState`·runner·세션 저장소), 테스트는 대본 기반 가짜 엔진. `DesktopBridge`(전달)·`RunLaunchDecorator`(run에 묶인 MCP 토큰·env, 토큰 폐기)는 AW가 주입한다. run 종료 후처리(orchestration worktree 감시)는 041부터 core가 소유한다.

### 데스크톱 대응

데스크톱은 창마다 작업대 하나를 둔다. AW `DesktopBenches`가 label ↔ `benchId` 표를 들고, 창이 처음 작업대가 필요한 command를 부를 때 연다(`ensure`, label별 직렬화). 창 `Destroyed`에서 닫는다 — 닫힌 label은 다시 열 수 없다. 창 label은 이 표 밖으로 나가지 않는다.

```mermaid
sequenceDiagram
    participant Win as 세션 창
    participant Cmd as Tauri command (compat)
    participant DB as DesktopBenches
    participant WB as Workbench.call
    participant Bench as 작업대 registry
    Win->>Cmd: start_agent_run(request)
    Cmd->>DB: ensure(label, cwd)
    DB->>WB: bench.open (처음 한 번)
    WB->>Bench: Open 작업대 생성
    Cmd->>WB: run.start {benchId, request}
    WB->>Bench: admit (read guard) → 소유 기록
    Note over Win,Bench: 창 닫힘
    Win->>DB: Destroyed → close(label)
    DB->>WB: bench.close
    WB->>Bench: Closing → write guard → run 취소 → 교환 삭제 → 스트림 제거
```

이벤트는 `TauriDesktopBridge`가 작업대 → 창을 찾아 창 삽입 경로 하나로 보낸다(run `agent-run-event-fallback`, 교환 `agent-exchange-requested-fallback`·`agent-exchange-status-fallback`, 제목 `mcp-window-title-fallback`). 네이티브 `emit`은 없다.

### MCP 도구와 agent principal

AW MCP 서버의 교환 도구 3개와 `set_window_title`은 run capability에서 만든 **agent principal**(`agent:<runId>`, scope `exchange:read`·`exchange:write`·`presentation:write`·`system:describe`)로 `Workbench.call`을 부른다 — [ADR 0006](adr/0006-mcp-tools-call-the-workbench-as-an-agent-principal.md). agent 전용 operation(`exchange.listPeers`·`sendFromRun`·`getForRun`·`bench.requestTitle`)은 주체의 run과 입력 `runId`가 같아야 한다. 제목 요청은 서버 상태를 바꾸지 않고 run의 작업대 알림 스트림(`bench.titleRequested.v1`)에 발행되어, 데스크톱이 그 작업대의 창에만 적용한다 — [ADR 0007](adr/0007-presentation-requests-are-bench-notifications.md). 도구 결과 형태(`TitleChangeResult`, 교환 `{code, message}`)는 fault `details.titleCode`·`details.exchangeCode`로 되돌려 오늘과 같다. orchestration 도구 16개도 041부터 같은 방식이다(아래 절).

## orchestration (041)

계약 정본: `specs/041-workbench-orchestration/contracts/{workbench-orchestration,tauri-compat}.md`, 결정 근거 `research.md` R1–R18.

- **묶임**: orchestration 작업 영역은 창 label이 아니라 **작업대에 묶인다**(메모리 묶임 표, 서버 재시작 시 비어 모든 작업 영역이 복구 가능). 작업대 하나에 작업 영역 하나. 작업대를 닫으면 hook이 작업 영역을 복구 가능으로 바꾸고(열린 과제의 노드는 주의 필요), 다른 작업대가 `bootstrap`의 `resumeWorkspaceId`로 재개한다. 묶일 때마다 새 묶임 id가 생기며 그것이 스트림 key다.
- **저장 경계**: `orchestration-sessions.json`은 작업 영역 전체를 한 파일에 담으므로, 모든 변경은 **파일 전체를 하나의 read-modify-write 경계**(경로별 단일 lock, 한 transaction) 안에서 일어난다 — [core ADR 0006](../crates/workbench-core/docs/adr/0006-orchestration-store-is-one-serialized-aggregate.md). 작업 영역 범위 async lock은 없다: 엔진 호출·대기·알림 전달은 경계 밖에서 하고, 결과는 상태 조건을 다시 확인하는 짧은 transaction으로 반영한다. 묶기(bootstrap·recover·release)만 binding mutex를 경계보다 먼저 잡는다(순서: binding mutex → 저장 경계).
- **역할**: agent 도구의 권한은 토큰 주장이 아니라 **서버 상태**로 정한다 — coordinator는 활성 세대의 run, 자식은 coordinator가 만든 자식 노드의 현재(또는 기동 중) run — [core ADR 0007](../crates/workbench-core/docs/adr/0007-agent-orchestration-roles-come-from-server-state.md). MCP 토큰은 run 하나만 가리키고, `tools/list`는 `orchestration.getAgentRole`로 고른다.
- **run 출처(R18)**: 작업 영역에 들어가는 run id는 이 작업대의 살아 있는 run, 흔적 없는 계획 id(화면은 Main run을 띄우기 전에 묶는다 — 묶을 때 hub 소유로 claim), agent principal run, 서버가 기동한 자식 run뿐이다. 모든 기동 경로는 claim에 실패하면 엔진을 부르기 전에 거절하고, 끝난 run id는 다시 쓸 수 없다.
- **스트림**: `orchestration:<bindingId>`(상태 복원용, journal 256). 그 작업대를 연 주체만 구독하고, 묶임이 풀리면 스트림이 제거되어 구독자는 `Gap(evicted)`를 받는다. run 스트림(`run:<id>`)과 `run.replay`는 hub의 run 소유 작업대 기록(기동 시 claim, journal과 같은 수명)으로 판단한다 — 소유 작업대, 또는 그 run을 기록한 묶인 작업 영역의 작업대. 보관 한도로 제거된 run은 제거 표식으로 판정해 Evicted 형태를 준다.
- **데스크톱**: command 18개는 호환 어댑터(창 → 작업대, 오류 `details.orchestrationError` JSON, 결과 `boundWindowLabel` 재구성). 창 전달은 발행과 같은 스트림 lock 안에서 그 작업대의 창에 한 번(`orchestration-workspace-updated-fallback` + 사유별 상세).

```mermaid
sequenceDiagram
    participant UI as 세션 창
    participant Cmd as Tauri compat
    participant RT as OrchestrationRuntime
    participant Store as 저장 경계 (파일 전체)
    participant Eng as RunEngine
    participant Hub as EventHub
    UI->>Cmd: bind_main_coordinator_run(planned runId)
    Cmd->>RT: orchestration.bindCoordinator {benchId}
    RT->>Hub: claim_run(runId, bench)
    RT->>Store: begin → 세대 기록 → commit
    Store-->>Hub: publish orchestration:<bindingId> (창 전달 1회)
    UI->>Cmd: start_agent_run(panel = Main)
    Cmd->>Eng: run.start (claim 확인 → 역할 확인 → start)
    Note over RT,Eng: 자식 기동·알림 전달은 저장 경계 밖에서 await
```

### 3단계 안내 (041 → HTTP/WS 어댑터)

041로 2단계가 끝났다: command 인벤토리의 이연 항목이 0이고, 서버 상태를 바꾸는 모든 경로가 `Workbench.call`·`Workbench.events`를 거친다. 다음 단계는 같은 seam 위에 독립 HTTP/WS 서버 어댑터를 올리고(3단계), 프런트엔드를 그 클라이언트로 옮기고(4단계), standalone server 생명주기를 붙인 뒤(5단계), 데스크톱을 thin client로 줄이고 호환 어댑터를 지운다(8단계).

## 호출 규칙

| 항목 | 규칙 |
|---|---|
| `protocolVersion` | 1만 허용. 다른 값은 `unsupportedProtocol`(HTTP 409) |
| 미존재 operation | `notFound`. 존재하지만 scope 부족 → `forbidden`. 둘 다 `system.describe` 목록에 없다 |
| 입력 검증 | typed 역직렬화 통과 후에만 handler로. 최상위 input만 `deny_unknown_fields`(중첩 객체는 허용). 실패는 `invalidArgument` |
| `requestId` / `idempotencyKey` | 별개. 전자는 시도별, 후자는 mutation 재시도에만 재사용. command는 키 필수, query에 키를 보내면 `invalidArgument` |
| 저장 단위를 읽는 query | ledger를 거치지 않지만 **aggregate lock은 잡는다**(복구 쓰기와 직렬화) |
| Git·파일·agent query | lock 없이 blocking pool에서 실행(사용자 저장소는 외부 프로세스도 바꾸므로 lock이 정합성을 보장하지 못한다) |
| `expectedRevision` | 저장 단위 변경만. Git 변경에 보내면 `invalidArgument` |
| 오류 message | 사람이 읽는 한 문장. Tauri 호환 어댑터는 이 문자열만 화면에 돌려준다. Git이 비정상 종료하면 Git이 낸 문장 그대로(`internal`, 해석하지 않음) |

principal은 두 종류다: 데스크톱(scope 13개 전부)과 테스트용 조회 전용(`*:read`·`system:describe`). scope는 도메인별 조회/변경(`project`·`savedPrompt`·`goal`·`agentRunSettings`·`git`)과 조회 전용(`worktree:read`·`agent:read`)이다. 입력으로 정체를 지정할 수 없다. `system.describe`는 데스크톱에 32개, 조회 전용에 19개를 보여 준다.

## intent-first 변경과 ledger

변경 13개(`project.create` 포함)는 부작용 전에 의도를 남기고, 적용 뒤 결과를 확정한다. 저장 파일·사용자 저장소와 ledger(SQLite)는 한 트랜잭션이 아니므로 **순서**가 계약이다. 절차는 `application/intent_first.rs`의 `IntentFirst::run` 하나이고, 각 handler는 `MutationSpec`(aggregate, 예약 정책, lock 안 부작용, 오류 매핑)만 채운다.

```mermaid
stateDiagram-v2
    [*] --> pending : begin(멱등성 키·입력 지문·예약) commit
    pending --> applied : lock 안 부작용 → complete(결과, revision+1) commit
    pending --> failed : 부작용 전 검증·사전 조건·저장 실패 → fail(fault) commit
    pending --> applied : (재시작) reconciler가 적용 증거를 관찰
    pending --> unknown : (재시작) 증거 없음 — 자동 재실행 없음
    applied --> [*] : 24h 뒤 GC
    failed --> [*] : 24h 뒤 GC
```

같은 키 재요청의 응답:

| 기존 상태 | 지문 동일 | 응답 |
|---|---|---|
| 없음 | — | 새 실행 |
| `applied` | 예 | 저장된 결과·revision 그대로(Git 변경은 revision 없음) |
| `applied` | 아니오 | `conflict`, outcome `applied` |
| `failed` | 예 | 저장된 Fault |
| `failed` | 아니오 | `conflict`, outcome `notApplied` |
| `pending` | 무관 | `conflict`, outcome `unknown`, retryable |
| `unknown` | 무관 | `conflict`, outcome `unknown`, not retryable |

지문은 정규화(trim)된 입력의 canonical JSON(키 정렬) SHA-256이다. `" AW "`와 `"AW"`는 같은 지문이다. `git.createWorktree`는 기본 branch·경로를 채우기 **전**의 입력으로 지문을 만든다(기본 branch 이름이 시각에서 만들어지므로).

### 저장 단위(aggregate)

| aggregate | 대상 | revision |
|---|---|---|
| `projects` · `saved-prompts` · `goals` · `agent-run-settings` | 저장 파일 하나 | 각자 독립. 응답과 `expectedRevision`에 쓰인다 |
| `git-worktrees:<canonical repo root>` | 한 사용자 저장소의 worktree 생성·삭제 | 응답에 싣지 않는다. 같은 저장소 안에서만 직렬화 |

### ledger schema v2 — 예약 수명

예약(`reserved_resource_id`)은 두 역할을 한다: 같은 자원에 대한 **동시 진행 배제**와 재시작 판정의 **증거**. v2는 unique index를 `state = 'pending'`인 행에만 걸어, 배제는 진행 중에만 두고 증거(값)는 남긴다. 037이 만든 v1 파일은 첫 기동에서 자동 승격된다(행 데이터 불변).

| 상태 | 예약 배타 | 근거 |
|---|---|---|
| `pending` | 유지 | 같은 자원의 두 실행이 동시에 부작용을 내면 안 됨 |
| `applied` | 해제 | 같은 자원의 다음 변경(삭제 뒤 재생성 등)은 새 실행 |
| `failed` | 해제 | 부작용 없음. 같은 자원으로 새 키 재시도는 정당 |
| `unknown` | 해제 | 어느 쪽이든 새 실행을 막을 이유가 없음 |

| operation | 예약 값 | 충돌(다른 `pending`이 잡음) 시 |
|---|---|---|
| `project.create` · `savedPrompt.create` | 서버가 만든 새 id | 새 id로 최대 3회 재시도 |
| `project.delete` · `savedPrompt.delete` · `goal.clear` | 대상 id(`goal.clear`는 `workingDirectory`) | `conflict` outcome `unknown`, retryable |
| `git.createWorktree` · `git.deleteWorktree` | 해석된 worktree 절대 경로 | `conflict` outcome `unknown`, retryable("Another change to this worktree path is still in progress.") |
| upsert(`goal.create` · `agentRunSettings.save`)·수정 | 없음 | — |

### 재시작 판정 규칙

기동 시 `pending` 기록마다 operation에 등록된 reconciler에게 묻는다(`application/reconcilers/`). 자동 재실행은 없다.

| operation | `applied` 조건 | 규칙 |
|---|---|---|
| `project.create` · `savedPrompt.create` | 예약 id가 파일에 있음 | 예약 증거(이 실행만 만들 수 있는 id) |
| `project.delete` · `savedPrompt.delete` · `goal.clear` | 대상이 파일에 없음 | 종료 상태 |
| `git.createWorktree` | 경로가 `git worktree list`에 있음 | 종료 상태([ADR](../crates/workbench-core/docs/adr/0001-end-state-reconciliation-for-external-side-effects.md)) |
| `git.deleteWorktree` | 경로가 목록에 없음(저장소를 읽을 수 없으면 `unknown`) | 종료 상태 |
| upsert 2개 · 수정 4개 | — | 관찰로 구별 불가 → 항상 `unknown` |

디렉터리만 생기고 Git 등록이 안 된 부분 상태는 `unknown`이며, 시스템은 그것을 정리하거나 다시 만들지 않는다.

### revision과 복구 — 설계 리뷰 반영

- **revision의 정본은 `aggregate_revision` 테이블**이다. `applied` 전이와 같은 SQLite 트랜잭션에서 +1 하고, TTL GC가 ledger row를 지워도 유지된다. ledger의 `MAX(revision)`으로 유도하면 GC 뒤 재시작에서 0으로 되돌아가 stale `expectedRevision`이 통과한다.
- **읽기 경로는 저장 파일에 쓰지 않는다.** `JsonCollectionStore::load`는 손상 시 오류만 내고, `.bak` 복구(`recover_from_backup::<T>`, temp+rename)는 `StorageCoordinator::with_aggregate`가 lock을 잡은 채로만 수행한다(손상 → 복구 → 한 번 재시도). 복구 검증은 `load`와 같은 문서 타입으로 한다. 저장 단위 4개 모두 이 경로만 쓴다.
- **저장 뒤 확정 실패는 `unknown`이다.** 부작용 뒤 ledger `complete`가 실패하면 row를 `pending`으로 남기고 `outcome: unknown`으로 응답한다. 다음 기동의 reconciler가 판정한다.
- **generic `CallReply.output`은 임의 JSON이다.** 목록·`null`·단일 객체가 섞이므로 200 응답 스키마의 `output`에 타입을 두지 않는다. operation별 typed 결과는 `CallReplyByOperation`이 제공한다.

### Git·파일 오류 분류

Git이 낸 오류 문장은 해석하지 않고 사전 검증만 분류한다(grill Q5, [ADR 0002](adr/0002-git-adapters-live-in-workbench-core.md)).

| 상황 | FaultCode |
|---|---|
| 필수 입력 공백(`Working directory is required.` 등) | `invalidArgument` |
| worktree 밖 경로(`File path must stay inside the worktree.`) | `forbidden` |
| 디렉터리 아님·일반 파일 아님·경로 없음·삭제 대상 worktree 없음 | `notFound` |
| 삭제 전 검사 실패(변경 있음·상태 미확정) | `preconditionFailed`, outcome `notApplied` |
| git 실행 파일 없음 | `unavailable`, retryable |
| git 비정상 종료·기타 실행 실패 | `internal`, 문장 그대로 |

저장소가 아닌 디렉터리에 대한 원격·브랜치·worktree 목록은 오늘처럼 빈 목록이다.

## 계약 생성과 drift 검사

```mermaid
flowchart LR
    Reg["operations::OPERATIONS (registry)"] --> OAS["openapi::build_openapi()<br/>oneOf를 프로그램적으로 조립"]
    OAS --> Bin["cargo run -p workbench-protocol --bin export_openapi"]
    Bin --> JSON["crates/workbench-protocol/openapi/workbench.openapi.json (커밋)"]
    JSON --> TS["openapi-typescript → packages/workbench-client/src/generated/workbench.ts (커밋)"]
    TS --> Map["operation-map.ts: Extract&lt;CallRequest, {operation: K}&gt;"]
    Map --> TD["operation-map.test-d.ts (vitest typecheck)"]
    JSON --> CI["CI: pnpm run generate:contracts && git diff --exit-code"]
```

- `pnpm run generate:contracts`가 두 생성물을 다시 만든다. 계약을 바꾸면 반드시 실행해 커밋한다.
- Rust 쪽 golden test(`openapi.rs::committed_openapi_matches_registry`)도 커밋 파일과 코드를 비교한다.
- `discriminator` object를 쓰지 않고 variant마다 `operation` 단일값 `enum`을 둔다. `openapi-typescript`가 이를 판별 union으로 읽는다.
- 도메인 타입은 protocol에 **DTO 미러**로 정의되고 core `application/*dto.rs`가 변환한다. 원본 타입의 serde JSON과 DTO JSON이 같음을 wire parity 테스트가 도메인마다 고정한다.

## 검증

- `cargo test -p workbench-protocol -p workbench-core -p agentic-workbench`: contract suite(fixture 128개 × in-memory·HTTP), 저장 단위별 crash point·동시성·GC 뒤 revision·손상 복구, Git 종료 상태 판정(`git_reconcile.rs`), 예약 수명(`reservation_lifecycle.rs`), compat 변환(fixture 입력 대조).
- `pnpm --filter @yoophi/workbench-client check-types test`: `OperationMap` 32키 상관 타입과 `@ts-expect-error`.
- 수동: `specs/038-workbench-domains/quickstart.md` §4(프로젝트·saved prompt·goal·설정·Git·worktree·agent 화면이 이전과 같고, ledger가 schema 2로 승격됨).

## 적용된 이관 절차 (038)

037의 템플릿을 도메인마다 반복했고, 038에서 다음이 공통 부품이 되었다.

1. 도메인·서비스·포트·어댑터를 `workbench-core`로 이동하고 오류를 `String`에서 enum으로 바꾼다. `Display`는 기존 문구를 바이트 단위로 유지한다(골든 테스트).
2. 저장 파일은 `JsonCollectionStore<T>` 위의 repository로 만들고 `Repositories`에 넣는다. aggregate 이름을 `STORE_AGGREGATES`에 추가해 revision을 복원한다.
3. `workbench-protocol/operations/`에 input(최상위 `deny_unknown_fields`)·DTO와 `OPERATIONS` 항목, `schema_for`, `openapi.rs`의 component·이름을 추가한다.
4. handler: 조회는 `query_handler`(lock 없음) 또는 aggregate lock 안 읽기, 변경은 `MutationSpec` + `IntentFirst::run`. 예약 정책과 reconciler를 함께 등록한다(`build_registry`).
5. DTO 변환과 wire parity 테스트, fixture(성공·실패, 변경은 재생·충돌·키 누락·stale revision)를 추가한다.
6. Tauri command를 `workbench_compat` 경유로 바꾸고(`Option::None`은 필드 생략, 서버 기본값 = 오늘의 command 기본값), 이동한 AW 파일을 삭제한다.
7. `pnpm run generate:contracts`로 생성물을 갱신해 커밋한다.

실행 환경을 읽는 어댑터(agent catalog, provider 세션)는 `RuntimeAdapters`로 주입한다. 테스트는 stub을 넣어 실제 환경 변수·홈 디렉터리를 읽지 않는다.

### 2단계 이관 안내 (040 → 041)

039(2a)가 이벤트 봉투·구독과 run 발행 경로를, 040(2b-1)이 소유자 식별(작업대)과 run 8·교환 4 이관을 끝냈다. 남은 orchestration 18개(조회 2개 포함, 도메인을 쪼개지 않는다 — [ADR 0001](adr/0001-defer-event-bound-commands-to-stage-2.md))는 041(2b-2)이 같은 절차로 옮기고 `orchestration:<id>` 스트림을 연다. 그때 AW에 남은 과도기 대응(orchestration의 작업대 → 창 label 조회, `RunTerminalHook`의 orchestration 실패 처리, 창 `Destroyed`의 `release_window`)을 core로 옮긴다.

## command 인벤토리 (71)

연번은 `lib.rs` `generate_handler!` 등록 순서다. 합계: 이관됨 63(037 2 + 038 29 + 039 2 + 040 12 + 041 18), 이연 0, 데스크톱 유지 8.

| # | command | 분류 | operation / 이유 |
|---|---|---|---|
| 1 | `list_projects` | 이관됨(037) | `project.list` |
| 2 | `create_project` | 이관됨(037) | `project.create` |
| 3 | `update_project` | 이관됨(038) | `project.update` |
| 4 | `delete_project` | 이관됨(038) | `project.delete` |
| 5 | `list_saved_prompts` | 이관됨(038) | `savedPrompt.list` |
| 6 | `create_saved_prompt` | 이관됨(038) | `savedPrompt.create` |
| 7 | `update_saved_prompt` | 이관됨(038) | `savedPrompt.update` |
| 8 | `delete_saved_prompt` | 이관됨(038) | `savedPrompt.delete` |
| 9 | `get_goal` | 이관됨(038) | `goal.get` |
| 10 | `create_goal` | 이관됨(038) | `goal.create` |
| 11 | `update_goal` | 이관됨(038) | `goal.update` |
| 12 | `clear_goal` | 이관됨(038) | `goal.clear` |
| 13 | `record_goal_progress` | 이관됨(038) | `goal.recordProgress` |
| 14 | `get_agent_run_settings` | 이관됨(038) | `agentRunSettings.get` |
| 15 | `save_agent_run_settings` | 이관됨(038) | `agentRunSettings.save` |
| 16 | `get_appearance_preferences` | 데스크톱 유지 | 클라이언트별 표현 상태(정본 배치표) |
| 17 | `set_font_size_step` | 데스크톱 유지 | 클라이언트별 표현 상태 |
| 18 | `adjust_font_size_step` | 데스크톱 유지 | 클라이언트별 표현 상태 |
| 19 | `get_worktree_workspace_layout` | 데스크톱 유지 | panel layout은 클라이언트별 표현 상태 |
| 20 | `save_worktree_workspace_layout` | 데스크톱 유지 | panel layout은 클라이언트별 표현 상태 |
| 21 | `list_git_remotes` | 이관됨(038) | `git.listRemotes` |
| 22 | `list_git_branches` | 이관됨(038) | `git.listBranches` |
| 23 | `list_git_worktrees` | 이관됨(038) | `git.listWorktrees` |
| 24 | `list_worktree_changes` | 이관됨(038) | `worktree.listChanges` |
| 25 | `create_git_worktree` | 이관됨(038) | `git.createWorktree` |
| 26 | `delete_git_worktree` | 이관됨(038) | `git.deleteWorktree` |
| 27 | `get_worktree_changes` | 이관됨(038) | `worktree.getChanges` |
| 28 | `get_worktree_file_diff` | 이관됨(038) | `worktree.getFileDiff` |
| 29 | `list_worktree_files` | 이관됨(038) | `worktree.listFiles` |
| 30 | `read_worktree_text_file` | 이관됨(038) | `worktree.readTextFile` |
| 31 | `start_worktree_watcher` | 이관됨(039) | `Workbench.events` `worktree:<경로>` 구독 task |
| 32 | `stop_worktree_watcher` | 이관됨(039) | 구독 task abort |
| 33 | `list_worktree_git_history` | 이관됨(038) | `worktree.listHistory` |
| 34 | `get_worktree_git_graph` | 이관됨(038) | `worktree.getGraph` |
| 35 | `get_worktree_commit_detail` | 이관됨(038) | `worktree.getCommitDetail` |
| 36 | `get_worktree_commit_file_diff` | 이관됨(038) | `worktree.getCommitFileDiff` |
| 37 | `list_agents` | 이관됨(038) | `agent.list` |
| 38 | `list_agent_tool_command_candidates` | 이관됨(040) | `run.listToolCandidates` (작업대 확보 후) |
| 39 | `list_provider_sessions` | 이관됨(038) | `agent.listProviderSessions` |
| 40 | `open_external_url` | 데스크톱 유지 | OS 셸(정본 배치표) |
| 41 | `open_worktree_window` | 데스크톱 유지 | 네이티브 창 |
| 42 | `open_settings_window` | 데스크톱 유지 | 네이티브 창 |
| 43 | `start_agent_run` | 이관됨(040) | `run.start` (작업대 확보 후) |
| 44 | `cancel_agent_run` | 이관됨(040) | `run.cancel` (작업대 확보 후) |
| 45 | `send_prompt_to_run` | 이관됨(040) | `run.sendPrompt` (작업대 확보 후) |
| 46 | `steer_prompt_to_run` | 이관됨(040) | `run.steer` (작업대 확보 후) |
| 47 | `cancel_current_prompt_and_send_to_run` | 이관됨(040) | `run.cancelAndSend` (작업대 확보 후) |
| 48 | `set_run_permission_mode` | 이관됨(040) | `run.setPermissionMode` (작업대 확보 후) |
| 49 | `respond_agent_permission` | 이관됨(040) | `run.respondPermission` (작업대 확보 후) |
| 50 | `sync_agent_workspace` | 이관됨(040) | `exchange.syncWorkspace` (작업대 확보 후) |
| 51 | `send_agent_exchange` | 이관됨(040) | `exchange.send` (작업대 확보 후) |
| 52 | `acknowledge_agent_exchange` | 이관됨(040) | `exchange.acknowledge` (작업대 확보 후) |
| 53 | `list_agent_exchanges` | 이관됨(040) | `exchange.list` (작업대 확보 후) |
| 54 | `bootstrap_orchestration_workspace` | 이관됨(041) | `orchestration.bootstrap` |
| 55 | `list_recoverable_orchestration_workspaces` | 이관됨(041) | `orchestration.listRecoverable` |
| 56 | `get_orchestration_workspace` | 이관됨(041) | `orchestration.get` |
| 57 | `bind_main_coordinator_run` | 이관됨(041) | `orchestration.bindCoordinator` |
| 58 | `delegate_orchestration_goal` | 이관됨(041) | `orchestration.delegateGoal` |
| 59 | `adopt_manual_orchestration_child` | 이관됨(041) | `orchestration.adoptManualChild` |
| 60 | `list_orchestration_tasks` | 이관됨(041) | `orchestration.listTasks` |
| 61 | `collect_orchestration_reports` | 이관됨(041) | `orchestration.collectReports` |
| 62 | `set_orchestration_presentation` | 이관됨(041) | `orchestration.setPresentation` |
| 63 | `replay_orchestration_runtime_events` | 이관됨(041) | `run.replay` |
| 64 | `respond_orchestration_input` | 이관됨(041) | `orchestration.respondInput` |
| 65 | `send_orchestration_child_command` | 이관됨(041) | `orchestration.sendChildCommand` |
| 66 | `cancel_orchestration_task` | 이관됨(041) | `orchestration.cancelTask` |
| 67 | `retry_orchestration_task` | 이관됨(041) | `orchestration.retryTask` |
| 68 | `reassign_orchestration_task` | 이관됨(041) | `orchestration.reassignTask` |
| 69 | `handoff_orchestration_coordinator` | 이관됨(041) | `orchestration.handoffCoordinator` |
| 70 | `dispatch_orchestration_prompt` | 이관됨(041) | `orchestration.dispatchPrompt` |
| 71 | `recover_orchestration_workspace` | 이관됨(041) | `orchestration.recover` |

## 결정 기록

- [ADR 0001 — 이벤트·창 정체에 묶인 command는 2단계로 이연](adr/0001-defer-event-bound-commands-to-stage-2.md)
- [ADR 0002 — Git 어댑터는 workbench-core에 둔다](adr/0002-git-adapters-live-in-workbench-core.md)
- [ADR 0003 — 데스크톱은 발행된 run 이벤트를 전달한다](adr/0003-desktop-forwards-published-run-events.md)
- [ADR 0004 — run 이벤트는 스크립트 삽입 경로만 남긴다](adr/0004-run-events-keep-only-the-script-injection-path.md)
- [ADR 0005 — 창 닫힘은 작업대를 명시적으로 닫는 것이며, 연결 끊김은 run 취소가 아니다](adr/0005-window-close-explicitly-closes-the-bench.md)
- [ADR 0006 — MCP 도구는 agent principal로 `Workbench.call`을 거친다](adr/0006-mcp-tools-call-the-workbench-as-an-agent-principal.md)
- [ADR 0007 — 창 제목 같은 표현 요청은 작업대 알림 스트림으로 보낸다](adr/0007-presentation-requests-are-bench-notifications.md)
- [workbench-core ADR 0001 — 외부 부작용의 종료 상태 판정](../crates/workbench-core/docs/adr/0001-end-state-reconciliation-for-external-side-effects.md)
- [workbench-core ADR 0002 — 이벤트 journal은 메모리에 두고 서버 세대로 구별한다](../crates/workbench-core/docs/adr/0002-event-journal-is-in-memory-with-server-epoch.md)
- [workbench-core ADR 0003 — 알림 이벤트는 replay하지 않는다](../crates/workbench-core/docs/adr/0003-notification-events-are-not-replayed.md)
- [workbench-core ADR 0004 — run과 교환 작업 영역의 주인은 작업대이며, 연 principal에 묶인다](../crates/workbench-core/docs/adr/0004-benches-own-runs-and-exchanges.md)
- [workbench-core ADR 0005 — `run.start`만 변경 기록을 쓰고 나머지는 세대 범위 멱등성을 쓴다](../crates/workbench-core/docs/adr/0005-only-run-start-uses-the-operation-ledger.md)
- [workbench-core ADR 0006 — orchestration 저장소는 하나의 직렬화된 aggregate다](../crates/workbench-core/docs/adr/0006-orchestration-store-is-one-serialized-aggregate.md)
- [workbench-core ADR 0007 — agent orchestration 역할은 서버 상태에서 나온다](../crates/workbench-core/docs/adr/0007-agent-orchestration-roles-come-from-server-state.md)

## 완료 기준

037·038·039·040·041 spec의 성공 기준이 테스트 또는 수동 절차로 확인되었고, 프론트엔드 변경이 037·038·040·041은 0건, 039는 run 화면 순번 처리(`features/agent-run`, `entities/agent-run/{api,model}`)에 한정되며, `crates/git-core`·`crates/acp-agent-core`·`packages/agent-client` 변경이 0건이고, CI에 drift 검사 단계가 있다.
