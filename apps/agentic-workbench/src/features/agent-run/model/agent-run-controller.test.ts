import { describe, expect, it } from "vitest";

import {
  AgentRunController,
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

  describe("hydration buffering (039)", () => {
    const live = (sequence: number, terminal = false) => ({
      runId: "run-1",
      sequence,
      event: { n: sequence },
      terminal,
    });
    const snapshot = (last: number) => ({
      runId: "run-1",
      events: Array.from({ length: last }, (_, index) => live(index + 1)),
      lastSequence: last,
      terminal: false,
      gapDetected: false,
    });
    const loading = () => {
      const controller = new AgentRunController("node-1", "run-1");
      controller.markLoading();
      return controller;
    };
    const numbers = (controller: AgentRunController) =>
      controller.snapshot.events.map((event) => (event as { n: number }).n);

    it("keeps live events that arrive before the replay snapshot", () => {
      const controller = loading();
      controller.applyLive(live(11));
      controller.applySnapshot(snapshot(10));
      expect(numbers(controller)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
      expect(controller.snapshot.lastSequence).toBe(11);
      expect(controller.snapshot.hydrationStatus).toBe("ready");
      expect(controller.snapshot.pendingLive).toEqual([]);
    });

    it("drains buffered live events in sequence order", () => {
      const controller = loading();
      controller.applyLive(live(12));
      controller.applyLive(live(11));
      controller.applySnapshot(snapshot(10));
      expect(numbers(controller).slice(-2)).toEqual([11, 12]);
      expect(controller.snapshot.hydrationStatus).toBe("ready");
    });

    it("drops buffered events the snapshot already covers", () => {
      const controller = loading();
      controller.applyLive(live(10));
      controller.applyLive(live(10));
      controller.applySnapshot(snapshot(10));
      expect(numbers(controller)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
      expect(controller.snapshot.hydrationStatus).toBe("ready");
    });

    it("marks a gap when the buffer skips a sequence", () => {
      const controller = loading();
      controller.applyLive(live(13));
      controller.applySnapshot(snapshot(10));
      expect(controller.snapshot.lastSequence).toBe(13);
      expect(controller.snapshot.hydrationStatus).toBe("gap");
    });

    it("marks a gap when a ready controller receives a skipped sequence", () => {
      const controller = loading();
      controller.applySnapshot(snapshot(10));
      controller.applyLive(live(12));
      expect(controller.snapshot.lastSequence).toBe(12);
      expect(controller.snapshot.hydrationStatus).toBe("gap");
    });

    it("applies buffered events but stays runtimeLost when hydration fails", () => {
      const controller = loading();
      controller.applyLive(live(1));
      controller.applyLive(live(2, true));
      controller.markRuntimeLost();
      expect(numbers(controller)).toEqual([1, 2]);
      expect(controller.snapshot.terminal).toBe(true);
      expect(controller.snapshot.hydrationStatus).toBe("runtimeLost");
      expect(controller.snapshot.pendingLive).toEqual([]);
    });
  });
});
