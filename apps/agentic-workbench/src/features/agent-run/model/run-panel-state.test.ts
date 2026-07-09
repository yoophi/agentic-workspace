import { describe, expect, it } from "vitest";

import {
  activateRunStartQueuedPrompt,
  addUserMessage,
  applyRunEvent,
  appendQueuedPrompt,
  appendPendingSteer,
  appendPromptHistory,
  buildSteerPrompt,
  createSteerInput,
  createQueuedPrompt,
  createRunStartQueuedPrompt,
  dequeueRunStartQueuedPrompt,
  hasUserMessage,
  initialPromptHistoryState,
  insertQueuedPrompt,
  isOverrideCommandFailure,
  isPromptHistoryNavigationBoundary,
  moveRejectedSteerToQueue,
  moveQueuedPrompt,
  navigatePromptHistory,
  prepareQueuedPromptSteer,
  rejectPendingSteer,
  removeQueuedPrompt,
  retryRejectedSteer,
  removeUserMessage,
  resolveRequestAgentLaunch,
  resolveSelectedProfileId,
  shouldAutoDispatchQueuedPromptWithSteers,
  shouldAutoDispatchQueuedPrompt,
  updateQueuedPrompt,
} from "./run-panel-state";
import type { RunEventState } from "./run-panel-state";

