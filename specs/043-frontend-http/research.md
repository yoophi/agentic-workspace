# Research: 043 데스크톱 화면의 네트워크 경로 전환

현재 구조(main `b682c6b`): 화면 저장소 21개 모듈이 `invoke`로 호환 command 67개를 부른다. 호환 층(`inbound/workbench_compat.rs`·`tauri_commands.rs`)이 창 label → 작업대(`desktop_benches::ensure`), 인자 → operation 입력, 출력 모양, fault → 문자열(`fault_to_string` = 메시지 그대로, 교환은 `{"code","message"}` JSON)을 맡는다. 이벤트는 앱이 창에 넣는 fallback DOM 이벤트(`agent-run-event-fallback` 등 7종, `tauri_desktop_bridge.rs`), orchestration은 Tauri `listen`+fallback, Worktree 변경은 `start_worktree_watcher` + 네이티브 emit(`workspace://worktree-changed`). 042까지 **모든 데스크톱 토큰의 주체는 `desktop` 하나**다.

## R1. 창 사이 격리 = 창별 데스크톱 주체 (사용자 검토 2)

**Decision**: 데스크톱 주체를 창 단위로 나눈다. `AuthenticatedPrincipal::desktop_window(label)` — kind `Desktop`, scope는 오늘 데스크톱과 같고, subject만 `desktop:window:<label>`. `DesktopTokenIssuer`는 토큰에 이 주체를 묶는다(발급 인자). 세션 창의 작업대는 `desktop_benches::ensure`가 **그 창의 주체로** 연다. 같은 창의 호환 경로 호출도 그 창의 주체를 쓴다(두 경로가 같은 소유 판정). 서버의 기존 판정 — 작업대 소유(`opened_by` subject 일치), run·교환·작업대·orchestration 스트림 구독의 작업대 소유 검사(040·041) — 가 그대로 창 격리가 된다.

**Rationale**: 오늘 호환 경로는 창 label에서 작업대를 도출하므로 창이 다른 창의 작업대를 지정할 수 없다. 네트워크 경로에서 작업대 id를 입력으로 받으면서 주체가 하나면, 다른 창의 id를 넣는 순간 격리가 사라진다. 화면이 대상을 고르는 방식은 보안 경계가 아니다. 주체를 나누면 040·041에서 이미 시험된 소유 판정이 격리를 강제한다. 프로젝트·prompt·목표·설정·Git 같은 전역 데이터는 오늘도 창과 무관하므로 scope가 같으면 그대로다.

**Alternatives**: 한 주체 유지 + 서버가 "토큰의 창 label"과 작업대의 label을 비교(작업대에 label 개념을 되살림 — 040에서 없앤 창 label 결합이 계약에 돌아온다), 화면 대상 선택만(격리 입증 불가 — 거절).

**창 재사용(incarnation) — 사용자 검토**: label만 subject로 쓰면 창을 닫고 **같은 label로 다시 열었을 때** 만료 전 옛 토큰이 새 창의 작업대를 조작할 수 있다(세션 창 label은 Worktree 경로에서 나와 재사용될 수 있다). 그래서 subject에 **창 incarnation**을 넣는다: `desktop:window:<label>:<incarnation>` — incarnation은 창 생성(첫 `get_workbench_connection`·`ensure_window_bench`) 때 만든 uuid이고 창 수명 동안 고정이다. 창 `Destroyed`에서 (a) 그 incarnation을 폐기하고 (b) 그 주체로 발급한 토큰을 모두 폐기한다(`DesktopTokenIssuer::revoke_principal`). 같은 label로 다시 연 창은 새 incarnation을 받는다. 시험: 닫힌 창의 미만료 토큰으로 호출 → `unauthenticated`(폐기), 폐기를 뺀 변이에서도 새 창 작업대 조작은 소유 판정으로 거절(incarnation이 다름).

**호환 경로도 창 주체로(설계 리뷰 D2)**: 작업대를 창 주체가 열므로 작업대 범위 호환 command(run·교환·orchestration·`bench.*`)도 호출 창의 주체로 불러야 한다 — 지금처럼 `desktop_principal()`로 부르면 소유 판정에 걸려 호환 경로 창이 깨진다. 전역 command도 창 주체로 부른다(scope 동일, 결과 같음).

