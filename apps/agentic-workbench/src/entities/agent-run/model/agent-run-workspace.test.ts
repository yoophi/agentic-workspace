import { describe, expect, it } from "vitest";

import {
  MAIN_AGENT_RUN_PANEL_ID,
  addExtraPanel,
  closePanel,
  createInitialAgentRunWorkspaceState,
  openAdjacentPanel,
  routePromptToPanel,
  selectPanel,
  setAgentRunViewMode,
  updatePanelRunState,
} from "./agent-run-workspace";
import { calculateTileLayoutFrames, flattenTilePanelIds } from "./tile-layout";

describe("agent run workspace", () => {
  it("starts in tab mode with a required main panel", () => {
    const state = createInitialAgentRunWorkspaceState();
    expect(state.viewMode).toBe("tabs");
    expect(state.focusedPanelId).toBe(MAIN_AGENT_RUN_PANEL_ID);
    expect(flattenTilePanelIds(state.layout)).toEqual([MAIN_AGENT_RUN_PANEL_ID]);
  });

  it("switches to an equal horizontal tile projection without changing slots", () => {
    const state = addExtraPanel(addExtraPanel(createInitialAgentRunWorkspaceState()));
    const tiles = setAgentRunViewMode(state, "tiles");
    const tabs = setAgentRunViewMode(tiles, "tabs");
    const frames = calculateTileLayoutFrames(tiles.layout).leaves;

    expect(tiles.slots).toBe(state.slots);
    for (const frame of Object.values(frames)) {
      expect(frame.width).toBeCloseTo(1 / 3);
    }
    expect(tabs.slots).toBe(state.slots);
    expect(tabs.layout).toBe(tiles.layout);
    expect(tabs.focusedPanelId).toBe("extra-agent-run-2");
  });

  it("opens adjacent panels in a deterministic split and enforces limits", () => {
    const initial = createInitialAgentRunWorkspaceState();
    const first = openAdjacentPanel(initial, MAIN_AGENT_RUN_PANEL_ID, "right");
    expect(first.opened).toBe(true);
    expect(first.state.viewMode).toBe("tiles");
    expect(first.state.focusedPanelId).toBe("extra-agent-run-1");

    const second = openAdjacentPanel(first.state, "extra-agent-run-1", "below");
    expect(second.opened).toBe(true);
    expect(flattenTilePanelIds(second.state.layout)).toEqual([
      MAIN_AGENT_RUN_PANEL_ID,
      "extra-agent-run-1",
      "extra-agent-run-2",
    ]);

    const limited = openAdjacentPanel(second.state, "extra-agent-run-2", "right", {
      maxPanels: 3,
    });
    expect(limited).toEqual({
      opened: false,
      state: second.state,
      reason: "panel-limit",
    });
  });

  it("keeps run state when focusing panels and collapses layout on close", () => {
    const opened = openAdjacentPanel(
      createInitialAgentRunWorkspaceState(),
      MAIN_AGENT_RUN_PANEL_ID,
      "right",
    ).state;
    const running = updatePanelRunState(opened, {
      panelId: "extra-agent-run-1",
      isRunning: true,
      activeRunId: "run-extra",
    });
    const selected = selectPanel(running, MAIN_AGENT_RUN_PANEL_ID);
    const closed = closePanel(selected, "extra-agent-run-1");

    expect(selected.slots[1]).toMatchObject({ isRunning: true, activeRunId: "run-extra" });
    expect(closed.slots.map((slot) => slot.id)).toEqual([MAIN_AGENT_RUN_PANEL_ID]);
    expect(flattenTilePanelIds(closed.layout)).toEqual([MAIN_AGENT_RUN_PANEL_ID]);
  });

  it("routes delivery requests to an exact open panel without changing focus", () => {
    const state = addExtraPanel(createInitialAgentRunWorkspaceState());
    const focusedMain = selectPanel(state, MAIN_AGENT_RUN_PANEL_ID);
    const result = routePromptToPanel(
      focusedMain,
      "extra-agent-run-1",
      {
        id: "exchange-1",
        text: "  review this  ",
        delivery: "queue",
      },
    );

    expect(result.routed).toBe(true);
    expect(result.state.focusedPanelId).toBe(MAIN_AGENT_RUN_PANEL_ID);
    expect(result.state.slots[1].externalPromptRequest).toEqual({
      id: "exchange-1",
      text: "review this",
      delivery: "queue",
    });
  });
});
