# Contract: `Workbench.call` v1 (037 부분집합)

**Spec**: [../spec.md](../spec.md) · **Data model**: [../data-model.md](../data-model.md)

이 문서는 037에서 세 Adapter(Tauri compat, in-memory, 테스트 HTTP)가 동일하게 지켜야 하는 계약이다. 정본 [서버-클라이언트 전환 조사 §HTTP와 WebSocket Interface 제안](../../../docs/client-server-architecture-research.md#http와-websocket-interface-제안)의 부분집합이며, 기계 계약의 원본은 `crates/workbench-protocol/openapi/workbench.openapi.json`이다.

## 1. 호출

```text
call(principal: AuthenticatedPrincipal, request: CallRequest) -> Result<CallReply, WorkbenchFault>
events(principal, subscription) -> Fault{ code: unsupportedSchema, message: "events는 2단계에서 제공" }   -- 037
```

### CallRequest 예시

```json
{
  "protocolVersion": 1,
  "operation": "project.create",
  "requestId": "req_7f3e…",
  "idempotencyKey": "idem_2a9c…",
  "input": { "name": "AW", "workingDirectory": "/Users/me/aw", "description": null }
}
```

### 성공 응답

```json
{ "kind": "complete", "output": { "id": "project-1758…", "name": "AW", "workingDirectory": "/Users/me/aw", "description": null }, "revision": 7 }
```

`revision`은 command에서만 채운다. query는 `null`.

### 실패 응답

```json
{ "code": "invalidArgument", "message": "Project name is required.", "retryable": false, "outcome": "notApplied", "requestId": "req_7f3e…", "details": { "path": "/name" } }
```

## 2. operation 세 개

### `project.list` (query, `project:read`)

- input: `{}` (추가 필드는 `invalidArgument`)
- output: `Project[]` — 저장 순서 그대로(기존 동작 유지)
- 오류: `unavailable`(저장 파일 읽기 실패; 기존 json_store 복구 실패 메시지를 message에)

### `project.create` (command, `project:write`, 멱등성 키 필수)

- input: `{ name: string, workingDirectory: string, description?: string|null }`
- 정규화: `name`·`workingDirectory` trim, `description` trim 후 빈 문자열 → null
- output: `Project`
- 오류:
  - `invalidArgument` — "Project name is required." / "Working directory is required." / 멱등성 키 누락 "idempotencyKey is required for project.create."
  - `conflict` — 같은 키·다른 지문, 진행 중(`pending`), `unknown`(표는 data-model §3)
  - `preconditionFailed` — `expectedRevision` 불일치, `details.currentRevision`
  - `unavailable` — ledger 잠김/열기 실패, JSON 저장 실패(이 경우 ledger는 `failed`)

### `system.describe` (query, `system:describe`)

- input: `{}`
- output: `{ protocolVersion: 1, operations: OperationDescriptor[] }` — principal의 scope로 필터한 결과만. `project:write`가 없으면 `project.create`가 목록에 없다.
- 각 descriptor는 `inputSchema`·`outputSchema`를 JSON Schema(OpenAPI 3.1 호환)로 포함한다.

## 3. 공통 규칙

1. `protocolVersion != 1` → `unsupportedProtocol`(HTTP 409).
2. 존재하지 않는 `operation` → `notFound`. 존재하지만 principal의 scope에 없는 operation → `forbidden`. 두 경우 모두 `system.describe` 목록에는 나타나지 않는다(정본 Invariant 2: "catalog에 보이지 않는 operation은 호출도 거절"). 존재 여부와 권한 여부를 코드로 구분하는 것은 의도된 노출이다.
3. 입력은 스키마 검증 뒤에만 handler로 간다. 출력은 debug 빌드와 테스트에서 스키마 검증한다.
4. `requestId`와 `idempotencyKey`는 별개 필드다. 같은 값을 넣어도 동작하지만 계약 위반으로 문서화한다.
5. query는 ledger를 거치지 않지만 **aggregate lock은 잡는다**. 저장 파일의 `.bak` 복구는 이 lock 안에서만 일어나므로, 읽기가 진행 중인 변경을 덮어쓰지 않는다(data-model §3, research R14).
6. Fault의 `message`는 사람이 읽는 한 문장이며 코드나 스택을 포함하지 않는다.

## 4. 테스트 HTTP 경로 (`crates/workbench-core/tests/support/http_harness.rs`)

운영 빌드에 없다. 계약 직렬화와 loopback 왕복만 검증한다.

| 항목 | 값 |
|---|---|
| bind | `127.0.0.1:0` (테스트가 포트를 읽음) |
| route | `POST /v1/calls` |
| 요청 | `Content-Type: application/json`, body = CallRequest |
| 인증 | `Authorization: Bearer test-desktop`(모든 scope) / `Bearer test-readonly`(`project:read`, `system:describe`) / 없음·기타 → 401 `unauthenticated` |
| 성공 | `200`, body = CallReply |
| 실패 | 정본 코드 표의 HTTP status, `Content-Type: application/problem+json`, body = WorkbenchFault + `type`/`title`/`status` |

## 5. 세 Adapter 동일성 fixture

`crates/workbench-protocol/fixtures/<scenario>.json`:

```json
{
  "name": "project-create-name-required",
  "principal": "desktop",
  "seed": { "projects": [] },
  "request": { "protocolVersion": 1, "operation": "project.create", "requestId": "r1", "idempotencyKey": "k1", "input": { "name": "  ", "workingDirectory": "/tmp/x" } },
  "expect": { "fault": { "code": "invalidArgument", "message": "Project name is required.", "outcome": "notApplied" } }
}
```

최소 시나리오 목록은 research R10. in-memory와 HTTP suite는 파일을 그대로 소비하고, Tauri compat 유닛 테스트는 `request.input`을 `ProjectInput`으로, `expect`를 `Result<_, String>`으로 변환해 같은 파일을 소비한다.