function runningState(overrides: Partial<RunEventState> = {}): RunEventState {
  return {
    items: [],
    usageContext: null,
    agentThreadStatus: { type: "unknown" },
    sessionUpdatedAt: null,
    availableCommandMetadata: null,
    isAwaitingPromptResponse: true,
    isRunning: true,
    activeRunId: "run-active",
    queuedPrompts: [],
    pendingSteers: [],
    rejectedSteers: [],
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

  it("keeps session info raw payloads out of the timeline for the active run", () => {
    const state = runningState({
      items: addUserMessage([], "run-active", "before"),
    });

    const activeState = applyRunEvent(state, {
      runId: "run-active",
      event: {
        type: "raw",
        method: "session/update",
        payload: {
          sessionUpdate: "session_info_update",
          _meta: { codex: { threadStatus: { type: "active" } } },
        },
      },
    });
    const idleState = applyRunEvent(activeState, {
      runId: "run-active",
      event: {
        type: "raw",
        method: "session/update",
        payload: {
          update: {
            sessionUpdate: "session_info_update",
            _meta: { codex: { threadStatus: { type: "idle" } } },
          },
        },
      },
    });
    const metadataOnlyState = applyRunEvent(idleState, {
      runId: "run-active",
      event: {
        type: "raw",
        method: "session/update",
        payload: {
          sessionUpdate: "session_info_update",
          title: "test",
          updatedAt: "2026-07-02T11:11:12.255Z",
        },
      },
    });

    expect(activeState.items).toHaveLength(2);
    expect(idleState.items).toHaveLength(2);
    expect(metadataOnlyState.items).toHaveLength(2);
    expect(metadataOnlyState.items[0].event).toMatchObject({
      type: "userMessage",
      text: "before",
    });
    expect(metadataOnlyState.items[1]).toMatchObject({
      group: "lifecycle",
      body: "sessionCreated: Agent session started.\nsessionIdle: Ready for the next prompt.",
    });
    expect(metadataOnlyState.items.map((item) => item.group)).not.toContain("raw");
  });

  it("keeps available command updates out of the timeline and stores metadata", () => {
    const nextState = applyRunEvent(runningState(), {
      runId: "run-active",
      event: {
        type: "raw",
        method: "session/update",
        payload: {
          sessionUpdate: "available_commands_update",
          availableCommands: [
            { name: "mcp", description: "List tools.", input: null },
            {
              name: "review",
              description: "Review changes.",
              input: { hint: "optional review instructions" },
            },
          ],
        },
      },
    });

    expect(nextState.items).toHaveLength(0);
    expect(nextState.availableCommandMetadata?.commands).toMatchObject([
      { name: "mcp", description: "List tools.", inputHint: null },
      {
        name: "review",
        description: "Review changes.",
        inputHint: "optional review instructions",
      },
    ]);
  });

  it("suppresses empty and malformed available command updates", () => {
    const emptyState = applyRunEvent(runningState(), {
      runId: "run-active",
      event: {
        type: "raw",
        method: "session/update",
        payload: {
          sessionUpdate: "available_commands_update",
          availableCommands: [],
        },
      },
    });
    const malformedState = applyRunEvent(emptyState, {
      runId: "run-active",
      event: {
        type: "raw",
        method: "session/update",
        payload: {
          sessionUpdate: "available_commands_update",
          availableCommands: [null, { description: "bad" }, { name: "valid" }],
        },
      },
    });

    expect(emptyState.items).toHaveLength(0);
    expect(emptyState.availableCommandMetadata?.commands).toHaveLength(0);
    expect(malformedState.items).toHaveLength(0);
    expect(malformedState.availableCommandMetadata?.commands.map((item) => item.name)).toEqual([
      "valid",
    ]);
  });

  it("keeps non-command raw events in the timeline for the active run", () => {
    const nextState = applyRunEvent(runningState(), {
      runId: "run-active",
      event: {
        type: "raw",
        method: "session/update",
        payload: { sessionUpdate: "other_update" },
      },
    });

    expect(nextState.items).toHaveLength(1);
    expect(nextState.items[0]).toMatchObject({
      group: "raw",
      title: "session/update",
    });
  });

  it("updates agent thread status from active and idle session info updates", () => {
    const activeState = applyRunEvent(runningState(), {
      runId: "run-active",
      event: {
        type: "raw",
        method: "session/update",
        payload: {
          sessionUpdate: "session_info_update",
          _meta: { codex: { threadStatus: { type: "active", activeFlags: [] } } },
        },
      },
    });
    const idleState = applyRunEvent(
      runningState({ agentThreadStatus: { type: "active" }, isAwaitingPromptResponse: true }),
      {
        runId: "run-active",
        event: {
          type: "raw",
          method: "session/update",
          payload: {
            sessionUpdate: "session_info_update",
            _meta: { codex: { threadStatus: { type: "idle" } } },
          },
        },
      },
    );

    expect(activeState.agentThreadStatus).toEqual({ type: "active", activeFlags: [] });
    expect(activeState.items[0]).toMatchObject({
      group: "lifecycle",
      body: "sessionCreated: Agent session started.",
    });
    expect(idleState.agentThreadStatus).toEqual({ type: "idle" });
    expect(idleState.isAwaitingPromptResponse).toBe(false);
    expect(idleState.items[0]).toMatchObject({
      group: "lifecycle",
      body: "sessionIdle: Ready for the next prompt.",
    });
  });

  it("turns typed active sessionInfo events into concise lifecycle status", () => {
    const nextState = applyRunEvent(runningState(), {
      runId: "run-active",
      event: {
        type: "sessionInfo",
        threadStatus: { type: "active" },
        title: "test",
        updatedAt: "2026-07-02T11:11:12.255Z",
      },
    });

    expect(nextState.items).toHaveLength(1);
    expect(nextState.items[0]).toMatchObject({
      group: "lifecycle",
      body: "sessionCreated: Agent session started.",
    });
    expect(nextState.agentThreadStatus).toEqual({ type: "active" });
    expect(nextState.sessionUpdatedAt).toBe("2026-07-02T11:11:12.255Z");
  });

  it("dedupes repeated lifecycle status messages and resets by run scope", () => {
    const activeState = applyRunEvent(runningState(), {
      runId: "run-active",
      event: { type: "sessionInfo", threadStatus: { type: "active" } },
    });
    const repeatedActiveState = applyRunEvent(activeState, {
      runId: "run-active",
      event: { type: "sessionInfo", threadStatus: { type: "active" } },
    });
    const idleState = applyRunEvent(repeatedActiveState, {
      runId: "run-active",
      event: { type: "sessionInfo", threadStatus: { type: "idle" } },
    });
    const repeatedIdleState = applyRunEvent(idleState, {
      runId: "run-active",
      event: { type: "sessionInfo", threadStatus: { type: "idle" } },
    });
    const nextRunState = applyRunEvent(
      runningState({ activeRunId: "run-next" }),
      {
        runId: "run-next",
        event: { type: "sessionInfo", threadStatus: { type: "active" } },
      },
    );

    expect(repeatedActiveState.items).toHaveLength(1);
    expect(repeatedActiveState.items[0].body).toBe("sessionCreated: Agent session started.");
    expect(repeatedIdleState.items).toHaveLength(1);
    expect(repeatedIdleState.items[0].body).toBe(
      "sessionCreated: Agent session started.\nsessionIdle: Ready for the next prompt.",
    );
    expect(nextRunState.items).toHaveLength(1);
    expect(nextRunState.items[0].runId).toBe("run-next");
  });

  it("preserves agent thread status on metadata-only session info updates", () => {
    const nextState = applyRunEvent(
      runningState({ agentThreadStatus: { type: "active" } }),
      {
        runId: "run-active",
        event: {
          type: "raw",
          method: "session/update",
          payload: {
            sessionUpdate: "session_info_update",
            title: "test",
            updatedAt: "2026-07-02T11:11:12.255Z",
          },
        },
      },
    );

    expect(nextState.agentThreadStatus).toEqual({ type: "active" });
    expect(nextState.items).toHaveLength(0);
  });

  it("preserves latest valid session updatedAt on malformed metadata updates", () => {
    const validState = applyRunEvent(runningState(), {
      runId: "run-active",
      event: {
        type: "sessionInfo",
        updatedAt: "2026-07-09T03:20:00.000Z",
      },
    });
    const malformedState = applyRunEvent(validState, {
      runId: "run-active",
      event: {
        type: "sessionInfo",
        updatedAt: "not-a-date",
      },
    });

    expect(validState.sessionUpdatedAt).toBe("2026-07-09T03:20:00.000Z");
    expect(malformedState.sessionUpdatedAt).toBe("2026-07-09T03:20:00.000Z");
  });

  it("keeps status and timeline stable on title-only typed session info updates", () => {
    const nextState = applyRunEvent(
      runningState({
        items: addUserMessage([], "run-active", "before"),
        agentThreadStatus: { type: "idle" },
      }),
      {
        runId: "run-active",
        event: {
          type: "sessionInfo",
          title: "Fix session metadata",
        },
      },
    );

    expect(nextState.items).toHaveLength(1);
    expect(nextState.agentThreadStatus).toEqual({ type: "idle" });
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

  it("activates a queued first-run prompt on promptSent without duplicating it", () => {
    const firstPrompt = createRunStartQueuedPrompt({
      id: "queued-first",
      text: " first prompt ",
    });
    expect(firstPrompt).toEqual({
      id: "queued-first",
      text: "first prompt",
      source: "first-run",
      dispatchAfterRunStart: true,
    });

    const awaitingState = applyRunEvent(
      runningState({
        isAwaitingPromptResponse: false,
        queuedPrompts: [firstPrompt!],
      }),
      {
        runId: "run-active",
        event: { type: "lifecycle", status: "promptSent", message: "sent" },
      },
    );
    const duplicateState = applyRunEvent(awaitingState, {
      runId: "run-active",
      event: { type: "lifecycle", status: "promptSent", message: "sent" },
    });

    expect(awaitingState.queuedPrompts).toEqual([]);
    expect(awaitingState.items.map((item) => item.event.type)).toEqual([
      "lifecycle",
      "userMessage",
    ]);
    expect(hasUserMessage(awaitingState.items, "run-active", "first prompt")).toBe(true);
    expect(
      duplicateState.items.filter(
        (item) => item.event.type === "userMessage" && item.event.text === "first prompt",
      ),
    ).toHaveLength(1);
  });

  it("keeps prompt output after queued first-run prompt activation", () => {
    const firstPrompt = createRunStartQueuedPrompt({
      id: "queued-first",
      text: "first prompt",
    });
    const afterPromptSent = applyRunEvent(
      runningState({
        isAwaitingPromptResponse: false,
        queuedPrompts: [firstPrompt!],
      }),
      {
        runId: "run-active",
        event: { type: "lifecycle", status: "promptSent", message: "sent" },
      },
    );
    const withOutput = applyRunEvent(afterPromptSent, {
      runId: "run-active",
      event: { type: "agentMessage", text: "answer" },
    });

    expect(withOutput.items.map((item) => item.event.type)).toEqual([
      "lifecycle",
      "userMessage",
      "agentMessage",
    ]);
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

  it("rejects blank queued prompts and appends nonblank prompts without duplicate ids", () => {
    const blankPrompt = createQueuedPrompt({ id: "blank", text: " \n " });
    const queuedPrompt = createQueuedPrompt({ id: "a", text: " first " });

    expect(blankPrompt).toBeNull();
    expect(queuedPrompt).toEqual({
      id: "a",
      text: "first",
      source: "manual-queue",
    });
    expect(appendQueuedPrompt([], blankPrompt)).toEqual([]);
    expect(appendQueuedPrompt([queuedPrompt!], queuedPrompt)).toEqual([queuedPrompt]);
  });

  it("does not auto-dispatch queued prompts that wait for run startup", () => {
    const firstPrompt = createRunStartQueuedPrompt({
      id: "queued-first",
      text: "first prompt",
    });
    const manualPrompt = createQueuedPrompt({
      id: "queued-manual",
      text: "manual prompt",
    });

    expect(shouldAutoDispatchQueuedPrompt([])).toBe(false);
    expect(shouldAutoDispatchQueuedPrompt([firstPrompt!])).toBe(false);
    expect(shouldAutoDispatchQueuedPrompt([manualPrompt!])).toBe(true);
  });

  it("suppresses queued prompt auto-dispatch while pending steers exist", () => {
    const queuedPrompt = createQueuedPrompt({
      id: "queued-a",
      text: "next prompt",
    });
    const steerInput = createSteerInput({
      id: "steer-a",
      targetRunId: "run-active",
      text: "change course",
      createdAtSequence: 1,
    });

    expect(
      shouldAutoDispatchQueuedPromptWithSteers({
        queue: [queuedPrompt!],
        pendingSteers: [steerInput!],
      }),
    ).toBe(false);
    expect(
      shouldAutoDispatchQueuedPromptWithSteers({
        queue: [queuedPrompt!],
        pendingSteers: [],
      }),
    ).toBe(true);
  });

  it("moves accepted pending steer out without clearing the active run", () => {
    const steerInput = createSteerInput({
      id: "steer-a",
      targetRunId: "run-active",
      text: "focus on tests",
      createdAtSequence: 1,
    });
    const state = runningState({
      pendingSteers: appendPendingSteer([], steerInput),
      isAwaitingPromptResponse: true,
    });

    const nextState = applyRunEvent(state, {
      runId: "run-active",
      event: {
        type: "lifecycle",
        status: "steerAccepted",
        message: "accepted",
      },
    });

    expect(nextState.activeRunId).toBe("run-active");
    expect(nextState.isRunning).toBe(true);
    expect(nextState.pendingSteers).toEqual([]);
    expect(nextState.isAwaitingPromptResponse).toBe(false);
  });

  it("ignores inactive run events without mutating active steer state", () => {
    const steerInput = createSteerInput({
      id: "steer-a",
      targetRunId: "run-active",
      text: "keep going",
      createdAtSequence: 1,
    });
    const state = runningState({
      pendingSteers: [steerInput!],
      queuedPrompts: [{ id: "queued-a", text: "next" }],
    });

    const nextState = applyRunEvent(state, {
      runId: "run-old",
      event: { type: "lifecycle", status: "cancelled", message: "old done" },
    });

    expect(nextState).toBe(state);
    expect(nextState.pendingSteers).toEqual([steerInput]);
    expect(nextState.queuedPrompts).toEqual([{ id: "queued-a", text: "next" }]);
  });

  it("keeps pending, rejected, and queued prompts in separate ownership lists", () => {
    const steerInput = createSteerInput({
      id: "steer-a",
      targetRunId: "run-active",
      text: "do this now",
      createdAtSequence: 1,
    })!;

    const rejected = rejectPendingSteer({
      pendingSteers: [steerInput],
      rejectedSteers: [],
      steerInputId: steerInput.id,
      reason: "unsupported",
    });

    expect(rejected.pendingSteers).toEqual([]);
    expect(rejected.rejectedSteers).toEqual([
      { ...steerInput, status: "rejected", errorMessage: "unsupported" },
    ]);

    const queued = moveRejectedSteerToQueue({
      queue: [],
      rejectedSteers: rejected.rejectedSteers,
      steerInputId: steerInput.id,
    });

    expect(queued.rejectedSteers).toEqual([]);
    expect(queued.queue).toEqual([
      {
        id: "steer-a:queued-fallback",
        text: "do this now",
        source: "steer-fallback",
      },
    ]);
  });

  it("preserves queued prompt order when converting one queued prompt to steer", () => {
    const queue = [
      { id: "a", text: "first" },
      { id: "b", text: "second" },
      { id: "c", text: "third" },
    ];

    const result = prepareQueuedPromptSteer({
      queue,
      queuedPromptId: "b",
      targetRunId: "run-active",
      steerInputId: "steer-b",
      createdAtSequence: 2,
    });

    expect(result.queue.map((item) => item.id)).toEqual(["a", "c"]);
    expect(result.steerInput).toMatchObject({
      id: "steer-b",
      text: "second",
      source: "queued-prompt",
      originalQueueIndex: 1,
    });
    expect(queue.map((item) => item.id)).toEqual(["a", "b", "c"]);
  });

  it("retries rejected steers without losing the original text", () => {
    const rejected = {
      id: "steer-a",
      targetRunId: "run-active",
      text: "retry this",
      status: "rejected" as const,
      source: "manual-input" as const,
      createdAtSequence: 1,
      errorMessage: "unsupported",
    };

    const result = retryRejectedSteer({
      pendingSteers: [],
      rejectedSteers: [rejected],
      steerInputId: rejected.id,
      nextId: "steer-b",
      createdAtSequence: 2,
    });

    expect(result.rejectedSteers).toEqual([]);
    expect(result.pendingSteers).toEqual([
      {
        ...rejected,
        id: "steer-b",
        status: "pending",
        errorMessage: undefined,
        createdAtSequence: 2,
      },
    ]);
  });

  it("dequeues only prompts that are waiting for run startup", () => {
    const firstPrompt = createRunStartQueuedPrompt({
      id: "queued-first",
      text: "first prompt",
    });
    const secondPrompt = createQueuedPrompt({
      id: "queued-second",
      text: "second prompt",
    });
    const queue = [firstPrompt!, secondPrompt!];

    expect(dequeueRunStartQueuedPrompt(queue)).toEqual({
      queue: [secondPrompt],
      queuedPrompt: firstPrompt,
    });
    expect(dequeueRunStartQueuedPrompt([secondPrompt!])).toEqual({
      queue: [secondPrompt],
      queuedPrompt: null,
    });
  });

  it("activates a run-start queued prompt without adding a duplicate user message", () => {
    const firstPrompt = createRunStartQueuedPrompt({
      id: "queued-first",
      text: "first prompt",
    });
    const items = addUserMessage([], "run-active", "first prompt");

    const result = activateRunStartQueuedPrompt({
      queue: [firstPrompt!],
      items,
      runId: "run-active",
    });

    expect(result.queue).toEqual([]);
    expect(result.queuedPrompt).toEqual(firstPrompt);
    expect(result.items).toHaveLength(1);
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
    expect(insertQueuedPrompt(queue, queue[0], 1)).toBe(queue);
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
