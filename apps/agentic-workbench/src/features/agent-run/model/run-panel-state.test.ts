import { describe, expect, it } from "vitest";

import {
  addUserMessage,
  applyRunEvent,
  buildSteerPrompt,
  insertQueuedPrompt,
  moveQueuedPrompt,
  removeQueuedPrompt,
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

  it("tracks prompt response lifecycle without finishing the run", () => {
    const awaitingState = applyRunEvent(runningState({ isAwaitingPromptResponse: false }), {
      runId: "run-active",
      event: { type: "lifecycle", status: "promptSent", message: "sent" },
    });
    const completedState = applyRunEvent(awaitingState, {
      runId: "run-active",
      event: { type: "lifecycle", status: "promptCompleted", message: "done" },
    });

    expect(awaitingState.isRunning).toBe(true);
    expect(awaitingState.isAwaitingPromptResponse).toBe(true);
    expect(completedState.isRunning).toBe(true);
    expect(completedState.isAwaitingPromptResponse).toBe(false);
    expect(completedState.activeRunId).toBe("run-active");
  });

  it("finishes the run and preserves the error event in the timeline", () => {
    const nextState = applyRunEvent(
      runningState({
        queuedPrompts: [{ id: "queued-a", text: "retry after failure" }],
      }),
      {
        runId: "run-active",
        event: { type: "error", message: "agent failed" },
      },
    );

    expect(nextState.isRunning).toBe(false);
    expect(nextState.activeRunId).toBeNull();
    expect(nextState.queuedPrompts).toEqual([]);
    expect(nextState.items[nextState.items.length - 1]?.event).toMatchObject({
      type: "error",
      message: "agent failed",
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

  it("returns the same queue reference for invalid move requests", () => {
    const queue = [
      { id: "a", text: "first" },
      { id: "b", text: "second" },
    ];

    expect(moveQueuedPrompt(queue, 0, 0)).toBe(queue);
    expect(moveQueuedPrompt(queue, -1, 1)).toBe(queue);
    expect(moveQueuedPrompt(queue, 0, 5)).toBe(queue);
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

  it("removes and restores queued prompts by id", () => {
    const queue = [
      { id: "a", text: "first" },
      { id: "b", text: "second" },
      { id: "c", text: "third" },
    ];

    const removed = removeQueuedPrompt(queue, "b");
    expect(removed).toEqual({
      queue: [
        { id: "a", text: "first" },
        { id: "c", text: "third" },
      ],
      queuedPrompt: { id: "b", text: "second" },
      index: 1,
    });
    expect(insertQueuedPrompt(removed.queue, removed.queuedPrompt!, removed.index)).toEqual(queue);
    expect(removeQueuedPrompt(queue, "missing")).toEqual({
      queue,
      queuedPrompt: null,
      index: -1,
    });
  });

  it("can remove a synthetic user message when run start fails", () => {
    const withUserMessage = addUserMessage([], "run-active", "start prompt");
    const nextItems = removeUserMessage(withUserMessage, "run-active", "start prompt");

    expect(withUserMessage).toHaveLength(1);
    expect(nextItems).toEqual([]);
  });

  it("removes only the matching user message for the matching run", () => {
    const items = [
      ...addUserMessage([], "run-active", "start prompt"),
      ...addUserMessage([], "run-other", "start prompt"),
      ...addUserMessage([], "run-active", "different prompt"),
    ];

    const nextItems = removeUserMessage(items, "run-active", "start prompt");

    expect(nextItems).toHaveLength(2);
    expect(nextItems.map((item) => item.runId)).toEqual(["run-other", "run-active"]);
    expect(nextItems.map((item) => item.event)).toEqual([
      expect.objectContaining({ type: "userMessage", text: "start prompt" }),
      expect.objectContaining({ type: "userMessage", text: "different prompt" }),
    ]);
  });

  it("builds a steer prompt with separate original and steering sections", () => {
    expect(buildSteerPrompt(" original task ", " new direction ")).toContain(
      "## Original prompt\noriginal task\n\n## Steering instruction\nnew direction",
    );
  });
});
