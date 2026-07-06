# UI Contract: Agent Run Prompt Order

## Scope

이 계약은 `apps/agentic-workbench` Worktree Session Page의 agent-run 영역에서 첫 프롬프트가 표시되고 실행되는 사용자 관찰 가능 순서를 정의한다.

## Initial Run Submission

### Preconditions

- Worktree Session Page가 열려 있다.
- agent run이 아직 활성 상태가 아니다.
- 사용자가 빈 문자열이 아닌 prompt를 입력했다.
- 현재 설정으로 agent run을 시작할 수 있다.

### User Action

사용자가 prompt textarea에서 Return 또는 Run 버튼으로 prompt를 제출한다.

### Required Visible Order

1. prompt 입력값은 textarea에서 비워진다.
2. agent-run 영역에 입력 prompt가 queued prompt 형식으로 표시된다.
3. agent 실행/준비 lifecycle 메시지가 표시될 수 있다.
4. agent가 prompt를 처리할 준비가 되면 queued prompt는 실행 중 또는 실행된 prompt 상태로 전환된다.
5. prompt에 대한 agent output은 prompt 실행 상태 이후에 표시된다.

### Forbidden States

- 첫 prompt가 agent 준비 lifecycle보다 먼저 completed user message처럼 표시된다.
- 같은 prompt가 queued prompt와 executed user message로 동시에 중복 표시된다.
- agent 시작 실패 후 prompt가 성공적으로 실행된 것처럼 남는다.
- 첫 prompt 이후 입력한 prompt가 첫 prompt보다 먼저 실행된 것처럼 보인다.

## Additional Queued Prompt Submission

### Preconditions

- agent run이 활성 상태다.
- prompt response를 기다리는 중이거나 queue에 이미 대기 prompt가 있다.

### Required Behavior

- 새 prompt는 queue 끝에 추가된다.
- queue 편집, 이동, 삭제 UI는 기존 동작을 유지한다.
- 실행 순서는 사용자가 명시적으로 queue를 이동하지 않는 한 제출 순서를 따른다.

## Failure and Cancellation

### Run Start Failure

- queued first prompt는 성공 실행 상태로 전환되지 않는다.
- 사용자는 실패 원인을 볼 수 있어야 한다.
- prompt text는 재시도할 수 있는 형태로 복구되거나 queue 실패 상태로 이해 가능해야 한다.

### User Cancellation

- 아직 실행되지 않은 queued prompts는 실행 완료처럼 표시되지 않는다.
- active run state는 종료 상태로 정리된다.

## Acceptance Checks

- 첫 실행 성공 경로에서 prompt-specific output은 queued prompt 표시 이후에만 나타난다.
- 첫 실행 실패 경로에서 executed user message가 남지 않는다.
- 빠른 agent startup에서도 queued prompt 상태가 최소한 순서상 먼저 반영된다.
- saved prompt와 external prompt도 첫 실행 시 같은 순서 규칙을 따른다.
