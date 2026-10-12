# Box 문서 도구를 위한 공용 HTTP MCP 호스트 계획

2026-10-13, box.md gh-20/gh-48/gh-49. 이 문서는 공용 라이브러리 확장 계획이며 구현 완료나 실제 agent 도구 사용의 증거가 아니다. 작업 브랜치는 `box-md-document-mcp`, 기반은 공유 HTTP ACP client와 UI 확장이 포함된 `8534a7b`이다.

## 충분한 공유 기능과 부족한 기능

`packages/workbench-agent-client`의 SessionOptions는 `mcpServers`를 받고 HTTP `run.createSession` 요청으로 전달한다. `crates/acp-agent-core` runner는 initialize의 HTTP MCP capability를 확인한 뒤 `session/new`에 전달한다. Box의 ACP 실행·채팅 경로는 이 공유 구현을 계속 사용한다.

`crates/workbench-host/src/mcp`의 서버는 Tauri 없이 실행되지만 `WorkbenchRuntime`과 AW orchestration/title/exchange 도구에 결합돼 있다. 앱별 handler를 주입하는 공개 HTTP 호스트가 없으므로 Box 문서 service를 그대로 등록할 수 없다. AW 도구에 Box 파일 저장 코드를 추가하지 않는다.

## 공용 crate와 책임

`crates/local-mcp-server`를 workspace에 추가한다. 공식 Rust SDK `rmcp = "=3.5.1"` 및 lockfile로 wire protocol을 맡긴다. Rust 1.88 이상과 실제 workspace compile 호환성을 먼저 확인한다. [공식 릴리스](https://github.com/modelcontextprotocol/rust-sdk/releases/tag/rmcp-v3.5.1), [고정 SDK Cargo.toml](https://github.com/modelcontextprotocol/rust-sdk/blob/79437f291b2c44053d00dcd5db969fd0cca7c887/Cargo.toml).

| 소유자 | 책임 |
| --- | --- |
| 공용 HTTP 호스트 | ServerHandler factory, `/mcp` router, loopback 임의 포트 listener, 인증 callback/요청 extension, Host/Origin 정책, 서버 종료 handle |
| SDK | initialize/initialized, 협상 버전, JSON-RPC 검증, tools dispatch/schema 및 HTTP 응답 |
| 앱 handler/application | 도구 목록/스키마/업무 오류, 권한 범위, 문서 저장·CAS·retry dedupe, 수락 mutation drain, 세션 grant 발급/폐기 |
| 기존 AW ACP 공유 client/core | HTTP run 요청, session/new, agent capability·진단·session 수명 전달 |

공용 crate는 Tauri, WorkbenchRuntime, Box 도메인/저장소에 의존하지 않는다. Box에 필요한 서버 기능을 AW에 먼저 공유 모듈로 마련해 달라는 사용자 요청에 따른 추출이다. 초기 소비자는 Box 문서 handler와 AW crate의 generic 예제/HTTP 통합 테스트다. 기존 AW runtime MCP의 공용 호스트 전환은 후속 적용 대상으로 남기고, Box 기능의 선행 조건으로 orchestration 전면 변경을 추가하지 않는다.

## API와 보안 계약

- start는 caller의 Tokio runtime에서 동작한다. listener는 `127.0.0.1:0`에 고정하며 endpoint와 owned shutdown handle을 반환한다. 실행 중인 다른 앱/server를 종료하지 않는다.
- handler factory는 official `ServerHandler`를 받는다. 인증 callback이 해석한 principal은 request parts extension으로 전달한다. 단순 bearer 문자열을 문서 ID나 권한으로 간주하지 않는다.
- 초기 protocol은 2025-11-25, stateless와 JSON 응답을 사용한다. 실제 협상 allowlist는 handler API로 제한하고 get_info 문자열만 바꾸지 않는다.
- SDK의 빈 Origin allowlist 기본은 검증 OFF다. 명시적으로 Origin 검증을 켜고 non-browser의 없는 Origin만 허용한다. browser client를 지원할 때는 scheme/host/port 정확한 allowlist를 사용한다. 모든 POST/GET/DELETE에 인증을 적용한다. [고정 SDK HTTP config](https://github.com/modelcontextprotocol/rust-sdk/blob/79437f291b2c44053d00dcd5db969fd0cca7c887/crates/rmcp/src/transport/streamable_http_server/tower.rs).
- wrapper는 credentials를 Debug/log/body에 출력하지 않는다. callback은 HTTP 401, Host/Origin 검증은 명시적 transport 오류를 반환한다. 보안 오류가 다른 정상 요청의 서버를 중단하지 않는다.
- HTTP disconnect가 handler future를 취소할 수 있으므로 수락한 mutation은 앱 소유 task에서 원자 완료하고 결과를 조회할 수 있어야 한다. 호스트 종료와 앱의 수락 작업 drain 순서를 사용법에 명시한다. [공식 disconnect 회귀 테스트](https://github.com/modelcontextprotocol/rust-sdk/blob/79437f291b2c44053d00dcd5db969fd0cca7c887/crates/rmcp/tests/test_streamable_http_disconnect_cancel.rs).

## 순서와 검증

1. 공용 API·generic example·dependency를 구현하고 독립 crate compile을 확인한다.
2. 실제 HTTP로 initialize → initialized → tools/list → tools/call, JSON structured 결과와 업무 오류를 확인한다. auth 실패·폐기·Origin/Host·잘못된 버전·GET/종료 회귀를 검증한다.
3. handler factory별 다른 도구와 principal이 섞이지 않는 것을 확인한다. 예제에 Box path/metadata/credential 값을 넣지 않는다.
4. 기존 AW 관련 Rust 검증을 통과하고 공용 crate 사용법, 런타임/보안/종료 책임과 실제 테스트 결과를 기록한다.
5. AW 단위 커밋을 만들고 `box-md-document-mcp` 브랜치를 push한다. Box는 이 커밋을 submodule pin으로 반영하고 기존 HTTP ACP/UI 기능 회귀를 검증한다.
6. Box gh-50~gh-53에서 scoped service, lossless metadata, 도구 handler, main-only grant, dirty UI 보호, 실제 설치 agent 도구 사용을 구현·검증한다. 일반 HTTP 테스트는 native UI/실제 agent 사용을 대신하지 않는다.

Box의 상세 도구 입력·출력·오류·범위 계약은 Box 저장소 `docs/document-mcp-implementation-plan.md`와 `docs/document-mcp-protocol-research.md`에서 관리한다. 부모 gh-20의 전체 완료는 해당 구현과 실제 ACP/native 전수 감사 뒤에 판단한다.
