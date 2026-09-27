# Data Model: Workbench HTTP/WebSocket 어댑터 (042)

모두 메모리 상태다. 새 영속 저장은 없다.

## Endpoint

| 필드 | 설명 |
|---|---|
| `baseUrl` | `http://127.0.0.1:<port>` |
| `instanceId` | 프로세스 기동 때 uuid(handshake) |

앱 수명 동안 하나. bind 실패면 없음(앱은 기존 경로로 계속).

## DesktopToken

| 필드 | 규칙 |
|---|---|
| 값 | 256비트 무작위, URL-safe base64. 원문은 발급 응답에만 있고 저장은 SHA-256 해시 키 |
| `principal` | `desktop` |
| `boundOrigin` | 발급을 요청한 WebView 출처. 진단 토큰은 "Origin 없음" |
| `clientInstanceId` | 발급 때 uuid |
| `expiresAt` | 발급 + 15분(진단 토큰 10분) |

상태: 발급됨 → 만료(해석 불가). 상한 256개, 발급 때 만료분 정리.

## EventTicket

| 필드 | 규칙 |
|---|---|
| 값 | 256비트 무작위 |
| `principal` | 발급 요청의 principal |
| `cursors` | 발급 요청의 cursor(1–64개) |
| `origin` | 발급 요청의 Origin(없으면 없음) |
| `expiresAt` | 발급 + 30초 |

상태: 발급됨 → (연결 때 원자적 take) 소모됨 | 만료. 소모·만료된 표는 다시 쓸 수 없다. 상한 1,024.

## HandshakeResult

`selectedProtocolVersion`, `supportedProtocolVersions`, `serverVersion`(CALVER, 빌드 버전), `apiMajor`(1), `contractHash`(OpenAPI JSON SHA-256), `instanceId`, `serverEpoch`, `storageSchemaVersion`(ledger schema), `features`([]).

## OriginPolicy

허용 출처 문자열 목록(정확 일치). 조립이 준다: `http://localhost:1420`(dev), `tauri://localhost`(macOS·Linux), `http://tauri.localhost`(Windows). HTTP router와 MCP 서버가 같은 정책 구현을 쓴다.
