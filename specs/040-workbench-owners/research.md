# Research: 040 작업대(Bench)와 run·교환 이관 (2b-1)

결정은 spec Clarifications(Q1–Q10)와 ADR 5건(core 0004·0005, docs 0005·0006·0007)을 전제로 한다. 아래는 plan 수준 결정이다. 코드 사실은 2026-09-27 조사 기준(main `54806dc`).

## 사실 요약(조사)

| 항목 | 오늘 |
|---|---|
| run 기계 | `acp-agent-core`(Tauri 의존 없음): `AppState`(= `AgentSessionRegistry`, `SessionRegistry<Session = AcpSession>` 구현, run 슬롯·소유자 `HashMap<run_id, String>`·`PermissionBroker`), 유스케이스 `StartAgentRun`·`SendPrompt`·`SteerPrompt`·`CancelPromptAndSend`·`SetPermissionMode`·`CancelAgentRun`, 실제 launcher `AcpAgentRunner`. 권한 응답은 유스케이스 없이 `PermissionBroker::respond_for_run` 직접 호출 |
| run의 Tauri 결합 | AW 네 곳: `TauriRunEventSink`(창 삽입 + 종료 시 worktree 가드·orchestration `fail_task_for_runtime`), `JsonAcpSessionStore::from_app`(`acp-sessions.json`), `McpServerState`(토큰 발급·MCP env 주입), Main Coordinator principal 해석(`JsonOrchestrationRepository::from_app`) |
| run 소유 검사 | 권한 응답(`"unknown or finished run: {id}"`, `"permission response was sent from a non-owner window"`), 도구 후보(`"tool command candidates were requested from a non-owner window"`)만. 프롬프트·조향·취소 후 전송·권한 모드·취소는 검사 없음 |
| 가짜 launcher | `start_agent_run.rs` 테스트 전용 private `FakeLauncher` 뿐. `AppState`의 `Session`이 `AcpSession`으로 고정이라 가짜 launcher를 `AppState`와 조합할 수 없다 |
| 교환 | AW `domain/agent_exchange.rs`(오류 코드 21종, 패널 ≤ 8, 메시지 ≤ 16 KiB), `application/agent_exchange_service.rs`(`<R, O, S>` 제네릭), `in_memory_agent_workspace_registry.rs`(창별 스냅샷·교환 이력 500개 FIFO, 요청 id 중복 = 같은 payload면 `Existing`). 백엔드는 프롬프트를 보내지 않는다 — 요청 이벤트를 받은 화면이 패널에 라우팅하고 확인(ack)한다 |
| 교환 확인 멱등성 | 같은 결과의 두 번째 확인은 성공하지만 **상태 이벤트를 또 보낸다**. 다른 결과면 `invalidTransition` |
| 교환 이벤트 | `agent-exchange-requested`(`AgentExchangeRequestedEvent`)·`agent-exchange-status`(`AgentExchange` 전체, `windowLabel` 포함) — 네이티브 `emit`(전체 방송) + `-fallback` 삽입. 화면은 두 경로를 모두 듣는다 |
| MCP | AW axum(127.0.0.1:0, `POST /mcp`), Bearer `awcap_<uuid>`, `CapabilityPrincipal{actor_kind: Coordinator\|Child\|LegacyRun, run_id, window_label?, …}`. 교환 도구 3개(`list_peer_agents`, `send_message_to_agent`, `get_agent_exchange_status`)와 `set_window_title`은 모든 principal에 노출, `runId == principal.run_id` 검사 후 서비스를 직접 호출. orchestration 도구 16개는 041 |
| 제목 | `McpTitleControlService`(80자, run → 창) → `window.set_title` + 메뉴 동기화 + `workspace://mcp-window-title`(`{title}`) 방송 + 삽입. 결과 `TitleChangeResult{ok, appliedTitle?, reason?, code?}` |
| principal | `AuthenticatedPrincipal{kind: Desktop\|Test, scopes}` — **주체 식별자가 없다**. ledger `principal_kind`는 TEXT(CHECK 없음), 읽을 때 문자열 파싱 |
| 창 닫힘 | `session-*` `Destroyed`에서 `cancel_runs_owned_by(label)` → `remove_window(label)` → orchestration `release_window(label)`(041) |
| AW 조립 | `lib.rs` `setup`에서 `WorkbenchRuntime::bootstrap` → `McpServerState::start(app, AppState, registry)`. `AppState`·교환 registry는 `run()` 시작 시 만들어 `.manage` |

