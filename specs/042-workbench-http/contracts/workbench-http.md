# Contract: Workbench HTTP/WebSocket (042)

로컬 루프백 전송 계약. 본문 형식(`CallRequest`·`CallReply`·`WorkbenchFault`·`EventFrame`)은 `workbench-protocol`이 정본이고 OpenAPI(`crates/workbench-protocol/openapi/workbench.openapi.json`)가 생성물이다. 이 문서는 전송·인증 규칙만 정한다. 결정 근거는 [research](../research.md).

## 1. 연결

- 주소: `127.0.0.1:<port>`(임의 포트)만 bind한다.
- 모든 요청: `Host`가 `127.0.0.1:<port>` 또는 `localhost:<port>`와 정확히 같아야 한다. 아니면 `403` problem(`code: forbidden`, `message: "host is not allowed."`).
- `Origin`이 있으면 허용 목록과 **문자열 전체 일치**여야 한다(`null`·접두사·접미사 거절). 아니면 `403`(`"origin is not allowed."`). 없으면 통과하고 자격 증명으로만 판단한다.
- 모든 응답에 `AW-Protocol-Version` 헤더(현재 `1`).

## 2. 인증

- `Authorization: Bearer <token>`. 없거나 해석되지 않으면 `401` problem(`code: unauthenticated`, 오늘 문구).
- 토큰 종류와 principal:

| 종류 | principal | Origin |
|---|---|---|
| 데스크톱 토큰(`get_workbench_connection`) | `desktop` | 필수, 발급 때 묶인 출처와 같아야 함 |
| 진단 토큰(debug 빌드, `AW_HTTP_DIAGNOSTIC_FILE`) | `desktop` | 없어야 함(비브라우저 전용) |
| MCP 실행 토큰(`awcap_…`) | `agent:<runId>` | 무관 |
| 테스트 토큰(`test-desktop` 등, 테스트 조립만) | 오늘 harness와 같음 | 무관 |

- 만료·폐기된 토큰은 해석되지 않은 것과 같다(`401`). 오류 본문은 토큰 종류·만료 사유를 드러내지 않는다.

## 3. 경로

| 메서드·경로 | 인증 | 요청 | 응답 |
|---|---|---|---|
| `GET /health/live` | 없음 | — | `200 {"status":"live"}` |
| `GET /health/ready` | bearer | — | `200 {"ready":true,"serverEpoch":"…"}` |
| `POST /v1/system/handshake` | bearer | `{supportedProtocolVersions:[1], client:{name,version}}` | `200 {selectedProtocolVersion, supportedProtocolVersions, serverVersion, apiMajor, contractHash, instanceId, serverEpoch, storageSchemaVersion, features}`; 교집합 없음 `409` fault `"protocol version is not supported."` |
| `POST /v1/calls` | bearer | `CallRequest` | `200 CallReply` 또는 problem |
| `POST /v1/event-tickets` | bearer | `{cursors:[StreamCursor]}` | `200 {ticket, expiresAt}`. 발급은 인증·형식·고정 상한(cursor 1,024개, 초과 `400` `"too many cursors for a ticket."`)만 본다. cursor 0개·hub 상한·권한은 연결 때 `fault` 프레임(§4) |
| `GET /v1/events?ticket=…` | 표 | WebSocket upgrade | §4 |
| `GET /openapi.json` | bearer | — | 계약 문서(커밋된 파일과 같은 내용) |
| `OPTIONS *` | 없음 | preflight | 허용 출처에만 CORS 헤더 |

