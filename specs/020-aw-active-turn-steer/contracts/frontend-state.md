# Contract: Frontend Prompt Dispatch State

## Scope

이 계약은 `apps/agentic-workbench/src/features/agent-run/model/run-panel-state.ts`가 제공해야 하는 순수 상태 전이와 `AgentRunPanel`이 기대하는 사용자-visible 상태를 정의한다.

## State Shape

```ts
type PromptDispatchState = {
  activeRunId: string | null;
  phase:
    | "idle"
    | "starting"
    | "awaitingPrompt"
    | "responding"
    | "steering"
    | "cancelling"
    | "restarting";
  activePrompt: string | null;
  queuedPrompts: QueuedPrompt[];
  pendingSteers: SteerInput[];
  rejectedSteers: SteerInput[];
  transitionToken: string | null;
  suppressQueueAutosend: boolean;
};
```

## Required Events

- `runStarted(runId, prompt, transitionToken)`
- `promptSent(runId)`
- `promptCompleted(runId)`
- `runTerminal(runId, status, transitionToken?)`
- `queueAdded(prompt)`
- `queueMoved(fromIndex, toIndex)`
- `queueRemoved(promptId)`
- `steerSubmitted(input)`
- `steerAccepted(inputId)`
- `steerRejected(inputId, reason)`
- `steerQueuedFallback(inputId)`
- `restartWithSteeringRequested(inputId)`
- `restartWithSteeringFailed(inputId, reason)`

## Invariants

- inactive `runId` event는 state를 변경하지 않는다.
- terminal event는 같은 active `runId`에만 적용된다.
- restart transition 중 old run terminal event는 새 run의 `queuedPrompts`, `pendingSteers`, `activeRunId`를 지우지 않는다.
- `pendingSteers`와 `queuedPrompts`는 같은 입력을 동시에 소유하지 않는다.
- `queuedPrompts` 자동 실행은 `pendingSteers` 처리 중에는 suppress된다.
- `steerRejected`는 입력을 `rejectedSteers`로 이동시키고 원문을 보존한다.

## UI Contract

- pending steer는 queued prompt와 다른 label/section으로 표시한다.
- rejected steer는 사용자가 queue 이동, restart-with-steering, 삭제 중 하나를 선택할 수 있어야 한다.
- restart-with-steering은 active-turn steer와 다른 command로 표현한다.
- queue item의 "즉시 전송" 버튼은 provider capability에 따라 active-turn steer 또는 fallback 선택으로 이어져야 한다.