## R1. 작업대 registry

**Decision**: core `application/bench_service.rs` + `infrastructure/bench/in_memory_bench_registry.rs`. `Bench{id: BenchId(uuid v4), working_directory: 실제 경로(canonicalize), opened_by: PrincipalSubject, opened_at}`. `bench.open{workingDirectory}` → 경로가 없거나 디렉터리가 아니면 `invalidArgument`(오늘 교환 동기화 문구 `"Workspace path must be a directory."`/`"Failed to resolve workspace path: …"` 재사용). 동시 작업대 상한 256(`BenchLimits`, 초과 `rateLimited`). `bench.close{benchId}`: 모르는 id·이미 닫힘 → 성공(`closed: false`), 있으면 소유 run 전부 취소 → 교환 작업 영역 삭제 → 교환·작업대 스트림 제거 → registry에서 삭제(`closed: true`).

**Rationale**: 작업대는 메모리 전용(Q3). 닫기 멱등은 창 `Destroyed` 중복에 대비. 경로 정규화는 교환 동기화가 오늘 하던 일과 같다.

**Alternatives**: 작업대를 ledger aggregate로 — Q8(세대 범위)과 모순.

## R2. principal 주체 식별자

**Decision**: `AuthenticatedPrincipal`에 `subject: PrincipalSubject(String)`를 추가한다. 데스크톱 `"desktop"`, 테스트 `"test:<name>"`(`test_readonly` → `"test:readonly"`, 새 `test_desktop_like(name)`로 여러 주체 재현), agent `"agent:<runId>"`. 새 `PrincipalKind::Agent`(ledger 문자열 `"agent"`, 파싱 추가). 작업대 소유 비교는 `subject` 동등. `AuthenticatedPrincipal::agent(run_id)`는 scope `exchange:read`·`exchange:write`·`presentation:write`만 가진다.

**Rationale**: Q4(작업대는 연 principal에 묶임)를 구현하려면 주체가 필요하다. `kind`만으로는 3단계에서 데스크톱 두 개를 구별할 수 없다. agent의 주체에 run id를 넣으면 "agent는 자기 run으로만 호출" 검사가 principal 하나로 끝난다.

**Alternatives**: 작업대 토큰(열 때 비밀값 발급, 호출마다 제시) — 강하지만 모든 입력에 비밀값이 섞이고, 3단계 인증 모델(bearer + principal)과 중복.

## R3. run 기계를 core 포트 뒤로 — `RunEngine`

**Decision**: core `ports/run_engine.rs`에 객체 안전한 `RunEngine` 트레이트를 둔다.

```text
start(request, owner: BenchId, sink: RunSink) -> Result<AgentRun, RunEngineError>
send_prompt / steer_prompt / cancel_current_prompt_and_send(run_id, prompt, sink)
set_permission_mode(run_id, mode, sink) / cancel(run_id, sink) / respond_permission(run_id, permission_id, option_id)
owner_of(run_id) / active_owner_of(run_id) -> Option<BenchId>
cancel_runs_owned_by(owner) -> Vec<run_id>
acp_registry() -> Option<AppState>   // 041 전 AW orchestration용 과도기 접근자
acp_session_store() -> Option<Arc<JsonAcpSessionStore>>   // 같은 이유
```

운영 구현 `infrastructure/run/acp_run_engine.rs`는 `AppState` + `AcpAgentRunner` + `JsonAcpSessionStore`를 감싸 기존 유스케이스를 그대로 호출한다(제네릭 유스케이스는 구현 내부에서 단형화). 테스트 구현 `tests/support/scripted_run_engine.rs`는 메모리에서 run 슬롯·소유·권한 대기를 흉내 내고 fixture가 정한 이벤트를 sink로 낸다. `JsonAcpSessionStore`는 AW에서 core `infrastructure/fs/acp_session_store.rs`로 옮겨 `DataPaths`로 만든다(`from_app` 제거).