**Consequences**: 창 label은 주체 subject 문자열에만 나타나고 작업대·계약에는 없다. 연구 표기와 CONTEXT: "데스크톱 창 주체". 시험: 다른 창 토큰으로 작업대·run·교환·orchestration 조작·구독 → `forbidden`/`notFound`(오늘 문구), 같은 창 두 경로는 같은 소유, 창 닫고 같은 label 재개 뒤 옛 토큰 거절.

## R2. 창의 작업대 id를 화면에 건네기

**Decision**: 새 Tauri command `ensure_window_bench(hint?) -> { benchId }`(창 label 기반, 오늘 `desktop_benches::ensure`와 같은 규칙: 세션 창만, 작업 디렉터리는 창 경로 또는 hint, 닫힌 창은 오늘 문구로 거절). 창 닫힘 → 작업대 닫기는 오늘처럼 Rust(`WindowEvent::Destroyed`)가 한다. 화면은 작업대 id를 연결 수명 동안 기억하고, 새 세대로 다시 연결하면 다시 받는다(R8).

**Rationale**: 작업대 수명은 창 수명이고 창 이벤트는 Rust가 가장 정확히 안다. 화면이 `bench.open`을 직접 부르면 창이 비정상 종료될 때 닫기를 놓친다.

**Alternatives**: 화면이 `bench.open`/`bench.close`(창 비정상 종료 시 누수), `get_workbench_connection` 응답에 포함(작업대는 작업 디렉터리가 필요해 늦게 열린다 — 분리).

## R3. 경로 선택은 창 부팅 때 한 번 (사용자 검토 3)

**Decision**: 창이 뜰 때(React 렌더 전 bootstrap) `get_workbench_connection` → handshake가 모두 성공하면 그 창은 **수명 동안** 네트워크 경로(`HttpTransport`)를 쓴다. 하나라도 실패하면 처음부터 호환 경로(`CompatTransport`)를 쓰고 이유를 진단 기록에 남긴다. 한번 정한 경로는 바꾸지 않는다 — 네트워크 경로 창이 끊기면 재연결만 한다. 저장소 모듈은 `shared/api/transport`의 선택된 transport 하나만 부른다(호출·이벤트 같은 transport).

**Rationale**: 실행 중 자동 전환은 같은 변경을 두 경로로 보내거나(응답 유실 뒤 호환 경로 재실행 → 두 번 적용), 같은 이벤트를 두 경로로 받게 한다. 042 FR-016(끝점 기동 실패 허용)의 안전장치는 부팅 시점 선택으로 충분하다.

**Alternatives**: 실패 시마다 호환 경로로 대체(중복 실행 — 거절), 전역 설정 플래그(창마다 상황이 다를 수 있음).

## R4. 네트워크 경로 창에는 앱 내부 전달을 끈다

**Decision**: `get_workbench_connection` 성공(그리고 화면이 네트워크 경로를 확정) 뒤 화면이 `declare_network_delivery()`를 부르면 Rust가 그 창 **incarnation**을 "네트워크 전달" 표에 올리고(label이 아니라 — 같은 label로 다시 연 호환 경로 창이 전달을 잃지 않게, 설계 리뷰 D4), `TauriDesktopBridge`는 그 창에 run·교환·제목·orchestration fallback 이벤트를 넣지 않는다. Worktree는 화면이 `start_worktree_watcher`를 부르지 않고 `worktree:<path>` 스트림을 구독한다. 호환 경로 창은 선언하지 않으므로 오늘 그대로 받는다. 창이 닫히면 표에서 뺀다.

**Rationale**: 같은 창에 두 경로로 이벤트가 오면 중복이다(SC-003). 전달 코드를 지우지 않고(8단계) 창 단위로 끈다.

**Alternatives**: 화면이 fallback 이벤트를 무시(앱은 계속 넣어 비용·혼란), 전역 끄기(호환 경로 창이 이벤트를 잃음).

## R5. 호출 클라이언트와 오류 문구 동등성

