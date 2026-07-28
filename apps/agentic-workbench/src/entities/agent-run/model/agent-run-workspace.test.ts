import { describe, expect, it } from "vitest";

import {
  MAIN_AGENT_RUN_PANEL_ID,
  MAX_AGENT_RUN_PANELS,
  MAX_AGENT_RUN_TILE_DEPTH,
  addExtraPanel,
  closePanel,
  createInitialAgentRunWorkspaceState,
  openAdjacentPanel,
  promoteOrchestrationNode,
  detachOrchestrationPanel,
  routePromptToPanel,
  selectPanel,
  setAgentRunViewMode,
  updatePanelRunState,
} from "./agent-run-workspace";
import {
  calculateTileLayoutFrames,
  flattenTilePanelIds,
  getTileDepth,
} from "./tile-layout";

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

  // FR-045: at the documented limits a further panel or promotion must be refused with a
  // reason, and must leave the existing layout and run state untouched.
  it("refuses a ninth panel at the real 8-panel limit without changing the layout", () => {
    // A balanced split keeps depth within the 4-level bound while reaching 8 panels.
    let state = createInitialAgentRunWorkspaceState();
    let frontier = [MAIN_AGENT_RUN_PANEL_ID];
    while (state.slots.length < MAX_AGENT_RUN_PANELS) {
      const nextFrontier: string[] = [];
      for (const target of frontier) {
        if (state.slots.length >= MAX_AGENT_RUN_PANELS) break;
        const opened = openAdjacentPanel(state, target, "right");
        expect(opened.opened).toBe(true);
        if (!opened.opened) throw new Error("expected the split to succeed");
        state = opened.state;
        nextFrontier.push(target, opened.panelId);
      }
      frontier = nextFrontier;
    }

    expect(state.slots).toHaveLength(MAX_AGENT_RUN_PANELS);
    expect(getTileDepth(state.layout)).toBeLessThanOrEqual(MAX_AGENT_RUN_TILE_DEPTH);

    const rejected = openAdjacentPanel(state, MAIN_AGENT_RUN_PANEL_ID, "right");

    expect(rejected.opened).toBe(false);
    if (rejected.opened) throw new Error("expected the ninth panel to be refused");
    expect(rejected.reason).toBe("panel-limit");
    expect(rejected.state).toBe(state);

    // Promotion uses the same bound and must not mutate the workspace either.
    const promoted = promoteOrchestrationNode(state, {
      id: "child-extra",
      title: "Reviewer",
      runId: "run-child-extra",
      isRunning: true,
    });
    expect(promoted).toBe(state);
    expect(promoted.slots).toHaveLength(MAX_AGENT_RUN_PANELS);
  });

  it("refuses a split that would exceed the 4-level tile depth", () => {
    let state = createInitialAgentRunWorkspaceState();
    let target = MAIN_AGENT_RUN_PANEL_ID;
    let rejection: ReturnType<typeof openAdjacentPanel> | null = null;

    // Always splitting the newest leaf deepens the tree, so depth is hit before the panel cap.
    for (let attempt = 0; attempt < MAX_AGENT_RUN_PANELS; attempt += 1) {
      const opened = openAdjacentPanel(state, target, "below");
      if (!opened.opened) {
        rejection = opened;
        break;
      }
      state = opened.state;
      target = opened.panelId;
    }

    expect(rejection).not.toBeNull();
    if (!rejection || rejection.opened) throw new Error("expected a depth rejection");
    expect(rejection.reason).toBe("depth-limit");
    expect(rejection.state).toBe(state);
    expect(getTileDepth(state.layout)).toBeLessThanOrEqual(MAX_AGENT_RUN_TILE_DEPTH);
    expect(state.slots.length).toBeLessThan(MAX_AGENT_RUN_PANELS);
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

  it("separates visible panel membership from a running orchestration node", () => {
    const promoted = promoteOrchestrationNode(createInitialAgentRunWorkspaceState(), {
      id: "worker-1",
      title: "Researcher",
      runId: "run-worker-1",
      isRunning: true,
    });
    const detached = detachOrchestrationPanel(promoted, "worker-1");

    expect(promoted.slots[1]).toMatchObject({
      id: "worker-1",
      activeRunId: "run-worker-1",
      isRunning: true,
    });
    expect(detached.slots.map((slot) => slot.id)).toEqual([
      MAIN_AGENT_RUN_PANEL_ID,
    ]);
  });
});