**Rationale / Q9와의 관계**: Q9는 "가짜 `SessionLauncher` 주입"이었으나 `AppState`의 `Session`이 `AcpSession`으로 고정되어 가짜 launcher와 조합할 수 없다(acp-agent-core를 제네릭으로 바꾸면 hushline·ask-code에 파급). 그래서 **한 단계 위(`RunEngine`)** 에서 가짜를 주입한다. 의도(프로세스 없는 결정적 fixture)는 같고, 작업대 소유 검사·멱등성·이벤트 전달은 모두 엔진 위에 있어 가짜로 검증된다. 운영 엔진은 기존 acp-agent-core 테스트(run 유스케이스 19건, runner 22건)가 덮는 얇은 어댑터다.

**Alternatives**: `AppState`를 `Session` 제네릭으로 — acp-agent-core 변경(FR-015 위배). 가짜 ACP 실행 파일 — Q9에서 기각.

## R4. run 이벤트 sink와 데스크톱 전달 포트

**Decision**: core `infrastructure/run/workbench_run_sink.rs`의 `WorkbenchRunSink{bench_id, publisher, desktop}`가 acp `RunEventSink`를 구현한다. `emit`은 `publish_run(run_id, event, terminal, deliver)`를 부르고 `deliver`에서 `desktop.deliver(DesktopDelivery::Run{bench_id, payload})`를 호출한다(039와 같이 스트림 lock 안). 종료 이벤트 뒤 `terminal_hook.on_terminal(run_id, bench_id)`을 lock 밖에서 호출한다.

새 core 포트 `ports/desktop_bridge.rs`:

- `DesktopBridge::deliver(&self, DesktopDelivery)` — 막히지 않아야 한다(lock 안). variant: `Run{bench, payload}`, `ExchangeRequested{bench, payload}`, `ExchangeStatus{bench, payload}`, `TitleRequested{bench, title}`.
- `RunTerminalHook::on_terminal(run_id)` — AW가 worktree 가드 검사·orchestration `fail_task_for_runtime`을 구현(041 전 과도기).
- `RunLaunchDecorator::decorate(&mut AgentRunRequest, LaunchContext{bench_id, panel_id, run_id}) -> Result<(), String>` — AW가 Main Coordinator principal 해석·MCP 토큰 발급·env 주입을 구현.

세 포트는 `RuntimeAdapters`의 `Option`(없으면 no-op; 테스트는 기록형 가짜). AW 구현은 `AppHandle`과 "작업대 → 창" 대응표를 쓴다. `McpServerState`와 런타임의 순환(런타임은 decorator가, MCP는 런타임이 필요)은 AW decorator가 `OnceLock<McpServerState>`로 늦게 묶어 푼다.

**Rationale**: ADR 0003(데스크톱은 발행 결과 전달)을 작업대 단위로 유지(Q5). Tauri 결합 네 곳 중 세 곳을 포트로, 한 곳(세션 저장소)을 core로 옮겨 run 서비스가 AW 없이 돈다.

**Alternatives**: 데스크톱이 작업대마다 구독 — Q5에서 기각.

## R5. run operation과 소유 검사

**Decision**: operation 8개, 입력은 모두 `benchId` 포함.

| operation | 종류 | scope | 멱등 | 소유 검사 · 오류 |
|---|---|---|---|---|
| `run.listToolCandidates{benchId, query}` | query | `run:read` | — | run id가 있으면 `active_owner_of == benchId`, 아니면 `"tool command candidates were requested from a non-owner window"`(문구 유지) |
| `run.start{benchId, request, panelId?}` → `AgentRun` | command | `run:write` | **영속** | run의 소유자 = benchId |
| `run.sendPrompt{benchId, runId, prompt}` | command | `run:write` | 세대 | 새 검사 → `forbidden` `"run is owned by another bench."` |
| `run.steer{…prompt}` · `run.cancelAndSend{…prompt}` · `run.setPermissionMode{…mode}` · `run.cancel{benchId, runId}` | command | `run:write` | 세대 | 새 검사, 같은 문구 |
| `run.respondPermission{benchId, runId, permissionId, optionId}` | command | `run:write` | 세대 | 오늘 문구 유지(`"unknown or finished run: …"`, `"permission response was sent from a non-owner window"`) |

