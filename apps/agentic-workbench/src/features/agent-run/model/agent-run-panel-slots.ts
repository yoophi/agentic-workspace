export const MAIN_AGENT_RUN_PANEL_ID = "main-agent-run";

export type AgentRunPanelKind = "main" | "extra";
export type AgentRunPanelCloseState = "open" | "confirmingClose" | "closing";

export type AgentRunPanelSlot = {
  id: string;
  kind: AgentRunPanelKind;
  title: string;
  externalPromptRequest: AgentPanelPromptRequest | null;
  isRunning: boolean;
  activeRunId: string | null;
  closeState: AgentRunPanelCloseState;
};

export type AgentPanelPromptRequest = {
  id: string;
  text: string;
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
  lastPromptTarget: { panelId: string; title: string } | null;
};

export type RoutedPromptResult =
  | {
      routed: true;
      state: WorktreeAgentRunAreaState;
      target: AgentRunPanelSlot;
    }
  | {
      routed: false;
      state: WorktreeAgentRunAreaState;
      reason: "empty" | "missing-target" | "closing-target";
    };

export function createMainAgentRunSlot(): AgentRunPanelSlot {
  return {
    id: MAIN_AGENT_RUN_PANEL_ID,
    kind: "main",
    title: "Main",
    externalPromptRequest: null,
    isRunning: false,
    activeRunId: null,
    closeState: "open",
  };
}

export function createInitialAgentRunAreaState(): WorktreeAgentRunAreaState {
  return {
    slots: [createMainAgentRunSlot()],
    activePanelId: MAIN_AGENT_RUN_PANEL_ID,
    nextExtraSequence: 1,
    lastPromptTarget: null,
  };
}

export function createExtraPanelSlot(sequence: number): AgentRunPanelSlot {
  return {
    id: `extra-agent-run-${sequence}`,
    kind: "extra",
    title: `Extra ${sequence}`,
    externalPromptRequest: null,
    isRunning: false,
    activeRunId: null,
    closeState: "open",
  };
}

export function addExtraPanel(
  state: WorktreeAgentRunAreaState,
): WorktreeAgentRunAreaState {
  const nextSlot = createExtraPanelSlot(state.nextExtraSequence);
  return {
    ...state,
    slots: [...state.slots, nextSlot],
    activePanelId: nextSlot.id,
    nextExtraSequence: state.nextExtraSequence + 1,
  };
}

export function selectPanel(
  state: WorktreeAgentRunAreaState,
  panelId: string,
): WorktreeAgentRunAreaState {
  if (!state.slots.some((slot) => slot.id === panelId)) {
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
  createId: () => string,
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

  const request: AgentPanelPromptRequest = {
    id: createId(),
    text: nextText,
  };
  const nextState = {
    ...state,
    lastPromptTarget: { panelId: target.id, title: target.title },
    slots: state.slots.map((slot) =>
      slot.id === target.id ? { ...slot, externalPromptRequest: request } : slot,
    ),
  };

  return {
    routed: true,
    state: nextState,
    target: { ...target, externalPromptRequest: request },
  };
}

export function requestClosePanel(
  state: WorktreeAgentRunAreaState,
  panelId: string,
): WorktreeAgentRunAreaState {
  const slot = state.slots.find((item) => item.id === panelId);
  if (!slot || slot.kind === "main" || slot.closeState !== "open") {
    return state;
  }
  if (!slot.isRunning || !slot.activeRunId) {
    return removePanel(state, panelId);
  }
  return {
    ...state,
    slots: state.slots.map((item) =>
      item.id === panelId ? { ...item, closeState: "confirmingClose" } : item,
    ),
  };
}

export function cancelClosePanel(
  state: WorktreeAgentRunAreaState,
  panelId: string,
): WorktreeAgentRunAreaState {
  return {
    ...state,
    slots: state.slots.map((slot) =>
      slot.id === panelId && slot.kind === "extra" && slot.closeState === "confirmingClose"
        ? { ...slot, closeState: "open" }
        : slot,
    ),
  };
}

export function confirmClosePanel(
  state: WorktreeAgentRunAreaState,
  panelId: string,
): { state: WorktreeAgentRunAreaState; activeRunId: string | null } {
  const slot = state.slots.find((item) => item.id === panelId);
  if (!slot || slot.kind === "main") {
    return { state, activeRunId: null };
  }
  if (slot.closeState === "closing") {
    return { state, activeRunId: null };
  }
  if (!slot.isRunning || !slot.activeRunId) {
    return { state: removePanel(state, panelId), activeRunId: null };
  }

  return {
    activeRunId: slot.activeRunId,
    state: {
      ...state,
      slots: state.slots.map((item) =>
        item.id === panelId ? { ...item, closeState: "closing" } : item,
      ),
    },
  };
}

export function removeClosedPanel(
  state: WorktreeAgentRunAreaState,
  panelId: string,
): WorktreeAgentRunAreaState {
  const slot = state.slots.find((item) => item.id === panelId);
  if (!slot || slot.kind === "main") {
    return state;
  }
  return removePanel(state, panelId);
}

export function getRunningPanelCount(state: WorktreeAgentRunAreaState): number {
  return state.slots.filter((slot) => slot.isRunning).length;
}

function removePanel(
  state: WorktreeAgentRunAreaState,
  panelId: string,
): WorktreeAgentRunAreaState {
  const slot = state.slots.find((item) => item.id === panelId);
  if (!slot || slot.kind === "main") {
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
