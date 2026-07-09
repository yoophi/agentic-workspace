# Quickstart: AW Active-Turn Steer Validation

## Prerequisites

- Node/pnpm workspace dependencies installed
- Rust toolchain available for Tauri checks
- Branch: `020-aw-active-turn-steer`

## Static Validation

```bash
pnpm --filter @yoophi/agentic-workbench check-types
pnpm --filter @yoophi/agentic-workbench test
cd apps/agentic-workbench/src-tauri && cargo test
```

Expected outcome:

- frontend type checks pass
- `run-panel-state` tests cover queue, pending steer, rejected steer, restart fallback, and late event isolation
- Rust use case tests cover empty prompt, inactive run, unsupported steer, accepted steer, and dispatch failure

## Manual Scenario 1: active-turn steer contract

1. Run the Rust `SteerPromptUseCase` tests with accepted and unsupported fake sessions.
2. Confirm accepted fake sessions emit `steerAccepted` without calling cancel.
3. Confirm the current ACP adapter emits an unsupported path instead of cancelling the run.

Expected outcome:

- accepted capability path keeps the same run active
- current ACP adapter reports unsupported steer without cancelling
- pending/rejected steer state remains associated with the same run timeline

## Manual Scenario 2: unsupported steer fallback

1. Use the current ACP-backed agent/provider state.
2. Start a running task.
3. Submit a steer input.

Expected outcome:

- input text is preserved
- current run is not automatically cancelled
- UI shows rejected steer and offers queue fallback, retry, delete, or explicit restart-with-steering

## Manual Scenario 3: queued prompt converted to steer

1. Start a running task.
2. Add two queued prompts.
3. Choose the first queued prompt's immediate/steer action.

Expected outcome:

- selected prompt leaves queued prompt list
- selected prompt appears as pending steer or rejected steer fallback
- remaining queued prompt order is unchanged

## Manual Scenario 4: late old-run event isolation

1. Trigger restart-with-steering explicitly.
2. Let the old run emit a delayed terminal event after the replacement run starts.

Expected outcome:

- replacement run stays active
- replacement queue and pending steer state are not cleared by the old event
- timeline remains understandable for both old and new run events

## Documentation and Storybook

Add `docs/aw-active-turn-steer.md` with Korean explanation and Mermaid state diagram. If the queue/steer timeline section is extracted into a reusable UI component, add Storybook states in `apps/agentic-workbench/src/stories/organisms.stories.tsx` for:

- empty queue
- queued prompts
- pending steer
- rejected steer
- restart fallback
- long prompt content
