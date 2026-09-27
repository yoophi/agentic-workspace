# Research: Workbench HTTP/WebSocket 어댑터 (042)

정본: `docs/client-server-architecture-research.md` 3단계·"보안"·"transport" 절, 2026-09-26 grill 결정 8(인증 모델). 현재 코드 사실은 main `2e7f359` 기준.

## 사실 요약 (현재 코드)

- 계약: `workbench_protocol::Workbench { call(principal, CallRequest), events(principal, Subscription) }`. 운영 구현은 `WorkbenchRuntime` 하나이고 AW `lib.rs`가 조립한다.
- 테스트 전용 HTTP 경로 `crates/workbench-core/tests/support/http_harness.rs`: `POST /v1/calls`(bearer 고정 토큰 → principal, 오류는 `application/problem+json` + `fault.code.http_status()`), `GET /v1/events`(bearer로 upgrade → `hello` → 클라이언트 `subscribe` 한 번 → `event`/`gap` 또는 `fault`). 계약 suite(`contract_suite.rs`)·이벤트 suite(`event_contract_suite.rs`)가 in-memory와 이 경로를 비교한다.
- operation 85개: query 32, 영속 멱등 command 14(ledger), 세대 범위 멱등 command 39(`idempotencyScope: epoch`).
- AW MCP 서버(`infrastructure/mcp/mod.rs`): 자기 포트의 `POST /mcp`. `origin_allowed`(`mcp/title_tool.rs:82`)는 `origin.starts_with("http://127.0.0.1")`·`starts_with("http://localhost")` 접두사 비교라 `http://127.0.0.1.evil.example` 등이 통과한다(결함). Origin 없음은 허용.
- 사용 가능한 의존성(Cargo.lock): axum 0.7.9(AW·core dev), tower-http 0.5/0.6, tower 0.5, sha2 0.10, uuid v4, rand 0.8. tracing은 전이 의존만 있다.

## R1. 어댑터 위치: 새 크레이트 `crates/workbench-server`

**Decision**: HTTP/WS router·인증 포트·출처 정책·구독 표·handshake·health·OpenAPI 제공을 새 크레이트 `crates/workbench-server`에 둔다. 이 크레이트는 `workbench-protocol`(계약 trait·타입·OpenAPI)에만 기대고 `workbench-core`에는 기대지 않는다. router는 `Arc<dyn Workbench>`와 주입받은 포트(자격 증명 해석, 서버 정보)만 안다. AW는 이 크레이트로 router를 조립하고, core 테스트는 dev 의존으로 같은 router를 띄운다.

**Rationale**: 5단계 독립 서버·이후 CLI가 같은 조립을 쓴다(정본 "`crates/workbench-server`: Axum HTTP/WS, auth, CORS, OpenAPI, event fan-out"). core에 두면 core가 HTTP 프레임워크에 묶이고, AW에 두면 독립 서버가 앱을 의존하게 된다. 계약 trait만 보므로 테스트 harness와 운영이 같은 코드를 쓴다(FR-002).

**Alternatives**: core `infrastructure/http` 모듈(core에 axum 운영 의존 추가 — 거절), AW 모듈(독립 서버 재사용 불가 — 거절).

## R2. 경로

| 경로 | 인증 | 설명 |
|---|---|---|
| `GET /health/live` | 없음 | `200 {"status":"live"}`만. 버전·경로·프로젝트 정보 없음(FR-013) |
| `GET /health/ready` | bearer | `{ready, serverEpoch}` — router는 런타임 조립(기동 복구 포함)이 끝난 뒤에만 bind되므로 받는 순간 ready |
| `POST /v1/system/handshake` | bearer | R6 |
| `POST /v1/calls` | bearer | 본문 = `CallRequest`, 응답 = `CallReply` 또는 problem+json(오늘 harness 형식) |
| `POST /v1/event-tickets` | bearer | R4 |
| `GET /v1/events?ticket=…` | 표 | R5 |
| `GET /openapi.json` | bearer | 계약 문서(`workbench_protocol::openapi`의 직렬화 — 커밋된 golden과 drift 검사로 같음) |