**Decision**: `packages/workbench-client`에 `createWorkbenchClient({ baseUrl, credentials, onEpoch })` — `call(operation, input, options)`이 `POST /v1/calls`(계약 타입의 `OperationMap`으로 입력·출력 타입 고정). 실패는 `WorkbenchCallError { fault, kind: "notApplied" | "unknown" | "fault" }`. 저장소는 오류를 오늘과 같은 **문자열**로 던진다: `faultToString(fault)` = 호환 층 `fault_to_string`과 같은 규칙(메시지 그대로; 교환 경로는 `details.exchangeCode`로 `{"code","message"}` JSON — 호환 층이 쓰던 형식). 인자 → 입력, 출력 모양 변환은 호환 층 Rust 코드를 저장소 옆 TS 매퍼로 옮기고 동등성 시험을 둔다(R11).

**Rationale**: 화면은 오류를 문자열로 보여 준다(`invoke` rejection). 문구가 바뀌면 FR-002 위반.

## R6. 보내기 전 offline과 응답 유실 구분 (사용자 검토 1)

**Decision**: 호출 결과를 세 가지로 나눈다.
1. **보내기 전 거절(notApplied)**: **클라이언트가 스스로 보내지 않은 경우만** — 연결 상태가 이미 `reconnecting`/`disconnected`(마지막 handshake·구독·호출 실패 뒤 복구 전)면 요청을 보내지 않고 `unavailable`·`outcome: notApplied`를 오늘 문구 형태로 돌려준다. 조회도 같다.
2. **응답 유실(unknown)**: 보내기를 **시도한 뒤**의 모든 실패(브라우저 `fetch` 거절은 "보내지 못함"과 "응답 유실"을 구별하지 못하므로 — 설계 리뷰 D1 — 모두 여기로). 변경이면 호출 시 만든 멱등성 키를 유지하고, 재연결 뒤 **같은 서버 세대**(handshake `serverEpoch` 동일)면 같은 키로 한 번 재시도해 저장된 결과를 받는다(042: 세대 범위는 기다림, ledger 경로는 retryable conflict → 짧게 반복). **세대가 바뀌었으면 자동 재전송하지 않고** `outcome: unknown`으로 알리고 화면 상태 재조회를 트리거한다. 조회는 새 요청으로 다시 보내도 안전하다.
3. **서버 fault**: 그대로 문자열로.
재시도는 사용자 조작 한 번당 최대 한 번, 요청 id는 새로, 멱등성 키는 같게.

**Rationale**: "끊김 = 미적용"은 거짓일 수 있다(보낸 뒤 끊김은 적용됐을 수 있다). 새 세대에서는 세대 멱등 기록이 사라져 같은 키가 새 요청이 되므로 자동 재전송은 두 번 적용이 될 수 있다(042 R13).

**Alternatives**: 무조건 재시도(새 세대 이중 적용), 재시도 없음(같은 세대의 응답 유실이 사용자 재조작 → 새 키 → 이중 적용).

## R7. 이벤트 클라이언트: 반영 완료 cursor와 수신자 교체 (사용자 검토 4)

**Decision**: `createEventClient({ baseUrl, credentials })`. 구독 단위는 **스트림 하나당 WebSocket 하나**(표에 cursor 하나). 스트림마다 상태 `{ streamId, epoch, receivedSequence, listeners[] }`, 수신자마다 `{ deliveredSequence, queue, state }`.
- **수신자 계약(Codex 설계 리뷰 H2)**: 수신자는 `(event) => void | Promise<void>`. 반환값이 Promise면 **settle까지 기다린다** — 이행되면 그 순번까지 반영 완료, 거절·동기 예외면 반영 실패. 수신자마다 자기 큐를 **순서대로 하나씩** 처리한다(앞 이벤트가 끝나야 다음). 실제 화면 수신자(`worktree-agent-run-area.tsx`의 orchestration 수신자는 `getOrchestrationWorkspace()`를 await)가 이 계약을 쓴다.
- **cursor**: 수신자별 `deliveredSequence`는 그 수신자가 반영을 마친 순번. 재연결 표의 cursor = 붙은 수신자들의 `deliveredSequence` 최솟값. 다시 받은 프레임은 `deliveredSequence`가 더 작은 수신자에게만 넘긴다 — 이미 성공한 수신자에게 중복 적용하지 않는다.
- **부분 실패**: 한 수신자가 실패하면 그 수신자만 `failed` → 그 수신자의 **재동기**(스트림 종류별 스냅샷, R8)를 실행하고 성공하면 `deliveredSequence`를 스냅샷 기준점으로 올린 뒤 큐를 이어 처리한다(스냅샷 이하 순번은 버림). 재동기도 실패하면 backoff로 재시도(상한 뒤 화면 오류 표시), 다른 수신자는 막지 않는다. 같은 이벤트를 실패한 수신자에게 그대로 무한 재시도하지 않는다 — 실패의 복구 단위는 스냅샷이다.
- **수신자 교체**: 수신자가 0명인 동안 도착한 프레임은 스트림 대기열에 남는다(cursor 안 올림). 새 수신자는 붙을 때 대기열부터 받는다(`deliveredSequence` = 붙기 직전 스트림 cursor). 구독 해제는 마지막 수신자가 떠나고 유예(React StrictMode·재마운트) 뒤. 대기열 상한은 스트림당 1,024 — 넘으면 연결을 닫고 cursor에서 다시 구독(설계 리뷰 D3).
- **준비**: `hello`를 받은 뒤 프레임을 기다린다. 복구 성공은 `hello`로 판정하지 않는다 — 서버는 등록이 실패해도 hello를 보내고 곧바로 gap을 보내므로, gap은 언제 와도 해당 절차를 다시 시작한다(Codex 설계 리뷰 H1).
- **중복 방지**: 수신자의 `deliveredSequence` 이하 프레임은 그 수신자에게 넘기지 않는다(재연결 경계).

