# 공용 로컬 HTTP MCP 호스트

`crates/local-mcp-server`는 앱의 공식 rmcp `ServerHandler`를 받아 인증된 루프백 MCP endpoint를 제공한다. Tauri, WorkbenchRuntime, Box 문서 저장소에 의존하지 않는다. 공유 확장 배경은 [설계 계획](shared-document-mcp-host-plan.md)에 있다.

## 공개 API와 기본 정책

| API | 역할 |
| --- | --- |
| `start(factory, authorize, config).await` | caller Tokio runtime에서 `127.0.0.1:0` listener를 생성한다. factory는 요청별 handler, authorize는 bearer credential을 앱 principal로 해석한다. |
| `HostConfig::default()` | Origin이 없는 비브라우저 client만 허용, 2 MiB body 상한, legacy MCP 2025-11-25. |
| `with_allowed_origin(origin)` | scheme/host/명시적 port의 정확한 browser origin을 추가한다. wildcard·userinfo·opaque origin·query·하위 path는 거부한다. |
| `with_protocol_versions(versions)` | SDK가 아는 legacy 버전의 비어 있지 않은 목록. handler의 `supported_protocol_versions`와 맞춰야 한다. 2026 inline protocol은 이 호스트의 초기 범위 밖이다. |
| `with_max_request_bytes(bytes)` | 0보다 큰 요청 body 상한. HTTP body와 SDK decoder에 같은 제한을 적용한다. |
| `host.endpoint()` | 실제 배정된 포트의 `/mcp` URL. 자격 증명을 포함하지 않는다. |
| `principal::<P>(&context)` | tool handler에서 인증된 앱 principal을 가져온다. caller는 principal에 credential 원문을 넣지 않는다. |
| `host.stop_accepting()` | 새 요청은 503. 이미 진입한 handler를 취소하지 않는다. |
| `host.shutdown().await` | SDK transport와 자신이 소유한 listener의 종료를 기다린다. Drop도 cancellation을 요청하지만 완료 대기는 명시적 shutdown을 사용한다. |

rmcp는 `=3.5.1`, Rust 최소 버전 선언은 1.88이다. 이번 compile 검증은 Rust 1.99에서 실행했으며 1.88 toolchain 검증을 수행했다고 주장하지 않는다. crate가 `rmcp`를 재내보내므로 consumer는 같은 SDK 타입을 사용할 수 있다. 공식 규격 parsing/initialize/notification/tool dispatch와 JSON/SSE 처리는 SDK에 맡긴다. 신규 공용 crate는 Axum 0.8을 사용하며 기존 AW 호스트는 0.7을 유지한다. Cargo.lock의 기존 package identity를 제거하거나 업그레이드하지 않았다.

인증 callback은 모든 MCP HTTP method에서 실행한다. Authorization 헤더가 없거나 여러 개, 잘못된 scheme/공백/쉼표, 알 수 없거나 폐기된 credential이면 401과 일반적인 오류만 반환한다. 인증 후 원문 Authorization 헤더를 SDK/handler request parts에서 제거하고 principal만 extension으로 전달한다. factory의 오류 이유도 일반적인 unavailable 오류로 치환한다. 앱의 business result나 별도 로그가 secret을 포함하지 않게 하는 책임까지 wrapper가 대신할 수는 없다.

Host는 실제 배정된 `127.0.0.1:port`로 제한한다. Origin이 있으면 RFC의 scheme/host/effective port를 비교하고 거부 시 403을 반환한다. 중복 Origin·Host도 거부한다. admission middleware에서 보안을 먼저 검사하며 SDK의 Host/Origin 검증도 같은 정책으로 활성화한다. CORS만 적용하거나 SDK 기본 빈 Origin allowlist를 사용하는 방식이 아니다.

## handler와 도구 계약

handler는 `get_info`와 `supported_protocol_versions`에 실제 버전과 capability를 광고한다. `list_tools`/`get_tool`로 schema를 제공하고 **runtime 인자를 직접 검증한다**. 도구 schema를 광고한 것만으로 extra key/type 검증이 완료되지는 않는다. `get_tool`은 SDK의 표준 MCP header/schema 처리에도 쓰이지만 전체 JSON Schema validation engine을 대신하지 않는다.

알려진 도구의 업무·인자 실패는 `CallToolResult::structured_error`의 `isError: true`로 반환한다. unknown tool과 잘못된 protocol 구조는 ErrorData로 구별한다. 초기 legacy 계약의 structured 결과는 객체이고 같은 JSON text도 제공한다. [echo handler](../crates/local-mcp-server/examples/support/echo.rs)는 이 경계와 인증 principal 전달을 보여 준다.

## HTTP 동작과 확인한 SDK 경계