모든 응답에 `AW-Protocol-Version: <선택된 버전>` 헤더. 알 수 없는 경로 404, 허용 안 된 메서드 405.

## R3. 인증: 자격 증명 해석 포트 + 두 발급원

**Decision**: `CredentialResolver` 포트(`resolve(bearer, origin) -> Option<ResolvedCredential{principal, bound_origin}>`)를 router에 주입한다. AW의 조립은 두 발급원을 합친다.

1. **데스크톱 토큰**: `workbench-server`의 메모리 발급기 `DesktopTokenIssuer`. 256비트 무작위(URL-safe base64), TTL 15분, 발급 시 WebView 출처와 클라이언트 인스턴스 id에 묶는다. 요청 Origin이 있으면 묶인 출처와 같아야 하고, 없으면(비브라우저) 거절한다 — 데스크톱 토큰은 WebView 전용이다. 상한 256개, 만료분은 발급 때 정리한다. 비교는 해시 표 조회(토큰 해시 SHA-256을 키로, 원문 미보관).
2. **MCP 실행 토큰**: AW `CapabilityRegistry`(토큰 → run id)를 해석해 `AuthenticatedPrincipal::agent(run_id)`. 요청마다 registry를 보므로 폐기가 즉시 반영된다(FR-011). Origin은 요구하지 않는다(agent는 비브라우저).

테스트는 `test-desktop` 등 고정 토큰 resolver를 주입한다(오늘 harness 토큰 이름 유지).

**Rationale**: grill 결정 8. 넓은 bootstrap 비밀을 WebView에 넘기지 않는다(정본 "broad bootstrap credential은 WebView JavaScript에 넘기지 않는다"). 이번 단계에서 데스크톱 발급은 같은 프로세스라 OS credential store가 필요 없다(5단계).

**Alternatives**: 장기 단일 토큰(유출 시 영구 — 거절), cookie(CSRF·CORS credentials — 거절).

## R4. 구독 표 발급

**Decision**: `POST /v1/event-tickets` 본문 `{cursors: [StreamCursor]}` → `{ticket, expiresAt}`. 표는 256비트 무작위, **TTL 30초, 1회용**, 발급 주체 principal·cursor 목록·요청 Origin(없으면 "없음")에 묶는다. 저장은 메모리(상한 1,024, 만료분 정리). 발급 때는 인증·cursor 수 상한(오늘 hub 상한 64)·형식만 본다. 스트림 권한·gap·세대 판정은 **연결 때** 오늘과 같은 `Workbench.events`가 한다.

**Rationale**: 이벤트 fixture가 요구하는 거절(`cursors-over-limit`, forbidden, notFound 등)을 두 경로에서 같게 내려면, 판정을 한 곳(`events`)에 두고 WS에서 오늘처럼 `fault` 프레임으로 알리는 편이 맞다. 발급 때 판정을 복제하면 두 판정이 어긋날 수 있다. 표가 짧고 1회용이라 발급 시점 권한과 연결 시점 권한 차이는 연결 때 다시 판정해 닫힌다.

## R5. WebSocket 흐름

**Decision**: `GET /v1/events?ticket=<t>` upgrade 전에 Host·Origin(R7)을 검사하고, 표를 **원자적으로 꺼내(take)** 소모한다(없음·만료·Origin 불일치 → upgrade 없이 401/403). upgrade 뒤 `hello{protocolVersion, epoch}` → 표의 cursor로 `Workbench.events(principal, …)` → `event`/`gap`… 또는 `fault` 후 close. 클라이언트 `subscribe` 프레임은 없어진다(cursor는 표에 있다). 연결 종료 = 구독 해제. 표 문자열은 어디에도 기록하지 않는다.