**Rationale**: "마지막으로 받은 순번"으로 이어 받으면 받기만 하고 화면에 반영하지 못한 이벤트(수신자 교체 중, 예외)를 잃는다. 스트림당 연결 하나면 cursor 추가·제거 때 다른 스트림 연결을 다시 만들 필요가 없다(표의 cursor는 연결 때 고정). 루프백이라 연결 수 비용이 작다(hub 동시 구독 상한 256 안).

**Alternatives**: 창당 연결 하나에 cursor 여럿(대상이 바뀔 때마다 전체 재연결), 받은 즉시 cursor 전진(유실).

## R8. gap·세대 변경 복구와 재조회 경계 (사용자 검토 4)

**protocol 대응표**(`workbench-protocol` `StreamKind::class`, `GapReason`, operation 목록으로 확인 — 사용자 검토: run 규칙을 일반화하지 않는다):

| 스트림 | 분류 | 스트림 순번을 주는 스냅샷 | 상태 스냅샷(순번 없음) |
|---|---|---|---|
| `run:<id>` | 상태 복원 | `run.replay{benchId, runId, afterSequence}` → `lastSequence`·`events`·`gapDetected` | — |
| `exchange:<bench>` | 상태 복원 | 없음 | `exchange.list{benchId}`(작업 영역 `revision`) |
| `orchestration:<binding>` | 상태 복원 | 없음 | `orchestration.get{benchId}`(작업 영역 `revision`) |
| `bench:<bench>` | 알림 | — | 없음(제목 요청 알림 — 재조회 대상 없음) |
| `worktree:<path>` | 알림 | — | Worktree 변경 목록·Git 조회 |

`GapReason`: `UnknownStream`·`Evicted`·`EpochChanged`·`RetentionExceeded`·`SubscriberLagged`·`Shutdown`.

**Decision**: 사유·스트림별(042 hub 코드로 확인: `event_hub/stream.rs::decide_existing`은 첫 보관 순번이 1보다 크면 `after = 0`에도 `RetentionExceeded`를 내고 live 수신자를 등록하지 않는다. `GapNotice`는 `firstSequence`·`lastSequence`(현재 보관 범위)를 싣는다):