| 상황 | 동작 |
| --- | --- |
| initialize | 2025-11-25 협상, stateless이므로 session ID 발급 없음. 처음에는 version header 없이도 협상 가능. |
| initialized notification | 인증·버전 확인 후 202, 빈 body. |
| tools/list, tools/call | Accept에 JSON/SSE를 둘 다 포함하고 negotiated version header를 보낸다. 단순 결과는 application/json. 중간 서버 메시지가 있으면 SDK가 SSE로 전환할 수 있다. |
| GET / DELETE | 인증·보안 검사 후 stateless 호스트에서 405. |
| unsupported / 중복 version header | 400. 기본 허용은 2025-11-25. |
| 후속 version header 누락 | legacy 기본 2025-03-26을 가정한다. 구성한 허용 목록에 없으면 400. initialize 분류에는 SDK message 타입만 사용한다. |
| body 상한 초과 | 413, 쓰기 handler까지 전달하지 않는다. |
| malformed JSON | rmcp 3.5.1 decoder의 415. JSON-RPC tool 오류와 구별한다. |
| handler factory 실패 | 일반적인 500. 다음 요청에서 factory가 복구되면 host를 재시작하지 않고 처리한다. |

실제 HTTP 회귀에서 SDK의 legacy stateless 경로가 handler의 지원 목록 대신 SDK가 아는 version header만 확인하는 것을 발견했다. 그래서 host는 구성한 legacy version header를 별도로 제한한다. SDK의 protocol parsing 자체를 복사하지 않는다. 공식 소스의 [stateless transport](https://github.com/modelcontextprotocol/rust-sdk/blob/79437f291b2c44053d00dcd5db969fd0cca7c887/crates/rmcp/src/transport/streamable_http_server/tower.rs)와 [ServerHandler](https://github.com/modelcontextprotocol/rust-sdk/blob/79437f291b2c44053d00dcd5db969fd0cca7c887/crates/rmcp/src/handler/server.rs)를 참고했다.

## 앱의 mutation 및 grant 책임

앱은 session/run과 workspace generation에 연결된 grant를 발급하고 authorize callback에서 매 요청 폐기를 반영한다. workspaces/document IDs를 HTTP 인자로 인증했다고 간주하지 않는다. domain 권한은 tool 호출 시 application에서도 다시 확인한다.

HTTP disconnect는 SDK handler future를 취소할 수 있다. 수락한 파일 mutation을 그 future에만 맡기지 않는다. 앱 소유 task에서 CAS·원자 저장·requestId 중복 억제를 완료하고 재조회 가능한 결과를 남긴다. 종료 순서는 다음과 같다.

1. host에서 신규 HTTP admission을 차단한다.
2. application에서 신규 mutation 수락을 막고 workspace/grant 상태를 직렬화한다.
3. application이 이미 수락한 mutation을 drain한다.
4. host shutdown을 호출하고 완료를 기다린다.

원문 파일 저장, accepted-task registry, retry journal, grant registry는 이 crate가 구현하지 않는다. Box gh-50~gh-53의 scope·변경·실제 ACP 검증이 필요하다. 기존 AW runtime의 orchestration MCP 도구는 그대로 유지한다.

## 실행과 검증

사용 예제는 credential을 `LOCAL_MCP_TOKEN` 환경변수에서 읽으며 표준 출력에는 endpoint만 출력한다. 비밀값을 명령/문서 예제에 붙여 넣지 않고 실행 환경에서 제공한다.

```sh
cargo run -p local-mcp-server --example echo
```

Ctrl-C로 자신의 host만 종료한다. 예제는 `echo` 도구를 제공하며 실제 문서 도구 구현은 아니다.

```sh
cargo test -p local-mcp-server --all-targets --locked
cargo check -p local-mcp-server --all-targets --locked
cargo fmt --all -- --check
cargo test -p workbench-host --lib --locked
cargo check -p agentic-workbench-server --locked
```

13개 HTTP/수명 회귀는 실제 ephemeral TCP listener와 HTTP client를 사용한다. 초기화/notification/schema/구조화 결과, 업무·protocol 오류, 인증·즉시 폐기·중복 헤더, 모든 method의 Origin/Host, 버전·body 제한, factory 복구, 동시 principal/host 격리, admission과 inflight 보존, 불완전 body shutdown 및 Drop listener 해제를 확인한다. 이 결과는 Box native 화면이나 실제 설치 agent의 문서 MCP 사용 증거를 대신하지 않는다.

## gh-49 검증 결과 (2026-10-13)

- 공용 crate 실제 HTTP/수명 테스트 13개 통과. 모든 target과 echo 예제 compile, clippy `-D warnings` 통과.
- 실제 echo executable에서 initialize → initialized → tools/list → tools/call을 호출하고 credential이 없는 표준 출력 및 SIGINT 정상 종료(exit 0)를 확인했다.
- 기존 `workbench-host` library 테스트 51개 통과. 독립 `agentic-workbench-server` compile 및 workspace fmt 통과. 서버 check의 기존 workbench-core unused import 경고는 이 작업에서 변경하지 않은 소스에 있다.
- Cargo.lock의 기존 package identity는 모두 유지하며 13개 identity를 추가했다. 기존 runtime MCP/ACP 도구와 shared UI 소스는 변경하지 않았다.
- 검증은 incremental/debug info 생성을 줄여 기존 Box vendor target을 재사용했다. 실행 중인 사용자의 앱/server를 종료하지 않았으며 예제 검증이 소유한 process만 정상 종료했다.

로그: `/tmp/box-md-gh49-{http-tests,clippy,example-smoke,aw-host-tests,aw-server-check,aw-fmt,aw-local-check}.log`. HTTP fixture와 CLI smoke는 실제 설치된 ACP provider 검증이 아니다.