- **실행 수명**: 서버가 받아들인 호출은 연결과 무관하게 끝까지 실행되고 멱등 결과가 기록된다(research R17). 연결이 끊긴 클라이언트는 같은 멱등성 키로 재시도해 저장된 결과를 받는다. 진행 중 재시도는 in-process와 같은 operation별 의미를 따른다 — 세대 범위 command(`run.sendPrompt`·orchestration 변경 등)는 끝날 때까지 기다리고, `run.start` 같은 ledger 경로는 retryable `conflict`(`outcome: unknown`)를 돌려주므로 같은 키로 다시 시도한다. 종료 신호 뒤 새 호출은 `503 unavailable`이고, 받아들인 호출은 drain(상한 없음, 경고 간격마다 기록)으로 끝난 뒤 서버가 내려간다. AW MCP 서버의 도구 호출도 같다.
- problem 형식(오늘 harness와 같음): `Content-Type: application/problem+json`, 본문 = `WorkbenchFault` 직렬화 + `type: "urn:aw:fault:<code>"`, `title: <code>`, `status: <http>`. HTTP 상태는 `FaultCode::http_status()`.
- 본문 상한 1 MiB, 초과 `413`.

## 4. WebSocket

1. upgrade 전: Host·Origin(§1) 검사 → 표를 원자적으로 꺼내 소모. 없음·만료·이미 사용 → `401`; 표의 Origin과 요청 Origin 불일치 → `403`. 이때 upgrade하지 않는다.
2. upgrade 뒤 서버가 표의 cursor로 구독을 등록한다(오늘 `events` 판정과 같음).
3. 서버가 `hello{protocolVersion, epoch}`를 보낸다 — **hello는 구독 준비 완료 신호**다: hello 뒤에 일어난 변경은 재생 없는 알림 스트림에서도 전달된다. 등록이 거절됐으면 hello 뒤 `fault{fault}`를 보내고 close한다(구현 리뷰 중 발견: 처음에는 등록 전에 hello를 보내 worktree 알림을 잃을 수 있었다).
4. 이어서 `event`/`gap` 프레임. 클라이언트가 보내는 것은 close뿐이다(수신 메시지 상한 64 KiB).
5. 연결 종료 = 구독 해제. 재연결은 새 표 + 마지막 cursor.

039 contracts §6의 클라이언트 `subscribe` 프레임은 이 흐름으로 대체한다.

## 5. CORS

허용 출처 목록(정확 일치), 메서드 `GET, POST`, 요청 헤더 `authorization, content-type`, 노출 헤더 `AW-Protocol-Version`, credentials 없음, `max-age` 600.

## 6. 기록

요청마다 `requestId, operation, principalKind, status, latencyMs` 한 줄. URI query·헤더·본문·토큰·표는 기록하지 않는다.

## 7. MCP 서버 출처 검사

AW MCP 서버(`POST /mcp`)의 Origin 검사는 §1과 같은 정확 일치 규칙을 쓴다(오늘 접두사 비교 결함 수정). Origin 없음은 오늘처럼 허용(agent는 비브라우저).

**도구 호출 재시도 식별(사용자 검토 추가)**: agent는 응답을 못 받으면 같은 도구를 같은 인자로 다시 부르고 JSON-RPC id는 새로 붙는다. 그래서 어댑터는 멱등성 키를 도구 인자의 `requestId`에서 만든다: `mcp-` + SHA-256(`runId` · operation · `requestId`). 같은 요청의 재전송·동시 전송·단절 뒤 재전송은 원 호출을 기다리거나 저장된 결과를 받는다(세대 범위). `requestId`가 없으면 유효한 JSON-RPC 요청 id(숫자·문자열 구분)로 같은 wire 요청의 재전송을 식별한다: `mcp-` + SHA-256(종류 · `runId` · operation · id · 인자 전체). 그래서 응답을 잃은 agent가 같은 id로 다시 보내면 처음 결과를 받는다(예: `aw_collect_child_results`는 그 사이 새 보고서가 와도 처음 목록). 한계: 새 id이고 `requestId`도 없으면 새 요청이다. MCP 클라이언트가 다시 연결해 id를 처음부터 다시 쓰고 인자까지 같으면, 세대 멱등 기록이 남은 동안 재전송으로 본다. 도구 호출은 서버 소유 task에서 실행하고(`spawn_accepted`), 종료 중 새 호출은 `503`(JSON-RPC `-32000`).