**보관 범위 gap(`RetentionExceeded`·`Evicted`·`UnknownStream`)의 공통 절차 — live 먼저 확보, 버퍼, 스냅샷 병합(Codex 설계 리뷰 H1)**:
1. gap이 준 `lastSequence = L`(없으면 스냅샷이 준 기준점)과 `epoch`로 **새 표를 연다(`after = L`)**. hub는 `after = L`이 보관 범위 안이므로 `Replay{after: L}` — live 수신자를 등록하고 L 이후 이벤트를 보낸다. 이 사이 새 gap(보관 범위가 L을 넘어 밀림)이 오면 1부터 다시(최대 3회 뒤 오류 표시).
2. 그동안 오는 이벤트는 **버퍼에 담고 수신자에게 넘기지 않는다**.
3. 스트림 종류별 스냅샷을 조회한다 — run `run.replay(after: 0)`(`lastSequence`는 스트림 순번과 같음), orchestration `orchestration.get`(작업 영역 `revision`), 교환 `exchange.list`(교환 목록, 아래 재조정).
4. 수신자에게 스냅샷을 "초기화"로 넘기고, 버퍼를 스냅샷 기준으로 걸러 이어 넘긴다 — run: 순번 > 스냅샷 `lastSequence`, orchestration: 본문 `revision` > 스냅샷 `revision`(본문 `OrchestrationEventDto`는 변경 신호 — 넘기면 수신자가 재조회), 교환: `requestId` 기준 upsert(상태는 스냅샷보다 늦은 `updatedAt`만).
5. 이후 live. run의 `Evicted`는 run 기록이 지워졌다는 뜻이라 replay도 `gapDetected`·빈 events — 화면은 "기록 일부 없음"을 오늘 방식으로 표시하고 종료 상태로 둔다.

**교환 전달 재조정(Codex 설계 리뷰 H3)**: 교환 요청은 서버가 agent에 직접 전달하지 않는다 — `exchange.requested` 이벤트를 화면이 받아 대상 패널로 라우팅(`routePromptToPanel`)하고 `exchange.acknowledge(delivered|rejected)`를 보낸다. 요청 이벤트를 잃으면(보관 한도·끊김) 목록에 upsert만 해서는 전달·ack가 일어나지 않는다. 규칙:
- 창은 교환 원장 `{ requestId → routedAt?, ackedOutcome? }`을 메모리에 둔다.
- 요청 이벤트를 받거나 **재조정**할 때: 원장에 라우팅 기록이 없으면 라우팅 후 ack, 라우팅은 했지만 ack가 확인되지 않았으면 **ack만 다시**(서버 ack는 `requestId` 기준 멱등 — 같은 결과는 값만 돌려준다, `agent_exchange_service.rs::acknowledge`).
- 재조정 시점: 교환 구독 시작(스냅샷), gap 복구 스냅샷, 같은 세대 재연결 뒤. 대상: `exchange.list`에서 이 창 패널이 대상이고 상태가 `Accepted`(요청 발행 뒤 ack 전)인 교환 — 정확한 상태 의미는 tasks 첫 단계에서 코드로 확인.
- **agent 중복 전달 방지**: 라우팅된 교환 prompt를 패널이 run에 보낼 때 멱등성 키를 `exchange-delivery:<requestId>`로 도출한다 — 원장이 사라진 경우(창 새로고침)나 라우팅이 두 번 된 경우에도 같은 세대에서는 서버가 두 번째를 재생해 agent에 한 번만 간다(세대 범위 멱등, 042). 새 세대면 작업대가 새로 열려 이전 교환 자체가 없다.

**알림 스트림(`worktree:`, `bench:`)**: 구독(`hello`) → 재조회(worktree만, bench는 재조회 대상 없음). 알림은 "다시 읽어라" 신호라 중복 알림은 재조회 한 번 더일 뿐이다. 보관 gap이 없는 스트림이다.

- `epochChanged`(또는 재연결 handshake의 `serverEpoch` 변경): 창 전체 재동기 — 작업대 id 다시 받기(R2), 열린 run·교환·orchestration 목록 재조회, 모든 구독을 새 세대 기준으로 다시. 응답 유실 변경은 R6대로 재전송 없음.
- `subscriberLagged`: 같은 cursor(수신자 최솟값)로 재연결.
- `shutdown`: 재연결 루프.

**Rationale**: 재조회와 구독 순서를 잘못 두면 그 사이 변경을 놓친다. 재생 스트림은 기준점을 가진 스냅샷이 있고, 알림 스트림은 멱등한 재조회라 순서가 반대다.

## R9. 재연결·자격 증명 갱신

**Decision**: 연결 상태 `connected | reconnecting | disconnected`(창 단위 store). 재연결은 지수 backoff(250ms → 최대 10초, jitter), 매 시도 handshake(세대 확인) → 스트림별 새 표. 자격 증명은 만료 시각의 80%에 `get_workbench_connection`으로 갱신(이전 토큰은 만료까지 유효하므로 경합 없음), 호출이 `401`이면 한 번 갱신하고 재시도(401은 미적용이므로 안전). WebSocket은 표로 연결 시점에만 인증하므로 토큰 만료가 열린 구독을 끊지 않는다.