**Rationale**: 정본 "ticket의 filter는 연결 후 확대할 수 없다, v1은 새 ticket/reconnect". 기록→실시간 경계의 원자성은 이미 `EventHub::subscribe`가 보장한다(039 경합 테스트) — WS 어댑터는 스트림을 그대로 흘린다.

**Contract change**: 039 contracts §6(테스트 전용 `subscribe` 프레임)을 이 흐름으로 대체한다(contracts/workbench-http.md에 정본).

## R6. handshake

**Decision**: 요청 `{supportedProtocolVersions: [u16], client: {name, version}}` → 응답 `{selectedProtocolVersion, supportedProtocolVersions, serverVersion, apiMajor: 1, contractHash, instanceId, serverEpoch, storageSchemaVersion, features: []}`. 교집합이 없으면 `409 conflict` fault `"protocol version is not supported."`(details `supportedProtocolVersions`). `contractHash` = OpenAPI 문서 JSON의 SHA-256 hex. `instanceId` = 프로세스 기동 때 uuid. `serverEpoch`·`storageSchemaVersion`은 주입한 `ServerInfo` 포트(AW: 런타임 epoch·ledger schema)에서.

## R7. Host·Origin·CORS

**Decision**:
- bind는 `127.0.0.1:0`(임의 포트)만. 설정으로 다른 주소를 줄 수 없게 한다(remote는 범위 밖).
- **Host**: `127.0.0.1:<port>` 또는 `localhost:<port>`와 정확히 같아야 한다. 아니면 `421`(misdirected) 없이 `403 forbidden` fault.
- **Origin**: 없으면 통과(비브라우저 — 인증으로만 판단). 있으면 허용 목록과 **문자열 전체 일치**(접두사·접미사 금지, `null` 거절). 허용 목록은 조립 쪽이 준다: 개발 `http://localhost:1420`, macOS/Linux 배포 `tauri://localhost`, Windows 배포 `http://tauri.localhost`. 실제 WebView Origin은 스모크에서 캡처해 목록과 맞춘다.
- **CORS**: tower-http `CorsLayer`에 같은 허용 목록(`AllowOrigin::list`), 메서드 `GET, POST`, 헤더 `authorization, content-type`, 노출 헤더 `AW-Protocol-Version`, credentials 끔. preflight `OPTIONS`는 인증 없이 Host·Origin만 본다.
- 같은 `OriginPolicy`(정확 일치)를 AW MCP 서버의 `origin_allowed`에 적용한다(FR-015). MCP의 허용 목록은 같은 WebView 출처 + 루프백 없음.

## R8. 본문 상한·시간 제한

**Decision**: 요청 본문 1 MiB(`DefaultBodyLimit`), 초과는 `413`. WS 메시지 수신 상한 64 KiB(클라이언트는 close 외에 보낼 것이 없다). 요청 시간 제한은 두지 않는다(`waitChildTasks`가 30초까지 기다린다) — `CallRequest.timeoutMs`는 오늘처럼 무시.

## R9. 로그

**Decision**: router는 요청마다 한 줄의 접근 기록을 주입받은 sink로 낸다: `requestId, operation, principalKind, status, latencyMs`. URI query·헤더·본문은 기록하지 않는다. AW는 sink를 stderr(`[workbench-http] …`)로, 테스트는 수집 sink로 두어 토큰·표 문자열이 0건임을 확인한다(SC-007).

## R10. AW 기동·종료

**Decision**: AW `setup`에서 런타임 조립 뒤 `workbench_server::serve(listener, router)`를 Tauri async 런타임에 띄운다. 실패(bind 실패 등)는 stderr에 남기고 앱은 계속(FR-016). 앱 종료(`RunEvent::Exit`)에서 graceful shutdown 신호를 보낸다(FR-017). 끝점은 `WorkbenchHttpState`로 관리한다.

## R11. 데스크톱 연결 정보 command

**Decision**: Tauri command `get_workbench_connection()` → `{baseUrl, token, expiresAt}`. 호출한 창의 WebView URL 출처로 토큰을 묶는다. 화면은 아직 부르지 않는다(4단계). 창 label은 서버에 넘기지 않는다.

