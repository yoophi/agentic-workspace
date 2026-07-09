# Contract: Tauri Commands and Backend Ports

## Scope

이 계약은 AW frontend가 backend에 요청할 steer 관련 command와 backend application/domain/infrastructure 경계를 정의한다.

## Command: `steer_prompt_to_run`

### Request

```ts
type SteerPromptToRunRequest = {
  runId: string;
  prompt: string;
};
```

### Success

```ts
type SteerPromptToRunResult = {
  status: "accepted";
  runId: string;
};
```

### Errors

```ts
type SteerPromptToRunError =
  | { code: "emptyPrompt"; message: string }
  | { code: "runNotActive"; message: string }
  | { code: "unsupported"; message: string }
  | { code: "dispatchFailed"; message: string };
```

## Backend Port Expectations

`SessionHandle`는 다음 의미를 제공해야 한다.

- `send_prompt`: 기존 prompt dispatch. 응답 중 동시 dispatch를 거절할 수 있다.
- `steer_prompt`: 현재 active turn에 입력을 주입한다. 지원하지 않으면 unsupported error를 반환한다.
- `set_permission_mode`: 기존 permission mode 변경.

## Lifecycle Events

새 lifecycle status가 추가되는 경우 frontend model과 Rust/TypeScript event types가 함께 갱신되어야 한다.

- `steerPending`: backend가 steer 요청을 받았고 provider dispatch를 시도 중이다.
- `steerAccepted`: provider가 현재 작업에 steer를 수락했다.
- `steerRejected`: provider가 현재 작업에 steer를 수락하지 않았다. 입력은 frontend에서 보존된다.

## Safety Rules

- `steer_prompt_to_run`은 `cancel_run`을 호출하면 안 된다.
- run이 active registry에 없으면 `runNotActive`를 반환한다.
- provider가 active-turn steer를 지원하지 않으면 `unsupported`를 반환한다.
- backend는 frontend가 보낸 prompt를 trim하고 빈 입력을 거절한다.
- infrastructure adapter는 provider-specific protocol 실패를 `dispatchFailed`로 변환한다.
