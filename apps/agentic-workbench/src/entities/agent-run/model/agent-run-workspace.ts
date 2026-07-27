import type { AgentPromptDelivery } from "./agent-exchange";
import {
  createEqualHorizontalTileLayout,
  createTileLeaf,
  flattenTilePanelIds,
  getTileDepth,
  removeTileLeaf,
  resizeTileSplit,
  splitTileLeaf,
  type TileLayoutNode,
  type TilePlacement,
} from "./tile-layout";

export const MAIN_AGENT_RUN_PANEL_ID = "main-agent-run";
export const MAX_AGENT_RUN_PANELS = 8;
export const MAX_AGENT_RUN_TILE_DEPTH = 4;

export type AgentRunViewMode = "tabs" | "tiles";
export type AgentRunPanelKind = "main" | "extra";
export type AgentRunPanelCloseState = "open" | "confirmingClose" | "closing";

export type AgentPromptRequest = {
  id: string;
  text: string;
  delivery?: AgentPromptDelivery;
};

export type AgentRunPanelSlot = {
  id: string;
  kind: AgentRunPanelKind;
  title: string;
  externalPromptRequest: AgentPromptRequest | null;
  isRunning: boolean;
  activeRunId: string | null;
  closeState: AgentRunPanelCloseState;
  pendingExchangeCount: number;
};

export type AgentPanelRunState = {
  panelId: string;
  isRunning: boolean;
  activeRunId: string | null;
};

export type AgentRunWorkspaceState = {
  slots: AgentRunPanelSlot[];
  focusedPanelId: string;
  /** Compatibility alias for existing tab-oriented consumers. */
  activePanelId: string;
  nextExtraSequence: number;
  nextSplitSequence: number;
  viewMode: AgentRunViewMode;
  layout: TileLayoutNode;
};

export type RoutedPromptResult =
  | {
      routed: true;
      state: AgentRunWorkspaceState;
      target: { id: string; title: string };
    }
  | {
      routed: false;
      state: AgentRunWorkspaceState;
      reason: "empty" | "missing-target" | "closing-target";
    };

export type OpenAdjacentResult =
  | { opened: true; state: AgentRunWorkspaceState; panelId: string }
  | {
      opened: false;
      state: AgentRunWorkspaceState;
      reason: "missing-target" | "panel-limit" | "depth-limit";
    };

function createSlot(
  kind: AgentRunPanelKind,
  id: string,
  title: string,
): AgentRunPanelSlot {
  return {
    id,
    kind,
    title,
    externalPromptRequest: null,
    isRunning: false,
    activeRunId: null,
    closeState: "open",
    pendingExchangeCount: 0,
  };
}

function withFocus(
  state: AgentRunWorkspaceState,
  panelId: string,
): AgentRunWorkspaceState {
  return {
    ...state,
    focusedPanelId: panelId,
    activePanelId: panelId,
  };
}

export function createInitialAgentRunWorkspaceState(): AgentRunWorkspaceState {
  return {
    slots: [createSlot("main", MAIN_AGENT_RUN_PANEL_ID, "Main")],
    focusedPanelId: MAIN_AGENT_RUN_PANEL_ID,
    activePanelId: MAIN_AGENT_RUN_PANEL_ID,
    nextExtraSequence: 1,
    nextSplitSequence: 1,
    viewMode: "tabs",
    layout: createTileLeaf(MAIN_AGENT_RUN_PANEL_ID),
  };
}

export const createInitialAgentRunAreaState = createInitialAgentRunWorkspaceState;
export type WorktreeAgentRunAreaState = AgentRunWorkspaceState;

export function setAgentRunViewMode(
  state: AgentRunWorkspaceState,
  viewMode: AgentRunViewMode,
): AgentRunWorkspaceState {
  if (state.viewMode === viewMode) {
    return state;
  }
  if (viewMode === "tabs") {
    return { ...state, viewMode };
  }

  let nextSplitSequence = state.nextSplitSequence;
  const layout = createEqualHorizontalTileLayout(
    state.slots.map((slot) => slot.id),
    () => `agent-run-split-${nextSplitSequence++}`,
  );
  return {
    ...state,
    viewMode,
    layout,
    nextSplitSequence,
  };
}

export function selectPanel(
  state: AgentRunWorkspaceState,
  panelId: string,
): AgentRunWorkspaceState {
  if (
    state.focusedPanelId === panelId ||
    !state.slots.some((slot) => slot.id === panelId)
  ) {
    return state;
  }
  return withFocus(state, panelId);
}

export function openAdjacentPanel(
  state: AgentRunWorkspaceState,
  targetPanelId: string,
  placement: TilePlacement,
  limits: { maxPanels?: number; maxDepth?: number } = {},
): OpenAdjacentResult {
  if (!state.slots.some((slot) => slot.id === targetPanelId)) {
    return { opened: false, state, reason: "missing-target" };
  }
  if (state.slots.length >= (limits.maxPanels ?? MAX_AGENT_RUN_PANELS)) {
    return { opened: false, state, reason: "panel-limit" };
  }

  const sequence = state.nextExtraSequence;
  const panelId = `extra-agent-run-${sequence}`;
  const split = splitTileLeaf(
    state.layout,
    targetPanelId,
    panelId,
    placement,
    `agent-run-split-${state.nextSplitSequence}`,
  );
  if (!split.changed) {
    return { opened: false, state, reason: "missing-target" };
  }
  if (getTileDepth(split.layout) > (limits.maxDepth ?? MAX_AGENT_RUN_TILE_DEPTH)) {
    return { opened: false, state, reason: "depth-limit" };
  }

  return {
    opened: true,
    panelId,
    state: {
      ...state,
      slots: [...state.slots, createSlot("extra", panelId, `Extra ${sequence}`)],
      focusedPanelId: panelId,
      activePanelId: panelId,
      nextExtraSequence: sequence + 1,
      nextSplitSequence: state.nextSplitSequence + 1,
      viewMode: "tiles",
      layout: split.layout,
    },
  };
}

