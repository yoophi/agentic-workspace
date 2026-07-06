# Research: MCP Session Title Control

**Feature**: 011-mcp-session-ui-control | **Date**: 2026-07-06

Technical Context에 미해결 질문은 없다. 사용자 입력에서 "HTTP 방식 MCP"와 "윈도우 타이틀 변경으로 한정"이 명시되었으므로, 연구 범위는 transport 호환성, 보안 경계, 앱 내부 배치, 검증 규칙 확정이다.

## R1. MCP HTTP transport 기준

**Decision**: 안정판 MCP `2025-11-25` Streamable HTTP transport에 맞춰 단일 local MCP endpoint를 제공한다. 최소 구현은 `POST /mcp`에서 JSON-RPC request를 받아 `initialize`, `tools/list`, `tools/call`을 처리하고, title 변경처럼 즉시 완료되는 요청은 `application/json` 응답으로 반환한다. SSE/long-lived GET은 initial title-change-only scope에서 필요하지 않으며, 지원하지 않는 method는 명확한 오류를 반환한다.

**Rationale**:
- 공식 transport 문서는 Streamable HTTP가 단일 MCP endpoint에서 HTTP POST/GET을 처리하고, 요청별로 JSON 또는 SSE를 반환할 수 있다고 정의한다.
- title 변경은 짧은 동기성 command라 SSE streaming이 필요 없다.
- HTTP+SSE 구 transport compatibility는 이번 기능 가치와 무관하고 surface area를 늘린다.
- 공식 문서는 local server가 localhost bind, Origin validation, authentication을 적용해야 한다고 경고한다. 이 feature는 로컬 앱 내부 control channel이므로 이 세 가지를 gate로 둔다.

**Alternatives considered**:
- **stdio MCP server**: agent process가 별도 subprocess로 서버를 띄워야 하며 사용자가 HTTP 방식을 명시했으므로 기각.
- **구 HTTP+SSE transport**: deprecated compatibility 부담이 있고 title 변경에 필요하지 않아 기각.
- **draft protocol 기준 구현**: 현재 안정판 agent/client 호환성이 더 중요하므로 stable `2025-11-25` 우선. draft 변경은 후속 호환성 작업에서 검토한다.

**Source**: Model Context Protocol official transport specification, `2025-11-25` and Streamable HTTP security guidance: https://modelcontextprotocol.io/specification/2025-11-25/basic/transports

## R2. Rust HTTP server dependency

**Decision**: `apps/agentic-workbench/src-tauri`에 Tokio 기반 HTTP stack으로 `axum`과 필요한 `tower-http` layer를 추가하는 계획을 채택한다. MCP protocol shape는 별도 crate로 공유하지 않고 AW app 내부 `infrastructure/mcp`에 둔다.

**Rationale**:
- 현재 Tauri backend는 이미 `tokio` runtime과 async services를 사용한다.
- `axum`은 JSON body extraction, header validation, state injection, graceful shutdown 구성이 단순하다.
- 직접 TCP/HTTP parsing을 구현하면 보안/프로토콜 오류 가능성이 크고 테스트 비용이 증가한다.
- MCP title control은 AW app-specific이라 `crates/*` 공유 crate로 승격할 근거가 없다.

**Alternatives considered**:
- **수동 `tokio::net::TcpListener` + HTTP parsing**: 표준 header/status/body 처리 실수 가능성이 커서 기각.
- **Tauri command only**: agent가 MCP client로 호출할 HTTP service가 필요하므로 user requirement를 만족하지 못해 기각.
- **새 workspace Rust crate**: 다른 앱 소비자가 없어 constitution의 shared-core 기준을 만족하지 못하므로 기각.

## R3. MCP service lifecycle과 agent env 주입

**Decision**: Tauri app startup에서 local MCP server를 1회 시작하고 app-managed state로 base URL과 server token을 보관한다. `start_agent_run`은 normalized run id와 owner window label이 확정된 뒤 agent environment에 MCP URL, run id, token을 내부 예약 key로 주입한다.

**Rationale**:
- 앱당 하나의 local server가 여러 run의 control request를 route하면 port 관리가 단순하다.
- 기존 `AppState`가 `run_id -> owner_window_label`을 보관하므로 session owner authorization에 재사용할 수 있다.
- run마다 별도 server를 띄우면 포트 수명, shutdown, race condition이 증가한다.
- agent profile/user env 병합과 충돌하지 않도록 app-reserved env key를 주입해야 한다.

