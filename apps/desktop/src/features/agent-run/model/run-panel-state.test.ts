import { describe, expect, it } from "vitest";

import {
  addUserMessage,
  applyRunEvent,
  moveQueuedPrompt,
  removeUserMessage,
  updateQueuedPrompt,
} from "./run-panel-state";
import type { RunEventState } from "./run-panel-state";

function runningState(overrides: Partial<RunEventState> = {}): RunEventState {
  return {
    items: [],
    usageContext: null,
    isAwaitingPromptResponse: true,
    isRunning: true,
    activeRunId: "run-active",
    queuedPrompts: [],
    ...overrides,
  };
}

describe("run panel state", () => {
  it("ignores events from inactive runs", () => {
    const state = runningState();

    const nextState = applyRunEvent(state, {
      runId: "run-other",
      event: { type: "usage", used: 50, size: 100 },
    });

    expect(nextState).toBe(state);
    expect(nextState.usageContext).toBeNull();
    expect(nextState.items).toHaveLength(0);
  });

  it("keeps usage events out of the timeline for the active run", () => {
    const nextState = applyRunEvent(runningState(), {
      runId: "run-active",
      event: { type: "usage", used: 25, size: 100 },
    });

    expect(nextState.usageContext).toEqual({ used: 25, size: 100 });
    expect(nextState.items).toHaveLength(0);
  });

  it("clears queue and active run state on terminal lifecycle events", () => {
    const nextState = applyRunEvent(
      runningState({
        isAwaitingPromptResponse: true,
        queuedPrompts: [{ id: "queued-a", text: "next prompt" }],
      }),
      {
        runId: "run-active",
        event: { type: "lifecycle", status: "completed", message: "done" },
      },
    );

    expect(nextState.isRunning).toBe(false);
    expect(nextState.isAwaitingPromptResponse).toBe(false);
    expect(nextState.activeRunId).toBeNull();
    expect(nextState.queuedPrompts).toEqual([]);
    expect(nextState.items[nextState.items.length - 1]?.event).toMatchObject({
      type: "lifecycle",
      status: "completed",
    });
  });

  it("moves queued prompts without mutating the original queue", () => {
    const queue = [
      { id: "a", text: "first" },
      { id: "b", text: "second" },
      { id: "c", text: "third" },
    ];

    const nextQueue = moveQueuedPrompt(queue, 2, 0);

    expect(nextQueue.map((item) => item.id)).toEqual(["c", "a", "b"]);
    expect(queue.map((item) => item.id)).toEqual(["a", "b", "c"]);
  });

  it("reports whether queued prompt editing found its target", () => {
    const queue = [{ id: "a", text: "before" }];

    expect(updateQueuedPrompt(queue, "a", "after")).toEqual({
      queue: [{ id: "a", text: "after" }],
      updated: true,
    });
    expect(updateQueuedPrompt(queue, "missing", "after")).toEqual({
      queue,
      updated: false,
    });
  });

  it("can remove a synthetic user message when run start fails", () => {
    const withUserMessage = addUserMessage([], "run-active", "start prompt");
    const nextItems = removeUserMessage(withUserMessage, "run-active", "start prompt");

    expect(withUserMessage).toHaveLength(1);
    expect(nextItems).toEqual([]);
  });
});
