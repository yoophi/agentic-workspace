# Quickstart: Queue Prompt Order

## Prerequisites

- 의존성이 설치되어 있어야 한다.
- `apps/agentic-workbench`가 type check와 test를 실행할 수 있어야 한다.
- 테스트 가능한 agent profile이 하나 이상 활성화되어 있어야 한다.

## Static Validation

```bash
pnpm --dir apps/agentic-workbench check-types
pnpm --dir apps/agentic-workbench test -- run-panel-state
```

Expected outcome:

- type check가 통과한다.
- `run-panel-state` 테스트가 첫 실행 prompt queue 등록, 중복 user message 방지, 실패/취소 상태 복구, multi-prompt 순서 보존을 검증한다.

## Manual UI Scenario: First Run Prompt

1. Worktree Session Page를 새로 연다.
2. agent run이 idle 상태인지 확인한다.
3. prompt textarea에 `첫 실행 순서 확인`을 입력하고 Return을 누른다.
4. agent-run 영역에서 입력 prompt가 queued prompt 형식으로 먼저 보이는지 확인한다.
5. agent 실행/준비 메시지가 표시되는지 확인한다.
6. agent가 prompt를 처리한 뒤 출력이 queued/executed prompt 이후에 표시되는지 확인한다.

Expected outcome:

- prompt가 agent 준비 전에 executed user message처럼 표시되지 않는다.
- 같은 prompt가 timeline에 중복 표시되지 않는다.
- output은 prompt 실행 이후에 표시된다.

## Manual UI Scenario: Fast Startup

1. 응답이 빠른 agent profile을 선택한다.
2. 첫 prompt를 입력하고 Return을 누른다.
3. timeline을 확인한다.

Expected outcome:

- startup이 빠르더라도 prompt가 먼저 completed user message처럼 보이지 않는다.
- visible order는 queue 표시, agent 실행/준비, prompt 실행/출력으로 유지된다.

## Manual UI Scenario: Multiple Prompts Before Completion

1. 첫 prompt를 입력하고 Return을 누른다.
2. 첫 prompt가 완료되기 전에 두 번째 prompt를 입력해 queue에 추가한다.
3. queue 순서와 출력 순서를 확인한다.

Expected outcome:

- 두 prompt는 제출 순서대로 queue에 표시된다.
- 사용자가 queue 순서를 직접 바꾸지 않았다면 실행과 출력도 제출 순서를 따른다.

## Manual UI Scenario: Start Failure

1. 실패하도록 설정된 agent command/profile을 선택한다.
2. 첫 prompt를 제출한다.
3. 실패 상태를 확인한다.

Expected outcome:

- error가 표시된다.
- 첫 prompt는 성공 실행 상태로 남지 않는다.
- prompt text는 재시도 가능한 형태로 복구되거나 실패 상태로 명확히 표시된다.

## Storybook Check

구현에서 queued first-run 상태를 reusable UI state로 노출한다면 `apps/agentic-workbench/src/stories/organisms.stories.tsx`에 AgentRun first-run queued 상태를 추가한다.

Expected outcome:

- Storybook에서 queued prompt, agent lifecycle, prompt output 순서를 정적으로 검토할 수 있다.

## Verification Results

- 2026-07-06: `pnpm --dir apps/agentic-workbench test -- run-panel-state` passed. The suite covered queued first-run prompt creation, duplicate prevention, lifecycle activation, output ordering, blank prompt rejection, and queue dispatch gating.
- 2026-07-06: `pnpm --dir apps/agentic-workbench check-types` passed.
- 2026-07-06: `pnpm --dir apps/agentic-workbench build-storybook` passed with the new `AgentRunQueuedFirstPrompt` story compiled successfully.
- 2026-07-06: Manual scenarios were reviewed against the Storybook mock path and implementation code. Runtime app validation with a real agent process was not executed in this session.