**Alternatives considered**:
- **run마다 HTTP server 생성**: 격리는 좋지만 lifecycle 복잡도와 포트 churn이 커서 title-only scope에는 과하다.
- **frontend에서 HTTP server 제공**: desktop webview 보안 모델과 background availability가 맞지 않아 기각.
- **인증 없이 localhost만 신뢰**: 공식 MCP guidance가 authentication을 권장하며 DNS rebinding 위험이 있어 기각.

## R4. Authorization model

**Decision**: 모든 MCP tool call은 bearer token 또는 동등한 secret 검증 후, tool arguments의 `runId`가 active registry에 존재하고 해당 run owner window가 아직 유효한지 확인한다. title 변경은 registry owner window label로만 전달하며 caller가 window label을 직접 지정할 수 없게 한다.

**Rationale**:
- FR-003/FR-004/SC-002는 cross-session 변경 방지를 요구한다.
- `runId -> owner_window_label` 매핑은 이미 permission response와 close cleanup에 사용되는 신뢰 가능한 scope boundary다.
- caller가 window label을 직접 넘기면 cross-window control risk가 생긴다.

**Alternatives considered**:
- **tool argument로 window label 허용**: 편하지만 cross-session spoofing surface가 커서 기각.
- **token만 검증하고 run owner 검증 생략**: 하나의 token leak으로 모든 session title 변경이 가능해져 기각.
- **frontend에서만 owner 검증**: backend가 이미 window event를 emit하는 권한 주체이므로 backend gate가 필요하다.

## R5. Title validation and result rule

**Decision**: title은 trim 후 non-empty, control character 제거/거부, 최대 80자 표시 규칙을 적용한다. 초과 title은 명시적으로 reject하는 쪽을 기본으로 하며, 구현 중 UX 판단으로 truncate를 선택하면 result message에 truncation을 명시해야 한다.

**Rationale**:
- spec은 blank/unreadable/invalid title rejection과 documented maximum rule을 요구한다.
- rejection은 agent가 더 좋은 제목을 다시 제안하도록 만드는 명확한 feedback이다.
- 80자는 window title bar와 app switcher에서 읽을 수 있는 현실적 상한이다.

**Alternatives considered**:
- **무제한 title 허용**: window title bar/app switcher 가독성 저하로 기각.
- **항상 truncate**: agent가 실제 적용 title을 예측하기 어렵다. 후속 UX 필요가 생기면 명시적 result와 함께 허용.
- **frontend만 validation**: backend가 MCP result를 즉시 반환해야 하므로 backend validation이 기준이어야 한다.

## R6. Frontend application model

**Decision**: 현재 route-level default title 계산을 유지하고, MCP title event가 오면 standalone Worktree Session window에 runtime override title을 적용한다. project/worktree가 바뀌거나 invalid request가 오면 default title을 유지한다.

**Rationale**:
- 기존 `ProjectWorktreeSessionRoute`가 `formatWorktreeWindowTitle(project.name, worktree.path)`로 title을 관리한다.
- title 변경은 workspace tab/file state와 독립적이므로 `WorktreeWorkspacePanel`에 상태를 넣을 필요가 없다.
- runtime override는 persistence out-of-scope와 맞다.

**Alternatives considered**:
- **ProjectWorktreeSessionPage 내부에서 title 직접 관리**: route가 standalone 여부와 default title을 이미 알고 있어 상태 중복이 생긴다.
- **agent run panel timeline event로만 표시**: 실제 window title 변경 요구를 만족하지 못한다.
- **title 영속 저장**: spec에서 runtime UI state로 가정했으므로 scope 초과.

## R7. Verification strategy

**Decision**:
- Rust unit tests: title validation, token/header validation, tools/list only exposes title tool, inactive/cross-session run rejection, valid owner emits target command.
- TS unit tests: default title 유지, runtime override 적용, invalid/empty command 무시 또는 fallback 유지.
- Manual quickstart: local app에서 run 시작 후 MCP `tools/list`와 `tools/call`로 title 변경 확인.

**Rationale**:
- session owner와 token validation은 high-risk boundary라 Rust tests가 필요하다.
- frontend는 title application helper 중심으로 빠르게 검증한다.
- 실제 window title은 Tauri webview integration이라 quickstart manual check가 필요하다.