export function addExtraPanel(state: AgentRunWorkspaceState): AgentRunWorkspaceState {
  const result = openAdjacentPanel(state, state.focusedPanelId, "right");
  return result.opened
    ? { ...result.state, viewMode: state.viewMode }
    : state;
}

export function resizeAgentRunSplit(
  state: AgentRunWorkspaceState,
  splitId: string,
  ratio: number,
): AgentRunWorkspaceState {
  const layout = resizeTileSplit(state.layout, splitId, ratio);
  return layout === state.layout ? state : { ...state, layout };
}

export function updatePanelRunState(
  state: AgentRunWorkspaceState,
  report: AgentPanelRunState,
): AgentRunWorkspaceState {
  const target = state.slots.find((slot) => slot.id === report.panelId);
  if (!target) {
    return state;
  }
  const nextActiveRunId = report.isRunning ? report.activeRunId : null;
  if (
    target.isRunning === report.isRunning &&
    target.activeRunId === nextActiveRunId
  ) {
    return state;
  }
  return {
    ...state,
    slots: state.slots.map((slot) =>
      slot.id === report.panelId
        ? { ...slot, isRunning: report.isRunning, activeRunId: nextActiveRunId }
        : slot,
    ),
  };
}

export function routePromptToPanel(
  state: AgentRunWorkspaceState,
  panelId: string,
  request: AgentPromptRequest,
): RoutedPromptResult {
  const text = request.text.trim();
  if (!text) {
    return { routed: false, state, reason: "empty" };
  }
  const target = state.slots.find((slot) => slot.id === panelId);
  if (!target) {
    return { routed: false, state, reason: "missing-target" };
  }
  if (target.closeState !== "open") {
    return { routed: false, state, reason: "closing-target" };
  }
  const normalized = { ...request, text };
  return {
    routed: true,
    state: {
      ...state,
      slots: state.slots.map((slot) =>
        slot.id === panelId ? { ...slot, externalPromptRequest: normalized } : slot,
      ),
    },
    target: { id: target.id, title: target.title },
  };
}

export function routePromptToActivePanel(
  state: AgentRunWorkspaceState,
  text: string,
  requestId: string,
  delivery?: AgentPromptDelivery,
): RoutedPromptResult {
  return routePromptToPanel(state, state.focusedPanelId, {
    id: requestId,
    text,
    delivery,
  });
}

export function closePanel(
  state: AgentRunWorkspaceState,
  panelId: string,
): AgentRunWorkspaceState {
  const slot = state.slots.find((item) => item.id === panelId);
  if (!slot || slot.kind !== "extra") {
    return state;
  }
  const removal = removeTileLeaf(state.layout, panelId);
  if (!removal.changed) {
    return state;
  }
  const slots = state.slots.filter((item) => item.id !== panelId);
  const fallback =
    removal.focusFallbackPanelId ??
    flattenTilePanelIds(removal.layout)[0] ??
    MAIN_AGENT_RUN_PANEL_ID;
  return {
    ...state,
    slots,
    layout: removal.layout,
    focusedPanelId: state.focusedPanelId === panelId ? fallback : state.focusedPanelId,
    activePanelId: state.activePanelId === panelId ? fallback : state.activePanelId,
  };
}

export function requestClosePanel(
  state: AgentRunWorkspaceState,
  panelId: string,
): AgentRunWorkspaceState {
  const slot = state.slots.find(
    (item) => item.id === panelId && item.kind === "extra",
  );
  if (!slot || slot.closeState !== "open") {
    return state;
  }
  if (!slot.isRunning || !slot.activeRunId) {
    return closePanel(state, panelId);
  }
  return setSlotCloseState(state, panelId, "confirmingClose");
}

export function cancelClosePanel(
  state: AgentRunWorkspaceState,
  panelId: string,
): AgentRunWorkspaceState {
  const slot = state.slots.find(
    (item) => item.id === panelId && item.kind === "extra",
  );
  return slot?.closeState === "confirmingClose"
    ? setSlotCloseState(state, panelId, "open")
    : state;
}

export function confirmClosePanel(
  state: AgentRunWorkspaceState,
  panelId: string,
): { state: AgentRunWorkspaceState; activeRunId: string | null } {
  const slot = state.slots.find(
    (item) => item.id === panelId && item.kind === "extra",
  );
  if (!slot || slot.closeState === "closing") {
    return { state, activeRunId: null };
  }
  if (!slot.isRunning || !slot.activeRunId) {
    return { state: closePanel(state, panelId), activeRunId: null };
  }
  return {
    state: setSlotCloseState(state, panelId, "closing"),
    activeRunId: slot.activeRunId,
  };
}

export const removeClosedPanel = closePanel;

export function getRunningPanelCount(state: AgentRunWorkspaceState): number {
  return state.slots.filter((slot) => slot.isRunning).length;
}

function setSlotCloseState(
  state: AgentRunWorkspaceState,
  panelId: string,
  closeState: AgentRunPanelCloseState,
): AgentRunWorkspaceState {
  return {
    ...state,
    slots: state.slots.map((slot) =>
      slot.id === panelId ? { ...slot, closeState } : slot,
    ),
  };
}