## R12. 앱 연결 스모크: 두 증거를 구분한다 (사용자 점검 반영)

**Decision**: 앱 연결 확인은 두 가지를 따로 모으고 섞어 보고하지 않는다. 둘 다 **debug 빌드에서만**(`cfg(debug_assertions)`) 컴파일되고, 환경 변수가 있을 때만 켜진다. release 빌드에는 코드가 없다.

1. **끝점 진단(외부 클라이언트)** — `AW_HTTP_DIAGNOSTIC_FILE`: 기동 직후 `{baseUrl, token, expiresAt}`를 owner-only(0600) 파일로 쓴다. 토큰은 **운영과 같은 발급기·검증 경로**(`DesktopTokenIssuer`)로 만들되 "Origin 없음"에 묶은 비브라우저용이다. 이것으로 `curl`/테스트 클라이언트가 health·handshake·calls·표·WS를 확인한다. **증명하는 것은 "끝점이 떠서 인증 규칙대로 응답한다"뿐이며, 데스크톱(WebView) 연결 성공으로 보고하지 않는다.**
2. **WebView probe(실제 데스크톱 경로)** — `AW_HTTP_WEBVIEW_PROBE_FILE`: 메인 창이 로드되면 Rust가 `window.eval`로 개발용 probe 스크립트를 넣는다(AW는 이미 이벤트 전달에 같은 삽입 경로를 쓴다). probe는 WebView 안에서
   - `invoke('get_workbench_connection')`로 **운영 command·발급기**에서 데스크톱 토큰을 받고,
   - 브라우저가 붙이는 **실제 Origin**으로 `fetch` handshake·`project.list`·표 발급, `new WebSocket(…?ticket=…)`로 `hello` 수신, 같은 표 재사용 거절, 토큰 없는 호출 401을 확인하고,
   - `location.origin`과 각 결과(상태 코드·프레임 종류만, 토큰·표 문자열 없음)를 debug 전용 command `report_http_probe`로 돌려준다. Rust가 파일에 쓴다.
   이것이 SC-005("발급한 데스크톱 토큰으로 연결")의 근거이며, 캡처한 `location.origin`으로 허용 출처 목록(R7)을 확정한다.

**Rationale**: 041 스모크에서 접근성 트리가 웹뷰 요소를 노출하지 않은 것은 **화면 조작** 자동화를 막을 뿐이다. WebView 안에서 요청을 보내는 것은 스크립트 삽입(`window.eval`)으로 할 수 있다(CSP 미설정, 앱 command는 기본 capability로 main·session 창에서 호출 가능 — `tauri.conf.json`·`capabilities/default.json` 확인). 진단 토큰만으로는 운영 command 경로·Origin 묶임·CORS를 증명하지 못한다.

**Alternatives**: devtools 수동 확인만(자동 증거 없음 — 보조로만), 별도 테스트 페이지(실제 WebView Origin이 아님 — 거절).

## R13. 변경 operation의 네트워크 공개 조건 (사용자 점검 반영)

**Decision**: 정상 경로 parity는 crash 안전성 근거가 아니다. 변경 operation을 네트워크에 여는 조건은 **그 operation의 중단·재시작 판정 증거**다. 네트워크는 in-process와 같은 멱등 기록을 쓰므로 새 중단 지점을 만들지는 않는다. 하지만 클라이언트가 응답을 잃고 **재시작 뒤 같은 키로 재시도**할 수 있게 되므로, 그 재시도가 다시 적용되지 않는다는 증거가 필요하다. 증거가 없는 operation은 042에서 테스트를 추가하고, 추가 전까지는 네트워크에 열지 않는다(router의 operation 허용 집합에서 뺀다 — 기본은 전부 공개가 목표이므로 태스크는 보강을 먼저 한다).