## R10. 연결 상태 표시

**Decision**: `widgets/connection-status`(작은 표시: 다시 연결 중·끊김일 때만 보임, 연결됨이면 숨김). 문구 새로 추가(이 기능의 유일한 새 화면 요소). Storybook story 3상태.

## R11. 시험 전략

**Decision**:
- **클라이언트 단위**(`packages/workbench-client`, vitest): 가짜 fetch·가짜 WebSocket 서버로 R6(세 결과·같은 세대 재시도·새 세대 무재전송), R7(반영 완료 cursor, 수신자 교체 중 도착, 예외 시 cursor 유지), R8(사유별 복구, 스냅샷→구독·구독→재조회 순서), R9(backoff·갱신·401) — 강제 끊김 100회 이상(SC-004).
- **저장소 동등성**(AW, vitest): 저장소 함수마다 같은 시나리오를 `CompatTransport`(기존 `invoke` mock)와 `HttpTransport`(계약 응답 mock)로 실행해 결과·오류 문자열이 같다.
- **화면 통합을 새 경로로(사용자 검토 — FR-012)**: 기존 화면 시험을 호환 기본값으로만 돌리면 새 경로의 근거가 없다. 기존 화면 시험 harness(`agent-run-panel.test-harness.tsx` 등)를 transport에 대해 매개변수화해 **같은 기대값으로 `HttpTransport`에서도** 돈다 — HTTP 쪽은 가짜 Workbench 서버(계약 형식의 `/v1/calls` 응답과 `/v1/events` WebSocket 프레임)를 쓴다. 이벤트를 쓰는 화면(run 패널·교환·orchestration)은 가짜 서버가 구독 프레임을 흘려 같은 화면 결과를 낸다.
- **서버 계약**(Rust): 창별 주체 격리 — 다른 창 토큰으로 작업대·run·교환·orchestration 조작·구독 거절(SC-004a), 같은 창 두 경로 같은 소유.
- **실제 042 hub 보관 한도 초과(Codex 설계 리뷰)**: Rust 시험(`workbench-core/tests/`)이 작은 journal 한도로 orchestration·교환·run 스트림의 `RetentionExceeded`를 만들고, gap의 `lastSequence`로 다시 연 표가 live를 등록해 그 뒤 이벤트를 받는지 단정. 그리고 **TS 클라이언트를 실제 서버에 붙이는 통합 시험** — 시험 전용 host(`workbench-core` example, 작은 보관 한도·운영 router·고정 토큰)를 vitest 통합 suite가 띄우고 `createEventClient`의 복구 절차(live 확보→버퍼→스냅샷 병합)를 실제 hub로 검증.
- **수신자 계약**: Promise 이행·거절, 동기 예외, 수신자 교체 중 도착, 한 수신자 실패 시 다른 수신자 무영향·실패 수신자 스냅샷 재동기 — 클라이언트 단위와 화면 통합(HttpTransport) 둘 다.
- **교환 재조정**: 요청 이벤트 유실(보관 초과) → 스냅샷 재조정으로 라우팅·ack 1회, 라우팅 뒤 ack 실패 → ack만 재시도, 창 새로고침 뒤 재조정 → agent 전달은 `exchange-delivery:<requestId>` 키로 1회(가짜 ACP agent prompt 수로 확인).
- **실제 앱**(debug probe 확장): 메인 창·세션 창에서 앱 자신의 transport로 프로젝트 조회, 세션 창 작업대 받기, run 시작(042 가짜 ACP agent — `agentCommand`)·출력 구독, 서버가 구독을 닫는 강제 끊김(debug 전용 hook) 뒤 자동 재연결·이어 받기, 앱 내부 전달 비활성 확인 — 개발·배포 frontend(`tauri build --debug --no-bundle`) 두 출처(SC-005).

## R12. 범위 밖

compat command·앱 내부 전달 코드 삭제(8단계), 서버 프로세스 분리(5단계), Windows 출처 실측, 최종 release 산출물 검증, 독립 서버 수명.
