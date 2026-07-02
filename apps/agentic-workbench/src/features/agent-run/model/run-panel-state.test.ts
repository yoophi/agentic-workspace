import { describe, expect, it } from "vitest";

import {
  addUserMessage,
  applyRunEvent,
  appendPromptHistory,
  buildSteerPrompt,
  initialPromptHistoryState,
  insertQueuedPrompt,
  isOverrideCommandFailure,
  isPromptHistoryNavigationBoundary,
  moveQueuedPrompt,
  navigatePromptHistory,
  removeQueuedPrompt,
  removeUserMessage,
  resolveRequestAgentLaunch,
  resolveSelectedProfileId,
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
  it("injects only overridden commands and merged env into run requests", () => {
    const agents = [{ id: "codex", label: "Codex", command: "codex-default" }];

    // legacy agentCommands는 기본 프로필 command로 매핑된다(specs/008 R2).
    expect(
      resolveRequestAgentLaunch({
        profileId: "codex",
        agents,
        overrides: { agentCommands: { codex: "codex-acp" } },
      }),
    ).toEqual({ agentId: "codex", agentCommand: "codex-acp" });

    // catalog 기본 command만 해석되면 request에 command를 싣지 않는다.
    expect(
      resolveRequestAgentLaunch({ profileId: "codex", agents, overrides: {} }),
    ).toEqual({ agentId: "codex" });

    // env는 병합 결과가 실린다(프로필 값 우선).
    expect(
      resolveRequestAgentLaunch({
        profileId: "codex",
        agents,
        overrides: {
          globalEnv: { FOO: "global" },
          profiles: [
            {
              id: "codex",
              name: "Codex",
              agentType: "codex",
              command: null,
              env: { FOO: "profile" },
              enabled: true,
              builtIn: true,
            },
          ],
        },
      }),
    ).toEqual({ agentId: "codex", agentEnv: { FOO: "profile" } });

    // 알 수 없는 프로필은 null.
    expect(
      resolveRequestAgentLaunch({ profileId: "missing", agents, overrides: {} }),
    ).toBeNull();
  });

  it("falls back to the first enabled profile when the stored id is missing or disabled", () => {
    const profiles = [
      {
        id: "codex",
        name: "Codex",
        agentType: "codex" as const,
        command: null,
        env: {},
        enabled: true,
        builtIn: true,
      },
      {
        id: "custom-1",
        name: "Custom",
        agentType: "claude-code" as const,
        command: null,
        env: {},
        enabled: true,
        builtIn: false,
      },
    ];

    expect(resolveSelectedProfileId(profiles, "custom-1")).toBe("custom-1");
    expect(resolveSelectedProfileId(profiles, "disabled-or-missing")).toBe("codex");
    expect(resolveSelectedProfileId([], "anything")).toBe("");
  });

  it("detects override command failures from runner messages", () => {
    expect(isOverrideCommandFailure("failed to spawn ACP agent: codex-acp")).toBe(true);
    expect(isOverrideCommandFailure("agent command cannot be parsed")).toBe(true);
    expect(isOverrideCommandFailure("ordinary provider error")).toBe(false);
  });

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

  it("does not nest generated steer prompts as the original prompt", () => {
    const first = buildSteerPrompt("original task", "first steer");
    const second = buildSteerPrompt(first, "second steer");

    expect(second).toContain(
      "## Original prompt\noriginal task\n\n## Steering instruction\nsecond steer",
    );
    expect(second).not.toContain("## Steering instruction\nfirst steer");
  });

  it("records trimmed prompt history while skipping blank prompts", () => {
    const withFirst = appendPromptHistory(initialPromptHistoryState, " first prompt ");
    const withBlank = appendPromptHistory(withFirst, " \n ");
    const withSecond = appendPromptHistory(withBlank, "second prompt");

    expect(withBlank).toBe(withFirst);
    expect(withSecond.entries).toEqual([
      { text: "first prompt", sequence: 1 },
      { text: "second prompt", sequence: 2 },
    ]);
  });

  it("records duplicate prompt history entries in submission order", () => {
    const withFirst = appendPromptHistory(initialPromptHistoryState, "repeat");
    const withSecond = appendPromptHistory(withFirst, "repeat");

    expect(withSecond.entries).toEqual([
      { text: "repeat", sequence: 1 },
      { text: "repeat", sequence: 2 },
    ]);
  });

  it("loads the newest prompt history entry when navigating previous from idle", () => {
    const state = ["first", "second", "third"].reduce(
      (current, prompt) => appendPromptHistory(current, prompt),
      initialPromptHistoryState,
    );

    const result = navigatePromptHistory({
      state,
      direction: "previous",
      currentInput: "",
      isEditableBoundary: true,
      hasModifierKey: false,
    });

    expect(result).toMatchObject({
      handled: true,
      nextInput: "third",
      nextState: {
        cursor: { status: "viewing", index: 2 },
        preservedDraft: "",
      },
    });
  });

  it("moves backward and forward through prompt history entries", () => {
    const state = ["first", "second", "third"].reduce(
      (current, prompt) => appendPromptHistory(current, prompt),
      initialPromptHistoryState,
    );

    const third = navigatePromptHistory({
      state,
      direction: "previous",
      currentInput: "",
      isEditableBoundary: true,
      hasModifierKey: false,
    });
    const second = navigatePromptHistory({
      state: third.nextState,
      direction: "previous",
      currentInput: third.nextInput,
      isEditableBoundary: true,
      hasModifierKey: false,
    });
    const newer = navigatePromptHistory({
      state: second.nextState,
      direction: "next",
      currentInput: second.nextInput,
      isEditableBoundary: true,
      hasModifierKey: false,
    });

    expect([third.nextInput, second.nextInput, newer.nextInput]).toEqual([
      "third",
      "second",
      "third",
    ]);
  });

  it("keeps oldest history entry on extra previous and restores draft past newest", () => {
    const state = ["first", "second"].reduce(
      (current, prompt) => appendPromptHistory(current, prompt),
      initialPromptHistoryState,
    );

    const second = navigatePromptHistory({
      state,
      direction: "previous",
      currentInput: "draft",
      isEditableBoundary: true,
      hasModifierKey: false,
    });
    const first = navigatePromptHistory({
      state: second.nextState,
      direction: "previous",
      currentInput: second.nextInput,
      isEditableBoundary: true,
      hasModifierKey: false,
    });
    const stillFirst = navigatePromptHistory({
      state: first.nextState,
      direction: "previous",
      currentInput: first.nextInput,
      isEditableBoundary: true,
      hasModifierKey: false,
    });
    const backToSecond = navigatePromptHistory({
      state: stillFirst.nextState,
      direction: "next",
      currentInput: stillFirst.nextInput,
      isEditableBoundary: true,
      hasModifierKey: false,
    });
    const draft = navigatePromptHistory({
      state: backToSecond.nextState,
      direction: "next",
      currentInput: backToSecond.nextInput,
      isEditableBoundary: true,
      hasModifierKey: false,
    });

    expect(stillFirst.nextInput).toBe("first");
    expect(draft.nextInput).toBe("draft");
    expect(draft.nextState.cursor).toEqual({ status: "idle" });
  });

  it("preserves draft when history navigation starts and restores it on exit", () => {
    const state = appendPromptHistory(initialPromptHistoryState, "previous prompt");

    const previous = navigatePromptHistory({
      state,
      direction: "previous",
      currentInput: "current draft",
      isEditableBoundary: true,
      hasModifierKey: false,
    });
    const restored = navigatePromptHistory({
      state: previous.nextState,
      direction: "next",
      currentInput: previous.nextInput,
      isEditableBoundary: true,
      hasModifierKey: false,
    });

    expect(previous.nextState.preservedDraft).toBe("current draft");
    expect(restored.nextInput).toBe("current draft");
    expect(restored.nextState.preservedDraft).toBeNull();
  });

  it("treats edited history text as the next draft when navigation restarts", () => {
    const state = ["first", "second"].reduce(
      (current, prompt) => appendPromptHistory(current, prompt),
      initialPromptHistoryState,
    );

    const second = navigatePromptHistory({
      state,
      direction: "previous",
      currentInput: "",
      isEditableBoundary: true,
      hasModifierKey: false,
    });
    const restarted = navigatePromptHistory({
      state: second.nextState,
      direction: "previous",
      currentInput: "edited second",
      isEditableBoundary: true,
      hasModifierKey: false,
    });
    const restored = navigatePromptHistory({
      state: restarted.nextState,
      direction: "next",
      currentInput: restarted.nextInput,
      isEditableBoundary: true,
      hasModifierKey: false,
    });

    expect(restarted.nextInput).toBe("second");
    expect(restarted.nextState.preservedDraft).toBe("edited second");
    expect(restored.nextInput).toBe("edited second");
  });

  it("detects first-line and last-line prompt history navigation boundaries", () => {
    const value = "first\nsecond\nthird";

    expect(
      isPromptHistoryNavigationBoundary({
        value,
        selectionStart: 2,
        selectionEnd: 2,
        direction: "previous",
      }),
    ).toBe(true);
    expect(
      isPromptHistoryNavigationBoundary({
        value,
        selectionStart: 8,
        selectionEnd: 8,
        direction: "previous",
      }),
    ).toBe(false);
    expect(
      isPromptHistoryNavigationBoundary({
        value,
        selectionStart: value.length,
        selectionEnd: value.length,
        direction: "next",
      }),
    ).toBe(true);
    expect(
      isPromptHistoryNavigationBoundary({
        value,
        selectionStart: 8,
        selectionEnd: 8,
        direction: "next",
      }),
    ).toBe(false);
  });

  it("does not handle prompt history navigation with modifier keys or non-boundaries", () => {
    const state = appendPromptHistory(initialPromptHistoryState, "previous prompt");

    expect(
      navigatePromptHistory({
        state,
        direction: "previous",
        currentInput: "draft",
        isEditableBoundary: true,
        hasModifierKey: true,
      }),
    ).toEqual({ handled: false, nextInput: "draft", nextState: state });
    expect(
      navigatePromptHistory({
        state,
        direction: "previous",
        currentInput: "draft",
        isEditableBoundary: false,
        hasModifierKey: false,
      }),
    ).toEqual({ handled: false, nextInput: "draft", nextState: state });
  });
});
