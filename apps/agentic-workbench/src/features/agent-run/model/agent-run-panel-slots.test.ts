import { describe, expect, it } from "vitest";

import {
  MAIN_AGENT_RUN_PANEL_ID,
  addExtraPanel,
  cancelClosePanel,
  confirmClosePanel,
  createInitialAgentRunAreaState,
  getRunningPanelCount,
  removeClosedPanel,
  requestClosePanel,
  routePromptToActivePanel,
  selectPanel,
  updatePanelRunState,
} from "./agent-run-panel-slots";

describe("agent run panel slots", () => {
  it("creates a required non-running main slot", () => {
    const state = createInitialAgentRunAreaState();

    expect(state.activePanelId).toBe(MAIN_AGENT_RUN_PANEL_ID);
    expect(state.slots).toEqual([
      expect.objectContaining({
        id: MAIN_AGENT_RUN_PANEL_ID,
        kind: "main",
        title: "Main",
        isRunning: false,
        activeRunId: null,
        closeState: "open",
      }),
    ]);
  });

  it("does not close the main panel", () => {
    const state = requestClosePanel(createInitialAgentRunAreaState(), MAIN_AGENT_RUN_PANEL_ID);

    expect(state.slots).toHaveLength(1);
    expect(state.activePanelId).toBe(MAIN_AGENT_RUN_PANEL_ID);
  });

  it("adds extra panels with unique sequence titles and activates the new slot", () => {
    const state = addExtraPanel(addExtraPanel(createInitialAgentRunAreaState()));

    expect(state.slots.map((slot) => slot.id)).toEqual([
      MAIN_AGENT_RUN_PANEL_ID,
      "extra-agent-run-1",
      "extra-agent-run-2",
    ]);
    expect(state.slots.map((slot) => slot.title)).toEqual(["Main", "Extra 1", "Extra 2"]);
    expect(state.activePanelId).toBe("extra-agent-run-2");
  });

  it("removes idle extras and falls back to another extra before main", () => {
    const state = addExtraPanel(addExtraPanel(createInitialAgentRunAreaState()));
    const selected = selectPanel(state, "extra-agent-run-2");
    const next = requestClosePanel(selected, "extra-agent-run-2");

    expect(next.slots.map((slot) => slot.id)).toEqual([
      MAIN_AGENT_RUN_PANEL_ID,
      "extra-agent-run-1",
    ]);
    expect(next.activePanelId).toBe("extra-agent-run-1");
  });

  it("tracks running state by panel and ignores unknown panels", () => {
    const state = addExtraPanel(createInitialAgentRunAreaState());
    const running = updatePanelRunState(state, {
      panelId: "extra-agent-run-1",
      isRunning: true,
      activeRunId: "run-extra",
    });
    const ignored = updatePanelRunState(running, {
      panelId: "missing",
      isRunning: true,
      activeRunId: "run-missing",
    });

    expect(ignored.slots.find((slot) => slot.id === "extra-agent-run-1")).toEqual(
      expect.objectContaining({ isRunning: true, activeRunId: "run-extra" }),
    );
    expect(getRunningPanelCount(ignored)).toBe(1);
  });

  it("keeps identical run state reports as no-op updates", () => {
    const state = updatePanelRunState(addExtraPanel(createInitialAgentRunAreaState()), {
      panelId: "extra-agent-run-1",
      isRunning: true,
      activeRunId: "run-extra",
    });
    const repeated = updatePanelRunState(state, {
      panelId: "extra-agent-run-1",
      isRunning: true,
      activeRunId: "run-extra",
    });

    expect(repeated).toBe(state);
  });

  it("routes trimmed prompts to the active open panel", () => {
    const state = addExtraPanel(createInitialAgentRunAreaState());
    const result = routePromptToActivePanel(state, "  review this  ", "request-1");

    expect(result.routed).toBe(true);
    if (!result.routed) {
      return;
    }
    expect(result.target).toEqual({ id: "extra-agent-run-1", title: "Extra 1" });
    expect(result.state.slots[1].externalPromptRequest).toEqual({
      id: "request-1",
      text: "review this",
    });
  });

  it("rejects empty prompts and prompts routed to closing panels", () => {
    const state = addExtraPanel(createInitialAgentRunAreaState());
    expect(routePromptToActivePanel(state, "   ", "ignored")).toEqual({
      routed: false,
      state,
      reason: "empty",
    });

    const running = updatePanelRunState(state, {
      panelId: "extra-agent-run-1",
      isRunning: true,
      activeRunId: "run-extra",
    });
    const confirming = requestClosePanel(running, "extra-agent-run-1");
    const { state: closing } = confirmClosePanel(confirming, "extra-agent-run-1");

    expect(routePromptToActivePanel(closing, "late prompt", "late")).toEqual({
      routed: false,
      state: closing,
      reason: "closing-target",
    });
  });

  it("confirms running extra close once and removes the closed slot idempotently", () => {
    const state = updatePanelRunState(addExtraPanel(createInitialAgentRunAreaState()), {
      panelId: "extra-agent-run-1",
      isRunning: true,
      activeRunId: "run-extra",
    });
    const confirming = requestClosePanel(state, "extra-agent-run-1");
    const first = confirmClosePanel(confirming, "extra-agent-run-1");
    const second = confirmClosePanel(first.state, "extra-agent-run-1");
    const removed = removeClosedPanel(first.state, "extra-agent-run-1");
    const removedAgain = removeClosedPanel(removed, "extra-agent-run-1");

    expect(first.activeRunId).toBe("run-extra");
    expect(second.activeRunId).toBeNull();
    expect(removed.slots.map((slot) => slot.id)).toEqual([MAIN_AGENT_RUN_PANEL_ID]);
    expect(removedAgain).toEqual(removed);
  });

  it("cancels close confirmation without interrupting running state", () => {
    const state = updatePanelRunState(addExtraPanel(createInitialAgentRunAreaState()), {
      panelId: "extra-agent-run-1",
      isRunning: true,
      activeRunId: "run-extra",
    });
    const confirming = requestClosePanel(state, "extra-agent-run-1");
    const cancelled = cancelClosePanel(confirming, "extra-agent-run-1");

    expect(cancelled.slots[1]).toEqual(
      expect.objectContaining({
        closeState: "open",
        isRunning: true,
        activeRunId: "run-extra",
      }),
    );
  });
});