모든 benchId 입력 operation은 먼저 "작업대가 있고 호출자 주체가 연 것인가"를 본다: 없으면 `notFound` `"bench not found."`, 다른 주체면 `forbidden` `"bench belongs to another principal."`. 소유 검사는 run이 **살아 있을 때** `owner_of`로 판단하고, 끝난 run(소유 기록 없음)은 오늘처럼 유스케이스 오류(`"agent run is not active"`)를 낸다 — 이미 끝난 run의 소유를 기억하지 않으므로.

오늘 `send_prompt_to_run`은 실패를 run 이벤트(`RunEvent::Error`)로 돌리고 `Ok`를 반환한다(프롬프트를 task로 보냄). operation도 같게 한다 — 검증 실패(빈 프롬프트·비활성 run·소유 불일치)만 fault, 전송 중 실패는 이벤트.

**Rationale**: FR-003. 문구 유지 대상은 화면이 실제로 보여 주는 두 경로다.

## R6. `run.start`와 변경 기록

**Decision**: `run.start`는 intent-first `MutationSpec`: aggregate `run:<runId>`(run id가 없으면 서버가 uuid를 만들어 **입력 정규화 단계에서** 확정하고 payload hash에 포함되지 않는 파생값으로 ledger 결과에 저장), reservation = 그 aggregate 배타, apply = decorator → 엔진 `start`(예약·spawn까지; 완료는 기다리지 않음), 성공 시 `applied`와 결과(`AgentRun`) 저장. 재생(같은 키·같은 payload)은 저장된 `AgentRun`을 그대로 돌려준다. 새 reconciler `RunStartReconciler`: 기동 시 `pending`인 `run.start` → `unknown`(ADR core 0005). 실패(`duplicate run id`, 동시 실행 상한 등)는 `notApplied`.

**Rationale**: 외부 효과(프로세스)가 있는 유일한 동작만 영속 기록. `unknown`은 "떴는지 알 수 없음"을 정직하게 표현한다.

## R7. 세대 범위 멱등성

**Decision**: core `application/epoch_idempotency.rs`: 메모리 표 `(subject, operation, key) → (payload hash, 결과 JSON)`, 상한 4,096(가장 오래된 것부터 버림). 같은 키·같은 payload → 저장된 결과, 다른 payload → `conflict`(ledger와 같은 fault·문구). 진행 중인 같은 키의 두 번째 요청은 첫 요청이 끝날 때까지 기다린다(키별 `tokio::sync::Mutex`). 세대 범위 operation도 command이므로 멱등성 키 필수. descriptor에 `idempotencyScope: "durable" | "epoch"`(command만, query는 생략)를 추가한다.

**Rationale**: FR-011. 3단계 HTTP 재시도에서 프롬프트 중복을 막는다.

**Alternatives**: 결과를 저장하지 않고 "이미 처리됨"만 기록 — 재시도 응답이 달라진다.

## R8. 교환 이관

**Decision**: 도메인·서비스·registry를 core로 옮기고 키를 창 label에서 `BenchId`로 바꾼다. `AgentExchange`의 `windowLabel` 필드는 **제거**(SC-002; 화면 타입에도 없음). `AgentWorkspaceSnapshot.window_label` → `bench_id`. 소유 조회 포트 `AgentRunOwnerLookup`는 `RunEngine::active_owner_of`를 쓰는 core 구현으로 바뀐다(오늘 문구 `"Panel run is inactive or owned by another window."` 유지). 오류 코드·문구 21종, 패널 8·메시지 16 KiB·이력 500개 유지.

operation:

| operation | 종류 | scope | 멱등 | 비고 |
|---|---|---|---|---|
| `exchange.syncWorkspace{benchId, request}` | command | `exchange:write` | 세대 | 경로 정규화(오늘 command가 하던 일)를 서비스로 |
| `exchange.send{benchId, request}` | command | `exchange:write` | 세대 | 요청 id 중복 규칙 유지 |
| `exchange.acknowledge{benchId, request}` | command | `exchange:write` | 세대 | 같은 결과의 두 번째 확인은 **상태 이벤트를 다시 내지 않는다**(Q5) |
| `exchange.list{benchId}` | query | `exchange:read` | — | |
| `exchange.listPeers{runId}` | query | `exchange:read` | — | agent 전용(주체 run == runId, 아니면 `forbidden` `"The requested run does not match the authenticated capability."`) |
| `exchange.sendFromRun{runId, request}` | command | `exchange:write` | 세대 | agent 전용 |
| `exchange.getForRun{runId, requestId}` | query | `exchange:read` | — | agent 전용 |