| 분류 | operation | 기존 증거 | 042 보강 |
|---|---|---|---|
| 영속(ledger), reconciler 있음 | `project.create` | `ledger_crash_points.rs` 5개(세 중단 지점 + ledger 완료 실패) | — |
| | `savedPrompt.create`·`savedPrompt.delete` | `us1_crash_points.rs` | — |
| | `goal.create`·`goal.recordProgress`·`agentRunSettings.save` | `us1_crash_points.rs`(recordProgress·settings는 모든 지점 unknown) | — |
| | `git.createWorktree`·`git.deleteWorktree` | `git_reconcile.rs` 7개, `reservation_lifecycle.rs` | — |
| | `run.start` | `run_start_reconcile.rs`(apply 뒤 중단 → unknown, 완료 재시도 → 저장 결과) | — |
| | `project.delete`·`goal.clear` | **없음**(reconciler만 등록) | 세 중단 지점 × 재시작 판정·같은 키 재요청 테스트 |
| 영속(ledger), reconciler 없음(unknown) | `project.update`·`savedPrompt.update`·`goal.update` | **없음** | 세 중단 지점 → unknown·자동 재실행 없음·같은 키 재요청 테스트 |
| 세대 범위(메모리 작업대) | `bench.*`·`run.*`(start 제외)·`exchange.*` | `epoch_idempotency.rs`(기록 넘침·요약 한도·닫힌 작업대 재시도 notFound). **재시작 뒤 재시도 증거 없음** | 서버 재시작 뒤 같은 키 재시도 → 작업대 없음(`notFound`)·재적용 없음 |
| 세대 범위 + 파일 영속 | `orchestration.*` 변경 27개 | 저장 경계·동시성 테스트, 작업대 닫힘 뒤 역할 거절. **재시작 뒤 재시도·파일 중단 증거 없음** | (a) 변경 뒤 재시작 → 같은 키 재시도 → `notFound`, 파일은 변경 한 번만 반영, (b) 저장 중단(임시 파일 쓰기 실패 주입) → 이전 파일 유지·`.bak` 복구 유지 |

- 조회 32개는 상태를 바꾸지 않으므로 공개 조건이 없다.
- 증거 테스트는 in-process 런타임으로 쓴다(중단 주입이 런타임 내부에 있다). 네트워크 경로는 "같은 키 재시도" 단계만 HTTP로 한 번 더 보내 같은 결과인지 확인한다.

**Alternatives**: 정본대로 opt-in 목록만 공개 — 보강이 끝나면 전부 공개와 같아지므로, 보강을 이번 범위에 넣고 목표(전부 공개)를 유지한다.

## R14. 계약 suite의 운영 router 전환

**Decision**: core 테스트 `support/http_harness.rs`를 `workbench-server` router를 띄우는 얇은 래퍼로 바꾼다(고정 토큰 resolver, 허용 Origin 없음, 접근 기록 수집). 이벤트 WS 경로는 표 발급 → 연결로 바뀐다. 기존 fixture 결과가 바뀌면 안 된다(FR-002).

## R15. ADR

**Decision**: `crates/workbench-server/docs/adr/0001-local-http-authenticates-every-request-with-short-lived-credentials.md`(루프백·정확 Host/Origin·짧은 데스크톱 토큰·1회용 표, Origin은 인증 아님), `0002-event-subscriptions-are-authorized-at-connect.md`(표 발급은 형식만, 권한은 연결 때 한 곳에서).

## R16. 스모크 판정

**Decision**: 실제 앱을 격리 데이터 디렉터리(041처럼 `tauri dev --config` identifier 변경)로 띄운다. (a) 끝점 진단 파일로 외부 health·handshake·calls·표·WS를 확인하고, (b) WebView probe 결과 파일로 데스크톱 토큰·실제 Origin 경로를 확인한다. 둘 다 재기동 뒤 한 번 더 한다. 보고할 때 (a)는 "끝점", (b)는 "데스크톱 연결"로 나눠 적는다. probe가 실패하면(예: Origin이 허용 목록과 다름) 그 사실과 캡처한 Origin을 그대로 적고 목록을 고친다.
