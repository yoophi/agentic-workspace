import { describe, expect, it } from "vitest";

import {
  AgentRunControllerRegistry,
  applyLiveRuntimeEvent,
  applyRuntimeSnapshot,
  createAgentRunControllerState,
} from "./agent-run-controller";

describe("agent run controller", () => {
  it("replays unseen runtime events and requests durable rehydrate on a gap", () => {
    const state = applyRuntimeSnapshot(createAgentRunControllerState("run-1"), {
      runId: "run-1",
      events: [
        { runId: "run-1", sequence: 4, event: { kind: "message" }, terminal: false },
      ],
      lastSequence: 4,
      terminal: false,
      gapDetected: true,
    });
    expect(state.lastSequence).toBe(4);
    expect(state.events).toEqual([{ kind: "message" }]);
    expect(state.hydrationStatus).toBe("gap");
  });

  it("deduplicates replay and live events by authoritative run sequence", () => {
    const hydrated = applyRuntimeSnapshot(createAgentRunControllerState("run-1"), {
      runId: "run-1",
      events: [
        { runId: "run-1", sequence: 1, event: { text: "one" }, terminal: false },
        { runId: "run-1", sequence: 2, event: { text: "two" }, terminal: false },
      ],
      lastSequence: 2,
      terminal: false,
      gapDetected: false,
    });
    const duplicate = applyLiveRuntimeEvent(hydrated, {
      runId: "run-1",
      sequence: 2,
      event: { text: "duplicate" },
      terminal: false,
    });
    const next = applyLiveRuntimeEvent(duplicate, {
      runId: "run-1",
      sequence: 3,
      event: { text: "three" },
      terminal: false,
    });
    expect(next.events).toEqual([
      { text: "one" },
      { text: "two" },
      { text: "three" },
    ]);
  });

  it("keeps one controller for the same child run", () => {
    const registry = new AgentRunControllerRegistry();
    const background = registry.getOrCreate("child-1", "run-1");
    const panel = registry.getOrCreate("child-1", "run-1");
    expect(panel).toBe(background);
    expect(registry.size).toBe(1);
  });
});