도메인 오류는 fault로 옮길 때 `code`를 `details.exchangeCode`에, 오늘 `Display`(`"code: message"`)… 대신 **오늘 command가 반환하던 문자열**(JSON `{"code","message"}`)을 compat 어댑터가 재구성한다: fault `message` = 도메인 message, `details.exchangeCode` = code. compat은 `serde_json::to_string(&{code, message})`로 오늘과 같은 문자열을 만든다(화면·MCP가 이 JSON을 파싱).

**Rationale**: 화면은 교환 오류를 JSON 문자열로 받는다 — 037의 "Fault.message만 반환" 규칙을 그대로 쓰면 문자열이 바뀐다. 코드 보존은 `details`로.

## R9. 교환·작업대 스트림

**Decision**:

- `exchange:<benchId>` — 상태 복원용, scope `exchange:read`. 요청·상태 이벤트가 한 스트림(`exchange.requested.v1`·`exchange.status.v1`, 039에서 예약한 이름). 작업대당 보관 512(`EventHubLimits.exchange_journal_capacity`). 보관 한도 계산(`published_runs`)에는 넣지 않는다 — 작업대 수 상한(256)이 이미 묶는다.
- `bench:<benchId>` — 알림용, 새 scope `bench:read`. `bench.titleRequested.v1{title}`.
- 작업대를 닫으면 두 스트림을 hub에서 제거한다: 새 hub API `remove_stream(kind, key)` — 구독자에게 `Gap(evicted)`, 상태 복원용이면 제거 표식(039 규칙 재사용).
- 알림용 발행에도 데스크톱 전달이 필요하므로 `publish_notification`에 `deliver` 인자를 추가한다(구독자가 없어도 `deliver`는 부른다; worktree는 no-op).
- `StreamKind::Exchange`를 구독 가능으로, 새 `StreamKind::Bench` 추가. `EVENT_SCHEMAS`에 `bench.titleRequested.v1` 추가, 교환 두 스키마에 본문 DTO(`ExchangeRequestedDto`, `AgentExchangeDto`).

**Rationale**: Q5·Q7. 제거 표식으로 "닫힌 작업대"와 "아직 없는 작업대"를 구별한다.

## R10. 데스크톱 어댑터 — 창 ↔ 작업대

**Decision**: AW `infrastructure/desktop_benches.rs`의 `DesktopBenches{by_label: HashMap<label, BenchId>, by_bench: HashMap<BenchId, label>}` + 창 label별 single-flight(`tokio::sync::Mutex`).

- `ensure(label, hint_path) -> BenchId`: 있으면 그대로, 없으면 `window_manager`의 label → worktree 경로(없으면 `hint_path`: run 요청 `cwd`·교환 `worktreePath`)로 `bench.open`. 경로가 전혀 없으면(작업대 없는 창에서 프롬프트) 오늘과 같은 결과가 나도록 run 제어는 `"agent run is not active"`를 낸다(작업대가 없으면 그 창이 소유한 run도 없다).
- `close(label)`: 창 `Destroyed`에서 `bench.close` 호출 후 대응 제거(orchestration `release_window`는 그대로 AW, 041).
- `TauriDesktopBridge`(core `DesktopBridge` 구현): `by_bench`로 창을 찾아 `window.eval(CustomEvent(...))` — run `agent-run-event-fallback`, 교환 `agent-exchange-requested-fallback`·`agent-exchange-status-fallback`(오늘 payload 형태), 제목은 lock 밖 async task에서 `set_title` + 메뉴 동기화 + `mcp-window-title-fallback`. 네이티브 `emit`은 모두 제거.

**Rationale**: FR-004·FR-005. 화면은 창 삽입 경로를 이미 듣는다(교환 `listenWithFallback`, 제목 `App.tsx`). `set_title`은 메인 스레드로 넘어가므로 스트림 lock 안에서 부르지 않는다.

## R11. MCP — agent principal과 도구

