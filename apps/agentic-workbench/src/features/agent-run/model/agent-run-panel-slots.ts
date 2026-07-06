export const MAIN_AGENT_RUN_PANEL_ID = "main-agent-run";

export type AgentRunPanelKind = "main" | "extra";
export type AgentRunPanelCloseState = "open" | "confirmingClose" | "closing";

export type AgentPromptRequest = {
  id: string;
  text: string;
};

export type AgentRunPanelSlot = {
  id: string;
  kind: AgentRunPanelKind;
  title: string;
  externalPromptRequest: AgentPromptRequest | null;
  isRunning: boolean;
  activeRunId: string | null;
  closeState: AgentRunPanelCloseState;
};

export type AgentPanelRunState = {
  panelId: string;
  isRunning: boolean;
  activeRunId: string | null;
};

export type WorktreeAgentRunAreaState = {
  slots: AgentRunPanelSlot[];
  activePanelId: string;
  nextExtraSequence: number;
};

export type RoutedPromptResult =
  | {
      routed: true;
      state: WorktreeAgentRunAreaState;
      target: { id: string; title: string };
    }
  | {
      routed: false;
      state: WorktreeAgentRunAreaState;
      reason: "empty" | "missing-target" | "closing-target";
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
  };
}

export function createInitialAgentRunAreaState(): WorktreeAgentRunAreaState {
  return {
    slots: [createSlot("main", MAIN_AGENT_RUN_PANEL_ID, "Main")],
    activePanelId: MAIN_AGENT_RUN_PANEL_ID,
    nextExtraSequence: 1,
  };
}

export function addExtraPanel(
  state: WorktreeAgentRunAreaState,
): WorktreeAgentRunAreaState {
  const sequence = state.nextExtraSequence;
  const nextSlot = createSlot("extra", `extra-agent-run-${sequence}`, `Extra ${sequence}`);
  return {
    ...state,
    slots: [...state.slots, nextSlot],
    activePanelId: nextSlot.id,
    nextExtraSequence: sequence + 1,
  };
}

export function selectPanel(
  state: WorktreeAgentRunAreaState,
  panelId: string,
): WorktreeAgentRunAreaState {
  if (
    state.activePanelId === panelId ||
    !state.slots.some((slot) => slot.id === panelId)
  ) {
    return state;
  }
  return { ...state, activePanelId: panelId };
}

export function updatePanelRunState(
  state: WorktreeAgentRunAreaState,
  report: AgentPanelRunState,
): WorktreeAgentRunAreaState {
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
        ? {
            ...slot,
            isRunning: report.isRunning,
            activeRunId: nextActiveRunId,
          }
        : slot,
    ),
  };
}

export function routePromptToActivePanel(
  state: WorktreeAgentRunAreaState,
  text: string,
  requestId: string,
): RoutedPromptResult {
  const nextText = text.trim();
  if (!nextText) {
    return { routed: false, state, reason: "empty" };
  }

  const target = state.slots.find((slot) => slot.id === state.activePanelId);
  if (!target) {
    return { routed: false, state, reason: "missing-target" };
  }
  if (target.closeState !== "open") {
    return { routed: false, state, reason: "closing-target" };
  }

  const request: AgentPromptRequest = {
    id: requestId,
    text: nextText,
  };
  const nextState = {
    ...state,
    slots: state.slots.map((slot) =>
      slot.id === target.id ? { ...slot, externalPromptRequest: request } : slot,
    ),
  };

  return {
    routed: true,
    state: nextState,
    target: { id: target.id, title: target.title },
  };
}

export function requestClosePanel(
  state: WorktreeAgentRunAreaState,
  panelId: string,
): WorktreeAgentRunAreaState {
  const slot = findExtraSlot(state, panelId);
  if (!slot || slot.closeState !== "open") {
    return state;
  }
  if (!slot.isRunning || !slot.activeRunId) {
    return removeClosedPanel(state, panelId);
  }
  return setSlotCloseState(state, panelId, "confirmingClose");
}

export function cancelClosePanel(
  state: WorktreeAgentRunAreaState,
  panelId: string,
): WorktreeAgentRunAreaState {
  const slot = findExtraSlot(state, panelId);
  if (!slot || slot.closeState !== "confirmingClose") {
    return state;
  }
  return setSlotCloseState(state, panelId, "open");
}

export function confirmClosePanel(
  state: WorktreeAgentRunAreaState,
  panelId: string,
): { state: WorktreeAgentRunAreaState; activeRunId: string | null } {
  const slot = findExtraSlot(state, panelId);
  if (!slot || slot.closeState === "closing") {
    return { state, activeRunId: null };
  }
  if (!slot.isRunning || !slot.activeRunId) {
    return { state: removeClosedPanel(state, panelId), activeRunId: null };
  }
  return {
    activeRunId: slot.activeRunId,
    state: setSlotCloseState(state, panelId, "closing"),
  };
}

export function removeClosedPanel(
  state: WorktreeAgentRunAreaState,
  panelId: string,
): WorktreeAgentRunAreaState {
  const slot = findExtraSlot(state, panelId);
  if (!slot) {
    return state;
  }

  const remainingSlots = state.slots.filter((item) => item.id !== panelId);
  const activePanelId =
    state.activePanelId === panelId
      ? (remainingSlots.find((item) => item.kind === "extra")?.id ??
        MAIN_AGENT_RUN_PANEL_ID)
      : state.activePanelId;

  return {
    ...state,
    slots: remainingSlots,
    activePanelId,
  };
}

export function getRunningPanelCount(state: WorktreeAgentRunAreaState): number {
  return state.slots.filter((slot) => slot.isRunning).length;
}

function findExtraSlot(
  state: WorktreeAgentRunAreaState,
  panelId: string,
): AgentRunPanelSlot | null {
  const slot = state.slots.find((item) => item.id === panelId);
  return slot && slot.kind === "extra" ? slot : null;
}

function setSlotCloseState(
  state: WorktreeAgentRunAreaState,
  panelId: string,
  closeState: AgentRunPanelCloseState,
): WorktreeAgentRunAreaState {
  return {
    ...state,
    slots: state.slots.map((slot) =>
      slot.id === panelId ? { ...slot, closeState } : slot,
    ),
  };
}