**Decision**: AW MCP 서버는 토큰 → `CapabilityPrincipal` 해석(오늘 그대로) 뒤 `AuthenticatedPrincipal::agent(run_id)`를 만들어 `Workbench.call`을 부른다. 교환 도구 3개 → `exchange.listPeers`·`exchange.sendFromRun`·`exchange.getForRun`, 제목 도구 → `bench.requestTitle{runId, title}`. 도구 쪽 `runId == principal.run_id` 검사와 문구는 유지(서버도 주체로 한 번 더 검사). fault → 오늘 도구 결과 형태(`structuredContent`에 `{code, message}`, 제목은 `TitleChangeResult`)로 변환. orchestration 도구(041)는 손대지 않는다.

`bench.requestTitle`: 제목 검증(80자, 오늘 문구) → run의 활성 소유 작업대 조회(없으면 `"Agent run is not active or is not owned by a session window."`) → `bench:<benchId>`에 발행 → `{ok: true, appliedTitle}`. 창을 찾지 못해 적용 못 하는 경우는 더 이상 도구 결과로 알 수 없다(발행은 성공) — 로그로만 남긴다(문서화).

**Rationale**: ADR 0006·0007.

## R12. 041 전 과도기 — AW orchestration

**Decision**: orchestration(AW)은 계속 `AppState`와 run 유스케이스를 직접 쓴다. 바뀌는 점만:

- `AppState`는 AW `run()`이 만들지 않고 런타임의 `run_engine().acp_registry()`에서 얻는다(한 run 기계). 자식 worker가 쓰던 `JsonAcpSessionStore::from_app`도 런타임의 같은 저장소(`acp_session_store()`)로 바꾼다(같은 파일을 두 인스턴스가 쓰지 않게).
- 자식 worker·Main 위임의 run 소유자는 창 label 대신 그 창의 `BenchId`(`DesktopBenches::ensure`). 창 닫힘의 `bench.close`가 자식 run도 취소한다(오늘 `cancel_runs_owned_by(label)`과 같은 결과).
- 자식 worker의 sink는 `TauriRunEventSink` 대신 core `WorkbenchRunSink`(런타임이 `run_sink(bench_id)`로 제공) — 전달 경로가 하나가 된다.
- orchestration 코드 안의 창 label(바인딩·scope 검사·이벤트 대상)은 041까지 그대로.

**Rationale**: Q1. 041이 과도기 접근자(`acp_registry`, `run_sink`)를 제거한다.

## R13. 계약 테스트

**Decision**:

- call fixture(`crates/workbench-protocol/fixtures/*.json`)에 `steps`를 추가한다: 앞선 호출의 출력에서 값을 꺼내 `{{bench}}`·`{{run}}`으로 치환(`capture: {"bench": "/output/benchId"}`). principal은 `desktop`·`readonly`·`agent:<runRef>`·`desktop2`(다른 주체).
- 가짜 엔진 스크립트는 fixture의 `runScript`(이벤트 목록·권한 요청·실패 주입)로 지정한다.
- 흐름 테스트(Rust): run 수명(열기 → 시작 → 권한 → 프롬프트 → 닫기), 교차 작업대·교차 주체 거절 6종, 교환 두 작업대 격리·확인 멱등, 작업대 닫기 시 스트림 gap, 세대 범위 멱등(재시도·충돌·동시), `run.start` reconciler(`pending` → `unknown`), 데스크톱 전달 순서(run·교환 동시 발행).
- AW: compat 변환 단위 테스트(교환 오류 JSON 문자열 재구성, 제목 결과 변환), `DesktopBenches` single-flight·닫기 멱등.

## R14. 한도·성능

**Decision**: 작업대 256, 교환 스트림 작업대당 512, 세대 멱등 표 4,096. `Workbench.call` 경유로 run 제어 지연 증가는 dispatch + 멱등 표 조회(µs 단위) — `#[ignore]` 측정 테스트로 `run.sendPrompt` p95 증가 < 5ms를 기록한다.

## R15. 문서·인벤토리

**Decision**: `docs/workbench-seam.md` — 인벤토리 이관 45(037 2 + 038 29 + 039 2 + 040 12), 이연 18, 데스크톱 유지 8. "작업대" 절(수명·소유 검사·principal 주체·데스크톱 대응), 이벤트 스트림 절에 교환·작업대 스트림, MCP agent principal 절. 정본 진행 각주 "040(2b-1) 완료". `openwiki`는 손대지 않는다.
